# Review Report — Spec 005 (TypeScript binding `prompting-press-node`)

**Date**: 2026-06-28 · **Cycle**: `/speckit.review.run` (code, errors, tests, types, comments)
**Scope**: `git diff main..HEAD` (the napi crate + TS facade + tests + CI). 5 parallel reviewers.

> **Provenance**: all 5 reviewers passed an integrity check (correct line counts, real file:line
> citations). Every load-bearing finding below was re-verified main-thread before inclusion (the two
> headline items — the `render` variant gap and the test-count drift — confirmed against the actual code).

## Headline

**No CRITICAL code defect.** The binding is clean and faithful: zero engine logic (render/compose →
`prompting_press_core::render`; check/getSource/loaders → consumer), SEC-004 scrub airtight (Rust routes
through the consumer scrubber first + never reads raw `.detail`; the Zod mapper copies `issue.message`+`path`
only, never `issue.input`), exhaustive wildcard-free Rust matches, panic-free across napi, FFI isolation
gate-enforced. One real **parity gap** (the `render` facade can't select a variant) + small polish.

## Findings (triaged)

### Fix now
| ID | Sev | Source | Finding | Evidence |
|----|-----|--------|---------|----------|
| F-A | IMPORTANT | code + tests | The TS facade `render` hardcodes `variant: undefined` (`index.ts:432`) and exposes NO variant parameter — a TS caller can only render the `default` arm. Python's `render` exposes `variant=None` (`render.rs:198`); the napi addon accepts it. Cross-binding parity + FR-009 ("variant selection is caller-owned via `render(name, variant=...)`") gap. The "rides through getSource/Composition" rationale is wrong (getSource = unrendered source; Composition = `Message[]`, not a single `RenderResult`). | `index.ts:429-432` |
| F-B | minor | comments | `test-node.sh:12` says "57 TS tests" — actually 49 top-level `test()` cases (node:test's `tests 57` summary counts subtests/assertions differently). Drop the hard number or use 49. | `scripts/ci/test-node.sh:12` |
| F-C | minor | comments + verify | Doc-drift: `plan.md:49/136` + `tasks.md:38/50` say pin `napi-derive` to `3.9.4`; the correct pin is `3.5.7` (the two crates version independently — `Cargo.toml:68` is right). Reconcile the 4 spec-doc lines. | `plan.md:49`, `tasks.md:50` |

### Defer (hardening / accepted-deviation — track, not blockers)
| ID | Sev | Source | Finding |
|----|-----|--------|---------|
| D-1 | IMPORTANT(eng) | errors | `decodeAddonError` validates the payload envelope (`code` string, `errors` array) but not each row's shape (`index.ts:188-194`); a malformed row could make the `summary` `.map` throw inside the catch → a raw error escapes. Live-unreachable today (Rust always emits well-formed rows), but the function's contract is "nothing raw escapes." Add a per-row `typeof` guard with base-class fallback. |
| D-2 | SUGGESTION | code+types | Static-form `render`/`append` duck-types on `safeParse` (`isSchema`, `index.ts:331`); plain data that happens to expose a `safeParse` method is misclassified as a schema. Inherent to positional-union duck-typing; document the collision risk. |
| D-3 | minor | errors | `decodeAddonError` non-JSON fallback codes the synthetic row `"render"` (a kernel code) — a transport failure looks like a kernel render error. Use a distinct `"internal"`/`"decode_failed"` code. |
| D-4 | minor | types | `Finding`/`Message` are `#[napi(object)]` (JS-mutable) while `RenderResult` is a getter-only class — output-DTO immutability asymmetry. Conscious tradeoff (plain-array ergonomics); document as accepted. |
| D-5 | SUGGESTION | tests | Missing-test edges (all low-risk, paths covered structurally): facade-level null/undefined marshaling (Q6), getSource named-variant body, compose-resolve SEC-004 scrub path, the `subclassForCode`/`decodeAddonError` fallback arms, getSource named-variant. |

### Noise / accepted (no action)
- comments-S3/S4 (Cargo feature-comment attribution; "ZodError mapper" wording), types-S1/S2/S3 (`code: String` mirrors upstream; `kernel_top_code` if-guards; saturating u32 cast), errors-S1/S2 — all correct-as-written, noted for the record.

## Confirmed strengths
SEC-004 discipline (4 tests, both Zod-reject + kernel-render paths, JS surface); zero-engine-logic real not asserted; exhaustive `ConsumerError`/`FindingKind` matches (new variant = compile error); binding-owned `Composition` correct (consumer's `append<V>` is garde-generic); napi6/serde-json features live + justified (bigint losslessness test); TS error hierarchy `instanceof`-correct after compile; structural Zod typing (no Zod-identity dependency); SC-002 multi-field + SC-003 both-hashes + check-purity + Norway-safe all genuinely tested.

## Recommended action
Fix **F-A** (variant — the one real gap; needs an API-shape decision, see below), **F-B**, **F-C** now.
Defer D-1…D-5 to cleanup/roadmap (D-1 is the highest-value deferral — a cheap defensive hardening).

---

## Dispositions (applied 2026-06-28)

- **F-A — DONE** (`329cd20`): variant parity via an options-object refactor — `render(reg, name,
  schema, data, opts?)`, `getSource(reg, name, opts?)`, and `Composition` entries as
  `{ name, schema?, data, variant? }` objects. The user elevated this to a **codebase-wide convention**
  (options objects over positional/optional params); recorded in the constitution + roadmap (see below).
  The Composition refactor also dissolved the `isSchema` duck-typing smell (D-2 / TS-audit-I2) for
  composition; `render` keeps schema-vs-data positional (both required) so `isSchema` remains there only.
- **F-B — DONE** (next commit): `test-node.sh` "57 TS tests" → corrected (the suite is 59 after the
  variant tests; node:test's `tests N` line counts differently from `test()` decls — drop the hard number).
- **F-C — DONE** (next commit): reconcile the `napi-derive 3.9.4` doc-drift in `plan.md`/`tasks.md` → 3.5.7.
- **D-1 (decodeAddonError per-row validation) — DEFERRED** to a tracked follow-up (also TS-audit-S1):
  the highest-value hardening; live-unreachable today (Rust always emits well-formed rows).
- **D-2…D-5, TS-audit I3/S2/S3/S4 — DEFERRED / accepted** as noted (mostly minor robustness/idiom).

## Deep idiom audits (refactoring.guru smells + per-language style guides) — 2026-06-28

Three background subagents (rust-pro, python-pro, typescript-pro), each integrity-checked + findings
re-verified main-thread. Headline: **the codebase is high quality across all three languages.**

- **Rust (all 4 crates)** — **no CRITICAL/IMPORTANT.** FFI isolation, zero `unsafe`, zero truncating
  `as` casts, zero non-test panics, exhaustive no-wildcard error/discriminant matches — all verified by
  construction. clippy `-W pedantic -W nursery` = 150 warnings, ~all doc-backtick/const-fn noise. The one
  real micro-nit: `prompting-press-py/src/error.rs:233` `summarize` uses `format!`-push-in-loop
  (`format_push_string`) where the consumer's `Display` already uses idiomatic `write!` — worth aligning
  for cross-crate consistency. The py↔node bindings mirror correctly and diverge only where idiom demands
  (error transport, validation ownership) — Principle VI done right.
- **Python binding** — ship-quality. 2 IMPORTANT: (I-1) no `py.typed`/`.pyi` stub → the compiled
  extension presents an *untyped* surface to mypy/pyright (notable for a typed-prompt library); (I-2)
  `render(reg, name, vars, data=None, variant=None, guard=None)` is a 6-param positional signature that
  fights the new options/keyword convention — fix is one line: `#[pyo3(signature = (reg, name, vars, *,
  data=None, variant=None, guard=None))]` (keyword-only). Plus a duplicated `_registry` test helper
  (→ conftest fixture). SEC-004, Pydantic-v2 idiom, error normalization all confirmed correct.
- **TypeScript binding** — well-built Adapter; zero `any`/`!`/unsafe casts, strict-tsc clean. The 2
  CRITICALs it raised (stale compose tests, stale getSource call) were **artifacts of the in-flight
  refactor and are already fixed** (59 tests pass). Remaining: I1 README (fixed), I3 redundant `| null`
  (accepted), S1 decodeAddonError row-shape (= D-1, deferred).

**Cross-cutting follow-ups to carry to roadmap-debrief** (none block this spec):
1. `py.typed` + `.pyi` stub for the Python binding (audit Py-I-1) — the one real capability gap.
2. Make Python `render` kwargs keyword-only (Py-I-2) — aligns with the new options/keyword convention.
3. `decodeAddonError` per-row shape validation (TS-S1 / D-1).
4. Rust `summarize` `write!` alignment (Rust py-1) — trivial.
5. Python test `conftest.py` to de-dup `_registry` (Py-S-1).
