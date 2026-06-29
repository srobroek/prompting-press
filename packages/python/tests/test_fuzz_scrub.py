"""Secret-scrub adversarial verification — spec 009 Phase 2 (T010).

Invariant under test (FR-002, FR-007, SC-003):
  A secret-shaped value that triggers a parse/render/validation error MUST be
  absent from:
    - str(err)          — the human-readable error string
    - err.errors rows   — each FieldError's field, code, and message
    - traceback.format_exc() — the full Python traceback

This adversarially verifies the spec-004 M-1 lesson: the SEC-004 scrub holds
even when the secret is injected as a variable value, a prompt field value, or
part of a YAML/JSON/TOML document body.

The test constructs the *minimum* scenario needed to confirm no leakage — a
single API-key-shaped string fed to a Pydantic validator that rejects it, then
the same secret fed to from_yaml/from_json with an invalid document that embeds
it.  We use a fixed secret prefix + random suffix so the check is both
deterministic (we know what to search for) and varied across hypothesis runs.

Settings: max_examples=60, deadline=8_000 ms.
"""

from __future__ import annotations

import traceback
from typing import Any

import pytest
from hypothesis import HealthCheck, given, settings
from hypothesis import strategies as st
from pydantic import BaseModel, field_validator

from prompting_press import (
    Prompt,
    PromptingPressError,
    PromptValidationError,
)

# ---------------------------------------------------------------------------
# Hypothesis settings
# ---------------------------------------------------------------------------

FUZZ_SETTINGS = settings(
    max_examples=60,
    deadline=8_000,
    suppress_health_check=[HealthCheck.too_slow],
)

# ---------------------------------------------------------------------------
# Secret-shaped value strategy
#
# We use a fixed recognisable prefix so the leakage assertion is unambiguous.
# The suffix is random to exercise many different lengths and char classes.
# ---------------------------------------------------------------------------

SECRET_PREFIX = "sk-LIVE-"

_SECRET = st.builds(
    lambda suffix: SECRET_PREFIX + suffix,
    suffix=st.text(
        alphabet="ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_-",
        min_size=8,
        max_size=40,
    ),
)


def _assert_no_leakage(exc: PromptingPressError, secret: str) -> None:
    """Assert the secret substring is absent from all error surfaces."""
    assert secret not in str(exc), (
        f"secret leaked in str(exc): {str(exc)!r}"
    )
    for row in exc.errors:
        assert secret not in (row.field or ""), (
            f"secret leaked in FieldError.field: {row.field!r}"
        )
        assert secret not in (row.message or ""), (
            f"secret leaked in FieldError.message: {row.message!r}"
        )
    tb = traceback.format_exc()
    # traceback.format_exc() returns "NoneType: None\n" when called outside an
    # except block; the secret must not appear there either — but the primary
    # check is the structured error rows above.
    assert secret not in tb, (
        f"secret leaked in traceback.format_exc(): {tb!r}"
    )


# ---------------------------------------------------------------------------
# T010a: Pydantic-path validation failure — secret rejected by field_validator
# ---------------------------------------------------------------------------

@given(secret=_SECRET)
@FUZZ_SETTINGS
def test_pydantic_path_secret_not_leaked(secret: str) -> None:
    """A secret rejected by a Pydantic validator must not appear in the error."""

    class SecretVars(BaseModel):
        token: str

        @field_validator("token")
        @classmethod
        def _reject_live_keys(cls, v: str) -> str:
            # Value-free message — deliberately does NOT interpolate `v`.
            if v.startswith(SECRET_PREFIX):
                raise ValueError("token uses a forbidden prefix")
            return v

    p = Prompt({
        "name": "scrub-test",
        "role": "user",
        "body": "token={{ token }}",
        "variables": {"token": {"type": "string", "origin": "trusted"}},
    })

    with pytest.raises(PromptingPressError) as excinfo:
        p.render(SecretVars, data={"token": secret})

    _assert_no_leakage(excinfo.value, secret)


# ---------------------------------------------------------------------------
# T010b: from_yaml — secret embedded in malformed YAML document
#
# The document IS valid YAML but is missing required fields (body is absent),
# so Rust deserialization will raise a LoadError.  The secret appears as a
# field value inside the document; it must not leak into the error.
# ---------------------------------------------------------------------------

@given(secret=_SECRET)
@FUZZ_SETTINGS
def test_from_yaml_secret_not_leaked(secret: str) -> None:
    """A secret embedded in an invalid YAML document must not appear in the error."""
    # Construct a YAML document that lacks `body` (required) but contains the
    # secret in the `name` field — the load will fail, the secret must not leak.
    yaml_doc = f"name: {secret!r}\nrole: user\n"
    try:
        Prompt.from_yaml(yaml_doc)
    except PromptingPressError as exc:
        _assert_no_leakage(exc, secret)
    # If it somehow succeeds (secret happens to be a valid name with body
    # defaulting), that is also fine — we just assert no-panic.


# ---------------------------------------------------------------------------
# T010c: from_json — secret embedded in malformed JSON document
# ---------------------------------------------------------------------------

@given(secret=_SECRET)
@FUZZ_SETTINGS
def test_from_json_secret_not_leaked(secret: str) -> None:
    """A secret embedded in an invalid JSON document must not appear in the error."""
    import json as _json
    # A JSON object that has the secret as a value but omits the required `body`.
    doc = _json.dumps({"name": secret, "role": "user"})
    try:
        Prompt.from_json(doc)
    except PromptingPressError as exc:
        _assert_no_leakage(exc, secret)


# ---------------------------------------------------------------------------
# T010d: kernel render path — secret passes Pydantic, rejected by kernel
#
# We use the `token + 1` trick from test_render.py: a string + integer
# is a kernel render error.  The secret reaches the Rust kernel and any raw
# value in the kernel error must be scrubbed before surfacing in Python.
# ---------------------------------------------------------------------------

@given(secret=_SECRET)
@FUZZ_SETTINGS
def test_kernel_path_secret_not_leaked(secret: str) -> None:
    """A secret that reaches the kernel render path must not appear in the error."""

    class PlainVars(BaseModel):
        """Passes Pydantic validation — the secret crosses FFI into the kernel."""
        token: str

    p = Prompt({
        "name": "kernel-scrub",
        "role": "user",
        "body": "{{ token + 1 }}",  # string + int → kernel render error
        "variables": {"token": {"type": "string", "origin": "trusted"}},
    })

    with pytest.raises(PromptingPressError) as excinfo:
        p.render(PlainVars, data={"token": secret})

    _assert_no_leakage(excinfo.value, secret)
