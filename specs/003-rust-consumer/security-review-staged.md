---
document_type: security-review
review_type: staged
assessment_date: 2026-06-26
codebase_analyzed: prompting-press (crates/prompting-press — Rust consumer, spec 003)
total_files_analyzed: 7
total_findings: 0
overall_risk: LOW
critical_count: 0
high_count: 0
medium_count: 0
low_count: 0
informational_count: 2
owasp_categories: []
cwe_ids: []
field_summaries:
  document_type: "Always 'security-review'. Allows indexers to skip non-review documents."
  review_type: "Which command generated this document: audit, branch, staged, plan, tasks, or followup."
  assessment_date: "ISO 8601 date the review was performed (YYYY-MM-DD)."
  overall_risk: "Highest severity tier with active findings (CRITICAL, HIGH, MODERATE, LOW, INFORMATIONAL)."
  critical_count: "Number of Critical findings (CVSS 9.0-10.0)."
  high_count: "Number of High findings (CVSS 7.0-8.9)."
  medium_count: "Number of Medium findings (CVSS 4.0-6.9)."
  low_count: "Number of Low findings (CVSS 0.1-3.9)."
  informational_count: "Number of Informational findings."
  owasp_categories: "OWASP Top 10 2025 categories (A01-A10) that have at least one finding."
  cwe_ids: "CWE identifiers referenced in this document."
  finding_id: "Unique finding identifier (SEC-NNN) for cross-referencing and task linkage."
  location: "File path and line number of the vulnerable code (path/to/file.ext:line)."
  owasp_category: "OWASP Top 10 2025 category for this finding (AXX:2025-Name)."
  cwe: "Common Weakness Enumeration identifier with short name (CWE-NNN: Name)."
  cvss_score: "CVSS v3.1 base score (0.0-10.0). 9.0+=Critical, 7.0-8.9=High, 4.0-6.9=Medium, 0.1-3.9=Low."
  spec_kit_task: "Spec-Kit task ID for backlog tracking and remediation follow-up (TASK-SEC-NNN)."
---

# SECURITY REVIEW REPORT — STAGED CHANGES (post-implementation)

## Executive Summary

Post-implementation security review of the spec-003 Rust consumer diff. **Zero vulnerability findings**;
overall risk **LOW**. This is a pure, I/O-free, FFI-free library layer over the kernel; its attack
surface is data marshaling and error formatting, both of which are handled defensively. The one
security-relevant requirement for this layer — **SEC-004 / FR-015**, that untrusted bound-value content
in a kernel error never resurfaces in a normalized error or log — is implemented and pinned by tests.
The pre-implementation plan/tasks review (`security-review-report.md`, step 5c) raised SEC-001..004 as
design constraints; this review confirms each is satisfied in code.

## Staged Diff Reviewed

Branch `003-rust-consumer` vs `main` (merge-base), source + tests:

- `crates/prompting-press/src/error.rs` — error normalization + SEC-004 scrub
- `crates/prompting-press/src/registry.rs` — dual-input loader (no I/O; deserialize-only)
- `crates/prompting-press/src/render.rs` — validate-then-render
- `crates/prompting-press/src/check.rs` — agreement + provenance lint
- `crates/prompting-press/src/compose.rs` — multi-message composition
- `crates/prompting-press/src/lib.rs` — public surface
- `crates/prompting-press/Cargo.toml` — deps (garde 0.23, serde_yaml_ng 0.10)

## Vulnerability Findings

**None.** No Critical / High / Medium / Low findings.

## Domain-by-domain analysis

- **Injection (SQL/NoSQL/command/template)** — N/A / safe. The crate executes no SQL, no shell, no
  network. Template *rendering* is the kernel's MiniJinja with `UndefinedBehavior::Strict` (spec 002);
  this layer only bridges already-validated values via `Value::from_serialize`. Untrusted input is data,
  never code; the provenance lint (`check.rs`) additionally flags untrusted-input-without-guard prompts
  at CI time.
- **Hardcoded secrets/credentials** — none. The only secret-looking strings are deliberate test fixtures
  (`error.rs` tests: `sk-super-secret-…`, `PASSWORD=hunter2`) that *assert the scrub works* — they are
  inputs proving secrets are NOT leaked, not embedded credentials.
- **Sensitive-data handling / leakage (SEC-004 / FR-015)** — ✅ secure. `From<KernelError>` discards the
  `Parse`/`Render`/`ExcludedFeature` `detail` (which may embed bound values / PII / secrets) and emits a
  fixed templated message + stable code (`error.rs:192-208`). `UnknownVariant`/`UndefinedVariable`
  surface only caller-supplied identifiers, not bound *values*. Verified: `render_detail_secret_is_scrubbed`,
  `parse_detail_secret_is_scrubbed`, `excluded_feature_maps_to_stable_code_without_leaking_detail` all
  assert neither the structured row nor the `Display` string contains the secret.
- **Input validation gaps** — ✅ covered. garde validation runs once *before* any templating
  (`render.rs:92`); composition validates eagerly at `append` and rejects bad entries without storing
  (`compose.rs:156`). Loaders use `#[serde(deny_unknown_fields)]` on the generated shape and never
  partially load on a deserialize error (`registry.rs:90/114`).
- **Cryptographic failures** — N/A here (SHA-256 provenance hashing lives in the kernel, out of this
  diff). No crypto is implemented in the consumer.
- **Broken access control / auth** — N/A (no auth surface; library has no users/sessions).
- **Security misconfiguration / DoS** — low. No unbounded recursion or user-controlled allocation beyond
  the input size the caller already holds. `with_capacity(self.entries.len())` is bounded by the
  caller-built composition.
- **Dependencies / supply chain** — `garde 0.23` (+ derive, serde) and `serde_yaml_ng 0.10` (maintained
  `serde_yaml` successor on pure-Rust `yaml-rust2`) are the only new deps; both are pinned (no floating
  versions — `ci:check-floating-versions`) and pass the `cargo-deny` advisory gate (`ci:check-advisories`).
  The FFI-isolation gate confirms no pyo3/napi/cpython entered the tree (CWE-1104 guard / C-02).

## Confirmed Secure Patterns

- **Fail-closed error scrubbing** — bound-value detail is dropped, not best-effort redacted (SEC-004).
- **No panics on the library path** — every fallible operation returns `Result`; `Entry`-API insert
  removed the last `.expect()` from `src` (ER-1).
- **Exhaustive, wildcard-free `KernelError` match** — a future kernel error variant cannot silently fall
  through to an unscrubbed default; it is a compile error until mapped (`error.rs:178`).
- **No-partial-as-success** — composition discards the partial result on any entry failure
  (`compose.rs:216`).

## Informational notes (no action)

- INFO-1: Test fixtures intentionally contain secret-shaped literals to prove the scrub; a repo-wide
  secret scanner should allowlist `crates/prompting-press/src/error.rs` test module.
- INFO-2: The provenance lint is a CI-time *advisory* (it flags untrusted-without-guard); it is not a
  runtime sanitizer and does not enforce guard expansion (that is the kernel's `GuardConfig`). This is by
  design (C-09) and correctly documented in `check.rs` + `lib.rs`.

## Action Plan

No remediation required. Proceed to cleanup (step 14).

---

## Memory Hub INDEX.md Row

```text
| specs/003-rust-consumer/security-review-staged.md | staged | 2026-06-26 | LOW | C:0 H:0 M:0 L:0 | (none) |
```
