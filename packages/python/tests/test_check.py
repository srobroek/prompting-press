"""Agreement + provenance lint tests for the PyO3 binding — spec 008 Phase 4.

The spec 008 reshape moves the lint from `check(reg)` (registry-keyed free function)
to `prompt.check()` (per-Prompt method). These tests prove every documented finding
`kind` is reachable from Python via `prompt.check()`, that `CheckReport` collection
protocol works (`passed` / `is_empty` / `len` / `bool`), and that construction-time
invariants (undeclared variables, reserved variant names, excluded features) are caught
at `Prompt(...)` construction, not deferred to a lint pass.

Key behavioral changes from the pre-reshape suite:
- `undeclared_variable` and `reserved_variant_name` findings are now CONSTRUCTION
  errors (Prompt(...) raises) — they are no longer lint findings. Only
  `untrusted_without_guard` is a post-construction advisory from `prompt.check()`.
- `analysis_error` (excluded template feature like `{% include %}`) is also a
  CONSTRUCTION error post-reshape.
- `check()` on a constructed Prompt returns at most `untrusted_without_guard` findings.
- Multiple-prompt determinism: iterate a list of Prompt objects and collect findings.

Observed `kind` strings:
- `untrusted_without_guard` → `.variant is None` (a prompt-level finding).
"""

from __future__ import annotations

import pytest
from pydantic import BaseModel

import prompting_press
from prompting_press import (
    Prompt,
    PromptingPressError,
)

# --------------------------------------------------------------------------------------
# The stable finding `kind` strings (the binding's public vocabulary). Asserted by value
# so a rename in the core is caught here.
# --------------------------------------------------------------------------------------

KIND_UNTRUSTED = "untrusted_without_guard"


def _kinds(report) -> list[str]:
    return [f.kind for f in report.findings]


def _signature(report) -> list[tuple[str, str, str | None]]:
    """A stable, comparable projection of a report's findings: (kind, prompt, variant)."""
    return [(f.kind, f.prompt, f.variant) for f in report.findings]


# --------------------------------------------------------------------------------------
# 1. Clean prompt passes — the no-findings contract (FR-019 baseline)
# --------------------------------------------------------------------------------------


def test_clean_prompt_passes_with_empty_report() -> None:
    """A well-formed prompt — every referenced var declared, no untrusted-without-guard,
    no reserved variant — produces no findings. The report's whole collection protocol
    agrees: passed / empty / len 0 / falsy."""
    p = Prompt(
        {
            "name": "greet",
            "role": "user",
            "body": "Hi {{ name }}, you have {{ count }} messages",
            "variables": {
                "name": {"type": "string", "trusted": True},
                "count": {"type": "integer", "trusted": True},
            },
        }
    )
    report = p.check()

    assert report.passed() is True
    assert report.is_empty() is True
    assert list(report.findings) == []
    assert len(report) == 0
    assert not bool(report)  # empty ⇒ falsy (inverse of passed())


def test_prompt_with_no_variables_passes() -> None:
    """A prompt with no variables and no body references passes with an empty report."""
    p = Prompt({"name": "bare", "role": "user", "body": "Hello, world!"})
    report = p.check()
    assert report.passed() is True
    assert len(report) == 0


# --------------------------------------------------------------------------------------
# 2. Undeclared variable — caught at construction (spec 008 Phase 4 invariant)
# --------------------------------------------------------------------------------------


def test_undeclared_variable_is_a_construction_error() -> None:
    """A body referencing `{{ ghost }}`, absent from the declared `variables`, is
    caught at Prompt construction as a hard error — not a lint finding."""
    with pytest.raises(PromptingPressError):
        Prompt(
            {
                "name": "ghosty",
                "role": "user",
                "body": "Hi {{ name }} and {{ ghost }}",
                "variables": {"name": {"type": "string", "trusted": True}},
            }
        )


def test_finding_attributes_are_read_only() -> None:
    """`Finding` is an immutable view — its fields cannot be set from Python."""
    p = Prompt(
        {
            "name": "search",
            "role": "user",
            "body": "Query: {{ q }}",
            "variables": {"q": {"type": "string", "trusted": False}},
        }
    )
    finding = p.check().findings[0]
    with pytest.raises(AttributeError):
        finding.kind = "tampered"  # type: ignore[misc]


# --------------------------------------------------------------------------------------
# 3. Untrusted without guard — SC-005 / FR-017 (the provenance lint advisory)
# --------------------------------------------------------------------------------------


def test_untrusted_variable_without_guard_is_flagged() -> None:
    """A prompt declaring an `untrusted` variable but configuring no `guard` is flagged.

    This is a **prompt-level** finding, so `.variant is None` (it is not tied to a single
    rendered arm), and `.detail` names the offending field `q`."""
    p = Prompt(
        {
            "name": "search",
            "role": "user",
            "body": "Query: {{ q }}",
            "variables": {"q": {"type": "string", "trusted": False}},
        }
    )
    report = p.check()

    assert _kinds(report) == [KIND_UNTRUSTED]
    finding = report.findings[0]
    assert finding.kind == KIND_UNTRUSTED
    assert finding.prompt == "search"
    assert finding.variant is None  # prompt-level, not per-variant
    assert "q" in finding.detail


def test_guard_presence_under_metadata_clears_the_finding() -> None:
    """The provenance lint is satisfied by the mere PRESENCE of a `guard` key under
    `metadata`."""
    p = Prompt(
        {
            "name": "search",
            "role": "user",
            "body": "Query: {{ q }}",
            "variables": {"q": {"type": "string", "trusted": False}},
            "metadata": {"guard": "sanitized upstream"},
        }
    )
    report = p.check()
    assert report.passed(), "a `guard` under `metadata` should satisfy the lint"
    assert KIND_UNTRUSTED not in _kinds(report)


# --------------------------------------------------------------------------------------
# 4. Reserved variant name — caught at construction (spec 008 Phase 4 invariant)
# --------------------------------------------------------------------------------------


def test_variant_named_default_is_a_construction_error() -> None:
    """A `variants` map containing a key literally named `default` is caught at Prompt
    construction as a hard error — not a lint finding."""
    with pytest.raises(PromptingPressError):
        Prompt(
            {
                "name": "rv",
                "role": "user",
                "body": "Base {{ x }}",
                "variables": {"x": {"type": "string", "trusted": True}},
                "variants": {"default": {"body": "Variant {{ x }}"}},
            }
        )


# --------------------------------------------------------------------------------------
# 5. Excluded template feature — caught at construction (spec 008 Phase 4 invariant)
# --------------------------------------------------------------------------------------


def test_excluded_feature_is_a_construction_error() -> None:
    """A body using an excluded feature (`{% include %}`) is caught at Prompt
    construction as a hard error (parse or excluded_feature error)."""
    with pytest.raises(PromptingPressError):
        Prompt({"name": "ae", "role": "user", "body": '{% include "x" %}'})


# --------------------------------------------------------------------------------------
# 6. Purity — FR-019: check mutates nothing and renders nothing
# --------------------------------------------------------------------------------------


class _Greeting(BaseModel):
    name: str


def test_check_is_pure_and_repeated_checks_are_equal() -> None:
    """`check()` is pure analysis (FR-019): it must not mutate the Prompt and must
    return the same result on repeated calls."""
    p = Prompt(
        {
            "name": "greet",
            "role": "user",
            "body": "Hi {{ name }}",
            "variables": {"name": {"type": "string", "trusted": True}},
        }
    )

    report_a = p.check()
    report_b = p.check()

    # Render is unaffected by check — check never renders.
    result = p.render(_Greeting(name="Ada"))
    assert result.text == "Hi Ada"

    # Repeated analysis is itself stable (no accumulating state).
    assert _signature(report_a) == _signature(report_b)
    assert report_a.passed() is True


# --------------------------------------------------------------------------------------
# 7. Multiple prompts / determinism — check each Prompt individually
# --------------------------------------------------------------------------------------


def test_multiple_untrusted_prompts_yield_findings() -> None:
    """Multiple prompts with untrusted variables each produce a finding when checked.
    Collect findings by iterating each prompt individually."""
    prompts = [
        Prompt(
            {
                "name": "alpha",
                "role": "user",
                "body": "Q {{ q }}",
                "variables": {"q": {"type": "string", "trusted": False}},
            }
        ),
        Prompt(
            {
                "name": "beta",
                "role": "user",
                "body": "Q {{ q }}",
                "variables": {"q": {"type": "string", "trusted": False}},
            }
        ),
    ]

    all_findings = []
    for p in prompts:
        all_findings.extend(p.check().findings)

    assert len(all_findings) == 2
    assert all(f.kind == KIND_UNTRUSTED for f in all_findings)

    # Each repeated call returns the same findings (determinism).
    for p in prompts:
        assert _signature(p.check()) == _signature(p.check())


# --------------------------------------------------------------------------------------
# 8. Surface smoke — the check API is exposed where the binding promises it
# --------------------------------------------------------------------------------------


def test_module_exposes_check_surface() -> None:
    assert hasattr(prompting_press, "CheckReport")
    assert hasattr(prompting_press, "Finding")
    for attr in ("findings", "passed", "is_empty"):
        assert hasattr(prompting_press.CheckReport, attr)
    for attr in ("prompt", "variant", "kind", "detail"):
        assert hasattr(prompting_press.Finding, attr)


def test_check_report_collection_protocol() -> None:
    """CheckReport's collection protocol: passed / is_empty / len / bool truthy-iff-findings."""
    p_clean = Prompt({"name": "clean", "role": "user", "body": "hi"})
    p_untrusted = Prompt(
        {
            "name": "untrusted",
            "role": "user",
            "body": "{{ q }}",
            "variables": {"q": {"type": "string", "trusted": False}},
        }
    )

    clean = p_clean.check()
    assert clean.passed() is True
    assert clean.is_empty() is True
    assert len(clean) == 0
    assert not bool(clean)

    flagged = p_untrusted.check()
    assert flagged.passed() is False
    assert flagged.is_empty() is False
    assert len(flagged) == 1
    assert bool(flagged)
    assert repr(flagged) == "CheckReport(findings=1)"
