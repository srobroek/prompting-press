# Security Review — Spec 005: TypeScript binding (`prompting-press-node` → `packages/typescript`)

**Date**: 2026-06-27 · **Mode**: read-only, pre-implementation (plan/tasks security review, step 5c)
**Reviewer**: main-thread (the systemic subagent-fabrication glitch makes a delegated review untrustworthy;
findings here are grounded in the verified consumer source + the 005 plan/tasks).

## Executive summary

Spec 005 is the **second FFI binding**, structurally identical in security posture to the merged spec-004
Python binding: same trust model (app-authored Vars, no I/O, no secrets, no network — Principle III), same
single security-relevant new surface (the **Rust→TypeScript error translation**, where the inherited
SEC-004 bound-value scrub must survive a new boundary), and the same lint-not-sanitizer carryover (SEC-002).
The 005 plan/tasks already place the SEC-004 scrub correctly (T006: route the raw `KernelError` through the
consumer's tested `From<KernelError> for ConsumerError` scrubber FIRST — verified in `error.rs:From<KernelError>`,
which discards `Parse`/`Render`/`ExcludedFeature` `detail`), so the residual is *verification*, not design.

**No HIGH/CRITICAL.** Two load-bearing items: **SEC-004-TS** (MODERATE — the scrub must be asserted at the
thrown JS `Error`'s `message`/`.stack`/`.errors` surface, not only the Rust translation; the `ZodError`
mapper must copy issue `message`+`path` only, never the rejected value) and **SEC-201** (LOW — no npm
dependency advisory gate exists; this spec adds `ci:check-advisories-node`). The rest are INFORMATIONAL
carryovers.

## Plan artifacts reviewed

- `specs/005-ts-binding/{spec.md,plan.md,tasks.md,research.md,data-model.md,contracts/ts-api.md,quickstart.md}`
- `specs/005-ts-binding/checklists/binding.md` (CHK012/CHK032 pin the scrub as a measurable requirement)
- `specs/004-python-binding/security-review-report.md` — the PRIOR binding's review; SEC-004 / SEC-002 /
  SEC-005 / SEC-101 are the carryovers this spec transposes to TS.
- Verified source: `crates/prompting-press/src/error.rs` (`From<KernelError>` scrub), `…/lib.rs` (kernel
  re-export), `crates/prompting-press-node/{Cargo.toml,src/lib.rs}` (napi 3.x, FFI-isolated).

## Threat model (transposed from 004)

- **data_protection**: No data at rest/in transit, no secrets storage. `Registry` wraps the consumer's
  in-memory map; validated Vars live in-memory for one synchronous validate+marshal+render. The two
  value-content boundary crossings are (a) the normalized thrown `Error` (FR-015 mandates the SEC-004 scrub
  survive the Rust→TS translation) and (b) the napi marshal step (FR-003a), which transports values into
  the kernel but emits nothing externally.
- **input_validation**: The binding OWNS validation at the render boundary (Q1/FR-002): `safeParse` runs
  ONCE before any templating; on failure no render, the kernel never sees the values, the `ZodError` is
  normalized to `PromptValidationError`. Only validated values cross napi (FR-003). Loader text → the
  CONSUMER's serde path (Q3), so accept/reject + YAML↔JSON parity stay structural. Malformed → `LoadError`,
  nothing partially loaded (FR-007).
- **injection**: Template/SSTI is the kernel's resolved class (values bound as data, never re-parsed;
  macros/includes excluded at parse). The binding adds NO new injection sink: render delegates to the
  kernel, the loader marshals text to the consumer's serde path (no JS-side `eval`, no second parser),
  `check()` is pure, and the napi marshal produces a serde/`minijinja::Value` (data), not code.
- **error_handling**: The Rust→TS error translation (the addon→facade hop) is the new security-relevant
  surface. The scrub is Rust-side (T006, the consumer's tested scrubber); the TS facade then builds the
  `Error` subclass from the already-scrubbed rows. The facade MUST introduce no logging primitive that
  emits row content, and the `ZodError` mapper MUST copy issue `message`+`path` only.
- **secrets_management**: N/A — no secrets/credentials/connection strings, no env access (FR-023). Caller
  Vars could contain secrets/PII, but the binding never persists or transmits them; the only
  boundary-crossing text path is the thrown error's message, which inherits the FR-015/SEC-004 scrub.

## Vulnerability findings

### SEC-004-TS — The scrub must survive the Rust→TS error translation; assert it at the thrown `Error`'s `message`/`.stack`/`.errors` surface (MODERATE) — inherited carryover at a new boundary

- **Severity**: MODERATE (same notch as 004's SEC-004-PY — the value content that spec 003 scrubs at the
  single Rust boundary now crosses an additional FFI hop into JS).
- **Status**: correctly DESIGNED; needs the verification asserted. The plan reuses the consumer's scrub
  (T006: `From<KernelError> for ConsumerError` runs FIRST — verified it discards `Parse`/`Render`/
  `ExcludedFeature` detail and emits fixed messages). FR-015 mandates no raw bound-value content in the
  error message/`.stack`/logs. The **new** TS-specific sub-surface is the `ZodError` mapper: a `ZodError`
  issue can carry the rejected `input`; the mapper MUST copy only `issue.message` + `issue.path` (research
  D3, CHK012). And the addon→facade error channel (critique E1) must pass *structured scrubbed rows*, not a
  raw string that could embed detail.
- **Action**: T009 (Rust-side: a seeded-secret `KernelError::Render` → scrubbed rows, no secret in
  message) + T010 (TS-side: the thrown `PromptRenderError`'s `message`/`.stack`/`.errors[*]` and a
  Zod-rejected secret value both provably exclude the secret). Pin at the **JS surface**, mirroring the
  004 `test_rejected_sensitive_input_is_not_leaked` + the M-1 fix (the introspection-failure fallback must
  withhold detail, never surface the raw native error).

### SEC-201 — New npm dependencies fall outside the Rust + Python advisory gates; no Node CVE scan exists (LOW)

- **Severity**: LOW. The repo has `ci:check-advisories` (cargo-deny) + `ci:check-advisories-py` (pip-audit),
  but no npm-dependency CVE gate. Zod, `@napi-rs/cli`, `json-schema-to-typescript` (+ a test runner) are
  uncovered.
- **Status**: ADDRESSED by the spec — FR-025 + T029 add `ci:check-advisories-node` (pnpm audit / osv-scanner
  over the pnpm lockfile), mirroring the Rust + Python gates. Residual is implementation. (This is the 005
  analogue of 004's SEC-101.)

### SEC-202-TS — `napi` deserialization of deeply nested / large untrusted JS objects has no asserted depth/size bound (INFORMATIONAL)

- **Severity**: INFORMATIONAL — out of scope for v1 under the app-authored-Vars trust model (mirrors the
  004 SEC-005-PY / spec-003 SEC-005 reasoning). Render/compose run `safeParse` first (Zod bounds the shape);
  the unguarded path is `Registry.insert(object)` of a raw object → napi serde. The 004 binding tracked the
  identical concern as TD001 (a deferred depth-bound). 005 inherits it as a deferred follow-up, not v1 work.
- **Action**: none required for v1; note in tasks/roadmap as the 005 analogue of 004's TD001 if desired.

### SEC-203-TS — Provenance lint is a lint, not a runtime sanitizer; carry the non-enforcement invariant to the TS `check`/`Finding` surface (INFORMATIONAL) — inherited carryover

- **Severity**: INFORMATIONAL. The plan does not over-promise: `check()` is pure analysis (FR-019), the
  provenance lint reports untrusted/external-without-guard but neither sanitizes nor enforces at runtime.
  The residual (identical to spec-003 SEC-002 / 004 SEC-002-PY): the words "untrusted"/"guard" connote a
  sanitizer to a TS reader. Ensure the README/docstrings (T025) state the lint is advisory, not a runtime
  control. Discharges the "carry forward to consumer spec 005" item.

### SEC-204-TS — FFI marshaling fidelity (lossless, no silent coercion) is a correctness AND a type-confusion control (INFORMATIONAL)

- **Severity**: INFORMATIONAL. The Q6 null/undefined rule + bigint/date wire shape (FR-003a; critique E4)
  is primarily a correctness/cross-binding-parity concern, but lossless marshaling also prevents a
  type-confusion class (a value silently coerced to a different type than the caller validated). Keep it
  pinned by the marshaling tests (T009). No action beyond the planned tests + the E4 wire-shape note.

### SEC-205 — napi floating version (`"3"`) is a supply-chain hygiene item, not a vulnerability (LOW)

- **Severity**: LOW. The crate declares `napi = "3"` / `napi-derive = "3"` (floating major-range); T001
  pins exact `3.9.4`. A floating range is a (minor) supply-chain reproducibility risk; pinning + the
  `ci:check-floating-versions` gate + the lockfile resolve it. Same disposition as the 004 packaging
  reconciliation (no security defect; the one committed-state divergence to fix).

## Verdict

**PASS (pre-implementation).** No HIGH/CRITICAL. The single security-critical surface (SEC-004-TS, the
Rust→TS error scrub) is correctly designed (scrub reused from the tested consumer path) and the
verification is specified (T009/T010, CHK012/CHK032); the one gap (SEC-201, no Node advisory gate) is
closed by FR-025/T029. SEC-202/203/204/205 are INFORMATIONAL/LOW carryovers with no v1 action beyond the
already-planned tasks. The TS-specific additions to watch during implementation: the `ZodError`
message-only mapper and the structured addon→facade error channel (critique E1) — both are SEC-004 surface.
