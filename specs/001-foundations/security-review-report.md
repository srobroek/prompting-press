---
document_type: security-review
review_type: plan
assessment_date: 2026-06-25
codebase_analyzed: prompting-press spec 001 (Foundations)
total_files_analyzed: 10
total_findings: 7
overall_risk: LOW
critical_count: 0
high_count: 0
medium_count: 2
low_count: 3
informational_count: 2
owasp_categories: ["A05:2021", "A06:2021", "A08:2021"]
cwe_ids: ["CWE-494", "CWE-829", "CWE-1104", "CWE-1357", "CWE-1059"]
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
  owasp_categories: "OWASP Top 10 categories that have at least one finding."
  cwe_ids: "CWE identifiers referenced in this document."
  finding_id: "Unique finding identifier (SEC-NNN) for cross-referencing and task linkage."
---

# Pre-Implementation Security Review — Spec 001 "Foundations"

**Verdict: LOW overall · 0 Critical · 0 High · 2 Medium · 3 Low · 2 Informational.** Calibrated for a
structural-scaffolding spec with no runtime. No finding blocks implementation. The two Mediums are
supply-chain hardening (turn "pin exactly, in prose" into "pin with lockfiles + hashes + least-priv
CI"); the Lows are guardrail-coverage and doc-clarity refinements. No 001 design choice forecloses a
secure path for specs 002+.

> Run by subagent (markdown-only flow; flash-mem/MCP not installed). Full reasoning retained in the
> session; this is the report of record. Findings cross-checked against live repo state.

## Plan artifacts reviewed

spec.md · plan.md · tasks.md · research.md · data-model.md · contracts/prompt-definition.schema.json ·
constitution v1.0.0 · docs/memory/{PROJECT_CONTEXT,INDEX}.md · memory-synthesis.md. Live cross-checks:
`.moon/workspace.yml`, `mise.toml`, `.pre-commit-config.yaml`, `.github/workflows/` (empty), `packages/`.

## Findings

### SEC-001 — Codegen tool pins live in prose, not lockfiles/hashes — MEDIUM
OWASP A08 / A06 · CWE-494, CWE-1357. research.md D1/D2 mandate exact version pins (no `^`) and T022
uses `cargo install --locked`, but the Python/Node tools are pinned only as version *strings* —
version-only pinning still resolves the artifact from PyPI/npm with no integrity (hash) check.
**Fix:** pin at the lock+hash layer — Python `requirements.txt --hash=` + `--require-hashes` (or a
`uv.lock`), Node committed lockfile + `npm ci`/`--frozen-lockfile`, Rust keep `--locked`. Add as an
explicit task; T020/T021 currently say only "pin … 0.65.1 / 15.0.4".

### SEC-002 — CI executes third-party codegen tools; the build pipeline is the one trust boundary 001 creates — MEDIUM
OWASP A08 · CWE-829, CWE-1104. `:codegen` runs three third-party generators in CI and commits their
output (re-exported by the consumer crate, T025). A trojaned tool version could inject code into the
committed shapes; the freshness gate re-runs the same tools so it wouldn't detect a consistently-
malicious generator. **Fix:** (1) hash-pin the toolchain (SEC-001); (2) codegen/freshness CI jobs must
hold **no** secrets/publish tokens; (3) follow-up spec: `cargo-deny`/`pip-audit`/`npm audit` advisory
gates. **Compensating control already present:** generated artifacts are committed + freshness-gated,
so a malicious change surfaces as a reviewable PR diff.

### SEC-003 — "No floating versions" asserted but not mechanically enforced — LOW
OWASP A06 · CWE-1104. Nothing in 001's gates rejects a future PR reintroducing `^`/`~`/`latest`.
**Live evidence:** `mise.toml` already has `jq = "latest"` (verified) — the exact anti-pattern (not in
the codegen path, so low impact, but real). **Fix:** a CI lint rejecting floating ranges in the
codegen toolchain manifests; pin `mise.toml`'s `jq`.

### SEC-004 — Provenance tags declared but unenforced in 001; deferral well-documented but schema-local wording could be stronger — LOW
OWASP A05 · CWE-1059. The schema declares `trusted|untrusted|external` but 001 does not enforce it
(specs 002+ do, per C-09). Well-handled (FR-010a, data-model.md, memory-synthesis, C-09), but an
external consumer reading the schema in isolation sees only "plumbed/enforced in later specs."
**Fix:** strengthen the in-schema `provenance` description to state explicitly it is declarative
metadata with **no runtime enforcement in the current version**, pointing to the C-09 guard. No design
change.

### SEC-005 — moon project globs don't match the planned `crates/*` layout and sweep in `packages/go`; risk of silent gate under-coverage — LOW
OWASP A05 · CWE-1059. **Live finding:** `.moon/workspace.yml` globs `apps/*`…`packages/*`…`tools/*` —
no `crates/*`, and `packages/*` matches `packages/go` (which FR-005/006 require excluded). The FFI and
freshness gates run as moon tasks; a crate/path outside the project set is silently not covered — and
the security-relevant failure mode of a gate is *silent non-coverage*. T013/T028 address re-globbing +
an explicit covered-crate list (good), but the live glob mismatch must not carry over. **Fix:** make
moon project membership explicit (include `crates/*`, exclude `packages/go`); apply the T028
"explicit reviewable list" principle to the moon project set too. T014/T031 manual checks are good
compensating controls.

### SEC-006 — No secrets/tokens/deploy config in 001 (confirmed) — INFORMATIONAL
Publish + registry reservation correctly deferred to spec 007; `gitleaks` + `detect-private-key`
pre-commit hooks already present (pinned). **Keep** codegen/gate CI jobs token-free; introduce publish
credentials only in 007 with their own review.

### SEC-007 — Trust-boundary surface in 001 is effectively empty (by design) — INFORMATIONAL
No runtime, no input parsing, no network/LLM/IO (Principle III, FR-021/022, SC-007). The only
boundaries are the CI pipeline (SEC-001/002/005) and the inert provenance declaration (SEC-004). Carry
the prompt-injection / untrusted-var threat model into specs 002+ where template expansion of
`untrusted`/`external` vars becomes live and the C-09 opt-in guard must be reviewed for soundness.

## Confirmed secure patterns

- Sealed schema by default (`additionalProperties:false` on root/Variant/VariableDecl → `extra=forbid`
  / sealed TS / `deny_unknown_fields`); `metadata`/`meta` intentionally open but library-opaque.
- Generated code committed + freshness-gated + segregated, never hand-edited — malicious changes
  surface as reviewable diffs (compensating control for SEC-002).
- Determinism as a first-class requirement (the property that makes the freshness gate meaningful).
- FFI-isolation gate via `cargo tree` (dependency graph, catches transitive) + explicit crate list.
- Output-model reference is an opaque string, never resolved/parsed (forecloses deserialization risk).
- `cargo install --locked` + pinned Rust toolchain channel; baseline secret scanning present.
- Security-relevant deferrals documented with forward references, not silent.

## Disposition

- **SEC-001, SEC-002, SEC-003, SEC-005** → folded into spec 001 tasks (see tasks.md T020/T021, T026,
  T028, T013, plus new T035 for the floating-version lint). Hardening, non-blocking.
- **SEC-004** → schema `provenance` description strengthened (T015 note).
- **SEC-006, SEC-007** → informational; carried to spec 007 (publish) and specs 002+ (threat model).

No critical/high findings; no remediation-plan (`security-review.followup`) needed. PROCEED.

## Memory Hub INDEX.md row

```text
| specs/001-foundations/security-review-report.md | plan | 2026-06-25 | LOW | C:0 H:0 M:2 L:3 | A05,A06,A08 |
```
