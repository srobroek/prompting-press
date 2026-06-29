#!/usr/bin/env python3
"""
extract-python-api.py — Python API-doc extractor for prompting-press (spec 011).

Produces the API-doc IR (contracts/api-doc-ir.md) for the Python binding by
combining:
  1. Python runtime introspection (inspect) of the compiled PyO3 extension (.so)
     for all non-shape public symbols — griffe cannot walk compiled .so files, so
     inspect is the only way to reach PyO3 class/method doc comments and signatures.
  2. griffe source parsing for the generated Pydantic shape types (PromptDefinition,
     PromptVariable, PromptVariant), which are pure Python source — but these are
     shapeRef symbols (FR-010) so their members are NOT expanded.

Invocation (as recorded in research.md R1b):
  uv run --with griffe==2.1.0 python3 docs/site/scripts/extract-python-api.py \
    --package packages/python/python/prompting_press \
    --out <path-to-ir.json>

Or standalone (emits to stdout):
  uv run --with griffe==2.1.0 --with pydantic --project packages/python \
    python3 docs/site/scripts/extract-python-api.py \
    --package packages/python/python/prompting_press

Jargon-stripping rules:
  The strip-jargon.mjs regexes are replicated EXACTLY in _strip_jargon() below.
  Both implementations must stay in sync; a comment marks each regex with its
  JS counterpart from strip-jargon.mjs.
"""

from __future__ import annotations

import argparse
import inspect
import json
import re
import sys
from pathlib import Path
from typing import Any

# ---------------------------------------------------------------------------
# Jargon-strip + MDX-escape — exact mirror of strip-jargon.mjs
#
# The JS functions are:
#   stripJargon(str)  — drops parenthetical jargon citations and bare trailing refs
#   escapeCell(str)   — escapes pipes and newlines (for Markdown table cells)
#
# The Python functions below replicate the SAME regexes.  If strip-jargon.mjs is
# updated its regexes MUST be propagated here, and vice versa.
# ---------------------------------------------------------------------------


def _strip_jargon(s: str | None) -> str:
    """Mirror of strip-jargon.mjs stripJargon()."""
    if s is None:
        return ""
    # JS: /\s*\((?:constitution\s+|roadmap\s+decision\s+)?(?:Principle\s+[IVXLC]+|FR-[0-9A-Za-z]+|SC-[0-9]+|SEC-[0-9]+|C-[0-9]+|spec\s+[0-9]+)[^)]*\)/gi
    s = re.sub(
        r"\s*\((?:constitution\s+|roadmap\s+decision\s+)?(?:Principle\s+[IVXLC]+|FR-[0-9A-Za-z]+|SC-[0-9]+|SEC-[0-9]+|C-[0-9]+|spec\s+[0-9]+)[^)]*\)",
        "",
        s,
        flags=re.IGNORECASE,
    )
    # JS: /\s*\(renamed from `provenance` in spec [0-9]+\)/gi
    s = re.sub(
        r"\s*\(renamed from `provenance` in spec [0-9]+\)",
        "",
        s,
        flags=re.IGNORECASE,
    )
    # JS: /\s*(?:per\s+)?(?:roadmap decision\s+C-[0-9]+|constitution Principle\s+[IVXLC]+)\b[^.]*/gi
    s = re.sub(
        r"\s*(?:per\s+)?(?:roadmap decision\s+C-[0-9]+|constitution Principle\s+[IVXLC]+)\b[^.]*",
        "",
        s,
        flags=re.IGNORECASE,
    )
    # JS: /\s{2,}/g   → " "
    s = re.sub(r"\s{2,}", " ", s).strip()
    return s


def _escape_cell(s: str | None) -> str:
    """Mirror of strip-jargon.mjs escapeCell()."""
    if s is None:
        return ""
    return s.replace("|", "\\|").replace("\n", " ")


def _sanitize_doc(raw: str | None) -> str | None:
    """
    Apply jargon-stripping to a full doc string (may be multi-line).
    Returns None iff the input is None or empty after stripping.
    Pipes and newlines are NOT escaped here — escapeCell is for table cells only;
    the rendered API ref page uses free-form MDX prose, not table cells.
    MDX brace-escaping: curly braces in MDX-rendered prose are safe as long as
    they appear in fenced code blocks or as literal characters in paragraphs —
    the renderer wraps signatures in code fences, so we only need to escape
    braces that appear in freeform doc prose outside code fences.
    """
    if raw is None:
        return None
    stripped = _strip_jargon(raw)
    if not stripped:
        return None
    return stripped


# ---------------------------------------------------------------------------
# API_GROUPS — canonical order (mirrors api-groups.mjs API_GROUPS)
# The extractor assigns every __all__ symbol to exactly one group title.
# ---------------------------------------------------------------------------

API_GROUPS: list[dict[str, Any]] = [
    {"title": "Prompt", "anchor": "prompt", "blurb": None},
    {"title": "RenderResult", "anchor": "render-result", "blurb": None},
    {"title": "GuardConfig", "anchor": "guard-config", "blurb": None},
    {"title": "CheckReport", "anchor": "check-report", "blurb": None},
    {"title": "Finding", "anchor": "finding", "blurb": None},
    {"title": "Composition", "anchor": "composition", "blurb": None},
    {"title": "Message", "anchor": "message", "blurb": None},
    {"title": "Errors", "anchor": "errors", "blurb": None},
    {
        "title": "Shape types",
        "anchor": "shape-types",
        "blurb": None,
    },
]

# Maps a public name → group title.  Every name in __all__ MUST appear here.
_GROUP_FOR: dict[str, str] = {
    "Prompt": "Prompt",
    "RenderResult": "RenderResult",
    "GuardConfig": "GuardConfig",
    "CheckReport": "CheckReport",
    "Finding": "Finding",
    "Composition": "Composition",
    "Message": "Message",
    "core_version": "Prompt",  # utility attached to the Prompt section
    "FieldError": "Errors",
    "PromptingPressError": "Errors",
    "PromptValidationError": "Errors",
    "PromptRenderError": "Errors",
    "LoadError": "Errors",
    # Re-exported shape types → link to prompt-definition.mdx, not re-rendered
    "PromptDefinition": "Shape types",
    "PromptVariable": "Shape types",
    "PromptVariant": "Shape types",
}

# Shape types that must carry shapeRef instead of being expanded (FR-010)
_SHAPE_REFS: set[str] = {"PromptDefinition", "PromptVariable", "PromptVariant"}

# Kind vocabulary for this language (IR Symbol.kind)
_KIND_CLASS = "class"
_KIND_FUNCTION = "function"
_KIND_METHOD = "method"
_KIND_ACCESSOR = "accessor"  # read-only property / getset_descriptor


# ---------------------------------------------------------------------------
# Signature helpers
# ---------------------------------------------------------------------------


def _kind_of_member(cls: type, mname: str) -> str:
    """Return the IR kind for a member of a class."""
    raw = cls.__dict__.get(mname)
    if raw is None:
        return _KIND_METHOD
    t = type(raw).__name__
    if t in ("getset_descriptor", "member_descriptor", "property"):
        return _KIND_ACCESSOR
    if t in ("classmethod_descriptor", "classmethod"):
        return _KIND_METHOD
    if t == "staticmethod":
        return _KIND_METHOD
    if t == "method_descriptor":
        return _KIND_METHOD
    if callable(raw):
        return _KIND_METHOD
    return _KIND_ACCESSOR


def _sig_for_member(cls: type, mname: str) -> str:
    """
    Return a Python-native signature string for a class member.
    Falls back to a minimal "(self)" form rather than crashing.
    """
    mobj = getattr(cls, mname, None)
    if mobj is None:
        return f"{mname}(self)"

    raw = cls.__dict__.get(mname)
    t = type(raw).__name__ if raw is not None else ""

    # getset_descriptor / property: no callable signature; show as read-only attr
    if t in ("getset_descriptor", "member_descriptor", "property"):
        return f"{mname}: <read-only property>"

    try:
        sig = inspect.signature(mobj)
        return f"{mname}{sig}"
    except (ValueError, TypeError):
        return f"{mname}(self)"


def _sig_for_top_level(obj: Any, name: str) -> str:
    """Return a Python-native signature for a top-level __all__ symbol."""
    if inspect.isclass(obj):
        # For PyO3 classes the constructor is __init__ or __new__; surface as
        # ClassName(shape, *, validators=None) if detectable, else just the class name.
        try:
            init = getattr(obj, "__init__", None)
            if init and init is not object.__init__:
                sig = inspect.signature(init)
                # Remove the leading `self` parameter for display
                params = list(sig.parameters.values())
                if params and params[0].name == "self":
                    params = params[1:]
                new_sig = sig.replace(parameters=params)
                return f"class {name}{new_sig}"
        except (ValueError, TypeError):
            pass
        return f"class {name}"
    elif callable(obj):
        try:
            sig = inspect.signature(obj)
            return f"def {name}{sig}"
        except (ValueError, TypeError):
            return f"def {name}()"
    else:
        return str(name)


# ---------------------------------------------------------------------------
# Member extraction
# ---------------------------------------------------------------------------


def _extract_members(cls: type) -> list[dict[str, Any]]:
    """
    Extract public members of a PyO3 class as IR Symbol dicts.
    Returns members sorted by (kind, name) as required by the IR contract.
    """
    members: list[dict[str, Any]] = []

    for mname in dir(cls):
        if mname.startswith("_"):
            continue

        mobj = getattr(cls, mname, None)
        raw_in_dict = cls.__dict__.get(mname)

        # Skip things that are clearly not part of the class's own surface
        # (e.g. methods inherited from Exception/BaseException that aren't re-declared)
        if raw_in_dict is None and mobj is not None:
            # Inherited from Python builtins — skip unless it's on a known class
            # Check: is it actually defined in this class's __dict__ hierarchy?
            for base in cls.__mro__:
                if mname in base.__dict__:
                    if base not in (object, Exception, BaseException):
                        # It's from a meaningful parent — include it
                        break
                    else:
                        # Only on builtins — skip
                        mobj = None
                        break

        if mobj is None:
            continue

        kind = _kind_of_member(cls, mname)
        sig = _sig_for_member(cls, mname)
        doc_raw = getattr(mobj, "__doc__", None)
        if doc_raw is None and raw_in_dict is not None:
            doc_raw = getattr(raw_in_dict, "__doc__", None)
        doc = _sanitize_doc(doc_raw)

        members.append(
            {
                "name": mname,
                "kind": kind,
                "signature": sig,
                "doc": doc,
                "members": [],
                "shapeRef": None,
                "deprecated": None,
            }
        )

    # Sort by (kind, name) — deterministic order required by IR contract
    members.sort(key=lambda m: (m["kind"], m["name"]))
    return members


def _extract_error_members(cls: type) -> list[dict[str, Any]]:
    """
    For exception classes: extract only the members declared in this class
    (not inherited from Exception/BaseException), plus `.errors` if present.
    """
    members: list[dict[str, Any]] = []
    own = set(cls.__dict__.keys())

    for mname in sorted(own):
        if mname.startswith("_"):
            continue
        mobj = cls.__dict__[mname]
        kind = _kind_of_member(cls, mname)
        sig = _sig_for_member(cls, mname)
        doc_raw = getattr(mobj, "__doc__", None)
        doc = _sanitize_doc(doc_raw)
        members.append(
            {
                "name": mname,
                "kind": kind,
                "signature": sig,
                "doc": doc,
                "members": [],
                "shapeRef": None,
                "deprecated": None,
            }
        )

    members.sort(key=lambda m: (m["kind"], m["name"]))
    return members


# ---------------------------------------------------------------------------
# Top-level symbol extraction
# ---------------------------------------------------------------------------

_ERROR_CLASSES = {
    "PromptingPressError",
    "PromptValidationError",
    "PromptRenderError",
    "LoadError",
}

# Classes whose members should NOT be expanded (pure data + errors on exception)
# field-bearing classes get full member expansion; error classes get minimal
_FIELD_CLASSES = {
    "Prompt",
    "RenderResult",
    "GuardConfig",
    "CheckReport",
    "Finding",
    "Composition",
    "Message",
    "FieldError",
}


def _extract_symbol(name: str, obj: Any) -> dict[str, Any]:
    """
    Build one IR Symbol dict for a top-level __all__ name.
    """
    is_shape_ref = name in _SHAPE_REFS

    if is_shape_ref:
        # FR-010: shape types link to prompt-definition.mdx; members not expanded.
        # The generated Pydantic classes have no class-level docstring in source
        # (code-generated). doc stays None per FR-008 — the gate will flag them,
        # which is correct: the shape page is the source of truth, not a class doc.
        sig = f"class {name}"
        doc = _sanitize_doc(getattr(obj, "__doc__", None))
        return {
            "name": name,
            "kind": _KIND_CLASS,
            "signature": sig,
            "doc": doc,
            "members": [],
            "shapeRef": name,
            "deprecated": None,
        }

    sig = _sig_for_top_level(obj, name)
    doc_raw = getattr(obj, "__doc__", None)
    doc = _sanitize_doc(doc_raw)

    if not inspect.isclass(obj):
        # top-level function (e.g. core_version)
        return {
            "name": name,
            "kind": _KIND_FUNCTION,
            "signature": sig,
            "doc": doc,
            "members": [],
            "shapeRef": None,
            "deprecated": None,
        }

    # Class: extract members
    if name in _ERROR_CLASSES:
        members = _extract_error_members(obj)
    else:
        members = _extract_members(obj)

    return {
        "name": name,
        "kind": _KIND_CLASS,
        "signature": sig,
        "doc": doc,
        "members": members,
        "shapeRef": None,
        "deprecated": None,
    }


# ---------------------------------------------------------------------------
# Main extractor
# ---------------------------------------------------------------------------


def extract(package_path: Path, version: str = "latest") -> dict[str, Any]:
    """
    Walk prompting_press.__all__ and produce the ApiDoc IR.

    package_path: path to the prompting_press package directory
                  (e.g. packages/python/python/prompting_press)
    version:      version string passed through to the IR (default "latest")
    """
    # Insert the parent of the package dir onto sys.path so `import prompting_press`
    # finds it even when pydantic is not installed in the current interpreter.
    pkg_parent = str(package_path.parent.resolve())
    if pkg_parent not in sys.path:
        sys.path.insert(0, pkg_parent)

    try:
        import prompting_press as pp  # noqa: PLC0415
    except ImportError as exc:
        sys.exit(
            f"ERROR: could not import prompting_press from {pkg_parent!r}.\n"
            f"  Ensure griffe + pydantic are available in the uv environment.\n"
            f"  Original error: {exc}\n"
            "  Hint: uv run --with griffe==2.1.0 --with pydantic --project packages/python ..."
        )

    public_names: list[str] = getattr(pp, "__all__", [])

    # Validate all names in __all__ have a group assignment
    unknown = [n for n in public_names if n not in _GROUP_FOR]
    if unknown:
        sys.exit(
            f"ERROR: extract-python-api: __all__ contains names with no group "
            f"assignment: {unknown!r}.\n"
            "  Update _GROUP_FOR in extract-python-api.py."
        )

    # Build per-group symbol lists
    # groups_map: title → list of Symbol dicts
    groups_map: dict[str, list[dict[str, Any]]] = {g["title"]: [] for g in API_GROUPS}

    for name in public_names:
        obj = getattr(pp, name, None)
        if obj is None:
            sys.exit(
                f"ERROR: {name!r} is in __all__ but not importable from prompting_press."
            )
        group_title = _GROUP_FOR[name]
        symbol = _extract_symbol(name, obj)
        groups_map[group_title].append(symbol)

    # Sort symbols within each group by (kind, name) — IR contract R5
    for title, syms in groups_map.items():
        syms.sort(key=lambda s: (s["kind"], s["name"]))

    # Build groups array in canonical API_GROUPS order (empty groups still emitted —
    # FR-009 parallel structure)
    groups: list[dict[str, Any]] = []
    for gdef in API_GROUPS:
        groups.append(
            {
                "title": gdef["title"],
                "anchor": gdef["anchor"],
                "blurb": gdef["blurb"],
                "symbols": groups_map[gdef["title"]],
            }
        )

    return {
        "language": "python",
        "package": "prompting-press",
        "version": version,
        "generatedFrom": "griffe 2.1.0",
        "groups": groups,
    }


# ---------------------------------------------------------------------------
# CLI entry point
# ---------------------------------------------------------------------------


def _parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Extract the prompting-press Python public API into API-doc IR JSON.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=(
            "Example:\n"
            "  uv run --with griffe==2.1.0 python3 docs/site/scripts/extract-python-api.py \\\n"
            "    --package packages/python/python/prompting_press \\\n"
            "    --out /tmp/ir-python.json\n"
            "\n"
            "  Emit to stdout (omit --out):\n"
            "  uv run --with griffe==2.1.0 python3 docs/site/scripts/extract-python-api.py \\\n"
            "    --package packages/python/python/prompting_press\n"
        ),
    )
    parser.add_argument(
        "--package",
        metavar="PATH",
        default="packages/python/python/prompting_press",
        help=(
            "Path to the prompting_press package directory "
            "(default: packages/python/python/prompting_press)"
        ),
    )
    parser.add_argument(
        "--out",
        metavar="FILE",
        default=None,
        help="Write IR JSON to FILE instead of stdout.",
    )
    parser.add_argument(
        "--version",
        metavar="VER",
        default="latest",
        help='Version string to embed in the IR (default: "latest"; see FR-016).',
    )
    return parser.parse_args()


def main() -> None:
    args = _parse_args()
    package_path = Path(args.package)
    if not package_path.exists():
        sys.exit(
            f"ERROR: package path does not exist: {package_path}\n"
            "  Pass --package <path-to-prompting_press-dir>."
        )

    ir = extract(package_path, version=args.version)
    output = json.dumps(ir, indent=2, ensure_ascii=False)

    if args.out:
        out_path = Path(args.out)
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(output, encoding="utf-8")
        print(f"Wrote {out_path} ({len(ir['groups'])} groups)", file=sys.stderr)
    else:
        print(output)


if __name__ == "__main__":
    main()
