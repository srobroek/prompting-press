---
document_type: security-review
review_type: plan
assessment_date: 2026-06-28
codebase_analyzed: prompting-press / 006-conformance-corpus
total_files_analyzed: 7
total_findings: 5
overall_risk: LOW
critical_count: 0
high_count: 0
medium_count: 0
low_count: 2
informational_count: 3
owasp_categories: [A05, A06, A08]
cwe_ids: [CWE-22, CWE-209, CWE-1104]
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

# Security Review (Plan) — 006 Conformance corpus + cross-language hardening

**Method note**: performed on the main thread against the in-context artifacts and source read this
session, not via a subagent (the project's systemic subagent-fabrication guard). flash-mem /
memory-hub MCP tools are not available in this session; used the markdown-only flow over the governance
layer + as-built spec memories.

## Executive Summary

**Overall risk: LOW.** This feature adds **no production attack surface**: it introduces test fixtures,
three test runners, and a CI gate — no I/O in the shipped library, no network, no authn/authz, no new
public API, no secrets handling, no data persistence (constitution Principle III, reaffirmed by the
plan's Constraints). The shipped library boundary is unchanged; only test harnesses and CI wiring are
added.

No Critical/High/Medium findings. The review surfaces **2 Low** and **3 Informational** design-time
hardening notes — all about keeping the *test scaffolding* from (a) weakening the existing SEC-004 scrub
guarantee, (b) leaking a test-only validation bypass into the shipped consumer, or (c) introducing an
unscanned dependency or an over-privileged CI job. None blocks implementation; each is a one-line guard to
honor during the implement phase.

## Plan Artifacts Reviewed

- `specs/006-conformance-corpus/plan.md` (architecture, constraints, structure, complexity tracking)
- `specs/006-conformance-corpus/spec.md` (FRs incl. FR-014 failure-output scrub posture, FR-017/018 boundary)
- `specs/006-conformance-corpus/research.md` (D1–D6; D4 the Rust `RawVars` no-op-validate newtype)
- `specs/006-conformance-corpus/data-model.md` (fixture schema; the manifest `path` field)
- `specs/006-conformance-corpus/contracts/corpus-format.md` (runner obligations, golden provenance, gate)
- `specs/006-conformance-corpus/quickstart.md` (local run + golden regen)
- Governance: `.specify/memory/constitution.md` (Principle III boundary; SEC-004 lineage via specs 004/005)

## Vulnerability Findings

### SEC-001 (Low) — Runner fixture-path resolution should be repo-confined
- **OWASP**: A05:2025-Security Misconfiguration · **CWE-22: Improper Limitation of a Pathname** · **CVSS ~2.0**
- **Where**: `data-model.md` schema-fixture `fixtures[].path`; the three runners (T011–T013) that read
  documents at those paths.
- **Observation**: the schema manifest carries a repo-relative `path` per fixture that each runner opens.
  The values are repo-committed (not user/network input), so this is not an exploitable traversal — but a
  runner that naively joins/opens an arbitrary `path` (e.g. an absolute path or `../` escaping the repo)
  would read outside the corpus. This is test-harness hygiene, not a production vuln.
- **Recommendation**: runners SHOULD resolve manifest paths **within the repo/corpus root** and reject
  absolute or parent-escaping paths. Add a one-line note to the corpus contract (§3). `[spec_kit_task: TASK-SEC-001]`

### SEC-002 (Low) — Runners MUST NOT assert against scrubbed kernel error *detail*
- **OWASP**: A09... (logging/error handling) / mapped here to **A05** · **CWE-209: Information Exposure
  Through an Error Message** · **CVSS ~2.5**
- **Where**: schema-reject assertions (T011–T013, FR-010); marshaling-error paths.
- **Observation**: the bindings already scrub `parse`/`render`/`excluded_feature` detail (SEC-004, specs
  004/005). A reject-path runner that asserts on the *content* of an error message risks two harms: (a)
  coupling the test to a fixed scrubbed message such that a future, more-thorough scrub "breaks" a test
  and tempts a reviewer to assert against *unscrubbed* detail; (b) a fixture/golden that captures raw
  bound-value content. FR-014 already forbids the corpus leaking bound values beyond fixture content; this
  finding extends that to the *assertion* surface.
- **Recommendation**: reject-path runners SHOULD assert on the **error type / normalized `code`** (the
  structured contract), NOT on free-text detail; never assert that scrubbed detail is *present*. Note in
  the corpus contract §3. (Aligns with critique E3, which proposed asserting `code`-agreement — same
  direction.) `[spec_kit_task: TASK-SEC-002]`

### SEC-003 (Informational) — The test-only no-op `Validate` must never reach the shipped consumer
- **OWASP**: A04...(insecure design, mapped INFORMATIONAL) · **CWE-1104: Use of Unmaintained/Improper
  Component** (analogous: a validation bypass) · **CVSS 0.0 (informational)**
- **Where**: `research.md` D4 / `plan.md` complexity-tracking — the `RawVars(serde_json::Value)` newtype
  with a no-op `garde::Validate`.
- **Observation**: the no-op validate is correct for the corpus (it tests marshaling, not validation), but
  validation IS a real guarantee of the shipped consumer's `render<V>`. If this newtype (or its no-op
  impl) ever leaked out of the test target, it would silently disable validation for a real caller. The
  plan already constrains it to the test file; this finding makes that boundary explicitly
  security-load-bearing.
- **Recommendation**: keep `RawVars` in `crates/prompting-press/tests/` only (not in `src/`, not `pub`);
  the existing review gates + a one-line comment on the impl suffice. No new control needed. `[spec_kit_task: none]`

### SEC-004 (Informational) — Any added test-harness dependency must be advisory-scanned or test-only-justified
- **OWASP**: A06:2025-Vulnerable and Outdated Components · **CWE-1104** · **CVSS 0.0 (informational)**
- **Where**: `plan.md` Constraints / Dependencies ("preference is zero new deps").
- **Observation**: the plan correctly forbids new *shipped-library* deps (no JS decimal lib, no second
  YAML parser) and pins any helper exact. The repo's advisory gates (`ci:check-advisories{,-py,-node}`)
  cover the shipped manifests. A new dev/test dependency could fall outside that coverage.
- **Recommendation**: prefer **zero** new deps (the plan already does). If one is unavoidable, pin it
  exact (the floating-version gate already enforces this repo-wide) and confirm it is either covered by an
  advisory gate or justified as test-only with no runtime reachability. `[spec_kit_task: none]`

### SEC-005 (Informational) — The conformance CI job must inherit least-privilege permissions
- **OWASP**: A05:2025-Security Misconfiguration · **CWE-732 (analogous: permissions)** · **CVSS 0.0 (informational)**
- **Where**: `.github/workflows/ci.yml` (the existing workflow sets `permissions: contents: read`); tasks
  T017 adds the conformance job/step.
- **Observation**: the existing workflow is least-privilege (`contents: read`, no packages/write). A new
  job that needs no extra scope must not introduce a broader `permissions:` block.
- **Recommendation**: the conformance job MUST keep `permissions: contents: read` (it only checks out,
  builds, and runs tests — no publish, no token writes), consistent with the existing jobs. `[spec_kit_task: none]`

## Confirmed Secure Patterns

- **Boundary held (Principle III)**: no I/O / LLM / request-assembly / token-count / output-parse / new
  public API added to the library (FR-017/018) — the corpus is tests + a gate. The runners reading fixture
  files are test harnesses, explicitly distinguished from the library.
- **SEC-004 scrub preserved (FR-014)**: failure output names binding+fixture+divergence-kind and must not
  add raw bound-value content beyond the (non-secret-by-construction) fixture — the existing scrub posture
  carries forward rather than being weakened.
- **No secrets in fixtures (FR-014, by construction)**: corpus fixtures contain only test data; no
  credentials, tokens, or PII.
- **Supply-chain discipline**: no new shipped dependency; existing advisory gates remain authoritative; any
  helper pinned exact (no floating versions).
- **Determinism/stability (FR-004)**: expected hashes are taken over canonical strings, OS/arch-stable — no
  environment-dependent expectation that could mask a real divergence.

## Action Plan & Next Steps

1. **Durable memory preservation**: no systemic vulnerability or new auth/boundary decision was identified
   — nothing rises to the bar for a durable security memory (the SEC-004 scrub lineage is already recorded
   in the as-built spec memories). No capture warranted.
2. **Remediation planning**: no Critical/High findings → `/speckit.security-review.followup` not required.
   SEC-001 and SEC-002 are one-line guards best folded into the corpus contract §3 during implementation
   (or now); SEC-003/004/005 are affirmations of existing constraints.

## Memory Hub INDEX.md Row

```text
| specs/006-conformance-corpus/security-review-report.md | plan | 2026-06-28 | LOW | C:0 H:0 M:0 L:2 | A05,A06,A08 |
```
