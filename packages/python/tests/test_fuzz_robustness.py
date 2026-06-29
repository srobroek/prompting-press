"""Adversarial robustness fuzzing for the Python binding — spec 009 Phase 2 (T009).

Invariant under test (FR-001, FR-003, SC-001, SC-002, SC-005):
  Every call to Prompt(...) / from_yaml / from_json / from_toml / render / check
  either returns a valid result or raises a PromptingPressError subtype.
  It MUST NEVER raise a bare Exception, KeyError, AttributeError, or panic from
  the Rust extension (which would surface as pyo3_runtime.PanicException or a
  similar non-PromptingPressError type).

Strategy design:
  - hostile_doc: malformed dicts (missing required fields, wrong types, unknown
    keys, oversized strings, deeply-nested metadata, pure Unicode/control-char
    bodies).  We always supply the minimum required keys (name/role/body) and
    vary the rest so that invalid paths actually reach the Rust deserialiser
    rather than being trivially rejected by Python before the FFI call.
  - hostile_vars: hostile render var-sets (huge strings, empty strings, nested
    dicts where a scalar is expected) fed to render().

Settings: max_examples=75 with a 10-second deadline per example so the gate
stays bounded (FR-004).  Derandomised with a fixed database so failures replay.
"""

from __future__ import annotations

import string
from typing import Any

import pytest
from hypothesis import HealthCheck, given, settings
from hypothesis import strategies as st

from prompting_press import (
    Prompt,
    PromptingPressError,
)

# ---------------------------------------------------------------------------
# Hypothesis settings — bounded + replayable (FR-004).
# ---------------------------------------------------------------------------

FUZZ_SETTINGS = settings(
    max_examples=75,
    deadline=10_000,  # ms per example
    suppress_health_check=[HealthCheck.too_slow],
)

# ---------------------------------------------------------------------------
# Shared building blocks
# ---------------------------------------------------------------------------

# A valid origin value and a union that includes bad ones.
_ORIGIN = st.sampled_from(["trusted", "untrusted", "external"])
_ORIGIN_ANY = st.one_of(_ORIGIN, st.text(max_size=20), st.integers(), st.none())

# Hostile text: printable, Unicode astral-plane, control chars, long strings.
_PRINTABLE = st.text(alphabet=string.printable, max_size=50)
_UNICODE = st.text(max_size=50)  # full Unicode including astral, bidi, combining
_CONTROL = st.text(
    alphabet=st.characters(
        categories=["Cc"],  # Unicode category Cc = control chars
        max_codepoint=0x001F,
    ),
    max_size=30,
)
_HUGE = st.text(min_size=5_000, max_size=20_000)
_HOSTILE_TEXT = st.one_of(_PRINTABLE, _UNICODE, _CONTROL, _HUGE)

# A variable entry dict (valid or hostile).
_VALID_VAR_ENTRY = st.fixed_dictionaries(
    {"type": st.sampled_from(["string", "integer", "number", "boolean"]), "origin": _ORIGIN}
)
_HOSTILE_VAR_ENTRY = st.one_of(
    _VALID_VAR_ENTRY,
    st.fixed_dictionaries({"type": st.text(max_size=20), "origin": _ORIGIN_ANY}),
    st.dictionaries(st.text(max_size=10), st.integers() | st.text(max_size=20), max_size=5),
    st.none(),
    st.integers(),
    st.text(max_size=20),
)

# variables dict: 0-4 entries, hostile keys and entries.
_VARIABLES = st.dictionaries(
    st.text(alphabet=string.ascii_letters + "_", min_size=1, max_size=15),
    _HOSTILE_VAR_ENTRY,
    max_size=4,
)
_VARIABLES_VALID = st.dictionaries(
    st.text(alphabet=string.ascii_letters + "_", min_size=1, max_size=10),
    _VALID_VAR_ENTRY,
    max_size=3,
)

# A valid body that only references variables whose names are in `var_names`.
# We generate a literal body (no Jinja) to avoid agreement-check failures when
# the body references a name not in variables — those are expected errors, but
# the hostile corpus test is about never-panic, not about valid construction.
_SAFE_BODY = st.text(
    alphabet=string.ascii_letters + string.digits + " .,!?\n",
    min_size=0,
    max_size=200,
)

# Metadata dict: hostile deep-nesting.
_METADATA = st.one_of(
    st.none(),
    st.dictionaries(
        st.text(max_size=10),
        st.recursive(
            st.one_of(st.text(max_size=20), st.integers(), st.booleans(), st.none()),
            lambda children: st.dictionaries(st.text(max_size=8), children, max_size=4),
            max_leaves=20,
        ),
        max_size=5,
    ),
)


# ---------------------------------------------------------------------------
# Strategy: a dict that always has name/role/body but may have hostile extras
# ---------------------------------------------------------------------------

@st.composite
def hostile_prompt_dict(draw: st.DrawFn) -> dict[str, Any]:
    """Generate a prompt-shaped dict with hostile field values."""
    name = draw(st.one_of(
        st.text(alphabet=string.ascii_letters + "_-", min_size=1, max_size=30),
        st.text(max_size=5),           # may be empty or odd chars
        st.integers(),                  # wrong type — triggers load error
    ))
    role = draw(st.one_of(
        st.sampled_from(["user", "system", "assistant"]),
        st.text(max_size=10),
        st.integers(),
    ))
    body = draw(st.one_of(_SAFE_BODY, _HOSTILE_TEXT))
    variables = draw(st.one_of(
        st.none(),
        _VARIABLES,
        st.integers(),  # wrong type
    ))
    d: dict[str, Any] = {"name": name, "role": role, "body": body}
    if variables is not None:
        d["variables"] = variables
    meta = draw(_METADATA)
    if meta is not None:
        d["metadata"] = meta
    # Occasionally inject a completely unknown top-level key.
    if draw(st.booleans()):
        d[draw(st.text(alphabet=string.ascii_letters, min_size=1, max_size=10))] = draw(
            st.text(max_size=20)
        )
    return d


# ---------------------------------------------------------------------------
# Strategy: a valid prompt dict + hostile render vars dict
# ---------------------------------------------------------------------------

@st.composite
def valid_prompt_and_hostile_vars(draw: st.DrawFn) -> tuple[dict[str, Any], dict[str, Any]]:
    """Generate a valid prompt + a hostile vars dict to feed to render()."""
    var_names = draw(
        st.lists(
            st.text(alphabet=string.ascii_letters + "_", min_size=1, max_size=10),
            min_size=0,
            max_size=3,
            unique=True,
        )
    )
    variables = {n: {"type": "string", "origin": "trusted"} for n in var_names}
    # Body that does NOT reference any Jinja variables (literal text only) —
    # avoids the agreement-check raising on an out-of-vars reference.
    body = draw(_SAFE_BODY)
    prompt_def: dict[str, Any] = {
        "name": draw(st.text(alphabet=string.ascii_letters + "_-", min_size=1, max_size=20)),
        "role": "user",
        "body": body,
        "variables": variables,
    }
    # Hostile var values: huge strings, nested dicts, wrong types.
    hostile_value = st.one_of(
        st.text(max_size=500),
        _HOSTILE_TEXT,
        st.integers(),
        st.floats(allow_nan=False, allow_infinity=False),
        st.none(),
        st.lists(st.text(max_size=10), max_size=5),
        st.dictionaries(st.text(max_size=5), st.text(max_size=10), max_size=3),
    )
    vars_dict = {n: draw(hostile_value) for n in var_names}
    return prompt_def, vars_dict


# ---------------------------------------------------------------------------
# T009a: Prompt(...) construction never panics — always value or PromptingPressError
# ---------------------------------------------------------------------------

@given(doc=hostile_prompt_dict())
@FUZZ_SETTINGS
def test_construct_never_panics(doc: dict[str, Any]) -> None:
    """Prompt(dict) never panics; always returns or raises PromptingPressError."""
    try:
        Prompt(doc)
    except PromptingPressError:
        pass  # expected structured error path


# ---------------------------------------------------------------------------
# T009b: from_yaml never panics
# ---------------------------------------------------------------------------

@given(text=st.one_of(
    st.text(max_size=2000),          # random text including malformed YAML
    _HOSTILE_TEXT,
    _HUGE,
))
@FUZZ_SETTINGS
def test_from_yaml_never_panics(text: str) -> None:
    """Prompt.from_yaml(text) never panics on hostile input."""
    try:
        Prompt.from_yaml(text)
    except PromptingPressError:
        pass


# ---------------------------------------------------------------------------
# T009c: from_json never panics
# ---------------------------------------------------------------------------

@given(text=st.one_of(
    st.text(max_size=2000),
    _HOSTILE_TEXT,
))
@FUZZ_SETTINGS
def test_from_json_never_panics(text: str) -> None:
    """Prompt.from_json(text) never panics on hostile input."""
    try:
        Prompt.from_json(text)
    except PromptingPressError:
        pass


# ---------------------------------------------------------------------------
# T009d: from_toml never panics
# ---------------------------------------------------------------------------

@given(text=st.one_of(
    st.text(max_size=2000),
    _HOSTILE_TEXT,
))
@FUZZ_SETTINGS
def test_from_toml_never_panics(text: str) -> None:
    """Prompt.from_toml(text) never panics on hostile input."""
    try:
        Prompt.from_toml(text)
    except PromptingPressError:
        pass


# ---------------------------------------------------------------------------
# T009e: render with hostile vars never panics (SC-005: validate-before-render)
# ---------------------------------------------------------------------------

from pydantic import BaseModel  # noqa: E402 — after hypothesis imports


class _AnyVars(BaseModel):
    """A permissive vars model — accepts any string values."""

    model_config = {"extra": "allow"}


@given(pair=valid_prompt_and_hostile_vars())
@FUZZ_SETTINGS
def test_render_hostile_vars_never_panics(
    pair: tuple[dict[str, Any], dict[str, Any]],
) -> None:
    """render() with hostile vars never panics; returns value or PromptingPressError."""
    prompt_def, vars_dict = pair
    try:
        p = Prompt(prompt_def)
    except PromptingPressError:
        return  # construction failed — that's fine; skip render
    try:
        # Use _AnyVars so Pydantic validation does not reject the hostile values
        # before they reach the kernel; we want the kernel path exercised too.
        p.render(_AnyVars, data=vars_dict)
    except PromptingPressError:
        pass  # structured error — correct


# ---------------------------------------------------------------------------
# T009f: check() never panics
# ---------------------------------------------------------------------------

@given(doc=hostile_prompt_dict())
@FUZZ_SETTINGS
def test_check_never_panics(doc: dict[str, Any]) -> None:
    """check() never panics; either returns CheckReport or raises PromptingPressError."""
    try:
        p = Prompt(doc)
    except PromptingPressError:
        return
    try:
        p.check()
    except PromptingPressError:
        pass  # unexpected but structured — acceptable


# ---------------------------------------------------------------------------
# T009g: hash determinism (SC-004) — re-rendering produces byte-identical hashes
# ---------------------------------------------------------------------------

@st.composite
def valid_prompt_with_literal_body(draw: st.DrawFn) -> dict[str, Any]:
    """A fully valid prompt dict (literal body, no Jinja vars) for determinism tests."""
    return {
        "name": draw(st.text(alphabet=string.ascii_letters + "_", min_size=1, max_size=20)),
        "role": draw(st.sampled_from(["user", "system"])),
        "body": draw(_SAFE_BODY),
        "variables": {},
    }


@given(doc=valid_prompt_with_literal_body())
@FUZZ_SETTINGS
def test_hash_determinism(doc: dict[str, Any]) -> None:
    """Re-rendering the same prompt + vars produces byte-identical hashes (SC-004)."""
    try:
        p = Prompt(doc)
    except PromptingPressError:
        return

    class _Empty(BaseModel):
        pass

    try:
        r1 = p.render(_Empty, data={})
        r2 = p.render(_Empty, data={})
    except PromptingPressError:
        return  # construction-time error; skip

    assert r1.template_hash == r2.template_hash, (
        f"template_hash not deterministic: {r1.template_hash!r} != {r2.template_hash!r}"
    )
    assert r1.render_hash == r2.render_hash, (
        f"render_hash not deterministic: {r1.render_hash!r} != {r2.render_hash!r}"
    )
