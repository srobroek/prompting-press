"""Spec 006 — schema round-trip conformance corpus, Python binding (`prompting_press`).

This is a TEST HARNESS, not engine logic. It drives the shared
`conformance/schema/manifest.json` fixtures through the binding's **real** loaders
(``Registry.load_json`` / ``Registry.load_yaml``) and asserts each reaches its
manifest ``verdict`` — an ``accept`` doc loads cleanly; a ``reject`` doc raises the
binding's normalized ``LoadError`` with no partial load and no crash.

What it guards (Principle VII): that the Python loader's accept/reject verdict for the
spec-001 prompt-definition shape matches the other bindings' across both the JSON and the
YAML input paths. Note (per the manifest's own description) the loaders do serde SHAPE
deserialization, not full JSON-Schema validation — so this pins the *loader's* verdict
(the binding-observable round-trip), which is intentionally looser than spec-001's
`validate-fixtures` gate.

Security hygiene baked into the harness:
  - SEC-001: a fixture ``path`` MUST be repo-relative and resolve within the repo root.
    An absolute path or any ``..`` segment is rejected by the harness itself (it never
    reaches the filesystem read), so a malicious manifest cannot escape the tree.
  - SEC-002: a ``reject`` is asserted on the EXCEPTION TYPE (`LoadError`) only — never on
    the message text, which may carry scrubbed/structured detail we must not couple to.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import pytest

from prompting_press import LoadError, Registry


def _repo_root() -> Path:
    """Walk up from this file until the directory containing ``conformance/`` is found."""
    for parent in Path(__file__).resolve().parents:
        if (parent / "conformance").is_dir():
            return parent
    raise RuntimeError("could not locate repo root (no `conformance/` ancestor)")


REPO_ROOT = _repo_root()
MANIFEST = REPO_ROOT / "conformance" / "schema" / "manifest.json"


def _safe_resolve(rel_path: str) -> Path:
    """Resolve a manifest-declared, repo-relative path under the repo root (SEC-001).

    Rejects absolute paths and any ``..`` segment BEFORE touching the filesystem, then
    double-checks the resolved real path is contained within the repo root. A failure
    here is a manifest-hygiene error, surfaced as a test failure (not silently skipped).
    """
    pure = Path(rel_path)
    assert not pure.is_absolute(), f"SEC-001: fixture path must be relative, got {rel_path!r}"
    assert ".." not in pure.parts, (
        f"SEC-001: fixture path must not contain `..`, got {rel_path!r}"
    )

    resolved = (REPO_ROOT / pure).resolve()
    root = REPO_ROOT.resolve()
    assert resolved == root or root in resolved.parents, (
        f"SEC-001: resolved path escaped the repo root: {resolved}"
    )
    return resolved


def _load_manifest() -> list[dict[str, Any]]:
    data = json.loads(MANIFEST.read_text(encoding="utf-8"))
    fixtures = data["fixtures"]
    assert fixtures, f"no schema fixtures listed in {MANIFEST}"
    return fixtures


_FIXTURES = _load_manifest()


@pytest.mark.parametrize(
    "fixture",
    _FIXTURES,
    ids=[f"{f['verdict']}-{f['form']}-{Path(f['path']).stem}" for f in _FIXTURES],
)
def test_schema_fixture_round_trips(fixture: dict[str, Any]) -> None:
    """Each schema fixture reaches its manifest verdict through the matching loader.

    `accept` ⇒ the loader does not raise; `reject` ⇒ the loader raises `LoadError`
    (asserted on type only — SEC-002) and inserts nothing partial.
    """
    form = fixture["form"]
    verdict = fixture["verdict"]
    assert form in ("json", "yaml"), f"unexpected form: {form!r}"
    assert verdict in ("accept", "reject"), f"unexpected verdict: {verdict!r}"

    doc_path = _safe_resolve(fixture["path"])
    text = doc_path.read_text(encoding="utf-8")

    reg = Registry()  # a fresh registry per fixture — no cross-contamination
    load = reg.load_json if form == "json" else reg.load_yaml

    if verdict == "accept":
        # Must NOT raise. Any exception (including LoadError) is a failure for an accept doc.
        load(text)
    else:
        # Must raise the binding's normalized LoadError — assert on TYPE, never message (SEC-002).
        with pytest.raises(LoadError):
            load(text)
