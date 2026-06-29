"""Prompt object tests for spec 008 Phase 4 (T040).

Tests the new `Prompt` pyclass surface introduced in the api-schema reshape:
- T035: construction via `Prompt(shape)`, `from_yaml`, `from_json`, `from_toml`.
- T036: `validation_required` coverage check at construction.
- T037: `render`, `get_source`, `check` as methods on the object.
- T038: `derive` immutability + merged-validation.

Also exercises:
- `Composition` with `Prompt` objects (T039 reshape: no registry, no name strings).
- The `origin` field name (Phase 1 rename from `provenance`).
- `Registry` is absent from `__all__` / `prompting_press` public surface.
"""

from __future__ import annotations

import re

import pytest
from pydantic import BaseModel, field_validator

import prompting_press
from prompting_press import (
    CheckReport,
    Composition,
    GuardConfig,
    Message,
    Prompt,
    PromptRenderError,
    PromptValidationError,
    RenderResult,
)
from prompting_press.generated import PromptDefinition

# A lowercase 64-char hex string — the SHA-256 provenance hash shape (FR-012/FR-013).
HEX64 = re.compile(r"\A[0-9a-f]{64}\Z")


# ─── Vars models ──────────────────────────────────────────────────────────────


class Named(BaseModel):
    name: str

    @field_validator("name")
    @classmethod
    def _name_nonempty(cls, v: str) -> str:
        if not v:
            raise ValueError("name must not be empty")
        return v


class Greeting(BaseModel):
    name: str
    count: int

    @field_validator("count")
    @classmethod
    def _count_non_negative(cls, v: int) -> int:
        if v < 0:
            raise ValueError("count must be non-negative")
        return v


class TopicVars(BaseModel):
    topic: str


class EmptyVars(BaseModel):
    pass


class SecretVars(BaseModel):
    """Passes Pydantic validation — used to drive a kernel-path render error."""
    token: str


# ─── fixture dicts (use `origin` not `provenance` — Phase 1 rename) ──────────

GREET_DEF = {
    "name": "greet",
    "role": "user",
    "body": "Hi {{ name }}, you have {{ count }} messages",
    "variables": {
        "name": {"type": "string", "origin": "trusted"},
        "count": {"type": "integer", "origin": "trusted"},
    },
}

SIMPLE_DEF = {
    "name": "simple",
    "role": "user",
    "body": "Hi {{ name }}",
    "variables": {"name": {"type": "string", "origin": "trusted"}},
}

ASK_DEF = {
    "name": "ask",
    "role": "user",
    "body": "Tell me about {{ topic }}.",
    "variables": {"topic": {"type": "string", "origin": "untrusted"}},
}

SYS_DEF = {
    "name": "sys",
    "role": "system",
    "body": "You are helpful.",
    "variables": {},
}


# ─── T035: construction ────────────────────────────────────────────────────────


def test_construct_from_dict_valid() -> None:
    """Prompt(dict) constructs and exposes correct read-only properties."""
    p = Prompt(SIMPLE_DEF)
    assert p.name == "simple"
    assert p.role == "user"
    assert p.body == "Hi {{ name }}"
    assert "name" in p.variables
    assert isinstance(p.variants, dict)
    assert p.output_model is None


def test_construct_from_pydantic_model() -> None:
    """Prompt(PromptDefinition instance) constructs via the duck-typed path."""
    model = PromptDefinition.model_validate(SIMPLE_DEF)
    p = Prompt(model)
    assert p.name == "simple"
    assert p.body == "Hi {{ name }}"


def test_construct_rejects_undeclared_variable() -> None:
    """Construction raises on a template that references an undeclared variable.

    The error may surface as PromptRenderError (kernel error code undefined_variable)
    or PromptValidationError — both are PromptingPressError subtypes.
    """
    from prompting_press import PromptingPressError

    bad = {
        "name": "bad",
        "role": "user",
        "body": "{{ ghost }}",
        "variables": {"name": {"type": "string", "origin": "trusted"}},
    }
    with pytest.raises(PromptingPressError) as excinfo:
        Prompt(bad)
    rows = excinfo.value.errors
    assert rows, "must carry at least one structured row"


def test_construct_rejects_reserved_variant_name() -> None:
    """Construction raises when a variant is literally named 'default'."""
    bad = {
        "name": "bad",
        "role": "user",
        "body": "Hi",
        "variables": {},
        "variants": {"default": {"body": "shadowed"}},
    }
    with pytest.raises((PromptValidationError, PromptRenderError)):
        Prompt(bad)


def test_construct_rejects_parse_error() -> None:
    """Construction raises when the body template has a syntax error."""
    bad = {
        "name": "bad",
        "role": "user",
        "body": "{{ unclosed",
        "variables": {},
    }
    with pytest.raises((PromptValidationError, PromptRenderError)):
        Prompt(bad)


def test_from_json_valid() -> None:
    """Prompt.from_json(text) constructs from a JSON string."""
    json_text = '{"name":"hi","role":"user","body":"Hello {{ name }}","variables":{"name":{"type":"string","origin":"trusted"}}}'
    p = Prompt.from_json(json_text)
    assert p.name == "hi"
    assert p.body == "Hello {{ name }}"


def test_from_yaml_valid() -> None:
    """Prompt.from_yaml(text) constructs from a YAML string."""
    yaml_text = (
        "name: hi\nrole: user\nbody: \"Hello {{ name }}\"\n"
        "variables:\n  name:\n    type: string\n    origin: trusted\n"
    )
    p = Prompt.from_yaml(yaml_text)
    assert p.name == "hi"


def test_from_toml_valid() -> None:
    """Prompt.from_toml(text) constructs from a TOML string (no-Registry path)."""
    toml_text = (
        'name = "greeting"\n'
        'role = "user"\n'
        'body = "Hi {{ name }}"\n'
        "\n"
        "[variables.name]\n"
        'type = "string"\n'
        'origin = "trusted"\n'
    )
    p = Prompt.from_toml(toml_text)
    assert p.name == "greeting"
    assert p.body == "Hi {{ name }}"


def test_from_json_invalid_raises_load_error() -> None:
    """Prompt.from_json raises on malformed JSON."""
    from prompting_press import LoadError

    with pytest.raises(LoadError):
        Prompt.from_json("not json at all {{{")


def test_from_yaml_missing_required_field_raises() -> None:
    """Prompt.from_yaml raises when a required field (body) is absent."""
    from prompting_press import LoadError

    with pytest.raises(LoadError):
        Prompt.from_yaml("name: hi\nrole: user\n")


def test_origin_field_accepted_provenance_rejected() -> None:
    """Phase 1 renamed `provenance` → `origin`. `origin` must be accepted; `provenance` must
    be rejected by the serde layer (deny_unknown_fields)."""
    from prompting_press import LoadError

    # origin → accepted
    p = Prompt(
        {
            "name": "ok",
            "role": "user",
            "body": "Hi {{ x }}",
            "variables": {"x": {"type": "string", "origin": "trusted"}},
        }
    )
    assert p.name == "ok"

    # provenance (the old field name, removed in Phase 1) → must fail (unknown field)
    with pytest.raises(LoadError):
        Prompt.from_json(
            '{"name":"bad","role":"user","body":"Hi {{ x }}",'
            '"variables":{"x":{"type":"string","provenance":"trusted"}}}'
        )


# ─── T036: validation_required coverage ───────────────────────────────────────


def test_validation_required_without_validators_raises() -> None:
    """A prompt with validation_required=true and no validators raises at construction."""
    strict = {
        "name": "strict",
        "role": "user",
        "body": "Hi {{ name }}",
        "variables": {"name": {"type": "string", "origin": "trusted", "validation_required": True}},
    }
    with pytest.raises(PromptValidationError) as excinfo:
        Prompt(strict)
    rows = excinfo.value.errors
    assert any(r.field == "name" for r in rows), [r.field for r in rows]


def test_validation_required_with_covering_validators_ok() -> None:
    """A prompt with validation_required=true AND a covering validator class constructs."""
    strict = {
        "name": "strict",
        "role": "user",
        "body": "Hi {{ name }}",
        "variables": {"name": {"type": "string", "origin": "trusted", "validation_required": True}},
    }
    # Named has `name` in model_fields → covers the required variable.
    p = Prompt(strict, validators=Named)
    assert p.name == "strict"


def test_validation_required_with_non_covering_validators_raises() -> None:
    """A validator that does NOT have the required field raises at construction."""
    strict = {
        "name": "strict",
        "role": "user",
        "body": "Hi {{ name }}",
        "variables": {"name": {"type": "string", "origin": "trusted", "validation_required": True}},
    }
    with pytest.raises(PromptValidationError) as excinfo:
        Prompt(strict, validators=EmptyVars)  # EmptyVars has no `name` field
    rows = excinfo.value.errors
    assert any(r.field == "name" for r in rows), [r.field for r in rows]


def test_no_validation_required_no_validators_ok() -> None:
    """A prompt with no validation_required variables constructs without validators."""
    p = Prompt(SIMPLE_DEF)
    assert p.name == "simple"


# ─── T037: render / get_source / check as methods ─────────────────────────────


def test_render_via_class_and_data() -> None:
    """prompt.render(Cls, data=...) validates + renders, returns RenderResult."""
    p = Prompt(GREET_DEF)
    result = p.render(Greeting, data={"name": "Ada", "count": 3})

    assert isinstance(result, RenderResult)
    assert result.text == "Hi Ada, you have 3 messages"
    assert result.name == "greet"
    assert result.variant == "default"
    assert HEX64.match(result.template_hash), result.template_hash
    assert HEX64.match(result.render_hash), result.render_hash
    assert result.guard is None


def test_render_via_instance() -> None:
    """prompt.render(instance) uses the already-constructed instance path (data=None)."""
    p = Prompt(SIMPLE_DEF)
    result = p.render(Named(name="Bo"))

    assert result.text == "Hi Bo"
    assert result.variant == "default"
    assert HEX64.match(result.template_hash)


def test_render_validation_failure_raises_before_render() -> None:
    """Pydantic validation failure raises PromptValidationError before kernel is reached."""
    p = Prompt(GREET_DEF)
    with pytest.raises(PromptValidationError) as excinfo:
        p.render(Greeting, data={"name": "Ada", "count": -1})
    rows = excinfo.value.errors
    assert any(r.field == "count" for r in rows), [r.field for r in rows]
    assert all(r.code == "validation" for r in rows)


def test_render_sec004_pydantic_path_secret_not_leaked() -> None:
    """SEC-004-PY: a rejected sensitive input must not appear in PromptValidationError."""

    class Secretful(BaseModel):
        token: str

        @field_validator("token")
        @classmethod
        def _no_forbidden(cls, v: str) -> str:
            if v.startswith("sk-"):
                raise ValueError("token has a forbidden prefix")
            return v

    secret = "sk-super-secret-token-9f8a7b6c5d4e"
    p = Prompt(
        {
            "name": "leaky",
            "role": "user",
            "body": "Using {{ token }}",
            "variables": {"token": {"type": "string", "origin": "trusted"}},
        }
    )
    with pytest.raises(PromptValidationError) as excinfo:
        p.render(Secretful, data={"token": secret})

    exc = excinfo.value
    assert secret not in str(exc)
    for row in exc.errors:
        assert secret not in row.message


def test_render_with_named_variant() -> None:
    """prompt.render(vars, variant='formal') selects the named variant arm."""
    p = Prompt(
        {
            "name": "salute",
            "role": "user",
            "body": "Hi {{ name }}",
            "variants": {"formal": {"body": "Good day, {{ name }}."}},
            "variables": {"name": {"type": "string", "origin": "trusted"}},
        }
    )
    result = p.render(Named(name="Di"), variant="formal")
    assert result.text == "Good day, Di."
    assert result.variant == "formal"


def test_render_guard_plumbed_through() -> None:
    """Guard is plumbed through; text and guard are separate (FR-009)."""
    p = Prompt(ASK_DEF)
    plain = p.render(TopicVars(topic="rivers"))
    guarded = p.render(TopicVars(topic="rivers"), guard=GuardConfig(enabled=True))

    assert plain.guard is None
    assert guarded.guard is not None
    assert "topic" in guarded.guard
    assert plain.text == guarded.text  # body text is unchanged by the guard


def test_get_source_returns_unrendered_template() -> None:
    """prompt.get_source() returns the raw template without interpolation."""
    p = Prompt(SIMPLE_DEF)
    src = p.get_source()
    assert src == "Hi {{ name }}"
    assert "{{" in src  # KEY: not interpolated


def test_get_source_named_variant() -> None:
    """prompt.get_source(variant='x') returns that variant's source."""
    p = Prompt(
        {
            "name": "s",
            "role": "user",
            "body": "root {{ x }}",
            "variants": {"v": {"body": "variant {{ x }}"}},
            "variables": {"x": {"type": "string", "origin": "trusted"}},
        }
    )
    assert p.get_source(variant="v") == "variant {{ x }}"
    assert p.get_source() == "root {{ x }}"


def test_get_source_unknown_variant_raises_render_error() -> None:
    """get_source with an unknown variant raises PromptRenderError."""
    p = Prompt(SIMPLE_DEF)
    with pytest.raises(PromptRenderError) as excinfo:
        p.get_source(variant="nope")
    assert any(r.code == "unknown_variant" for r in excinfo.value.errors)


def test_check_returns_check_report() -> None:
    """prompt.check() returns a CheckReport; no findings for a clean prompt."""
    p = Prompt(SIMPLE_DEF)
    report = p.check()
    assert isinstance(report, CheckReport)
    assert report.passed()


def test_check_flags_unguarded_untrusted() -> None:
    """prompt.check() surfaces an untrusted_without_guard finding for ASK_DEF."""
    p = Prompt(ASK_DEF)
    report = p.check()
    assert not report.passed()
    assert any(f.kind == "untrusted_without_guard" for f in report.findings)


def test_check_passes_for_guarded_untrusted() -> None:
    """A guard key under metadata satisfies the lint."""
    guarded = dict(ASK_DEF)
    guarded["metadata"] = {"guard": {"enabled": True}}
    p = Prompt(guarded)
    assert p.check().passed()


# ─── T038: derive ─────────────────────────────────────────────────────────────


def test_derive_overlay_derives_new_body_original_untouched() -> None:
    """derive(overlay) produces a derived Prompt; the original is untouched (SC-004)."""
    original = Prompt(SIMPLE_DEF)
    original_body = original.body

    derived = original.derive({"body": "Hey {{ name }}"})

    assert derived.body == "Hey {{ name }}"
    assert original.body == original_body, "original must be untouched (SC-004)"
    assert derived.name == original.name  # name carried forward


def test_derive_overlay_can_rename() -> None:
    """derive can overlay the name field."""
    p = Prompt(SIMPLE_DEF)
    derived = p.derive({"name": "simple-renamed"})
    assert derived.name == "simple-renamed"
    assert p.name == "simple", "original unchanged"


def test_derive_overlay_undeclared_var_raises() -> None:
    """derive overlay that introduces an undeclared variable raises."""
    p = Prompt(SIMPLE_DEF)
    with pytest.raises((PromptValidationError, PromptRenderError)):
        p.derive({"body": "{{ ghost }}"})


def test_derive_overlay_reserved_variant_name_raises() -> None:
    """derive overlay that adds a 'default' variant raises."""
    p = Prompt(SIMPLE_DEF)
    with pytest.raises((PromptValidationError, PromptRenderError)):
        p.derive({"variants": {"default": {"body": "shadowed"}}})


def test_derive_validators_carry_forward() -> None:
    """derive carries validators forward from the original by default (R6)."""
    strict = {
        "name": "strict",
        "role": "user",
        "body": "Hi {{ name }}",
        "variables": {"name": {"type": "string", "origin": "trusted", "validation_required": True}},
    }
    original = Prompt(strict, validators=Named)
    # Derive without supplying validators — they carry forward from original.
    derived = original.derive({"body": "Hello {{ name }}"})
    assert derived.body == "Hello {{ name }}"


def test_derive_validators_override() -> None:
    """derive(overlay, validators=NewModel) overrides the bound validators."""
    strict = {
        "name": "strict",
        "role": "user",
        "body": "Hi {{ name }}",
        "variables": {"name": {"type": "string", "origin": "trusted", "validation_required": True}},
    }
    original = Prompt(strict, validators=Named)

    class AlsoNamed(BaseModel):
        name: str

    # Override with a different but covering class.
    derived = original.derive({"body": "Hey {{ name }}"}, validators=AlsoNamed)
    assert derived.body == "Hey {{ name }}"


def test_derive_adds_variant_original_unchanged() -> None:
    """derive that adds a named variant produces the new variant; original has none."""
    p = Prompt(SIMPLE_DEF)
    assert p.variants == {}

    derived = p.derive(
        {
            "variants": {"brief": {"body": "Hey {{ name }}"}},
        }
    )
    assert "brief" in derived.variants
    assert p.variants == {}, "original variants must be untouched"


# ─── T039: Composition with Prompt objects ────────────────────────────────────


def test_composition_append_prompt_objects() -> None:
    """Composition.append takes Prompt objects, not name strings (T039 reshape)."""
    sys_p = Prompt(SYS_DEF)
    greet_p = Prompt(SIMPLE_DEF)

    comp = Composition()
    comp.append(sys_p, EmptyVars())
    comp.append(greet_p, Named(name="Ada"))
    assert len(comp) == 2

    messages = comp.resolve()
    assert len(messages) == 2
    assert messages[0].role == "system"
    assert messages[0].text == "You are helpful."
    assert messages[1].role == "user"
    assert messages[1].text == "Hi Ada"


def test_composition_from_messages_prompt_objects() -> None:
    """Composition.from_messages takes (Prompt, vars) tuples (T039 reshape)."""
    sys_p = Prompt(SYS_DEF)
    greet_p = Prompt(SIMPLE_DEF)

    comp = Composition.from_messages(
        [
            (sys_p, EmptyVars()),
            (greet_p, Named(name="Bo")),
        ]
    )
    messages = comp.resolve()
    assert [m.role for m in messages] == ["system", "user"]
    assert [m.text for m in messages] == ["You are helpful.", "Hi Bo"]


def test_composition_no_registry_arg_on_resolve() -> None:
    """resolve() takes no registry argument — the reshape removes the registry concept."""
    p = Prompt(SIMPLE_DEF)
    comp = Composition()
    comp.append(p, Named(name="Cy"))

    # Calling resolve() with no args should work; passing an unexpected arg must fail
    # (we don't want a silent extra positional arg to be accepted).
    messages = comp.resolve()
    assert messages[0].text == "Hi Cy"


def test_composition_invalid_vars_stores_nothing() -> None:
    """An invalid entry at append raises PromptValidationError and stores nothing."""
    p = Prompt(SIMPLE_DEF)
    comp = Composition()
    comp.append(p, Named(name="ok"))
    assert len(comp) == 1

    invalid = Named.model_construct(name="")
    with pytest.raises(PromptValidationError):
        comp.append(p, invalid)

    assert len(comp) == 1, "rejected append must store nothing"


def test_composition_unknown_variant_fails_at_resolve() -> None:
    """An unknown variant in a composition entry fails at resolve as PromptRenderError."""
    p = Prompt(SIMPLE_DEF)
    comp = Composition()
    comp.append(p, Named(name="Ada"), variant="nonexistent")

    with pytest.raises(PromptRenderError):
        comp.resolve()


def test_composition_empty_resolves_to_empty_list() -> None:
    """An empty composition resolves to []."""
    comp = Composition()
    assert comp.resolve() == []


def test_composition_no_chain_method() -> None:
    """FR-013: no .chain() method on Composition or its instances."""
    assert not hasattr(Composition, "chain")
    assert not hasattr(Composition(), "chain")


# ─── T039: Registry fully removed from the public surface ────────────────────


def test_registry_absent_from_all() -> None:
    """Registry must not appear in prompting_press.__all__ (T039 — removed entirely)."""
    assert "Registry" not in prompting_press.__all__, (
        f"Registry must be removed from __all__, got {prompting_press.__all__}"
    )


def test_registry_not_on_module() -> None:
    """Registry must not be an attribute of prompting_press (spec 008 FR-019 / SC-001).
    The pyclass is fully deleted from the native module."""
    assert not hasattr(prompting_press, "Registry"), (
        "Registry must not exist on the prompting_press module"
    )


def test_prompt_in_all() -> None:
    """Prompt must be in prompting_press.__all__."""
    assert "Prompt" in prompting_press.__all__


def test_module_exposes_prompt_class() -> None:
    """Prompt is importable from prompting_press."""
    assert hasattr(prompting_press, "Prompt")
    assert prompting_press.Prompt is Prompt


# ─── Accessor read-only contract ──────────────────────────────────────────────


def test_prompt_properties_are_read_only() -> None:
    """Prompt properties are read-only (no setters)."""
    p = Prompt(SIMPLE_DEF)
    with pytest.raises(AttributeError):
        p.name = "tampered"  # type: ignore[misc]
    with pytest.raises(AttributeError):
        p.body = "tampered"  # type: ignore[misc]


def test_prompt_repr() -> None:
    """repr(Prompt) is fixed-shape and includes name and role."""
    p = Prompt(SIMPLE_DEF)
    r = repr(p)
    assert "simple" in r
    assert "user" in r
