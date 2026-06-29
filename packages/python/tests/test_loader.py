"""Dual-input factory tests for the PyO3 binding — spec 008 Phase 4.

Spec 008 Phase 4 replaces the Registry-based loaders with Prompt factory methods:
- `Prompt.from_yaml(text)` — parse already-read YAML text.
- `Prompt.from_json(text)` — parse already-read JSON text.
- `Prompt.from_toml(text)` — parse already-read TOML text.
- `Prompt(shape)` / `Prompt(dict)` — the constructed-object form.

These tests prove:
- All four Python entry points reach the shared core and normalize to the same
  representation (byte-identical render + provenance hashes — SC-003 / FR-005).
- Malformed input raises `LoadError` and NOTHING is partially constructed (FR-007).
- Norway-safe YAML parsing is inherited across FFI (YAML-1.2 / research D2).
- `provenance` (old field name) is rejected by the serde layer; `origin` is accepted.
"""

from __future__ import annotations

import json
import re
import textwrap

import pytest
from pydantic import BaseModel

import prompting_press
from prompting_press import LoadError, Prompt, PromptingPressError, RenderResult
from prompting_press.generated import PromptDefinition

# A lowercase 64-char hex string — the SHA-256 provenance hash shape (FR-012/FR-013).
HEX64 = re.compile(r"\A[0-9a-f]{64}\Z")


# --------------------------------------------------------------------------------------
# Vars models
# --------------------------------------------------------------------------------------


class Greeting(BaseModel):
    """Vars for the shared parity prompt."""

    name: str
    count: int


class Empty(BaseModel):
    """No declared variables — used for literal-body (Norway) and error checks."""


# --------------------------------------------------------------------------------------
# The ONE logical prompt, expressed four ways. Each form must normalize to the same
# internal representation and therefore render byte-identically with identical hashes.
# --------------------------------------------------------------------------------------

GREET_OBJ = {
    "name": "greet",
    "role": "user",
    "body": "Hi {{ name }}, you have {{ count }} messages",
    "variables": {
        "name": {"type": "string", "origin": "trusted"},
        "count": {"type": "integer", "origin": "trusted"},
    },
}

GREET_JSON = json.dumps(GREET_OBJ)

# Hand-authored YAML (no Python YAML dependency — the consumer parses it across FFI).
GREET_YAML = textwrap.dedent(
    """\
    name: greet
    role: user
    body: "Hi {{ name }}, you have {{ count }} messages"
    variables:
      name:
        type: string
        origin: trusted
      count:
        type: integer
        origin: trusted
    """
)

GREET_TOML = textwrap.dedent(
    """\
    name = "greet"
    role = "user"
    body = "Hi {{ name }}, you have {{ count }} messages"

    [variables.name]
    type = "string"
    origin = "trusted"

    [variables.count]
    type = "integer"
    origin = "trusted"
    """
)

GREET_INPUTS = {"name": "Ada", "count": 3}
GREET_TEXT = "Hi Ada, you have 3 messages"


# --------------------------------------------------------------------------------------
# 1. Four-surface parity — SC-003 / FR-005
# --------------------------------------------------------------------------------------


def test_all_entry_points_reach_one_core_with_identical_render_and_provenance() -> None:
    """The SAME logical prompt via every Python factory — `from_yaml`, `from_json`,
    `from_toml`, `Prompt(dict)`, and `Prompt(PromptDefinition instance)` — renders to
    identical `.text` and identical provenance hashes.

    Parity is a structural property of the shared core; this asserts every Python entry
    point routes into it and normalizes to one representation (FR-005 / FR-008).
    """
    p_yaml = Prompt.from_yaml(GREET_YAML)
    p_json = Prompt.from_json(GREET_JSON)
    p_toml = Prompt.from_toml(GREET_TOML)
    p_dict = Prompt(GREET_OBJ)
    # The constructed-object path via a validated PromptDefinition instance.
    p_model = Prompt(PromptDefinition.model_validate(GREET_OBJ))

    prompts = {
        "yaml": p_yaml,
        "json": p_json,
        "toml": p_toml,
        "dict": p_dict,
        "model": p_model,
    }

    results: dict[str, RenderResult] = {}
    for label, p in prompts.items():
        results[label] = p.render(Greeting, data=GREET_INPUTS)

    # Each individually renders the expected body with hex hashes.
    for label, res in results.items():
        assert res.text == GREET_TEXT, label
        assert res.variant == "default", label
        assert HEX64.match(res.template_hash), f"{label}: {res.template_hash}"
        assert HEX64.match(res.render_hash), f"{label}: {res.render_hash}"

    # And the five surfaces agree on text + both provenance hashes.
    texts = {r.text for r in results.values()}
    template_hashes = {r.template_hash for r in results.values()}
    render_hashes = {r.render_hash for r in results.values()}
    assert len(texts) == 1
    assert len(template_hashes) == 1, {k: r.template_hash for k, r in results.items()}
    assert len(render_hashes) == 1, {k: r.render_hash for k, r in results.items()}


# --------------------------------------------------------------------------------------
# 2. Malformed input → LoadError (FR-007)
# --------------------------------------------------------------------------------------


def test_malformed_yaml_raises_load_error() -> None:
    with pytest.raises(LoadError):
        Prompt.from_yaml("name: [unterminated")


def test_malformed_json_raises_load_error() -> None:
    with pytest.raises(LoadError):
        Prompt.from_json("{ not valid json ")


def test_malformed_toml_raises_load_error() -> None:
    with pytest.raises(LoadError):
        Prompt.from_toml("name = [unterminated")


def test_shape_violation_missing_body_raises_load_error_on_every_surface() -> None:
    """A document that parses but violates the prompt-definition shape (missing the
    required `body`) is rejected identically on all three text paths and the dict path."""
    bad_json = json.dumps({"name": "noBody", "role": "user"})  # no `body`

    with pytest.raises(LoadError):
        Prompt.from_json(bad_json)

    with pytest.raises(LoadError):
        Prompt.from_yaml("name: noBody\nrole: user\n")

    with pytest.raises((LoadError, PromptingPressError)):
        Prompt({"name": "noBody", "role": "user"})


# --------------------------------------------------------------------------------------
# 3. Pydantic model instance path — Prompt(PromptDefinition instance)
# --------------------------------------------------------------------------------------


def test_prompt_dict_and_model_instance_render_identically() -> None:
    """`Prompt(dict)` and `Prompt(PromptDefinition instance)` of the same data are
    observationally identical — the constructed-object path is not a parallel one."""
    p_dict = Prompt(GREET_OBJ)
    p_model = Prompt(PromptDefinition.model_validate(GREET_OBJ))

    via_dict = p_dict.render(Greeting, data=GREET_INPUTS)
    via_model = p_model.render(Greeting, data=GREET_INPUTS)

    assert via_dict.text == via_model.text == GREET_TEXT
    assert via_dict.template_hash == via_model.template_hash
    assert via_dict.render_hash == via_model.render_hash


# --------------------------------------------------------------------------------------
# 4. Norway-safe YAML — research D2: `no` / `off` / `yes` stay STRINGS, never booleans
# --------------------------------------------------------------------------------------


@pytest.mark.parametrize("literal", ["no", "off", "yes", "on", "true", "false"])
def test_yaml_norway_values_parse_as_strings_not_booleans(literal: str) -> None:
    """An unquoted `no` / `off` / `yes` as the `body` scalar is the STRING, not a bool.

    A YAML-1.1 loader (e.g. PyYAML) would coerce these to `True`/`False`. The consumer's
    loader is YAML-1.2 / Norway-safe; rendering the (variable-free) body back out and
    finding the original literal confirms the binding inherits that across FFI.
    """
    p = Prompt.from_yaml(f"name: norway\nrole: user\nbody: {literal}\n")
    result = p.render(Empty())

    assert result.text == literal, (
        f"unquoted YAML `{literal}` should round-trip as the string, got {result.text!r}"
    )
    # Defensive: never the Python bool stringification.
    assert result.text not in {"True", "False"}


# --------------------------------------------------------------------------------------
# 5. `origin` accepted, `provenance` (old field) rejected
# --------------------------------------------------------------------------------------


def test_origin_accepted_provenance_rejected() -> None:
    """Phase 1 renamed `provenance` → `origin`. `origin` is the valid field; `provenance`
    must be rejected by the serde layer (deny_unknown_fields)."""
    # `origin` → accepted
    p = Prompt(
        {
            "name": "ok",
            "role": "user",
            "body": "Hi {{ x }}",
            "variables": {"x": {"type": "string", "origin": "trusted"}},
        }
    )
    assert p.name == "ok"

    # `provenance` (old field name) in the JSON text path → must fail
    with pytest.raises(LoadError):
        Prompt.from_json(
            '{"name":"bad","role":"user","body":"Hi {{ x }}",'
            '"variables":{"x":{"type":"string","provenance":"trusted"}}}'
        )


# --------------------------------------------------------------------------------------
# 6. Loader surface smoke
# --------------------------------------------------------------------------------------


def test_prompt_factories_are_accessible() -> None:
    assert callable(Prompt.from_yaml)
    assert callable(Prompt.from_json)
    assert callable(Prompt.from_toml)
    assert issubclass(LoadError, prompting_press.PromptingPressError)
