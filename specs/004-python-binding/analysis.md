# Cross-Artifact Analysis — Spec 004 (Python binding)

**Date**: 2026-06-27 | **Artifacts**: spec.md, plan.md, tasks.md, contracts/python-api.md, data-model.md,
research.md | **Constitution**: v1.0.0 (Principles I–VII) | **Phase**: analyze gate (Step 6)

> Findings re-verified main-thread against source per the project's subagent-fabrication guard. The
> analyze pass ran as a forked skill; its load-bearing claims (I1 kernel-error name; coverage) were
> confirmed against `crates/prompting-press-core/src/error.rs` + `crates/prompting-press/src/error.rs`.

## Result

**0 CRITICAL, 0 HIGH. Coverage 100% (25 FR, 11 SC, 28 tasks all mapped). No constitution violations.**
Safe to proceed to implementation (agent-assign flow). The critique-stage E1/E2 corrections are
reflected consistently across all artifacts; the analyze I-row confirms kernel-direct render is still
Principle-I-sound (the kernel is the shared core; no engine logic in the binding).

## Findings & disposition

| ID | Severity | Finding | Disposition |
|----|----------|---------|-------------|
| I1 | MEDIUM (verify-only) | Kernel error type naming + FR-014 "closed `KernelError`; loader errors" implies two error sources | **VERIFIED CLEAN.** Kernel enum is exactly `KernelError` (`prompting-press-core/src/error.rs:19`); loader errors are the distinct consumer `ConsumerError::Load` (`prompting-press/src/error.rs:116`). Both sources handled (kernel path via scrubber; loader path via `Load`). Names align across docs. No change. |
| C1 | LOW | T025/quickstart token grep `rg -ri "...\|token"` too broad → false-positive risk on incidental "token" substrings | **FIXED.** Narrowed to `count_tokens\|token_count\|TokenCount\|count-tokens` and scoped to `packages/python/python` + `crates/prompting-press-py/src` (tasks.md T025, quickstart.md). |
| U1 | LOW | Stale roadmap "token hook" amendment (spec Assumptions) had no owning task → could be silently dropped | **FIXED.** Added **T027** (doc-only roadmap amendment, run at roadmap-debrief/sync). |
| SEC-101 | LOW (security review) | New Python deps (pydantic/maturin/dmcg) fall outside the Rust-only `cargo-deny` advisory gate (`scripts/ci/check-advisories.sh` reads only Cargo.lock) → no CVE coverage for the Python side | **RESOLVED — in scope for 004** (user decision 2026-06-27). Added **FR-025** + **SC-011** + **T028** (a `ci:check-advisories-py` gate: `pip-audit` over `packages/python/uv.lock`, mirroring the Rust `ci:check-advisories`). Coverage now 25 FR / 11 SC / 28 tasks. |
| A1 | LOW | `output_model` metadata-only passthrough asserted but not directly tested | Accepted — negative property structurally guaranteed by the kernel; optional assertion may be added in the loader/render tests. |
| A2 | LOW | CPython 3.10 EOL 2026-10-31 (~4 months out) | Accepted risk (plan watch-item); abi3-py310 floor still runs post-EOL; revisit at release (spec 007). |
| D1 | LOW | Q1–Q4 restated in both Clarifications and Assumptions | Accepted — intentional redundancy for spec self-containment. |

## Coverage

24/24 FR and 10/10 SC have ≥1 backing task (full table in the analyze run output). Setup/foundational/
polish/verification tasks (T001–T004, T012/T015/T018/T021, T022–T027) are legitimate cross-cutting work
with no 1:1 FR, as expected. No orphan tasks; no task references a component absent from spec/plan.

## Constitution alignment

No violations. Principles I (no engine logic; render parity structural), II (pyo3/pythonize only in
`-py`; `ci:check-ffi`), III (no I/O / no token counter — F4), VI (Pydantic, `from_messages` not
`.chain()`, normalized errors), VII (codegen'd shape, dual-input into one shape) all confirmed. Scope
Discipline (R1): no new pluggable interface; the token-hook candidate is dropped.

## Open decision routed to the user — RESOLVED

**SEC-101** — Python-dependency CVE gate. **Decision (2026-06-27): in scope for spec 004.** Added FR-025,
SC-011, and T028 (`ci:check-advisories-py` via `pip-audit` over `packages/python/uv.lock`). No open
decisions remain before `taskstoissues`.
