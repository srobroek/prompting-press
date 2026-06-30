"""Multi-message composition tests (spec 008 Phase 4 reshape) — T040.

Tests the `Composition` surface after the object-model reshape: entries are
`(Prompt, vars)` or `(Prompt, vars, variant)` tuples, not `(name, vars)` pairs.
`resolve()` takes no registry argument.

The tests verify the same behavioral guarantees as before the reshape:
- Order + roles (SC-008): entries resolve in append order with the correct role/text.
- One bad entry → no partial result (FR-013 / SC-008).
- No fluent `.chain()` (FR-013).
- Variant selection via 3-tuple.
- Empty composition → `[]`.
"""

from __future__ import annotations

import pytest
from pydantic import BaseModel, field_validator

import prompting_press
from prompting_press import (
    Composition,
    Message,
    Prompt,
    PromptRenderError,
    PromptValidationError,
)


# ─── Vars models ──────────────────────────────────────────────────────────────


class Named(BaseModel):
    name: str

    @field_validator("name")
    @classmethod
    def _name_non_empty(cls, value: str) -> str:
        if not value:
            raise ValueError("name must be non-empty")
        return value


class Empty(BaseModel):
    pass


# ─── Prompt fixtures ──────────────────────────────────────────────────────────

SYS_PREAMBLE = Prompt(
    {
        "name": "sys_preamble",
        "role": "system",
        "body": "You are helpful.",
        "variables": {},
    }
)

GREET = Prompt(
    {
        "name": "greet",
        "role": "user",
        "body": "Hi {{ name }}",
        "variables": {"name": {"type": "string", "trusted": True}},
    }
)

FAREWELL = Prompt(
    {
        "name": "farewell",
        "role": "user",
        "body": "Bye {{ name }}",
        "variables": {"name": {"type": "string", "trusted": True}},
    }
)

WITH_VARIANT = Prompt(
    {
        "name": "salute",
        "role": "user",
        "body": "Hi {{ name }}",
        "variants": {"formal": {"body": "Good day, {{ name }}"}},
        "variables": {"name": {"type": "string", "trusted": True}},
    }
)


# ─── 1. Order + roles (SC-008) — append path ──────────────────────────────────


def test_append_path_preserves_order_roles_and_per_entry_text() -> None:
    comp = Composition()
    assert comp.append(SYS_PREAMBLE, Empty()) is None, (
        "append is non-fluent (returns None)"
    )
    assert comp.append(GREET, Named(name="Ada")) is None
    assert len(comp) == 2

    messages = comp.resolve()

    assert [type(m) for m in messages] == [Message, Message]
    assert len(messages) == 2
    assert messages[0].role == "system"
    assert messages[0].text == "You are helpful."
    assert messages[1].role == "user"
    assert messages[1].text == "Hi Ada"


def test_from_messages_path_preserves_order_roles_and_per_entry_text() -> None:
    comp = Composition.from_messages(
        [
            (SYS_PREAMBLE, Empty()),
            (GREET, Named(name="Bo")),
        ]
    )
    assert isinstance(comp, Composition)
    assert len(comp) == 2

    messages = comp.resolve()
    assert len(messages) == 2
    assert [m.role for m in messages] == ["system", "user"]
    assert [m.text for m in messages] == ["You are helpful.", "Hi Bo"]


def test_both_construction_paths_produce_identical_messages() -> None:
    entries = [(SYS_PREAMBLE, Empty()), (GREET, Named(name="Cy"))]

    via_append = Composition()
    for p, v in entries:
        via_append.append(p, v)
    via_factory = Composition.from_messages(entries)

    appended = [(m.role, m.text) for m in via_append.resolve()]
    factoried = [(m.role, m.text) for m in via_factory.resolve()]
    assert appended == factoried == [("system", "You are helpful."), ("user", "Hi Cy")]


# ─── 2. One invalid entry → no partial (FR-013 / SC-008) ─────────────────────


def test_invalid_vars_at_append_raises_and_stores_nothing() -> None:
    comp = Composition()
    comp.append(GREET, Named(name="ok"))
    assert len(comp) == 1

    invalid = Named.model_construct(name="")
    with pytest.raises(PromptValidationError) as excinfo:
        comp.append(GREET, invalid)

    assert len(comp) == 1, "a rejected append must store nothing (no partial state)"
    rows = excinfo.value.errors
    assert any(r.field == "name" for r in rows), [r.field for r in rows]
    assert all(r.code == "validation" for r in rows if r.field == "name")

    messages = comp.resolve()
    assert [m.text for m in messages] == ["Hi ok"]


def test_invalid_vars_in_from_messages_raises_and_yields_no_composition() -> None:
    with pytest.raises(PromptValidationError):
        Composition.from_messages(
            [
                (GREET, Named(name="ok")),
                (GREET, Named.model_construct(name="")),
            ]
        )

    good = Composition.from_messages([(GREET, Named(name="ok"))])
    assert [m.text for m in good.resolve()] == ["Hi ok"]


# ─── 3. Empty composition → [] ───────────────────────────────────────────────


def test_empty_composition_resolves_to_empty_list() -> None:
    empty = Composition()
    assert len(empty) == 0
    assert empty.resolve() == []


# ─── 4. No .chain() (FR-013) ─────────────────────────────────────────────────


def test_no_fluent_chain_on_class_or_instance() -> None:
    assert not hasattr(Composition, "chain")
    assert not hasattr(Composition(), "chain")
    assert Composition().append(GREET, Named(name="x")) is None  # type: ignore[func-returns-value]


# ─── 5. from_messages / append variant arg ────────────────────────────────────


def test_three_tuple_selects_named_variant() -> None:
    via_factory = Composition.from_messages(
        [(WITH_VARIANT, Named(name="Di"), "formal")]
    )
    assert [m.text for m in via_factory.resolve()] == ["Good day, Di"]

    via_append_kw = Composition()
    via_append_kw.append(WITH_VARIANT, Named(name="Di"), variant="formal")
    assert via_append_kw.resolve()[0].text == "Good day, Di"


def test_two_tuple_defaults_to_the_default_arm() -> None:
    comp = Composition.from_messages([(WITH_VARIANT, Named(name="Eli"))])
    assert [m.text for m in comp.resolve()] == ["Hi Eli"]


def test_unknown_variant_fails_at_resolve_as_render_error() -> None:
    comp = Composition()
    comp.append(WITH_VARIANT, Named(name="Fa"), variant="nonexistent")
    with pytest.raises(PromptRenderError):
        comp.resolve()


# ─── 6. Mixed — system + two user entries resolve in order ───────────────────


def test_mixed_system_and_two_user_entries_resolve_in_order() -> None:
    comp = Composition.from_messages(
        [
            (SYS_PREAMBLE, Empty()),
            (GREET, Named(name="Ada")),
            (FAREWELL, Named(name="Bo")),
        ]
    )
    assert len(comp) == 3

    messages = comp.resolve()
    assert len(messages) == 3
    assert [m.role for m in messages] == ["system", "user", "user"]
    assert [m.text for m in messages] == ["You are helpful.", "Hi Ada", "Bye Bo"]


# ─── 7. Surface smoke — Message is read-only, errors are in the binding ──────


def test_message_role_and_text_are_read_only() -> None:
    message = Composition.from_messages([(GREET, Named(name="Gu"))]).resolve()[0]
    assert message.role == "user"
    assert message.text == "Hi Gu"
    with pytest.raises((AttributeError, TypeError)):
        message.text = "tampered"  # type: ignore[misc]


def test_module_exposes_us4_composition_surface() -> None:
    assert hasattr(prompting_press, "Composition")
    assert hasattr(prompting_press, "Message")
    assert hasattr(prompting_press.Composition, "append")
    assert hasattr(prompting_press.Composition, "from_messages")
    assert hasattr(prompting_press.Composition, "resolve")
    assert issubclass(PromptRenderError, prompting_press.PromptingPressError)
