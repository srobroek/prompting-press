"""US1 render-path tests for the PyO3 binding (`prompting_press`) — spec 004, T009.

These exercise the *Python-observable* render path that the Rust `#[cfg(test)]` suite
cannot reach because it needs a real Pydantic Vars model: validate-in-Python (FR-002),
the normalized error contract (FR-014, C-06), the SEC-004-PY scrub, the three-sets
agreement gap (loud `undefined_variable`, never a silent empty render), and the guard
plumb-through (FR-009).

Construction note (US2 loaders are not implemented yet): a prompt is built by
validating a plain dict into the generated `PromptDefinition` and handing it to
`Registry.insert`, which extracts the kernel struct via `pythonize::depythonize`.

Model invariant under test: a single `render` returns the BODY as `.text` and any
guard instruction as the SEPARATE `.guard` field — the library never concatenates the
two. Tests assert `.text` is body-only and `.guard` stands apart.
"""

from __future__ import annotations

import re

import pydantic
import pytest
from pydantic import BaseModel, field_validator

import prompting_press
from prompting_press import (
    GuardConfig,
    PromptingPressError,
    PromptRenderError,
    PromptValidationError,
    Registry,
    UnknownPromptError,
    get_source,
    render,
)
from prompting_press.generated import PromptDefinition

# A lowercase 64-char hex string — the SHA-256 provenance hash shape (FR-012/FR-013).
HEX64 = re.compile(r"\A[0-9a-f]{64}\Z")


# --------------------------------------------------------------------------------------
# Vars models (Pydantic — the per-language idiom; Principle VI). Each carries a real
# `@field_validator` so validation is genuinely exercised, not a no-op pass-through.
# --------------------------------------------------------------------------------------


class Greeting(BaseModel):
    """The happy-path Vars model: a validator rejects a negative message count."""

    name: str
    count: int

    @field_validator("count")
    @classmethod
    def _count_non_negative(cls, value: int) -> int:
        if value < 0:
            raise ValueError("count must be non-negative")
        return value


class Secretful(BaseModel):
    """A model whose validator rejects a value that is itself sensitive (SEC-004-PY).

    The validator's error message is fixed and value-free; the rejected `token`
    string (a stand-in secret) must never reach the Python error surface.
    """

    token: str

    @field_validator("token")
    @classmethod
    def _no_forbidden_token(cls, value: str) -> str:
        if value.startswith("sk-"):
            # Deliberately does NOT interpolate `value` into the message.
            raise ValueError("token has a forbidden prefix")
        return value


class Misnamed(BaseModel):
    """Three-sets gap: the field is `nam`, the template references `{{ name }}`.

    Validation passes (the model is internally consistent); the *agreement* between
    the Vars field set and the template's referenced roots is the caller's job, and a
    miss is surfaced loudly by the kernel — not as a silent empty render.
    """

    nam: str


class Topic(BaseModel):
    """Vars for the guard-plumb prompt: a single (untrusted) `topic` string."""

    topic: str


class Secret(BaseModel):
    """A single string that *passes* Pydantic validation, so it reaches the kernel.

    Unlike `Secretful` (whose validator rejects in Python before the kernel is touched),
    this model carries the secret all the way into the render, where a value-misusing
    template triggers a real `KernelError::Render`. SEC-004 must scrub the bound value
    out of that kernel-path error too — not only out of the Pydantic-path error.
    """

    token: str


class TwoFields(BaseModel):
    """Two independently-validated fields, so a single bad input produces a
    multi-row `pydantic.ValidationError` (SC-002: name *every* offending field)."""

    name: str
    count: int

    @field_validator("name")
    @classmethod
    def _name_nonempty(cls, value: str) -> str:
        if not value:
            raise ValueError("name must not be empty")
        return value

    @field_validator("count")
    @classmethod
    def _count_non_negative(cls, value: int) -> int:
        if value < 0:
            raise ValueError("count must be non-negative")
        return value


# --------------------------------------------------------------------------------------
# Registry helpers
# --------------------------------------------------------------------------------------


def _registry(definition: dict) -> Registry:
    """Validate `definition` into a generated `PromptDefinition`, then insert it.

    `Registry.insert` reads the object via `pythonize::depythonize`, which requires a
    plain Mapping (a Pydantic model *instance* is not a Mapping, and an explicit
    ``null`` for an absent sequence field is rejected by the kernel's serde struct).
    So we validate the dict through the generated `PromptDefinition` (proving the
    shape) and hand `insert` the canonical JSON dump with absent fields omitted
    (`mode="json"` stringifies enums/dates; `exclude_none=True` drops the optional
    nulls). This is the US2-precursor of the eventual `load_json` path.
    """
    model = PromptDefinition.model_validate(definition)
    reg = Registry()
    reg.insert(model.model_dump(mode="json", exclude_none=True))
    return reg


GREET_DEF = {
    "name": "greet",
    "role": "user",
    "body": "Hi {{ name }}, you have {{ count }} messages",
    "variables": {
        "name": {"type": "string", "provenance": "trusted"},
        "count": {"type": "integer", "provenance": "trusted"},
    },
}

# A prompt whose only declared variable is `untrusted`, so an enabled guard has a
# field to name. The body references `{{ topic }}`.
ASK_DEF = {
    "name": "ask",
    "role": "user",
    "body": "Tell me about {{ topic }}.",
    "variables": {
        "topic": {"type": "string", "provenance": "untrusted"},
    },
}


# --------------------------------------------------------------------------------------
# 1. Valid render (SC-001) — class + data path
# --------------------------------------------------------------------------------------


def test_valid_render_produces_text_and_hex_hashes() -> None:
    reg = _registry(GREET_DEF)

    result = render(reg, "greet", Greeting, data={"name": "Ada", "count": 3})

    assert result.text == "Hi Ada, you have 3 messages"
    assert result.name == "greet"
    assert result.variant == "default", "no variant selected ⇒ the reserved default arm"
    # Provenance hashes: 64-char lowercase hex (FR-012/FR-013).
    assert HEX64.match(result.template_hash), result.template_hash
    assert HEX64.match(result.render_hash), result.render_hash
    # No guard requested ⇒ the separate guard field is absent (model: guard ≠ text).
    assert result.guard is None


# --------------------------------------------------------------------------------------
# 2. Validation failure (SC-002 / FR-002) — caught in Python, before any templating
# --------------------------------------------------------------------------------------


def test_validation_failure_raises_before_render() -> None:
    reg = _registry(GREET_DEF)

    with pytest.raises(PromptValidationError) as excinfo:
        render(reg, "greet", Greeting, data={"name": "Ada", "count": -1})

    exc = excinfo.value
    # The normalized contract: a list of {field, code, message} rows.
    rows = exc.errors
    assert rows, "a validation failure must carry at least one structured row"
    offending = [r for r in rows if r.field == "count"]
    assert offending, f"expected a row naming `count`, got {[r.field for r in rows]}"
    assert all(r.code == "validation" for r in offending), [r.code for r in offending]


def test_validation_failure_names_every_offending_field() -> None:
    # SC-002: a structured exception naming EVERY offending field — exercise the real
    # Pydantic→rows extraction (collect_validation_rows) with a multi-error
    # ValidationError, not a single field.
    reg = _registry(GREET_DEF)

    with pytest.raises(PromptValidationError) as excinfo:
        # Both fields violate their validators in one model_validate pass.
        render(reg, "greet", TwoFields, data={"name": "", "count": -1})

    fields = {r.field for r in excinfo.value.errors}
    assert {"name", "count"} <= fields, (
        f"both offending fields must be named, got {sorted(fields)}"
    )
    assert all(r.code == "validation" for r in excinfo.value.errors)


# --------------------------------------------------------------------------------------
# 3. No native error type leaks across the boundary (SC-006 / C-06)
# --------------------------------------------------------------------------------------


def test_validation_error_is_not_a_pydantic_error() -> None:
    reg = _registry(GREET_DEF)

    with pytest.raises(PromptingPressError) as excinfo:
        render(reg, "greet", Greeting, data={"name": "Ada", "count": -1})

    exc = excinfo.value
    # The raised type is the binding's, and is specifically the validation subtype ...
    assert isinstance(exc, PromptValidationError)
    # ... and is NOT pydantic's native ValidationError (it must not cross the boundary).
    assert not isinstance(exc, pydantic.ValidationError)


# --------------------------------------------------------------------------------------
# 4. SEC-004-PY — the rejected (sensitive) input never appears on the error surface
# --------------------------------------------------------------------------------------


def test_rejected_sensitive_input_is_not_leaked() -> None:
    secret = "sk-super-secret-token-9f8a7b6c5d4e"
    reg = _registry(
        {
            "name": "leaky",
            "role": "user",
            "body": "Using {{ token }}",
            "variables": {"token": {"type": "string", "provenance": "trusted"}},
        }
    )

    with pytest.raises(PromptValidationError) as excinfo:
        render(reg, "leaky", Secretful, data={"token": secret})

    exc = excinfo.value
    # Neither str(exc) nor any row message may contain the rejected value — only the
    # validator's own value-free `msg` is copied across (SEC-004-PY copies `msg`, never
    # pydantic's `input`/`ctx`).
    assert secret not in str(exc), f"str(exc) leaked the secret: {exc}"
    for row in exc.errors:
        assert secret not in row.message, (
            f"row message leaked the secret: {row.message}"
        )
    # Positive check: the value-free validator message survives.
    assert any("forbidden prefix" in row.message for row in exc.errors)


def test_secret_in_a_kernel_render_error_is_not_leaked() -> None:
    """SEC-004 on the *kernel* path (not the Pydantic path).

    `Secret` validates cleanly, so the secret crosses the FFI boundary into the kernel,
    where `{{ token + 1 }}` (string + int) is a genuine `KernelError::Render`. The
    binding routes that through the consumer's scrubber, which discards the raw detail
    (which embeds the bound value) and emits the fixed `"render error"` message. The
    secret must appear in neither `str(exc)`, `repr(exc)`, nor any row.
    """
    secret = "sk-super-secret-token-9f8a7b6c5d4e"
    reg = _registry(
        {
            "name": "kernely",
            "role": "user",
            "body": "Using {{ token + 1 }}",  # string + int ⇒ kernel render error
            "variables": {"token": {"type": "string", "provenance": "trusted"}},
        }
    )

    with pytest.raises(PromptRenderError) as excinfo:
        render(reg, "kernely", Secret, data={"token": secret})

    exc = excinfo.value
    assert secret not in str(exc), f"str(exc) leaked the secret: {exc}"
    assert secret not in repr(exc), "repr(exc) leaked the secret"
    for row in exc.errors:
        assert secret not in row.message, f"row leaked the secret: {row.message}"
        assert secret not in row.field, f"row.field leaked the secret: {row.field}"
    # The scrub replaces the value-bearing detail with the consumer's fixed message + code.
    assert [r.code for r in exc.errors] == ["render"], [r.code for r in exc.errors]
    assert any(r.message == "render error" for r in exc.errors)


# --------------------------------------------------------------------------------------
# 5. Three-sets gap — a Vars/template field-name mismatch is LOUD, not a silent empty render
# --------------------------------------------------------------------------------------


def test_field_name_mismatch_is_loud_undefined_variable() -> None:
    # The Vars model has `nam`; the template references `{{ name }}`.
    reg = _registry(
        {
            "name": "greet",
            "role": "user",
            "body": "Hi {{ name }}!",
            "variables": {"name": {"type": "string", "provenance": "trusted"}},
        }
    )

    # Validation passes (Misnamed is internally consistent) — the failure is at render.
    with pytest.raises(PromptRenderError) as excinfo:
        render(reg, "greet", Misnamed, data={"nam": "Ada"})

    exc = excinfo.value
    codes = [r.code for r in exc.errors]
    assert "undefined_variable" in codes, (
        f"a referenced-but-undefined root must be a loud undefined_variable, got {codes}"
    )


# --------------------------------------------------------------------------------------
# 6. Guard plumb-through (FR-009) — guard text is SEPARATE from body text
# --------------------------------------------------------------------------------------


def test_guard_is_plumbed_through_and_separate_from_text() -> None:
    reg = _registry(ASK_DEF)

    plain = render(reg, "ask", Topic, data={"topic": "rivers"})
    guarded = render(
        reg, "ask", Topic, data={"topic": "rivers"}, guard=GuardConfig(enabled=True)
    )

    # Default render ⇒ no guard.
    assert plain.guard is None
    # Enabled guard on a prompt declaring an untrusted field ⇒ guard text present ...
    assert guarded.guard is not None
    assert isinstance(guarded.guard, str)
    # ... and it names the untrusted field.
    assert "topic" in guarded.guard, guarded.guard

    # The body text is IDENTICAL in both: the guard is the caller's system-prompt
    # addendum, never concatenated into `.text` (the model invariant).
    assert plain.text == guarded.text == "Tell me about rivers."
    # And the guard text is not smuggled into the body.
    assert guarded.guard not in guarded.text


def test_disabled_guard_config_matches_no_guard() -> None:
    reg = _registry(ASK_DEF)

    no_guard = render(reg, "ask", Topic, data={"topic": "rivers"})
    disabled = render(
        reg, "ask", Topic, data={"topic": "rivers"}, guard=GuardConfig(enabled=False)
    )

    # GuardConfig() / enabled=False is equivalent to passing no guard at all.
    assert no_guard.guard is None
    assert disabled.guard is None
    assert no_guard.text == disabled.text


# --------------------------------------------------------------------------------------
# 7. Instance path — `data=None`, `vars` is an already-constructed model instance
# --------------------------------------------------------------------------------------


def test_render_accepts_a_model_instance() -> None:
    reg = _registry(GREET_DEF)

    result = render(reg, "greet", Greeting(name="Bo", count=1))

    assert result.text == "Hi Bo, you have 1 messages"
    assert result.variant == "default"
    assert HEX64.match(result.template_hash)
    assert HEX64.match(result.render_hash)


def test_module_exposes_us1_surface() -> None:
    # A light smoke check that the US1 public names are importable and callable shapes.
    assert callable(prompting_press.render)
    assert callable(prompting_press.get_source)
    assert prompting_press.GuardConfig(enabled=True).enabled is True


# --------------------------------------------------------------------------------------
# 8. get_source (FR-010) — returns the UNRENDERED template; no vars, no validation
# --------------------------------------------------------------------------------------


def test_get_source_returns_unrendered_template() -> None:
    reg = _registry(GREET_DEF)

    source = get_source(reg, "greet")

    # The KEY property: get_source returns the raw template, it does NOT interpolate.
    assert source == "Hi {{ name }}, you have {{ count }} messages"
    assert "{{" in source, "get_source must return the unrendered source"


def test_get_source_unknown_name_raises_unknown_prompt() -> None:
    reg = _registry(GREET_DEF)

    with pytest.raises(UnknownPromptError):
        get_source(reg, "does-not-exist")


def test_get_source_unknown_variant_raises_render_error() -> None:
    reg = _registry(GREET_DEF)

    with pytest.raises(PromptRenderError) as excinfo:
        get_source(reg, "greet", variant="nope")

    assert any(r.code == "unknown_variant" for r in excinfo.value.errors), [
        r.code for r in excinfo.value.errors
    ]


# --------------------------------------------------------------------------------------
# 9. No token surface anywhere on the package (F4 / SC-010) — asserted, not only grep-gated
# --------------------------------------------------------------------------------------


def test_no_token_counting_surface() -> None:
    # The library ships NO token counter (roadmap decision F4 / SC-010). Pin the absence
    # at the package surface so dropping the CI grep gate cannot silently reintroduce it.
    for forbidden in ("count_tokens", "token_count", "TokenCount", "count-tokens"):
        assert not hasattr(prompting_press, forbidden), (
            f"token-counting surface {forbidden!r} must not exist (F4)"
        )
    assert not any(
        "token" in name.lower() and "count" in name.lower()
        for name in prompting_press.__all__
    ), f"no token-count symbol may appear in __all__: {prompting_press.__all__}"
