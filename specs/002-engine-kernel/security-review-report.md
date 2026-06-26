---
document_type: security-review
review_type: plan
assessment_date: 2026-06-26
codebase_analyzed: prompting-press / spec-002-engine-kernel
findings_total: 4
findings_critical: 0
findings_high: 0
findings_moderate: 1
findings_low: 2
findings_informational: 1
overall_risk: low
owasp_categories:
  - "A03:2021 Injection (template/SSTI surface) — out of scope by construction for v1, residual consumer-misuse risk"
  - "A06:2021 Vulnerable and Outdated Components (dependency / supply chain)"
  - "A04:2021 Insecure Design (provenance-tag over-trust / honest-boundary documentation)"
cwe_ids:
  - "CWE-1336: Improper Neutralization of Special Elements Used in a Template Engine (SSTI) — bound-value re-parse path; confirmed not present in design"
  - "CWE-655: Improper Initialization (security feature mistaken for an enforcement control)"
  - "CWE-1104: Use of Unmaintained Third-Party Components (dependency-currency tracking)"
  - "CWE-209: Generation of Error Message Containing Sensitive Information (render-error detail strings)"
field_summaries:
  scope: "FFI-free, validation-blind, NO-I/O Rust template-rendering kernel (prompting-press-core). Renders repo-canonical Jinja-family templates via MiniJinja 2.21 over already-validated caller-supplied values; emits SHA256 content hashes + var-provenance metadata. No authn/authz, no sessions, no I/O, no network, no LLM calls, no secrets handling — those surfaces are out of scope by constitutional construction (Principle III / C-03)."
  authentication: "N/A — kernel performs no authentication; it is a pure library function over pushed-in data. No credentials, tokens, or identity handling exist in scope."
  authorization: "N/A — no access-control decisions are made by the kernel. Variant selection is caller-owned (C-05); the kernel only validates a variant name exists."
  data_protection: "No data at rest, no transport, no secrets. Bound values are held in-memory for the duration of a synchronous render and never persisted, logged, or emitted to a sink (FR-005, FR-015). SHA256 hashes are content-addressing identifiers, not security tokens — correct usage confirmed (D8)."
  input_validation: "Kernel is deliberately validation-blind (FR-004): input validation is the consumer layer's job (spec 003+). The relevant kernel-level input hardening is template-feature restriction (macros/multi_template disabled → include/import/extends/macro/block are parse errors, FR-002) and strict-undefined handling (FR-001a). Bound values are treated as data, never re-parsed as template syntax — the SSTI-defeating invariant."
  injection: "Template-injection surface is the kernel's primary real risk class. v1 templates are repo-canonical / PR-gated (not attacker-controlled); bound VALUES may be untrusted but are passed as a minijinja::Value context and are NOT re-parsed as template source. MiniJinja autoescape is not security-relevant for plain prompt text (no HTML sink). Residual risk is consumer over-trust of provenance tags, addressed in SEC-002."
  cryptography: "SHA256 via the sha2 (RustCrypto) crate for template_hash/render_hash. Used purely for content identity in provenance/traces — not for authentication, integrity-against-adversary, or secrecy. Collision/second-preimage concerns are not applicable to this content-addressing use. No misuse found."
  dependencies: "Two new pure-Rust deps: minijinja 2.21 (default-features=false) and sha2. Neither pulls pyo3/napi (D7) so the FFI-isolation gate stays green. Versions are exact-pinned and the spec-001 check-floating-versions gate + cargo build --locked enforce hash-locking. No known-CVE concern identified for these crates/versions at review time; see SEC-001 for the dependency-currency tracking recommendation."
  error_handling: "KernelError variants carry free-form detail strings sourced from MiniJinja parse/render errors. Native KernelError is returned (not normalized — that is the consumer's job, C-06). Strict-undefined errors name the missing variable (a template-authored identifier, low sensitivity). Render-error detail MAY transitively echo bound-value content; flagged as SEC-004 (low) for the consumer's error-normalization layer to scrub before logging."
  secrets_management: "N/A — the kernel handles no secrets, credentials, API keys, or connection strings. It does no environment access (FR-005). Bound values could contain caller secrets, but the kernel never persists or transmits them; see SEC-004 for the only path (error detail) where value content could transit a boundary."
---

# Security Review — Spec 002: Engine kernel (`prompting-press-core`)

## Executive summary

**Overall risk: LOW.** Spec 002 plans a deliberately narrow, FFI-free, validation-blind, no-I/O Rust
template-rendering kernel. By constitutional construction (Principle III / C-03) it performs no I/O,
makes no network/LLM calls, assembles no request body, counts no tokens, parses no model output, and
holds no telemetry sink. The entire classic web-application threat surface — authentication,
authorization, sessions, SQL injection, SSRF, transport security, secrets-at-rest — is **absent by
design, not merely unaddressed.** I did not manufacture findings for those absent surfaces.

The genuinely security-relevant areas for a template kernel were assessed directly:

1. **Template injection / SSTI** — the design is sound. v1 templates are repo-canonical and PR-gated
   (not attacker-controlled); bound values are passed as a `minijinja::Value` context and are **never
   re-parsed as template source**, which is the load-bearing anti-SSTI property. Disabling `macros`
   and `multi_template` (research D1) removes the highest-leverage SSTI/sandbox-escape constructs
   (`include`/`import`/`extends`/`macro`/`block`) at **parse time**, which is meaningful hardening
   beyond its stated FR-002 purpose. This is recorded as a confirmed secure-by-design pattern, not a
   finding.

2. **Var-provenance / guard (C-09)** — the explicit security feature. The plan states its boundary
   honestly: tags are **metadata-only with no runtime enforcement**, and the guard is **opt-in,
   additive, and does not sanitize** (FR-023, FR-025). The one residual risk is a consumer
   *mistaking* the `untrusted`/`external` tag or the guard for a sanitizer (SEC-002, MODERATE — a
   documentation/insecure-design risk, not a kernel defect).

3. **Hashing** — SHA256 used correctly for content addressing, not as a security token. No misuse.

4. **Dependencies / supply chain** — both new crates are pure-Rust, pin exactly, inherit spec-001's
   floating-version gate and `--locked` build, and keep the FFI gate green. One low-severity tracking
   recommendation (SEC-001) and one informational denial-of-service note (SEC-003).

5. **Error info-leakage** — one low-severity note (SEC-004): render-error detail strings can
   transitively echo bound-value content; the consumer's error-normalization layer (spec 003) should
   scrub before logging.

Finding counts: **0 critical, 0 high, 1 moderate, 2 low, 1 informational.** No finding blocks
implementation. SEC-002 is the only one worth a design-time action (a one-line doc invariant); the
rest are forward-looking notes for specs 003+ or routine hygiene.

## Plan artifacts reviewed

- `specs/002-engine-kernel/plan.md` — implementation plan, Constitution Check, complexity tracking.
- `specs/002-engine-kernel/spec.md` — FR-001..FR-029, SC-001..SC-009, edge cases, assumptions.
- `specs/002-engine-kernel/research.md` — D1–D8 dependency/design decisions; FR-010/011 contradiction
  resolution.
- `specs/002-engine-kernel/data-model.md` — kernel-defined types (`RenderResult`, `Agreement`,
  `GuardConfig`, `ProvenanceView`, `KernelError`).
- `specs/002-engine-kernel/contracts/kernel-api.md` — public Rust API contract.
- `specs/002-engine-kernel/quickstart.md` — ~25 validation scenarios.
- `specs/002-engine-kernel/tasks.md` — T001–T035 task breakdown.
- `.specify/memory/constitution.md` — Principles I–VII (esp. III, IV).
- `.specify/memory/roadmap.md` — C-01..C-10 (esp. C-09 var-provenance decision).
- `docs/memory/INDEX.md` — durable memory index (architecture/decisions/bugs dirs empty; fresh project).
- Corroborating existing CI (spec 001): `.github/workflows/ci.yml`, `ci/moon.yml`, root `Cargo.toml`
  — to confirm the floating-version gate, `--locked` build, FFI-isolation gate, and workspace pins
  that spec 002's dependency posture inherits.

## Vulnerability findings

### SEC-001 — Dependency-currency tracking for `minijinja`/`sha2` not yet established for the kernel (LOW)

- **OWASP 2025:** A06:2021 Vulnerable and Outdated Components.
- **CWE:** CWE-1104 (Use of Unmaintained Third-Party Components) / CWE-1395 (dependency on
  vulnerable third-party component) — *forward-looking*, no current vulnerable component identified.
- **Severity:** LOW.
- **Evidence:** Plan/research D1/D7/D8 add `minijinja = "2.21"` (default-features=false) and `sha2`.
  Both are exact-pinned, both are pure-Rust, neither pulls `pyo3`/`napi`, and the inherited spec-001
  gates (`ci:check-floating-versions` rejecting `^`/`~`/`*`/`latest`; `cargo build --workspace
  --locked` in `.github/workflows/ci.yml`) mean the lockfile hash-locks the transitive set. That
  posture is good. What is **not** present in the plan or task list is any recurring
  vulnerability-audit step (e.g. `cargo audit` / `cargo deny` against the RustSec advisory DB) or an
  explicit owner for the roadmap Q3 "re-confirm on each MiniJinja bump" commitment. A pinned-but-
  unmonitored dependency is the standard way a library accretes a known-CVE transitive dep over time.
  No advisory is known to affect `minijinja 2.21` or current `sha2` at this review date — this is a
  process gap, not a live exposure.
- **Remediation:** Add a CI advisory-scan gate (`cargo audit` or `cargo deny check advisories`,
  pinned, against a pinned advisory-DB revision) as a routine workspace gate, and bind the roadmap Q3
  "re-confirm at each MiniJinja bump" line to that gate. Low priority; can land with this spec or in a
  later hardening pass. This is a workspace-level concern, not a kernel-code change.

### SEC-002 — Provenance tags / guard text are non-enforcing and could be over-trusted as a sanitizer (MODERATE)

- **OWASP 2025:** A04:2021 Insecure Design (security-control assumption mismatch). Touches A03
  (Injection) only indirectly — the kernel does not inject; the risk is a *downstream* consumer
  treating a non-control as a control.
- **CWE:** CWE-655 (Improper Initialization of a security mechanism — here, a feature that reads like
  a control but enforces nothing) / CWE-1059 (insufficient documentation of a security-relevant
  decision).
- **Severity:** MODERATE. (Calibrated above LOW because the misuse, if it occurs in a consumer, can
  produce a prompt-injection exposure that the tag's name actively invites; calibrated below HIGH
  because the kernel itself is correct, the boundary is *already* documented honestly in this spec,
  and no exploit exists within spec-002's scope — the risk is realized only by a future consumer's
  misreading.)
- **Evidence:** The spec is commendably explicit that provenance is **"metadata + lint + opt-in
  guard only"** (FR-025), that the guard is **additive and non-mutating** and **MUST NOT** strip,
  escape, or sanitize values (FR-023, FR-025), and that guard text placement is the caller's decision
  (FR-022). C-09 records that sanitization/stripping was *deliberately rejected*, and the roadmap
  "Never" list reaffirms it. **The design decision is sound and the boundary is stated honestly — that
  is exactly right and is the reason this is MODERATE and not HIGH.** The residual risk is purely one
  of *naming and downstream expectation*: a field literally tagged `untrusted` and a kernel method
  named "guard" strongly connote a sanitizer. A consumer author (spec 003+, or an external user) who
  enables guard expansion and concludes "the untrusted field is now handled" would ship a
  prompt-injection-exposed prompt while believing the opposite. The kernel pass-through is correct
  (SC-005 proves the value is byte-unchanged); the gap is that nothing in the *type or method
  surface* signals "this does not sanitize."
- **Remediation:**
  1. In the kernel's public rustdoc (T033 already plans crate-level docs), state as a normative
     invariant on `GuardConfig` / the guard field / `ProvenanceView`: *"Provenance tags and guard
     text are advisory metadata only. They do NOT sanitize, strip, escape, or otherwise neutralize
     untrusted values; values pass through byte-for-byte (SC-005). A guard naming a field is not a
     mitigation of that field's content."* This costs one doc block and converts an implicit
     assumption into an explicit, discoverable contract.
  2. Consider naming that does not over-promise (e.g. `advisory_guard` / `guard_notice` rather than a
     bare `guard`) — optional, but reduces the connotation at the API surface. Defer to the
     implementer; the doc invariant is the load-bearing fix.
  3. Carry this invariant forward to the consumer specs (003/004/005) so the binding-level Vars facades
     and the `check`/lint surfaces repeat it where consumers actually wire tags to fields.
- **Note:** No finding is raised against the *decision* to reject sanitization (it is sound: a kernel
  that silently mutated untrusted values would be both a correctness hazard and a C-03/C-09 boundary
  violation, and offline universal sanitization of prompt-injection does not exist). The finding is
  solely about ensuring the honest boundary cannot later be misread as a vuln/gap or, worse, relied on
  as a control.

### SEC-003 — No render/analysis resource bounds (`fuel`/recursion limits) — out of scope for this threat model (INFORMATIONAL)

- **OWASP 2025:** A06:2021-adjacent (availability) — informational; not a classic injection/exposure.
- **CWE:** CWE-400 (Uncontrolled Resource Consumption) — assessed and judged not applicable to the
  current threat model.
- **Severity:** INFORMATIONAL.
- **Evidence:** MiniJinja exposes a `fuel` feature to bound rendering work; it is **not** in the
  chosen feature set (`["builtins", "deserialization", "serde", "std_collections"]`, D1). A
  pathological template (e.g. a loop over a very large bound list, or deep nesting) could in principle
  consume unbounded CPU/memory. **For this kernel's threat model this is correctly out of scope:**
  templates are repo-canonical and PR-gated (a template authoring a billion-iteration loop is a
  code-review defect, not an external attack vector), `macros`/`multi_template` are disabled (removing
  recursion-via-macro and include-expansion blowup), and the bound *values* — the only
  caller-supplied input — are sized by the consuming application, which owns its own request limits
  and runs the kernel synchronously in-process. The kernel is not a multi-tenant service taking
  attacker-controlled templates; adding `fuel` here would harden against a vector the architecture
  does not expose.
- **Remediation:** None required for v1. **Re-evaluate `fuel`/iteration limits only if** a future
  spec lets templates or loop-bound collection sizes become externally/attacker-controlled (e.g. a
  hosted authoring backend — explicitly on the roadmap "Never" list — or untrusted template upload).
  Record this as the trigger condition so the decision is revisitable rather than silently permanent.
  This is a calibrated judgment, not a reflexive "add limits."

### SEC-004 — Render-error detail strings may transitively echo bound-value content (LOW)

- **OWASP 2025:** A09:2021 Security Logging and Monitoring Failures (info leakage via error detail) /
  A04 Insecure Design.
- **CWE:** CWE-209 (Generation of Error Message Containing Sensitive Information).
- **Severity:** LOW.
- **Evidence:** `KernelError` variants `Parse { detail }`, `Render { detail }`, and `ExcludedFeature
  { detail }` carry free-form strings sourced from MiniJinja errors (data-model §KernelError). The
  `UndefinedVariable { name }` variant names a missing variable, but that name is a *template-authored*
  identifier (repo-canonical, low sensitivity). The genuine concern is `Render { detail }` for a
  render-time failure inside an allowed feature (e.g. a type error iterating a value): MiniJinja's
  error message for such a failure can include a representation of the offending *value*, which may be
  caller-supplied and could carry sensitive content (a secret, PII). The kernel returning rich detail
  to its caller is correct and useful; the leakage risk materializes only if the consumer logs or
  surfaces the raw `KernelError` detail to an untrusted audience. The spec already routes
  normalization to the consumer (C-06, spec 003), so the kernel is not the wrong place to *hold* the
  detail — it is the wrong place to assume the detail is safe to surface.
- **Remediation:** No kernel change required. Add a note to the error contract (kernel-api.md error
  section, or T033 docs) that `Render`/`Parse` detail may embed bound-value content and is therefore
  **not** safe to log verbatim in untrusted contexts; the consumer's error-normalization layer
  (C-06, spec 003) should scrub/elide value fragments from the `message` it emits and confine raw
  detail to trusted debug logging. Track as a requirement for the spec-003 normalization design.

## Confirmed secure-by-design patterns

These are not findings — they are properties I verified hold in the plan and that materially reduce
the kernel's real risk. Recorded so a later reviewer does not re-litigate them.

- **Values are data, never re-parsed as template syntax (anti-SSTI core).** The render API binds
  values as a `minijinja::Value` context (D5); there is no path in the design where a bound value's
  string content is fed back through `add_template`/parse. A value containing `{{ ... }}` is rendered
  as literal text, not interpreted. This is the single most important property for a template kernel
  and the design preserves it. (CWE-1336 — assessed, not present.)
- **Excluded-feature disablement is real, parse-time hardening.** `default-features = false` makes
  `macros`/`multi_template` off, so `include`/`import`/`extends`/`macro`/`block` are *unrecognized
  tags → parse errors at `add_template` time* (D1/D4), not merely render-time failures. Beyond its
  stated soundness purpose (FR-002 / agreement-check), this removes the constructs most associated
  with Jinja-family SSTI and sandbox escape. Verified against the 2.21.0 default-feature set in
  research D1.
- **Strict-undefined handling (FR-001a) is a defense-in-depth backstop.** `UndefinedBehavior::Strict`
  (D3) turns a missing variable into a loud error rather than a silent empty substitution, closing the
  "silent empty render" failure mode that is both a correctness and a subtle security concern (a
  silently-dropped guard clause).
- **Hashing is content-addressing, not a security token.** `template_hash`/`render_hash` are
  lowercase-hex SHA256 over the source/output strings (D8), used for trace identity. The design never
  treats them as secrets, auth tokens, or adversary-resistant integrity proofs, so collision/second-
  preimage resistance is irrelevant to the use; SHA256 is more than adequate for content addressing.
  No `vars_hash` is computed (FR-014), so no structured-input canonicalization attack surface exists.
- **FFI isolation and no-I/O boundary are CI-enforced, not just asserted.** The inherited spec-001
  `ci:check-ffi` gate (`cargo tree -i pyo3 / -i napi`) and the absence of any I/O/network/LLM API in
  the kernel surface (FR-005, contract cross-cutting invariants) mean the minimal-boundary guarantee
  is structurally verifiable, not aspirational. New deps `minijinja`/`sha2` were confirmed pure-Rust
  and FFI-clean (D7).
- **Analysis and guard expansion are pure / non-mutating (FR-018, FR-023, SC-006).** No side-channel
  via mutation of caller inputs; the agreement analysis cannot alter what is later rendered, removing a
  class of analysis/render-disagreement bugs.
- **Determinism (FR-003, SC-001).** No time/random/global state in the render or hash path; output and
  hashes are reproducible, which is a prerequisite for provenance hashes to be trustworthy identifiers.

## Proposed Memory Hub INDEX.md routing row

Add under the **Security** (or **Decisions**) section of `docs/memory/INDEX.md` (create the Security
heading if absent):

```markdown
## Security

- [Spec 002 plan-stage security review](../../specs/002-engine-kernel/security-review-report.md) —
  PLAN review of the engine kernel. Overall risk LOW. Key durable decisions: values are data
  (never re-parsed → no SSTI), macros/multi_template disabled at parse time, SHA256 is content-
  addressing not a token, provenance tags/guard are advisory metadata only (NON-enforcing — do not
  rely on as a sanitizer; SEC-002). Open follow-ups: add cargo-audit/deny advisory gate (SEC-001);
  consumer error-normalization must scrub bound-value content from error detail (SEC-004, spec 003);
  revisit `fuel`/limits only if templates/value sizes ever become externally controlled (SEC-003).
```
