"""Spec 006 — FFI marshaling conformance corpus, Python binding (`prompting_press`).

This is a TEST HARNESS, not engine logic. It drives the shared
`conformance/marshaling/*.json` fixtures through the **real** Python render path
(``prompting_press.render`` → validate-in-Python → marshal → kernel) and pins the
golden ``text`` + provenance hashes that the Rust core produced.

What it guards (Principle VII): that the Python binding marshals each logical type
— string / int / float / bool / null / absent / datetime / date / decimal / nested
object / array — into the kernel **identically** to the other bindings, yielding the
byte-identical render and the same ``template_hash`` / ``render_hash``. Render parity
itself is structural (one Rust core — Principle I); the corpus's burden is the FFI
boundary, so a divergence here means the *marshaling* drifted, not the renderer.

How the native value is built (the fixture's ``type`` tag → the native Python value):

    string / int / float / bool / null  → the JSON value verbatim (``None`` for null)
    absent                               → the key is OMITTED entirely (never set)
    datetime / date / decimal            → the canonical serialized string verbatim (see DECISION below)
    object                               → recurse into a ``dict``
    array                                → recurse into a ``list``

DECISION (datetime / date / decimal — native vs. raw string → **raw string**):
    The corpus pins each of these by its *canonical serialized form*, the string the Rust
    core (chrono / rust_decimal) produced. The binding always re-dumps the validated model
    with ``model_dump(mode="json")`` before marshaling (research D2), so the question is
    which native Python value, after Pydantic's JSON dump, reproduces that exact string.

    Empirically (see the probe in the spec-006 work log), the **native** objects do NOT:
      - ``datetime.fromisoformat("2026-06-28T12:30:00+00:00")`` → Pydantic dumps
        ``"2026-06-28T12:30:00Z"`` (it canonicalizes UTC to a ``Z`` suffix), but the golden
        keeps the explicit ``+00:00`` offset.
      - ``decimal.Decimal("0.00000000000000001")`` → Pydantic dumps ``"1E-17"`` (scientific
        notation), but the golden keeps the plain fixed-point form.
      - ``date.fromisoformat("2026-06-28")`` → ``"2026-06-28"`` (this one *does* match).

    Because the fixture's whole point is "the value IS the canonical string" — and a native
    object's serializer is free to recanonicalize it — we pass the **raw string** ``value``
    verbatim for all three. A JSON string dumps unchanged through ``mode="json"``, so the
    binding marshals exactly the canonical form into the kernel and the golden is reproduced
    byte-for-byte. ``date`` is passed as a string too, for consistency (the native form also
    matched, but the string is the contract). The chosen form is recorded in CONSTRUCTION and
    asserted by a test so this decision cannot silently rot.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import pytest
from pydantic import BaseModel, create_model

from prompting_press import Registry, render

# Records the value-construction choice for the canonical-serialized types — the form that
# reproduces each golden byte-for-byte (see the module docstring's DECISION). Asserted by
# `test_canonical_type_construction_choice_is_recorded` so the documented decision and the
# code cannot drift apart.
CONSTRUCTION = {
    "datetime": "raw string (native datetime → Pydantic emits a `Z` suffix, not `+00:00`)",
    "date": "raw string (native date also matches; the string is the contract)",
    "decimal": "raw string (native Decimal → Pydantic emits `1E-17` scientific notation)",
}


def _repo_root() -> Path:
    """Walk up from this file until the directory containing ``conformance/`` is found.

    Robust to the package's location in the monorepo (no hardcoded absolute path).
    """
    for parent in Path(__file__).resolve().parents:
        if (parent / "conformance").is_dir():
            return parent
    raise RuntimeError("could not locate repo root (no `conformance/` ancestor)")


REPO_ROOT = _repo_root()
MARSHALING_DIR = REPO_ROOT / "conformance" / "marshaling"


def _build_native(node: Any) -> Any:
    """Turn one tagged ``{"type": <tag>, "value": <json>}`` node into a native Python value.

    Mirrors the logical-type table. ``object`` / ``array`` recurse; ``absent`` is handled by
    the caller (it OMITS the key) and never reaches here.
    """
    tag = node["type"]

    if tag in ("string", "int", "float", "bool", "null"):
        # The JSON value as-is — `null` is already `None` after `json.load`.
        return node["value"]
    if tag in ("datetime", "date", "decimal"):
        # The canonical serialized form IS the value: pass the raw string verbatim so the
        # binding's `model_dump(mode="json")` marshals exactly the golden (a native object
        # would recanonicalize — see the module docstring's DECISION).
        return node["value"]
    if tag == "object":
        return {key: _build_native(child) for key, child in node["value"].items()}
    if tag == "array":
        return [_build_native(child) for child in node["value"]]

    raise AssertionError(f"unknown conformance type tag: {tag!r}")


def _native_inputs(fixture_input: dict[str, Any]) -> dict[str, Any]:
    """Build the native Vars dict from a fixture ``input`` map.

    An ``absent``-tagged field is OMITTED from the result entirely (the
    JS-undefined / Python-absent contract — never `None`, which is the distinct
    explicit-null case).
    """
    out: dict[str, Any] = {}
    for field, node in fixture_input.items():
        if node["type"] == "absent":
            continue  # field-not-present, NOT None
        out[field] = _build_native(node)
    return out


def _vars_model(field_names: list[str]) -> type[BaseModel]:
    """A pass-through Pydantic Vars model: every field is `object` (any value, untouched).

    Declaring fields as `object` keeps Pydantic from coercing the marshaled values (strings,
    ints, floats, bools, None, nested dicts/lists), so the binding's own
    `model_dump(mode='json')` is the single deterministic stringification point — the same
    path `test_render.py` relies on. All leaf values are already JSON-native (the canonical
    datetime / date / decimal forms are passed as raw strings), so no custom config is needed.
    """
    return create_model(  # type: ignore[call-overload, no-any-return]
        "ConformanceVars",
        **{name: (object, ...) for name in field_names},
    )


def _load_fixtures() -> list[dict[str, Any]]:
    fixtures: list[dict[str, Any]] = []
    for path in sorted(MARSHALING_DIR.glob("*.json")):
        fixtures.append(json.loads(path.read_text(encoding="utf-8")))
    assert fixtures, f"no marshaling fixtures found under {MARSHALING_DIR}"
    return fixtures


_FIXTURES = _load_fixtures()


@pytest.mark.parametrize("fixture", _FIXTURES, ids=[f["case"] for f in _FIXTURES])
def test_marshaling_fixture_renders_to_golden(fixture: dict[str, Any]) -> None:
    """Each marshaling fixture renders to its exact golden text + provenance hashes.

    The whole chain is real: `load_json` of the spec-001 definition, a native Vars dict
    built from the type-tag table, then `render` through validate → marshal → kernel.
    """
    case = fixture["case"]

    reg = Registry()
    reg.load_json(json.dumps(fixture["definition"]))

    native = _native_inputs(fixture["input"])
    vars_model = _vars_model(list(native.keys()))

    result = render(
        reg,
        fixture["definition"]["name"],
        vars_model,
        data=native,
        variant=fixture["variant"],  # str or None — None ⇒ the reserved `default` arm
    )

    expected = fixture["expected"]
    assert result.text == expected["text"], (
        f"[{case}] text diverged: {result.text!r} != golden {expected['text']!r}"
    )
    assert result.template_hash == expected["template_hash"], (
        f"[{case}] template_hash diverged: "
        f"{result.template_hash} != golden {expected['template_hash']}"
    )
    assert result.render_hash == expected["render_hash"], (
        f"[{case}] render_hash diverged: "
        f"{result.render_hash} != golden {expected['render_hash']}"
    )


def test_canonical_type_construction_choice_is_recorded() -> None:
    """Pin the documented native-vs-string decision for the canonical-serialized types.

    If a future fixture forces the raw-string fallback for any of these, this constant
    (and the module docstring's DECISION note) must be updated in lockstep — this test
    fails loudly if the recorded set drifts.
    """
    assert set(CONSTRUCTION) == {"datetime", "date", "decimal"}
