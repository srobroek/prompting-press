"""US2 dual-input loader tests for the PyO3 binding (`prompting_press`) — spec 004, T013.

US2 lands three Python entry points into the **one** consumer loader (Q3 / FR-005):

- `Registry.load_yaml(text)` — marshal an already-read YAML document to the consumer.
- `Registry.load_json(text)` — marshal an already-read JSON document to the consumer.
- `Registry.insert(obj)`     — `depythonize` a constructed shape (a plain `dict` / Mapping),
  serialize to JSON, and feed the **same** `load_json` path.

All three return `None` and key the inserted definition by its `name`. These tests prove
the three Python surfaces all reach the shared core, that malformed input is loud and
leaves nothing partially loaded (FR-007), and that the consumer's YAML-1.2 / Norway-safe
parsing is inherited across FFI (research D2).

Parity itself is structural (one Rust core renders for every language — Principle I); what
is verified here is that each *Python* entry point routes into that core and that the four
ways to describe the same logical prompt are observationally identical (SC-003).

Observed loader API (inspected before asserting):
- `load_yaml` / `load_json` / `insert` each return `None`; they raise `LoadError` on a
  malformed document or a shape violation and insert **nothing**.
- `insert` accepts a plain `dict` / Mapping OR a Pydantic `PromptDefinition` *instance*
  directly (the binding duck-types `model_dump_json` and feeds the same consumer loader).
  Both are the FR-005 / Q3 constructed-object form and normalize to the one representation.
"""

from __future__ import annotations

import json
import re
import textwrap

import pytest
from pydantic import BaseModel

import prompting_press
from prompting_press import LoadError, Registry, UnknownPromptError, render
from prompting_press.generated import PromptDefinition

# A lowercase 64-char hex string — the SHA-256 provenance hash shape (FR-012/FR-013).
HEX64 = re.compile(r"\A[0-9a-f]{64}\Z")


# --------------------------------------------------------------------------------------
# Vars models (Pydantic — the per-language idiom; Principle VI).
# --------------------------------------------------------------------------------------


class Greeting(BaseModel):
    """Vars for the shared parity prompt."""

    name: str
    count: int


class Empty(BaseModel):
    """No declared variables — used for literal-body (Norway) and post-error checks."""


# --------------------------------------------------------------------------------------
# The ONE logical prompt, expressed four ways. Each form must normalize to the same
# internal representation and therefore render byte-identically with identical hashes.
# --------------------------------------------------------------------------------------

GREET_OBJ = {
    "name": "greet",
    "role": "user",
    "body": "Hi {{ name }}, you have {{ count }} messages",
    "variables": {
        "name": {"type": "string", "provenance": "trusted"},
        "count": {"type": "integer", "provenance": "trusted"},
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
        provenance: trusted
      count:
        type: integer
        provenance: trusted
    """
)

GREET_INPUTS = {"name": "Ada", "count": 3}
GREET_TEXT = "Hi Ada, you have 3 messages"


# --------------------------------------------------------------------------------------
# 1. Three-input (four-surface) parity — SC-003 / FR-005
# --------------------------------------------------------------------------------------


def test_all_entry_points_reach_one_core_with_identical_render_and_provenance() -> None:
    """The SAME logical prompt via every real Python entry point — `load_yaml`,
    `load_json`, `insert(dict)`, and `insert(PromptDefinition instance)` — renders to
    identical `.text` and identical provenance hashes.

    Parity is a structural property of the shared core; this asserts every *Python* entry
    point routes into it and normalizes to one representation (no parallel shape — FR-008).
    """
    reg_yaml = Registry()
    reg_yaml.load_yaml(GREET_YAML)

    reg_json = Registry()
    reg_json.load_json(GREET_JSON)

    reg_dict = Registry()
    reg_dict.insert(GREET_OBJ)

    # The constructed-object path: a validated `PromptDefinition` *instance* handed to
    # `insert` directly. The binding duck-types `model_dump_json` and feeds the result
    # into the same consumer loader as the text paths (FR-005 / Q3).
    reg_model = Registry()
    reg_model.insert(PromptDefinition.model_validate(GREET_OBJ))

    results = {
        "yaml": render(reg_yaml, "greet", Greeting, GREET_INPUTS),
        "json": render(reg_json, "greet", Greeting, GREET_INPUTS),
        "insert_dict": render(reg_dict, "greet", Greeting, GREET_INPUTS),
        "insert_model_instance": render(reg_model, "greet", Greeting, GREET_INPUTS),
    }

    # Each individually renders the expected body with hex hashes.
    for label, res in results.items():
        assert res.text == GREET_TEXT, label
        assert res.variant == "default", label
        assert HEX64.match(res.template_hash), f"{label}: {res.template_hash}"
        assert HEX64.match(res.render_hash), f"{label}: {res.render_hash}"

    # And the four surfaces agree on text + both provenance hashes.
    assert len({r.text for r in results.values()}) == 1
    assert len({r.template_hash for r in results.values()}) == 1, {
        k: r.template_hash for k, r in results.items()
    }
    assert len({r.render_hash for r in results.values()}) == 1, {
        k: r.render_hash for k, r in results.items()
    }


# --------------------------------------------------------------------------------------
# 2. Object path EQUALS text path — FR-005 (insert(dict) ≡ load_json(json.dumps(dict)))
# --------------------------------------------------------------------------------------


def test_insert_dict_equals_load_json_of_same_data() -> None:
    """`insert(d)` and `load_json(json.dumps(d))` of the same data are observationally
    identical — the constructed-object path is the text path, not a parallel one."""
    reg_obj = Registry()
    reg_obj.insert(GREET_OBJ)

    reg_txt = Registry()
    reg_txt.load_json(json.dumps(GREET_OBJ))

    via_obj = render(reg_obj, "greet", Greeting, GREET_INPUTS)
    via_txt = render(reg_txt, "greet", Greeting, GREET_INPUTS)

    assert via_obj.text == via_txt.text == GREET_TEXT
    assert via_obj.template_hash == via_txt.template_hash
    assert via_obj.render_hash == via_txt.render_hash


# --------------------------------------------------------------------------------------
# 3. Malformed input → LoadError, and NOTHING is partially loaded — FR-007
# --------------------------------------------------------------------------------------


def test_malformed_yaml_raises_load_error_and_loads_nothing() -> None:
    reg = Registry()
    with pytest.raises(LoadError):
        reg.load_yaml("name: [unterminated")

    # FR-007: a failed load inserts nothing — that name is not in the registry, so a
    # render against it is an UnknownPromptError (raised before validation/templating).
    with pytest.raises(UnknownPromptError):
        render(reg, "unterminated", Empty, {})


def test_malformed_json_raises_load_error_and_loads_nothing() -> None:
    reg = Registry()
    with pytest.raises(LoadError):
        reg.load_json("{ not valid json ")

    with pytest.raises(UnknownPromptError):
        render(reg, "greet", Empty, {})


def test_shape_violation_missing_body_raises_load_error_on_every_surface() -> None:
    """A document that parses but violates the prompt-definition shape (missing the
    required `body`) is rejected identically on the JSON text path and the object path."""
    bad = {"name": "noBody", "role": "user"}  # no `body`

    reg_json = Registry()
    with pytest.raises(LoadError):
        reg_json.load_json(json.dumps(bad))

    reg_insert = Registry()
    with pytest.raises(LoadError):
        reg_insert.insert(bad)

    # Same shape violation expressed in YAML.
    reg_yaml = Registry()
    with pytest.raises(LoadError):
        reg_yaml.load_yaml("name: noBody\nrole: user\n")

    # None of the three left a usable entry behind (FR-007).
    for reg in (reg_json, reg_insert, reg_yaml):
        with pytest.raises(UnknownPromptError):
            render(reg, "noBody", Empty, {})


def test_failed_reload_does_not_corrupt_an_existing_entry() -> None:
    """A malformed re-load that reuses an existing name must leave the prior, good entry
    intact — the failed load is atomic (FR-007: nothing inserted on error)."""
    reg = Registry()
    reg.insert(
        {
            "name": "keep",
            "role": "user",
            "body": "Hi {{ n }}",
            "variables": {"n": {"type": "string", "provenance": "trusted"}},
        }
    )

    class V(BaseModel):
        n: str

    assert render(reg, "keep", V, {"n": "Ada"}).text == "Hi Ada"

    # A malformed document keyed by the same logical name fails and changes nothing.
    with pytest.raises(LoadError):
        reg.load_yaml("keep: [bad")

    # The original entry still renders unchanged.
    assert render(reg, "keep", V, {"n": "Bo"}).text == "Hi Bo"


def test_insert_accepts_a_pydantic_model_instance() -> None:
    """`insert` accepts a constructed `PromptDefinition` *instance* directly — the
    FR-005 / Q3 "constructed object" input form. The binding duck-types `model_dump_json`
    and routes the instance through the **same** consumer loader as the text paths, so it
    renders identically to `insert(dict)` of the same data."""
    reg_model = Registry()
    assert reg_model.insert(PromptDefinition.model_validate(GREET_OBJ)) is None

    reg_dict = Registry()
    reg_dict.insert(GREET_OBJ)

    via_model = render(reg_model, "greet", Greeting, GREET_INPUTS)
    via_dict = render(reg_dict, "greet", Greeting, GREET_INPUTS)

    assert via_model.text == via_dict.text == GREET_TEXT
    assert via_model.template_hash == via_dict.template_hash
    assert via_model.render_hash == via_dict.render_hash


# --------------------------------------------------------------------------------------
# 4. Norway-safe YAML — research D2: `no` / `off` / `yes` stay STRINGS, never booleans
# --------------------------------------------------------------------------------------


@pytest.mark.parametrize("literal", ["no", "off", "yes", "on", "true", "false"])
def test_yaml_norway_values_parse_as_strings_not_booleans(literal: str) -> None:
    """An unquoted `no` / `off` / `yes` as the `body` scalar is the STRING, not a bool.

    A YAML-1.1 loader (e.g. PyYAML) would coerce these to `True`/`False`, which would
    either violate the `body: str` shape or render as `"True"`/`"False"`. The consumer's
    loader is YAML-1.2 / Norway-safe; rendering the (variable-free) body back out and
    finding the original literal confirms the binding inherits that across FFI.
    """
    reg = Registry()
    reg.load_yaml(f"name: norway\nrole: user\nbody: {literal}\n")

    result = render(reg, "norway", Empty, {})

    assert result.text == literal, (
        f"unquoted YAML `{literal}` should round-trip as the string, got {result.text!r}"
    )
    # Defensive: never the Python bool stringification.
    assert result.text not in {"True", "False"}


# --------------------------------------------------------------------------------------
# 5. Loader surface smoke — return contract and module exposure
# --------------------------------------------------------------------------------------


def test_loaders_return_none_and_key_by_name() -> None:
    """The three loaders return `None` (insertion is a side effect; the registry keys by
    the document's own `name`)."""
    reg = Registry()
    assert reg.load_json(GREET_JSON) is None
    assert reg.load_yaml(GREET_YAML) is None  # same name ⇒ replaces, still None
    assert reg.insert(GREET_OBJ) is None

    # The single declared name renders; an undeclared one is unknown.
    assert render(reg, "greet", Greeting, GREET_INPUTS).text == GREET_TEXT
    with pytest.raises(UnknownPromptError):
        render(reg, "absent", Greeting, GREET_INPUTS)


def test_module_exposes_us2_loader_surface() -> None:
    assert hasattr(prompting_press.Registry, "load_yaml")
    assert hasattr(prompting_press.Registry, "load_json")
    assert hasattr(prompting_press.Registry, "insert")
    assert issubclass(LoadError, prompting_press.PromptingPressError)
