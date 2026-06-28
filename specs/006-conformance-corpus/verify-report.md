# Verification Report — 006 Conformance corpus + cross-language hardening

**Date**: 2026-06-28 · **Against**: spec.md (19 FR, 7 SC), plan.md, tasks.md (21/21 `[X]`), constitution v1.1.0
**Method note**: read-only spec-conformance analysis. Run on the main thread (the delegated subagent hit
the `tool_uses:0` channel glitch on the prior step); every FR/SC verdict is grounded in the committed
diff (`8919130`) + live execution, not self-report. `max_findings=50`.

## Verification Report (findings)

**Zero CRITICAL / HIGH / MEDIUM / LOW findings.** All requirements have implementation evidence; no
constitution conflicts; no missing files; no spec-intent divergence. Success report follows.

| ID | Category | Severity | Result |
|----|----------|----------|--------|
| A | Task completion | — | 21/21 complete; all verified non-phantom (verify-tasks-report.md) |
| B | File existence | — | All task-referenced files present on disk + in commit 8919130 |
| C | Requirement coverage | — | 19/19 FR have code evidence (table below) |
| D | Scenario & test coverage | — | The deliverable IS tests; all 3 user stories covered by runners; seeded-divergence proves non-vacuous |
| E | Spec intent alignment | — | Behavior matches acceptance criteria (gate green across all 3 bindings) |
| F | Constitution alignment | — | No MUST violated (I/II/III/VII all upheld — see below) |
| G | Design & structure | — | Matches plan (conformance/ + 3 runners + gate); mirrors existing CI pattern |

## Requirement Coverage (FR → evidence)

| FR | Evidence |
|----|----------|
| FR-001 shared corpus, 3 runners | `conformance/` single set; Rust/Py/TS runners read it |
| FR-002 marshaling fixtures, 5 cases | `conformance/marshaling/{date,decimal,nested-model,null-undefined-none,int-vs-float}.json` |
| FR-003 schema fixtures, reuse existing | `conformance/schema/manifest.json` references `schemas/jsonschema/fixtures/` (not forked) |
| FR-004 OS/arch-stable expectations | goldens are SHA-256 over canonical strings; no locale/float-format dependence |
| FR-005 3-way identical text+hashes + golden | all 3 runners assert == committed golden (transitive); golden is the tripwire |
| FR-006 canonical serialized form | date=ISO-8601, decimal=string; type-tag builder in each runner |
| FR-007 real render path | Rust `prompting_press::render`, Py `prompting_press.render`, TS facade `render` (no kernel bypass) |
| FR-008 null/undefined/None pinned | `null-undefined-none.json` + all runners (absent omits key; null→JSON null) |
| FR-009 3-way same verdict via own loader | schema runners call each binding's `load_json`/`load_yaml` |
| FR-010 structured rejection, no partial/crash | reject path asserts on error TYPE (`ConsumerError`/`LoadError`), not detail |
| FR-011 YAML + JSON forms | manifest has 2 `form:"yaml"` entries; all 3 schema runners handle both forms |
| FR-012 CI gate fails on divergence | `scripts/ci/conformance.sh` + `ci:conformance` + ci.yml job (exit-non-zero on fail) |
| FR-013 locally reproducible | `moon run ci:conformance` (verified runs green) |
| FR-014 names binding+case+kind, no leak | runner failure msgs: `[rust] case={} divergence={text|template_hash|render_hash}` |
| FR-015 Rust consumer first-class | gate runs `cargo test -p prompting-press --test conformance_marshaling --test conformance_schema` (ran nowhere in CI before) |
| FR-016 no render-parity fixtures | spec-002 render set byte-unchanged; golden set bounded to 5 named cases |
| FR-017 no engine logic in bindings | `ci:check-ffi` PASSED; runners call public APIs only; `RawVars` test-only no-op |
| FR-018 no boundary expansion | no new public API, no I/O in library (runners read fixtures AS test harnesses) |
| FR-019 corpus measures fidelity, no alt rendering | goldens generated FROM the Rust reference binding (D3) |

## Success Criteria (SC → evidence)

| SC | Result |
|----|--------|
| SC-001 100% marshaling parity | ✓ Rust 2 + Py 17 + TS 16 green vs shared golden |
| SC-002 all 5 hard cases | ✓ 5 marshaling fixtures, each through all 3 bindings |
| SC-003 100% schema verdict parity + structured reject | ✓ manifest verdicts hold in all 3 loaders incl. YAML |
| SC-004 seeded divergence fails + names | ✓ re-proven this cycle: text + hash seeds fail Rust+Py+TS, naming case+kind |
| SC-005 locally reproducible | ✓ `moon run ci:conformance` |
| SC-006 zero render-parity/engine-logic; spec-002 unchanged | ✓ check-ffi green; render fixtures byte-unchanged; 5-fixture golden set |
| SC-007 gate runs on PRs, enforced | ✓ ci.yml `conformance` job on push/PR |

## Constitution Alignment

No MUST violated. **I** — render parity NOT re-tested (corpus tests marshaling + schema only; spec-002
set untouched). **II** — no engine logic in any binding/runner; `ci:check-ffi` green; the one `RawVars`
newtype is test-only Serialize-delegation + no-op Validate. **III** — no I/O in the library, no new public
API, no token surface (runners read fixtures as test harnesses). **VII** — schema round-trip is the second
guarantee; fixtures reuse the one schema's fixtures. C-01/C-07 upheld. SEC-001/SEC-002 (path confinement,
assert-on-code) implemented in the runners.

## Metrics

- Total tasks: **21 / 21** complete (all verified non-phantom)
- Requirement coverage: **19/19 FR (100%)**, **7/7 SC (100%)** with implementation evidence
- Files verified: 24 (corpus + 4 Rust + 4 Py/TS runners + CI glue)
- CRITICAL issues: **0** · HIGH: **0** · MEDIUM: **0** · LOW: **0**

## Next Actions

**Implementation verified — ready for review.** No CRITICAL/HIGH/MEDIUM/LOW findings; no `/speckit.converge`
needed (nothing unbuilt); no `/speckit.bugfix` needed (no defects). Proceed to the review pass
(`/speckit.review.run`) → QA → code-review + security-review.
