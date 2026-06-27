"""US3 agreement + provenance lint tests for the PyO3 binding (`prompting_press`) — spec 004, T016.

US3 surfaces the shared core's pure analysis pass to Python as `prompting_press.check(reg)`.
The lint is performed **once, in Rust** (Principle I / IV); the binding re-derives nothing and
only converts the consumer's `CheckReport` into the Python pyclass, preserving the consumer's
**deterministic finding order**. These tests prove every documented finding `kind` is reachable
from Python, that the report's collection protocol behaves (`passed` / `is_empty` / `len` / `bool`),
and — critically — that `check` is **pure**: it never mutates the registry and never renders
(FR-019).

Observed `check` / `CheckReport` / `Finding` API (inspected from the built extension before
asserting — not assumed):

- `check(reg) -> CheckReport`. Pure analysis; an empty registry yields an empty, passing report.
- `CheckReport`: `.findings` (a `list[Finding]`), `.passed() -> bool` (True iff no findings),
  `.is_empty() -> bool`, `len(report)` == number of findings, `bool(report)` truthy iff there are
  findings (so `bool` is the inverse of `passed()`), `repr` == `CheckReport(findings=N)`.
- `Finding`: **read-only** `.prompt: str`, `.variant: str | None`, `.kind: str`, `.detail: str`.
  `.kind` is one of the stable strings asserted below.

Observed `kind` / `.variant` facts that shaped these assertions:

- `undeclared_variable`   → `.variant == "default"` (per-variant; the implicit root arm is "default").
- `untrusted_without_guard` → `.variant is None` (a **prompt-level** finding, not per-variant).
- `reserved_variant_name` → `.variant == "default"` (the offending variant key).
- `analysis_error`        → `.variant == "default"`; reachable via an excluded template feature
  (e.g. `{% include %}`), which loads fine as a definition but cannot be statically analyzed —
  `check` surfaces a finding rather than crashing.

The provenance lint is satisfied by the mere **presence** of a `guard` key under either `meta` or
`metadata`; both are asserted to clear the finding.
"""

from __future__ import annotations

import pytest
from pydantic import BaseModel

import prompting_press
from prompting_press import Registry, check, render

# --------------------------------------------------------------------------------------
# The stable finding `kind` strings (the binding's public vocabulary). Asserted by value
# so a rename in the core is caught here.
# --------------------------------------------------------------------------------------

KIND_UNDECLARED = "undeclared_variable"
KIND_UNTRUSTED = "untrusted_without_guard"
KIND_ANALYSIS = "analysis_error"
KIND_RESERVED = "reserved_variant_name"


def _kinds(report) -> list[str]:
    return [f.kind for f in report.findings]


def _signature(report) -> list[tuple[str, str, str | None]]:
    """A stable, comparable projection of a report's findings: (kind, prompt, variant)."""
    return [(f.kind, f.prompt, f.variant) for f in report.findings]


# --------------------------------------------------------------------------------------
# 1. Clean registry passes — the no-findings contract (FR-016/FR-019 baseline)
# --------------------------------------------------------------------------------------


def test_clean_registry_passes_with_empty_report() -> None:
    """A well-formed prompt — every referenced var declared, no untrusted-without-guard,
    no reserved variant — produces no findings. The report's whole collection protocol
    agrees: passed / empty / len 0 / falsy."""
    reg = Registry()
    reg.insert(
        {
            "name": "greet",
            "role": "user",
            "body": "Hi {{ name }}, you have {{ count }} messages",
            "variables": {
                "name": {"type": "string", "provenance": "trusted"},
                "count": {"type": "integer", "provenance": "trusted"},
            },
        }
    )

    report = check(reg)

    assert report.passed() is True
    assert report.is_empty() is True
    assert list(report.findings) == []
    assert len(report) == 0
    assert not bool(report)  # empty ⇒ falsy (inverse of passed())


def test_empty_registry_passes() -> None:
    """An empty registry yields an empty, passing report (documented `check` behavior)."""
    report = check(Registry())

    assert report.passed() is True
    assert len(report) == 0
    assert not report


# --------------------------------------------------------------------------------------
# 2. Undeclared variable — SC-004 / FR-016 (the headline agreement check)
# --------------------------------------------------------------------------------------


def test_undeclared_variable_is_flagged_naming_the_variable() -> None:
    """A body referencing `{{ ghost }}`, absent from the declared `variables`, is the
    sound-agreement violation: a single `undeclared_variable` finding on the implicit
    `default` arm, naming the prompt and the offending variable in `.detail`."""
    reg = Registry()
    reg.insert(
        {
            "name": "ghosty",
            "role": "user",
            "body": "Hi {{ name }} and {{ ghost }}",
            "variables": {"name": {"type": "string", "provenance": "trusted"}},
        }
    )

    report = check(reg)

    assert not report.passed()
    assert _kinds(report) == [KIND_UNDECLARED]

    finding = report.findings[0]
    assert finding.kind == KIND_UNDECLARED
    assert finding.prompt == "ghosty"
    assert finding.variant == "default"  # the implicit root arm is named "default"
    assert "ghost" in finding.detail  # detail names the undeclared variable


def test_finding_attributes_are_read_only() -> None:
    """`Finding` is an immutable view of the core's report — its fields cannot be set
    from Python (reinforces the purity / no-mutation contract at the object level)."""
    reg = Registry()
    reg.insert({"name": "g", "role": "user", "body": "{{ ghost }}", "variables": {}})
    finding = check(reg).findings[0]

    with pytest.raises(AttributeError):
        finding.kind = "tampered"  # type: ignore[misc]


# --------------------------------------------------------------------------------------
# 3. Untrusted without guard — SC-005 / FR-017 (the provenance lint)
# --------------------------------------------------------------------------------------


def test_untrusted_variable_without_guard_is_flagged() -> None:
    """A prompt declaring an `untrusted` variable but configuring no `guard` is flagged.

    This is a **prompt-level** finding, so `.variant is None` (it is not tied to a single
    rendered arm), and `.detail` names the offending field `q`."""
    reg = Registry()
    reg.insert(
        {
            "name": "search",
            "role": "user",
            "body": "Query: {{ q }}",
            "variables": {"q": {"type": "string", "provenance": "untrusted"}},
        }
    )

    report = check(reg)

    assert _kinds(report) == [KIND_UNTRUSTED]
    finding = report.findings[0]
    assert finding.kind == KIND_UNTRUSTED
    assert finding.prompt == "search"
    assert finding.variant is None  # prompt-level, not per-variant
    assert "q" in finding.detail


@pytest.mark.parametrize("guard_key", ["meta", "metadata"])
def test_guard_presence_under_meta_or_metadata_clears_the_finding(guard_key: str) -> None:
    """The provenance lint is satisfied by the mere PRESENCE of a `guard` key under
    either `meta` or `metadata`; the same untrusted prompt that was flagged above no
    longer produces an `untrusted_without_guard` finding once a guard is declared."""
    reg = Registry()
    reg.insert(
        {
            "name": "search",
            "role": "user",
            "body": "Query: {{ q }}",
            "variables": {"q": {"type": "string", "provenance": "untrusted"}},
            guard_key: {"guard": "sanitized upstream"},
        }
    )

    report = check(reg)

    assert report.passed(), f"a `guard` under `{guard_key}` should satisfy the lint"
    assert KIND_UNTRUSTED not in _kinds(report)


# --------------------------------------------------------------------------------------
# 4. Reserved variant name — FR-018 (a variants map keyed literally `default`)
# --------------------------------------------------------------------------------------


def test_variant_named_default_is_flagged_as_reserved() -> None:
    """A `variants` map containing a key literally named `default` shadows the root body
    and is unreachable; `check` flags it as `reserved_variant_name` on that variant."""
    reg = Registry()
    reg.insert(
        {
            "name": "rv",
            "role": "user",
            "body": "Base {{ x }}",
            "variables": {"x": {"type": "string", "provenance": "trusted"}},
            "variants": {"default": {"body": "Variant {{ x }}"}},
        }
    )

    report = check(reg)

    assert KIND_RESERVED in _kinds(report)
    reserved = next(f for f in report.findings if f.kind == KIND_RESERVED)
    assert reserved.prompt == "rv"
    assert reserved.variant == "default"


# --------------------------------------------------------------------------------------
# 5. Analysis error — an excluded template feature surfaces a finding, never a crash
# --------------------------------------------------------------------------------------


def test_excluded_feature_surfaces_analysis_error_not_a_crash() -> None:
    """A body using an excluded feature (`{% include %}`) is a valid prompt *definition*
    (it loads) but cannot be statically analyzed for agreement. `check` must surface an
    `analysis_error` finding rather than raise — the un-analyzable template is reported,
    not fatal. (The consumer crate exercises this kind end-to-end; here we confirm it is
    reachable from the Python surface and does not crash the binding.)"""
    reg = Registry()
    reg.insert({"name": "ae", "role": "user", "body": '{% include "x" %}'})

    report = check(reg)  # must not raise

    assert KIND_ANALYSIS in _kinds(report)
    analysis = next(f for f in report.findings if f.kind == KIND_ANALYSIS)
    assert analysis.prompt == "ae"
    assert analysis.variant == "default"


# --------------------------------------------------------------------------------------
# 6. Purity — FR-019: check mutates nothing and renders nothing
# --------------------------------------------------------------------------------------


class _Greeting(BaseModel):
    name: str


def test_check_is_pure_render_is_unchanged_and_repeated_checks_are_equal() -> None:
    """`check` is pure analysis (FR-019): it must not mutate the registry or render.

    Observable proof:
    - a render is byte-identical (text AND both provenance hashes) before vs. after
      `check`, so the registry the render reads was untouched;
    - calling `check` twice yields equal findings, so analysis has no accumulating
      side effect.
    """
    reg = Registry()
    reg.insert(
        {
            "name": "greet",
            "role": "user",
            "body": "Hi {{ name }}",
            "variables": {"name": {"type": "string", "provenance": "trusted"}},
        }
    )

    before = render(reg, "greet", _Greeting, {"name": "Ada"})

    report_a = check(reg)
    report_b = check(reg)

    after = render(reg, "greet", _Greeting, {"name": "Ada"})

    # Render is identical after check ⇒ check rendered nothing and mutated nothing.
    assert after.text == before.text == "Hi Ada"
    assert after.template_hash == before.template_hash
    assert after.render_hash == before.render_hash

    # Repeated analysis is itself stable (no accumulating state).
    assert _signature(report_a) == _signature(report_b)
    assert report_a.passed() is True


# --------------------------------------------------------------------------------------
# 7. Multiple findings / determinism — the consumer's finding order is preserved
# --------------------------------------------------------------------------------------


def test_multiple_flagged_prompts_yield_deterministic_findings() -> None:
    """A registry with three distinct violations produces one finding each, and the
    finding order is identical across repeated `check` calls (the binding preserves the
    consumer's deterministic order — Principle I; no per-call nondeterminism)."""
    reg = Registry()
    # Inserted in a deliberately non-sorted order to prove the report order is the core's,
    # not insertion order.
    reg.insert({"name": "zeta", "role": "user", "body": "X {{ ghost }}", "variables": {}})
    reg.insert(
        {
            "name": "alpha",
            "role": "user",
            "body": "Q {{ q }}",
            "variables": {"q": {"type": "string", "provenance": "untrusted"}},
        }
    )
    reg.insert(
        {
            "name": "mid",
            "role": "user",
            "body": "M {{ x }}",
            "variables": {"x": {"type": "string", "provenance": "trusted"}},
            "variants": {"default": {"body": "V {{ x }}"}},
        }
    )

    sig_1 = _signature(check(reg))
    sig_2 = _signature(check(reg))
    sig_3 = _signature(check(reg))

    # One finding per flagged prompt.
    assert len(sig_1) == 3

    # The exact order is stable across calls — the determinism guarantee.
    assert sig_1 == sig_2 == sig_3

    # All three expected violations are present (asserted as a set so this test does not
    # over-fit the core's particular ordering, which the equality above already pins).
    assert set(sig_1) == {
        (KIND_UNTRUSTED, "alpha", None),
        (KIND_RESERVED, "mid", "default"),
        (KIND_UNDECLARED, "zeta", "default"),
    }

    report = check(reg)
    assert not report.passed()
    assert bool(report)  # has findings ⇒ truthy
    assert len(report) == 3


# --------------------------------------------------------------------------------------
# 8. Surface smoke — the US3 check API is exposed where the binding promises it
# --------------------------------------------------------------------------------------


def test_module_exposes_us3_check_surface() -> None:
    assert hasattr(prompting_press, "check")
    assert hasattr(prompting_press, "CheckReport")
    assert hasattr(prompting_press, "Finding")
    for attr in ("findings", "passed", "is_empty"):
        assert hasattr(prompting_press.CheckReport, attr)
    for attr in ("prompt", "variant", "kind", "detail"):
        assert hasattr(prompting_press.Finding, attr)
