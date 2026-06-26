---
document_type: security-review
review_type: plan
assessment_date: 2026-06-26
codebase_analyzed: prompting-press / spec-003-rust-consumer
findings_total: 4
findings_critical: 0
findings_high: 0
findings_moderate: 1
findings_low: 1
findings_informational: 2
overall_risk: low
owasp_categories:
  - "A04:2021 Insecure Design — error-detail leakage scrub (SEC-004 carryover) and provenance-lint honest-boundary"
  - "A09:2021 Security Logging and Monitoring Failures — normalizer must not echo bound-value content into message/logs"
  - "A06:2021 Vulnerable and Outdated Components — garde 0.23 + serde_yaml_ng 0.10 (new pure-Rust deps under the workspace advisory gate)"
  - "A03:2021 Injection — untrusted-YAML deserialization resource consumption (out of scope for v1 trust model; flagged)"
cwe_ids:
  - "CWE-209: Generation of Error Message Containing Sensitive Information — the inherited SEC-004 leak is now in the layer responsible for fixing it (FR-015)"
  - "CWE-655: Improper Initialization (provenance lint mistaken for a runtime control / sanitizer) — SEC-002 carryover into the lint surface"
  - "CWE-776 / CWE-400: Improper Restriction of Recursive Entity References / Uncontrolled Resource Consumption — deep-nesting / large-doc deserialization; out of scope for v1 (repo-canonical PR-gated input) but flagged for untrusted-YAML callers"
  - "CWE-1104: Use of Unmaintained Third-Party Components — serde_yaml (archived) must NOT be used; serde_yaml_ng is the maintained successor; under the cargo-deny gate"
field_summaries:
  scope: "FFI-free, NO-I/O Rust CONSUMER crate (prompting-press) layered on the spec-002 kernel. Adds the language-native layer the kernel omits: a garde typed-Vars facade (validate-once-at-render), a dual-input YAML/JSON/object loader, a registry (name→PromptDefinition), the check() agreement+provenance lint, error normalization to [{field,code,message}], Vec+append_* composition, and a count_tokens hook seam. It reads NO files/network/env (the caller hands in already-read YAML/JSON text or a constructed object), makes NO LLM calls, assembles NO request body, parses NO model output, ships NO built-in token counter, and duplicates NO kernel render/agreement/variant/hash logic. authn/authz, sessions, SQLi, SSRF, transport, secrets-at-rest are absent by constitutional construction (Principle III / C-03), not unimplemented."
  authentication: "N/A — the consumer performs no authentication. It is a set of pure library functions (Registry load/get, render, get_source, check, Composition::resolve) over pushed-in data. No credentials, tokens, or identity handling exist in scope."
  authorization: "N/A — no access-control decisions. Variant selection is caller-owned (C-05); render() validates a name exists in the registry and delegates variant resolution to the kernel (UnknownPrompt / kernel UnknownVariant are the only resolution errors). No privilege logic."
  data_protection: "No data at rest, no transport, no secrets storage. The registry is an in-memory BTreeMap<String, PromptDefinition> built from caller-supplied text/objects; bound Vars values live in-memory for one synchronous validate+render and are never persisted or sent to a sink (no ProvenanceSink — provenance is data on RenderResult, C-05/C-08). The only path where caller value content could transit a boundary is normalized error detail (SEC-004), which FR-015 mandates be scrubbed."
  input_validation: "This crate IS the input gate the kernel deliberately lacks (FR-002/003): garde validates the whole Vars struct ONCE before any templating; on failure no render is performed and the kernel never sees the values. Two distinct input boundaries: (1) typed Vars → garde validate() then minijinja::Value::from_serialize → kernel (validate-blind kernel preserved); (2) the dual-input loader → serde deserialization of YAML/JSON into the kernel's PromptDefinition. serde_yaml_ng (yaml-rust2, YAML-1.2) resolves the Norway problem (no/yes/off parse as strings). Malformed input is a structured ConsumerError, never a partial load (FR-007). The deep-nesting/large-doc deserialization surface is the one genuinely new risk (SEC-005) and is out of scope for v1's repo-canonical/PR-gated trust model."
  injection: "Template/SSTI is the kernel's risk class and is resolved there (values bound as data, never re-parsed; macros/multi_template off at parse time — confirmed in the spec-002 staged review). The consumer adds no new injection sink: render() delegates to the kernel, the loader deserializes into a typed shape (no eval, no template re-entry), and check() is pure set-analysis. The provenance lint (SEC-002 carryover) is a LINT, not a runtime sanitizer — over-trusting it as injection mitigation is the only residual, addressed by carrying the kernel's honest-boundary invariant into the consumer doc/check surface."
  cryptography: "N/A in this crate. template_hash/render_hash are computed once in the kernel (SHA-256 content-addressing, confirmed correct in spec-002) and surfaced unchanged on RenderResult. The consumer computes no hashes and treats them as opaque trace identifiers (FR-011: no hashing logic here)."
  error_handling: "The crate's normalization layer (FR-014/015) is the new security-relevant surface and the home of the inherited SEC-004 concern. garde Report and kernel KernelError are mapped to a closed ConsumerError = [{field,code,message}] at the public boundary and never leak (C-06). The KernelError::Parse/Render detail strings may transitively carry bound-value content; FR-015 requires the normalizer NOT to copy raw detail into the externally-surfaced message or any log. The crate must also avoid introducing logging primitives (println/log/tracing) that would emit that content (SEC-006)."
  secrets_management: "N/A — the crate handles no secrets, credentials, API keys, or connection strings, and does no environment access (C-03/FR-024). Caller Vars values could contain secrets/PII, but the crate never persists, transmits, or (per FR-015 + SEC-006) logs them; the only boundary-crossing path is the normalized error message, which FR-015 scrubs."
---

# Security Review — Spec 003: Rust consumer crate (`prompting-press`)

## Executive summary

**Overall risk: LOW.** Spec 003 plans the Rust consumer layer over the spec-002 kernel — a typed-Vars
validation facade (garde), a dual-input YAML/JSON/object loader, a registry, the `check()` agreement +
provenance lint, error normalization, composition, and a `count_tokens` hook seam. It is FFI-free,
performs no I/O (the caller hands in already-read text or a constructed object — the crate reads no
files/network/env), makes no LLM calls, assembles no request body, parses no model output, and ships
no built-in token counter. The classic web-app threat surface (authn/authz, sessions, SQLi, SSRF,
transport, secrets-at-rest) is **absent by constitutional construction (Principle III / C-03), not
merely unaddressed.** I did not manufacture findings for those absent surfaces.

The design is sound. Calibrated to what a validation+templating consumer library actually risks, the
genuinely security-relevant areas were assessed directly, and **the headline carryover — SEC-004,
error-detail leakage — lands in exactly the layer (003's normalizer) responsible for fixing it, and
the plan specifies that fix (FR-015) in normative MUST-NOT language across spec/data-model/contract.**
The verdict is therefore "design sound; no new high-signal findings beyond the SEC-004 carryover and
two carried-forward invariants," plus one genuinely new (and correctly out-of-scope) deserialization
consideration worth recording so its trust boundary is explicit and revisitable.

1. **Validation bypass (the real vuln class for this crate)** — assessed and **not present in design**.
   The only path values reach the kernel is `render` → garde `validate()` → on success
   `Value::from_serialize` → kernel; the contract states "validate ONCE before any templating; on
   failure no render performed" (FR-002, V1.2). Composition (`resolve`) validates each entry before
   rendering it (V4.2, US4 sc.3). `check()` and `get_source` take no Vars values, so there is no
   value-bearing path that skips `validate()`. Recorded as a confirmed secure-by-design pattern.

2. **Error-detail leakage (SEC-004, inherited from spec 002)** — the most important carryover, and the
   plan handles it honestly. FR-015 ("Error normalization MUST NOT echo raw, potentially sensitive
   bound-value content into error messages or logs") is restated as a normative scrub rule in
   data-model.md (§NormalizedError, explicit "MUST NOT copy raw detail verbatim") and contracts
   (§5, "`Parse`/`Render` kernel detail is **sanitized**, never copied raw"). This is specified, not
   hand-waved. The residual is purely implementation discipline; raised as SEC-004 (LOW) to keep the
   obligation visible and add a concrete acceptance check.

3. **Provenance lint (C-09) is a security FEATURE with an honest boundary** — the plan keeps the same
   honesty bar as spec-002 SEC-002: `check()` is a CI **lint** over `def.variables`/`provenance_view`
   metadata, pure analysis, no mutation/render (FR-019). Nothing in the plan over-promises that tagging
   a field `untrusted`/`external` *protects* it. Carried forward as SEC-002 (INFORMATIONAL) only to
   require the non-enforcement invariant be repeated at the consumer surface where users wire tags.

4. **Deserialization safety (genuinely new surface)** — the dual-input loader deserializes YAML/JSON
   into `PromptDefinition` via serde. For v1's trust model (repo-canonical, PR-gated prompt defs) this
   is fine. A future consumer loading *untrusted* YAML could hit unbounded resource consumption (deep
   nesting / large docs); yaml-rust2 (serde_yaml_ng's backend) does not expand `&anchor`/`*alias`
   into serde structures the way a billion-laughs-vulnerable YAML lib would, but recursion/size bounds
   are not asserted. Raised as SEC-005 (INFORMATIONAL) with an explicit trigger condition, not a v1
   blocker.

5. **Dependencies / supply chain** — `garde 0.23` and `serde_yaml_ng 0.10` are both pure-Rust (the
   FFI gate, which I confirmed explicitly covers the `prompting-press` consumer crate, stays green),
   both fall under the spec-002 `cargo-deny` advisory gate (`deny.toml` + `ci:check-advisories`,
   confirmed present and workspace-wide), and the `ci:check-floating-versions` gate enforces exact
   pins. The one real hygiene item: `serde_yaml` (dtolnay) is **archived** (`0.9.34+deprecated`) and
   MUST NOT be used; the research correctly selects `serde_yaml_ng`. One LOW note (SEC-001) to bind
   that constraint and pin the exact patch.

6. **No secrets / no logging in the crate** — confirmed the plan keeps the consumer free of I/O and
   secrets handling (C-03/FR-024); the new risk spot per SEC-004 is the normalizer emitting content.
   SEC-006 (INFORMATIONAL) records that the crate MUST NOT introduce `println`/`log`/`tracing` of
   error detail or bound values — the same forbidden-primitive discipline spec-002 verified for the
   kernel.

Finding counts: **0 critical, 0 high, 1 moderate, 1 low, 2 informational.** No finding blocks
implementation. SEC-004 is the one carrying a concrete pre-merge acceptance check; the rest are
forward-looking notes or carried-forward invariants.

## Plan artifacts reviewed

- `specs/003-rust-consumer/plan.md` — implementation plan, Constitution Check (all PASS), structure,
  complexity tracking, verified-this-cycle dependency notes.
- `specs/003-rust-consumer/spec.md` — FR-001..FR-024, SC-001..SC-009, US1–US4, edge cases (incl. the
  explicit "Validation error detail leakage" edge case), assumptions, dependencies.
- `specs/003-rust-consumer/research.md` — D1–D7 (garde 0.23 + Report API, serde_yaml_ng 0.10 /
  yaml-rust2, Report→[{field,code,message}] mapping, `Value::from_serialize` bridge, lint set-ops,
  API shapes, crate structure/FFI). Includes the recorded correction of a research subagent's
  fabricated version numbers.
- `specs/003-rust-consumer/data-model.md` — consumed kernel types, consumer-defined types
  (`Registry`, `ConsumerError`/`FieldError`, `CheckReport`/`Finding`, `Message`, `TokenCountHook`),
  the SEC-004 scrub rule, state/lifecycle.
- `specs/003-rust-consumer/contracts/consumer-api.md` — the normative public Rust API contract and
  cross-cutting invariants (FFI-free, validation-blind kernel, no leaked native types, no I/O,
  pure `check()`).
- `specs/003-rust-consumer/quickstart.md` — ~18 validation scenarios (US1–US4 + token/boundary),
  including V5.3 `cargo tree` FFI check.
- `.specify/memory/constitution.md` — Principles III (minimal boundary), IV (agreement check),
  VI (per-language idiom / error normalization).
- `.specify/memory/roadmap.md` — C-03/C-06/C-07 (003's governing decisions) + C-09 (provenance);
  the 003 roadmap entry.
- `docs/memory/INDEX.md` — durable memory index (architecture/decisions/bugs dirs empty — fresh).
- `specs/002-engine-kernel/security-review-report.md` + `security-review-report-staged.md` — the
  inherited plan- and code-stage reviews; SEC-004 (error-detail leak) and SEC-002 (non-enforcing
  provenance) are the carryovers 003 must honor.
- **Corroborating live state** (read-only confirmation, not a 003 artifact): `deny.toml`,
  `scripts/ci/check-ffi-isolation.sh` (confirmed `COVERED_CRATES` includes `prompting-press`),
  `ci/moon.yml` (`check-ffi` / `check-floating-versions` / `check-advisories` all wired), and the
  current `crates/prompting-press/Cargo.toml` stub.

## Vulnerability findings

### SEC-004 — Error normalizer must scrub bound-value content from `message`/logs (LOW) — inherited carryover

- **OWASP 2025:** A09:2021 Security Logging and Monitoring Failures (info leakage via error detail) /
  A04 Insecure Design.
- **CWE:** CWE-209 (Generation of Error Message Containing Sensitive Information).
- **Severity:** LOW. (Carried at the same severity the kernel plan-stage review assigned it. It is the
  consumer's responsibility — this is the layer that surfaces the message — so it cannot be downgraded
  to informational here the way the *kernel's* version was, because in the kernel the detail was
  latent-by-type and unrealized; in the consumer the `message` field is the externally-surfaced
  string, so the scrub must actually be implemented.)
- **Location:** `crates/prompting-press/src/error.rs` (the `KernelError` → `ConsumerError`/`FieldError`
  mapping); contract §5; data-model §NormalizedError.
- **Evidence:** The kernel's `KernelError::Parse { detail }` and `Render { detail }` carry free-form
  strings sourced from MiniJinja `err.to_string()`, which for a render-time type error can embed a
  representation of the offending — possibly caller-supplied, possibly sensitive (secret/PII) — bound
  *value* (spec-002 SEC-004, confirmed in the staged review). 003 is where that detail is consumed and
  re-emitted: FR-014 maps `KernelError` to `FieldError { field, code, message }`. **The plan specifies
  the fix correctly and in normative language:** FR-015 ("Error normalization MUST NOT echo raw,
  potentially sensitive bound-value content into error messages or logs"); data-model §NormalizedError
  ("the normalizer MUST NOT copy raw detail verbatim into `message`/logs; use a sanitized/templated
  message"); contract §5 ("`Parse`/`Render` kernel detail is **sanitized**, never copied raw into
  message/logs (FR-015 / SEC-004)"). This is specified, not hand-waved — the finding is about ensuring
  it is *implemented and tested*, not about a planning gap.
- **Remediation:**
  1. In `error.rs`, map `KernelError::Parse`/`Render`/`ExcludedFeature` to a **fixed, templated**
     `message` keyed on the error class (e.g. `"template render failed"` / `"template parse failed"`),
     and do NOT interpolate the kernel `detail` string into the `message` that `ConsumerError` exposes
     publicly. If the raw detail is retained at all, confine it to a separate, non-default debug-only
     field that the public `Display`/serialization does not surface, and document it as
     trusted-debug-only (mirroring the kernel's own `error.rs` rustdoc).
  2. Add an **acceptance test** (extend quickstart V1.4 / add a boundary scenario): render with a
     Vars value containing a sentinel secret string that triggers a kernel `Render` error, assert the
     sentinel does **not** appear anywhere in the resulting `ConsumerError`'s public string
     representation. This turns FR-015 from an assertion into a verified property and is the
     load-bearing addition this finding asks for.
  3. Synthesize a stable `code` per `KernelError` variant (the plan already does this — D3) so callers
     can branch on the class without ever needing the raw detail.

### SEC-002 — Provenance lint is a lint, not a runtime sanitizer; carry the non-enforcement invariant to the consumer surface (INFORMATIONAL) — inherited carryover

- **OWASP 2025:** A04:2021 Insecure Design (security-control assumption mismatch).
- **CWE:** CWE-655 (Improper Initialization of a security mechanism — a feature that reads like a
  control but enforces nothing).
- **Severity:** INFORMATIONAL. (Downgraded from the kernel plan-stage MODERATE: the plan does **not**
  over-promise — `check()` is consistently described as a pure CI lint, not enforcement — so the only
  residual is the naming connotation, exactly the kernel's realized-in-code state.)
- **Location:** `crates/prompting-press/src/check.rs` (`CheckReport`/`Finding::UntrustedOutsideGuard`);
  contract §3; data-model §CheckReport.
- **Evidence:** The provenance lint (FR-018) reports an `untrusted`/`external` field used outside a
  declared guard position. The plan states its boundary honestly: `check()` is "pure analysis —
  pass/fail" that "MUST NOT mutate any prompt, definition, or input, render anything, or produce side
  effects" (FR-019), runs over `def.variables` / `provenance_view` metadata, and surfaces a `Finding`,
  not a mutation. **Nothing in the plan claims that tagging a field `untrusted`/`external` sanitizes,
  strips, or protects it** — there is no over-promise to flag. The residual is identical to the
  kernel's SEC-002: the *words* `untrusted`/`guard` connote a sanitizer to a casual reader, and the
  consumer's `check`/lint surface is exactly where a user wires a tag to a field and is most likely to
  conclude "the untrusted field is now handled."
- **Remediation:** No design change. When implementing `check.rs` (and its rustdoc / `Finding` docs),
  repeat the kernel's normative invariant: *"Provenance tags and the guard are advisory metadata only;
  `check()` reports their misuse but does NOT sanitize, strip, or neutralize any value. A passing
  `check()` is not evidence that an untrusted field's content is safe."* This is the spec-002 SEC-002
  remediation item 3 ("carry forward to consumer specs 003/004/005") being discharged at 003.

### SEC-005 — Untrusted-YAML/JSON deserialization has no asserted depth/size bound (INFORMATIONAL)

- **OWASP 2025:** A03:2021-adjacent / A06 availability — informational; not a classic injection/exposure.
- **CWE:** CWE-776 (Improper Restriction of Recursive Entity References) / CWE-400 (Uncontrolled
  Resource Consumption) — assessed; out of scope for v1's trust model.
- **Severity:** INFORMATIONAL.
- **Location:** `crates/prompting-press/src/registry.rs` (`load_yaml` / `load_json`); research D2.
- **Evidence:** The dual-input loader deserializes a caller-supplied document into the kernel's
  `PromptDefinition` via `serde_yaml_ng` (YAML arm) and `serde_json` (JSON arm). A maliciously deep or
  very large document could in principle drive unbounded stack/heap during deserialization. Two
  mitigating facts make this **out of scope for v1**: (1) the trust model — prompt definitions are
  repo-canonical and PR-gated (spec.md Assumptions; constitution Principle V), so a pathological def is
  a code-review defect, not an external attack vector; the caller, not the crate, decides what text to
  hand in; (2) the backend — `serde_yaml_ng` is built on `yaml-rust2`, a pure-Rust YAML-1.2 parser
  that does not perform the unbounded `&anchor`/`*alias` *expansion into the deserialized structure*
  that makes the classic "billion laughs" amplification catastrophic in entity-expanding parsers; the
  `PromptDefinition` shape is also finite-keyed (the open-object `meta`/`metadata` maps are the only
  unbounded arms). `serde_json` by default rejects pathological nesting via its recursion limit. The
  user prompt's specific question — "could a malicious/huge YAML cause unbounded memory (deep nesting,
  anchors/aliases)?" — resolves to: *deep nesting is the residual concern; anchor/alias amplification
  is not the dominant risk for yaml-rust2; and the whole class is out of scope under the PR-gated trust
  model.* Worth recording because the spec.md text "another part of the application builds a prompt
  definition programmatically" and "a consumer might load untrusted YAML" are both plausible future
  patterns the crate does not structurally prevent.
- **Remediation:** None required for v1 — do not add limits reflexively against a vector the v1 trust
  model does not expose. **Trigger to revisit:** if a future spec or consumer lets *untrusted/external*
  parties supply prompt-definition YAML/JSON (e.g. a hosted authoring backend — on the roadmap "Never"
  list — or user-uploaded prompt files), assert an explicit input-size cap and a deserialization
  depth/recursion bound before `load_yaml`/`load_json`, and re-confirm `yaml-rust2`'s anchor-handling
  posture at that version. Record this trigger so the decision stays revisitable rather than silently
  permanent (mirrors spec-002 SEC-003).

### SEC-001 — New deps (`garde`/`serde_yaml_ng`) must be exact-pinned and `serde_yaml` (archived) must not creep in (LOW)

- **OWASP 2025:** A06:2021 Vulnerable and Outdated Components.
- **CWE:** CWE-1104 (Use of Unmaintained Third-Party Components) — forward-looking; no vulnerable
  component identified at review time.
- **Severity:** LOW.
- **Location:** `crates/prompting-press/Cargo.toml` (to be edited per plan); `deny.toml`;
  `Cargo.lock`.
- **Evidence:** 003 adds two new pure-Rust deps: `garde 0.23` (`derive`+`serde`; only non-pure dep
  `js-sys` is optional/non-default, research D1) and `serde_yaml_ng 0.10` (research D2). I confirmed
  the inherited workspace gates cover them: `scripts/ci/check-ffi-isolation.sh` explicitly lists
  `prompting-press` in `COVERED_CRATES` (so the FFI gate catches any pyo3/napi creep into the consumer
  — SC-007 / V5.3); `deny.toml` + `ci:check-advisories` (cargo-deny `advisories`, present and green per
  the spec-002 staged review) scan the whole `Cargo.lock`, so the new deps fall under the advisory gate
  automatically; `ci:check-floating-versions` rejects `^`/`~`/`*`/`latest`. The two residual hygiene
  items: (1) the plan/research note the versions must be **pinned at the exact current patch level at
  implementation time** ("Pin the exact current patch", "confirm the exact current version") — that pin
  must actually land, and the floating-versions gate enforces it; (2) `serde_yaml` (dtolnay) is
  **archived** (`0.9.34+deprecated`, CWE-1104) and MUST NOT be used — the research correctly selects
  the maintained `serde_yaml_ng`, but a later edit could regress. No advisory is known to affect
  `garde 0.23` or `serde_yaml_ng 0.10` / `yaml-rust2` at this review date — this is hygiene, not a live
  exposure.
- **Remediation:** Pin `garde` and `serde_yaml_ng` to exact patch versions in
  `crates/prompting-press/Cargo.toml` (or the workspace dep table) and let `cargo build --locked` +
  `ci:check-floating-versions` enforce it. Optionally add a one-line `deny.toml` `[bans]` entry (or a
  review note) denying `serde_yaml` so the archived crate cannot re-enter the tree. Rely on the
  existing `ci:check-advisories` gate for ongoing RustSec coverage of the new deps — no new gate
  needed; the workspace gate already covers them. Low priority; lands with this spec.

## Confirmed secure-by-design patterns

Not findings — properties I verified hold in the plan and that materially reduce the consumer's real
risk. Recorded so a later reviewer does not re-litigate them.

- **No validation bypass — there is no value-bearing path that skips `validate()`.** `render` and
  `Composition::resolve` both run garde `validate()` before any templating, and on failure perform no
  render (FR-002, contract §2/§4, V1.2, V4.2). The two operations that do *not* validate —
  `check()` and `get_source` — take **no** Vars values at all (contract §1/§3), so they cannot smuggle
  unvalidated input to the kernel. The kernel stays validation-blind (FR-003); only validated values,
  bridged via `Value::from_serialize`, reach it. This is the core vuln class for this crate, and the
  design closes it.
- **Native error types never leak (closed normalized boundary).** `ConsumerError`/`FieldError` is the
  only public error type; garde `Report` and kernel `KernelError` are mapped at the boundary and never
  exposed (FR-014, C-06, SC-006, V1.4). This both satisfies the per-language-idiom principle and bounds
  the info-leakage surface to a single, auditable mapping site (where SEC-004's scrub is enforced).
- **`check()` is pure analysis — no mutation, no render, no side effects.** The lint computes set-ops
  (`required_roots` ∖ `def.variables`) and a provenance comparison over kernel-provided metadata,
  using deterministic `BTreeSet`/`BTreeMap` ordering, and "MUST NOT mutate any prompt, definition, or
  input, render anything, or produce side effects" (FR-019, contract §3, V3.4). No analysis/render
  disagreement surface; no side-channel via input mutation.
- **No logic duplication — render/agreement/variant/hash stay in the kernel.** The consumer wraps
  `render`/`get_source`/`required_roots`/`provenance_view` and computes only set differences and the
  registry walk (FR-011, C-01, research D5). The SHA-256 hashes are computed once in the kernel and
  surfaced unchanged; the consumer treats them as opaque trace identifiers and adds no crypto.
- **FFI isolation is CI-enforced for the consumer, with pure-Rust new deps.** I confirmed
  `check-ffi-isolation.sh` lists `prompting-press` in `COVERED_CRATES`; `garde` (with `js-sys`
  optional/off) and `serde_yaml_ng`/`yaml-rust2` are pure-Rust (research D1/D2), so the gate stays
  green (SC-007, V5.3). The minimal boundary is structurally verifiable, not aspirational.
- **No I/O / no LLM / no output parsing / no built-in token counter.** The crate accepts already-read
  text or a constructed object (the caller reads files — C-03/FR-024); `output_model` is carried as
  metadata only and never parsed; the `count_tokens` hook is the *only* token-counting mechanism and
  ships no estimate (FR-021/022). The boundary-defense list (roadmap "Never") is respected — no
  boundary-expanding capability is added.
- **Dual-input parity is a load (not transform) — one shape, no parallel definition.** YAML, JSON, and
  a constructed object all normalize to the kernel's single `PromptDefinition` (FR-005/006/008, SC-003,
  V2.1–V2.3); the crate defines no parallel shape, so there is no schema↔shape divergence surface in
  the consumer. YAML-1.2 (Norway-safe) deserialization means `no`/`yes`/`off` are strings, removing a
  silent-coercion footgun (V2.5).

## Proposed INDEX.md routing row

flash-mem / memory-hub not installed (markdown-only flow). Proposed row for the **Security** section of
`docs/memory/INDEX.md` (create the heading if absent), kept distinct from the spec-002 rows:

```markdown
## Security

- [Spec 003 plan-stage security review](../../specs/003-rust-consumer/security-review-report.md) —
  PLAN review of the Rust consumer crate. Overall risk LOW; design sound. No validation bypass
  (validate-once-before-render; check()/get_source take no values). Headline carryover SEC-004
  (error-detail leakage) lands in 003's normalizer and FR-015 specifies the scrub — finding adds a
  sentinel-secret acceptance test as the load-bearing pre-merge check (LOW). Carried-forward
  invariants: SEC-002 provenance lint is a LINT not a sanitizer — repeat the non-enforcement doc at
  check.rs (INFORMATIONAL); SEC-001 pin garde 0.23 + serde_yaml_ng 0.10 exactly, never serde_yaml
  (archived), under the existing FFI + cargo-deny + floating-version gates which were confirmed to
  cover the consumer crate (LOW). New surface SEC-005: untrusted-YAML/JSON deserialization has no
  asserted depth/size bound — out of scope under the repo-canonical/PR-gated trust model; revisit only
  if untrusted parties ever supply prompt-def text (INFORMATIONAL). SEC-006: crate must introduce no
  println/log/tracing of error detail or bound values (INFORMATIONAL).
```
