# QA Report — Spec 003 (Rust consumer)

**Date**: 2026-06-26 · **Mode**: CLI QA (library crate — no web UI/server) · **Verdict**: ✅ ALL PASSED

`prompting-press` is a Rust library; its acceptance criteria (the 8 live SCs + V-scenarios) are
exercised behaviorally by `cargo test`. QA = run the suite + confirm each acceptance scenario maps to
passing test evidence. (No browser/Playwright mode applies.)

## Test Suite Results (evidence base)

`mise exec -- cargo test -p prompting-press` — **44 passed, 0 failed** (8 lib units + 34 integration +
2 doctests), across: lib 8, check 7, check_purity 1, compose 4, loader 8, render 10, render_validation
4, doctests 2. Whole-workspace: **94 tests** (kernel 50 + consumer 44) — no regressions.

## Acceptance Scenario → Test Coverage Matrix

| TC | Scenario | Backing test | Result |
|----|----------|--------------|--------|
| TC-001 | US1 V1.1 validate + render → RenderResult + provenance | render.rs | ✅ |
| TC-002 | US1 V1.2/V1.3 invalid input → Validation error, no render — SC-002 | render_validation.rs | ✅ |
| TC-003 | US1 V1.4 no native type on public API — SC-006 | render_validation.rs | ✅ |
| TC-004 | US1 V1.5 determinism (render twice) | render.rs | ✅ |
| TC-005 | US1 guard-plumb (F5) | render.rs | ✅ |
| TC-006 | three-sets gap (E1) misnamed field → undefined_variable | render_validation.rs | ✅ |
| TC-007 | get_source happy + UnknownPrompt + unknown-variant (review TS-1) | render.rs | ✅ |
| TC-008 | named-variant render + unknown-variant (review TS-2) | render.rs | ✅ |
| TC-009 | US2 V2.1/V2.3 load YAML / constructed object | loader.rs | ✅ |
| TC-010 | US2 V2.2 YAML≡JSON structural parity — SC-003 | loader.rs | ✅ |
| TC-011 | US2 V2.4 malformed → Load, nothing partial | loader.rs | ✅ |
| TC-012 | US2 V2.5 Norway-safe (`no`→string) | loader.rs | ✅ |
| TC-013 | US3 V3.2/V3.5 undeclared-var detection (per variant) — SC-004 | check.rs | ✅ |
| TC-014 | US3 V3.3 provenance lint (untrusted + no guard) — SC-005 | check.rs | ✅ |
| TC-015 | US3 reserved `default` variant flagged (CR-1) | check.rs | ✅ |
| TC-016 | US3 V3.4 check() purity — FR-019 | check_purity.rs | ✅ |
| TC-017 | US3 empty registry → empty report (F7) | check.rs | ✅ |
| TC-018 | US4 V4.1 composition N→N ordered — SC-008 | compose.rs | ✅ |
| TC-019 | US4 V4.2 partial-failure = error, not partial-success | compose.rs | ✅ |
| TC-020 | US4 V4.4 empty composition → Ok(vec![]) (F7) | compose.rs | ✅ |
| TC-021 | SEC-004 secret scrub (Render/Parse/ExcludedFeature) | error.rs units | ✅ |
| TC-022 | error normalization codes (closed vocab) | error.rs units | ✅ |

## CI Gate Results

| Gate | Result |
|------|--------|
| `moon run :build` | ✅ |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ |
| `cargo fmt --check` | ✅ |
| `ci:check-ffi` (SC-007 — consumer FFI-free) | ✅ |
| `ci:check-floating-versions` | ✅ |
| `ci:check-advisories` (cargo-deny) | ✅ |
| `schemas:codegen-check` | ✅ |
| strict `RUSTDOCFLAGS=-D warnings cargo doc` | ✅ (clean) |

## Metrics

- Acceptance scenarios: 22 mapped, 22 passed, 0 failed/partial/skipped.
- Test suite: 44 consumer / 94 workspace, all green.
- Coverage: every live SC-001..008 (SC-009 dropped/F4) has ≥1 passing backing test.

## Verdict

✅ **ALL PASSED** — every acceptance criterion met with passing test evidence; all gates green; no
failures. Library crate → CLI/test-suite validation is the complete, appropriate QA surface. Safe to
proceed to code-review + security-review (steps 12/13).
