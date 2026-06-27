"""US4 multi-message composition tests for the PyO3 binding (`prompting_press`) — spec 004, T019.

US4 lands the ordered-composition surface (FR-012 / FR-013): an explicit, ordered sequence of
`(prompt-name, vars, variant)` entries that resolves to a `list[Message]` in append order. There
is **no** fluent `.chain()` (FR-013) — composition is built with `Composition()` + `.append(...)`
or the `Composition.from_messages([...])` staticmethod.

What these tests pin (all Python-observable, none re-verifying cross-language render parity, which
Principle I makes structural):

- order + roles (SC-008): N entries resolve to exactly N `Message` in input order, each `.role`
  matching its prompt definition's role and `.text` rendered with that entry's own vars — proven
  on BOTH construction paths (`append` and `from_messages`).
- one bad entry → no partial (FR-013 / SC-008): (a) vars that fail Pydantic validation raise
  `PromptValidationError` at append/from_messages with NOTHING stored; (b) an unknown prompt name
  surfaces at `resolve` as `UnknownPromptError` and NO partial list is returned.
- empty composition → `[]`.
- no `.chain()` on the class or an instance (FR-013).
- `from_messages` variant arg: a 3-tuple `(name, vars, variant)` selects that variant; a 2-tuple
  defaults to the reserved `default` arm; an unknown variant fails at `resolve` as `PromptRenderError`.

Observed Composition / Message API (inspected from the built extension before asserting):
- `Composition()` — empty builder. `len(comp)` is the stored-entry count.
- `comp.append(name, vars, variant=None) -> None` — eager-validates the vars **instance** via the
  US1 Python-validation path (re-validated, so a `model_construct`-bypassed invalid instance is
  still caught), marshals it, and stores ONE entry. On `PromptValidationError` it stores nothing
  (len unchanged). `variant` accepts positional or keyword. The prompt `name` is NOT resolved here.
- `Composition.from_messages(entries)` — staticmethod over an iterable of 2- or 3-element tuples;
  validates each in order; the first invalid entry raises `PromptValidationError` and NO
  `Composition` is returned (no partial state).
- `comp.resolve(reg) -> list[Message]` — renders each entry IN ORDER through the kernel. Unknown
  name ⇒ `UnknownPromptError`; kernel rejection (unknown variant, strict-undefined, parse/render)
  ⇒ `PromptRenderError`; on any failure the partial result is discarded (never returned). An empty
  composition ⇒ `[]`.
- `Message` — frozen/read-only `.role: str` ("system"/"user"/"assistant", the prompt def's role)
  and `.text: str` (that prompt rendered with the entry's own validated vars). Produced only by
  `resolve`, never constructed from Python.

Construction note (mirrors test_render.py / test_loader.py): a prompt is built by validating a
plain dict through the generated `PromptDefinition`, then `Registry.insert`-ing its canonical JSON
dump (`mode="json"`, `exclude_none=True`) so absent optional fields are omitted for the kernel's
serde struct.
"""

from __future__ import annotations

from collections.abc import Mapping

import pytest
from pydantic import BaseModel, field_validator

import prompting_press
from prompting_press import (
    Composition,
    Message,
    PromptRenderError,
    PromptValidationError,
    Registry,
    UnknownPromptError,
)
from prompting_press.generated import PromptDefinition


# --------------------------------------------------------------------------------------
# Vars models (Pydantic — the per-language idiom; Principle VI).
# --------------------------------------------------------------------------------------


class Named(BaseModel):
    """A single `name` field whose validator rejects an empty string.

    The validator lets us build a genuinely-invalid instance via `model_construct`
    (which bypasses validation): handing that instance to `append` / `from_messages`
    forces the eager re-validation to fail, exercising the no-partial guarantee.
    """

    name: str

    @field_validator("name")
    @classmethod
    def _name_non_empty(cls, value: str) -> str:
        if not value:
            raise ValueError("name must be non-empty")
        return value


class Empty(BaseModel):
    """No declared variables — used for variable-free, role-carrying prompts."""


# --------------------------------------------------------------------------------------
# Registry helper + prompt definitions
# --------------------------------------------------------------------------------------


def _registry(*definitions: Mapping) -> Registry:
    """Validate each definition into a generated `PromptDefinition` and `insert` its
    canonical JSON dump (absent optional fields omitted for the kernel's serde struct)."""
    reg = Registry()
    for definition in definitions:
        model = PromptDefinition.model_validate(definition)
        reg.insert(model.model_dump(mode="json", exclude_none=True))
    return reg


# A `system`-role prompt with a variable-free body.
SYS_PREAMBLE = {
    "name": "sys_preamble",
    "role": "system",
    "body": "You are helpful.",
    "variables": {},
}

# A `user`-role prompt that interpolates its own `name` var.
GREET = {
    "name": "greet",
    "role": "user",
    "body": "Hi {{ name }}",
    "variables": {"name": {"type": "string", "provenance": "trusted"}},
}

# A second `user`-role prompt, for the mixed-roles ordering test.
FAREWELL = {
    "name": "farewell",
    "role": "user",
    "body": "Bye {{ name }}",
    "variables": {"name": {"type": "string", "provenance": "trusted"}},
}

# A prompt carrying a named variant `formal`; the root body is the default arm.
WITH_VARIANT = {
    "name": "salute",
    "role": "user",
    "body": "Hi {{ name }}",
    "variants": {"formal": {"body": "Good day, {{ name }}"}},
    "variables": {"name": {"type": "string", "provenance": "trusted"}},
}


# --------------------------------------------------------------------------------------
# 1. Order + roles (SC-008) — both construction paths
# --------------------------------------------------------------------------------------


def test_append_path_preserves_order_roles_and_per_entry_text() -> None:
    """Built via `Composition()` + `.append(...)`: N entries resolve to exactly N
    `Message` in append order, each `.role` matching its prompt def's role and `.text`
    rendered with that entry's own vars (SC-008)."""
    reg = _registry(SYS_PREAMBLE, GREET)

    comp = Composition()
    assert comp.append("sys_preamble", Empty()) is None, (
        "append is non-fluent (returns None)"
    )
    assert comp.append("greet", Named(name="Ada")) is None
    assert len(comp) == 2, "len reflects the two stored entries"

    messages = comp.resolve(reg)

    assert [type(m) for m in messages] == [Message, Message]
    assert len(messages) == 2, "exactly N messages, one per entry"
    # Order is append order; each role is the prompt def's role; each text is per-entry.
    assert messages[0].role == "system"
    assert messages[0].text == "You are helpful."
    assert messages[1].role == "user"
    assert messages[1].text == "Hi Ada"


def test_from_messages_path_preserves_order_roles_and_per_entry_text() -> None:
    """Same N-entry guarantee via the `from_messages([...])` staticmethod, proving the
    two construction paths agree (SC-008)."""
    reg = _registry(SYS_PREAMBLE, GREET)

    comp = Composition.from_messages(
        [
            ("sys_preamble", Empty()),
            ("greet", Named(name="Bo")),
        ]
    )
    assert isinstance(comp, Composition)
    assert len(comp) == 2

    messages = comp.resolve(reg)

    assert len(messages) == 2
    assert [m.role for m in messages] == ["system", "user"]
    assert [m.text for m in messages] == ["You are helpful.", "Hi Bo"]


def test_both_construction_paths_produce_identical_messages() -> None:
    """`append` and `from_messages` are two spellings of one builder: the same entries
    resolve to the same ordered (role, text) message sequence."""
    reg = _registry(SYS_PREAMBLE, GREET)
    entries = [("sys_preamble", Empty()), ("greet", Named(name="Cy"))]

    via_append = Composition()
    for name, vars_ in entries:
        via_append.append(name, vars_)
    via_factory = Composition.from_messages(entries)

    appended = [(m.role, m.text) for m in via_append.resolve(reg)]
    factoried = [(m.role, m.text) for m in via_factory.resolve(reg)]
    assert appended == factoried == [("system", "You are helpful."), ("user", "Hi Cy")]


# --------------------------------------------------------------------------------------
# 2. One invalid entry → no partial (FR-013 / SC-008)
# --------------------------------------------------------------------------------------


def test_invalid_vars_at_append_raises_and_stores_nothing() -> None:
    """(a) An entry whose Pydantic vars fail validation raises `PromptValidationError`
    at `append`, and NOTHING is stored — the composition's length is unchanged, so a
    later `resolve` never sees a half-validated entry (FR-013)."""
    reg = _registry(GREET)

    comp = Composition()
    comp.append("greet", Named(name="ok"))  # one good entry
    assert len(comp) == 1

    # `model_construct` bypasses the validator, yielding a structurally-invalid instance
    # (empty name). `append` eager-RE-validates via the US1 path, so this is rejected.
    invalid = Named.model_construct(name="")

    with pytest.raises(PromptValidationError) as excinfo:
        comp.append("greet", invalid)

    # Nothing stored from the failed append — length is exactly the prior good count.
    assert len(comp) == 1, "a rejected append must store nothing (no partial state)"
    rows = excinfo.value.errors
    assert any(r.field == "name" for r in rows), [r.field for r in rows]
    assert all(r.code == "validation" for r in rows if r.field == "name")

    # The composition still resolves cleanly to just the one good entry.
    messages = comp.resolve(reg)
    assert [m.text for m in messages] == ["Hi ok"]


def test_invalid_vars_in_from_messages_raises_and_yields_no_composition() -> None:
    """The first invalid entry in `from_messages` raises `PromptValidationError` and the
    whole construction fails — no `Composition` is returned (no partial state)."""
    reg = _registry(GREET)

    with pytest.raises(PromptValidationError):
        Composition.from_messages(
            [
                ("greet", Named(name="ok")),
                ("greet", Named.model_construct(name="")),  # invalid second entry
            ]
        )

    # A composition built from only the valid prefix is what a partial would have looked
    # like; the all-or-nothing factory does not hand one back. We can still build + resolve
    # a clean one to confirm the registry/path is otherwise healthy.
    good = Composition.from_messages([("greet", Named(name="ok"))])
    assert [m.text for m in good.resolve(reg)] == ["Hi ok"]


def test_unknown_prompt_name_at_resolve_raises_and_returns_no_partial() -> None:
    """(b) A composition where one entry's prompt NAME is unknown: `resolve` raises
    `UnknownPromptError` and returns NO partial list. The name is only resolved at
    `resolve`, so `append` accepts it without error."""
    reg = _registry(SYS_PREAMBLE)

    comp = Composition()
    comp.append("sys_preamble", Empty())  # valid + present
    comp.append("does_not_exist", Empty())  # name not in the registry
    assert len(comp) == 2, "append does not resolve the name; both entries are stored"

    with pytest.raises(UnknownPromptError):
        comp.resolve(reg)

    # No partial result leaked out as a return value — resolve raised instead of
    # returning the one successfully-rendered prefix. The sentinel proves the
    # assignment never executed (resolve raised before returning).
    sentinel: object = object()
    result: object = sentinel
    try:
        result = comp.resolve(reg)
    except UnknownPromptError:
        pass
    assert result is sentinel, (
        "resolve must RAISE (not return a partial list) when an entry's name is unknown"
    )


# --------------------------------------------------------------------------------------
# 3. Empty composition → []
# --------------------------------------------------------------------------------------


def test_empty_composition_resolves_to_empty_list() -> None:
    reg = _registry(SYS_PREAMBLE)
    empty = Composition()
    assert len(empty) == 0
    assert empty.resolve(reg) == []


# --------------------------------------------------------------------------------------
# 4. No .chain() (FR-013)
# --------------------------------------------------------------------------------------


def test_no_fluent_chain_on_class_or_instance() -> None:
    """FR-013: the builder is intentionally non-fluent; there is no `.chain()` — neither
    on the class nor on an instance."""
    assert not hasattr(Composition, "chain")
    assert not hasattr(Composition(), "chain")
    # And `append` returns None rather than self, so accidental chaining is impossible.
    assert Composition().append("greet", Named(name="x")) is None  # type: ignore[func-returns-value]


# --------------------------------------------------------------------------------------
# 5. from_messages / append variant arg
# --------------------------------------------------------------------------------------


def test_three_tuple_selects_named_variant() -> None:
    """A 3-tuple `(name, vars, variant)` selects that variant arm; the body rendered is
    the variant's, not the root default."""
    reg = _registry(WITH_VARIANT)

    via_factory = Composition.from_messages([("salute", Named(name="Di"), "formal")])
    assert [m.text for m in via_factory.resolve(reg)] == ["Good day, Di"]

    # The same variant selection via append (positional and keyword forms agree).
    via_append_pos = Composition()
    via_append_pos.append("salute", Named(name="Di"), "formal")
    via_append_kw = Composition()
    via_append_kw.append("salute", Named(name="Di"), variant="formal")
    assert via_append_pos.resolve(reg)[0].text == "Good day, Di"
    assert via_append_kw.resolve(reg)[0].text == "Good day, Di"


def test_two_tuple_defaults_to_the_default_arm() -> None:
    """A 2-tuple `(name, vars)` defaults `variant` to the reserved `default` arm (the
    prompt's root body), NOT the named variant."""
    reg = _registry(WITH_VARIANT)

    comp = Composition.from_messages([("salute", Named(name="Eli"))])
    # The root body is the default arm — "Hi {{ name }}", not the `formal` "Good day, ...".
    assert [m.text for m in comp.resolve(reg)] == ["Hi Eli"]


def test_unknown_variant_fails_at_resolve_as_render_error() -> None:
    """Selecting a variant that does not exist is a kernel rejection surfaced at
    `resolve` as `PromptRenderError` (the name validates at append; the variant is only
    resolved when rendering)."""
    reg = _registry(WITH_VARIANT)

    comp = Composition()
    comp.append("salute", Named(name="Fa"), variant="nonexistent")

    with pytest.raises(PromptRenderError):
        comp.resolve(reg)


# --------------------------------------------------------------------------------------
# 6. Mixed — system + two user entries resolve to 3 correctly-ordered messages
# --------------------------------------------------------------------------------------


def test_mixed_system_and_two_user_entries_resolve_in_order() -> None:
    """A 3-entry composition (one `system`, two `user`) resolves to exactly 3 role-tagged
    messages in append order, each rendered with its own vars (SC-008)."""
    reg = _registry(SYS_PREAMBLE, GREET, FAREWELL)

    comp = Composition.from_messages(
        [
            ("sys_preamble", Empty()),
            ("greet", Named(name="Ada")),
            ("farewell", Named(name="Bo")),
        ]
    )
    assert len(comp) == 3

    messages = comp.resolve(reg)

    assert len(messages) == 3
    assert [m.role for m in messages] == ["system", "user", "user"]
    assert [m.text for m in messages] == ["You are helpful.", "Hi Ada", "Bye Bo"]


# --------------------------------------------------------------------------------------
# 7. Surface smoke — Message is read-only, errors are in the binding's hierarchy
# --------------------------------------------------------------------------------------


def test_message_role_and_text_are_read_only() -> None:
    """`Message` is frozen — its `.role` / `.text` are produced by `resolve`, never set
    from Python."""
    reg = _registry(GREET)
    message = Composition.from_messages([("greet", Named(name="Gu"))]).resolve(reg)[0]

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
    assert issubclass(UnknownPromptError, prompting_press.PromptingPressError)
    assert issubclass(PromptRenderError, prompting_press.PromptingPressError)
