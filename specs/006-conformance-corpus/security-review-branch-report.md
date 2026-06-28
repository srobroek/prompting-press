---
document_type: security-review
review_type: branch
assessment_date: 2026-06-28
codebase_analyzed: prompting-press / 006-conformance-corpus (diff 8919130 vs origin/main)
total_files_analyzed: 22
total_findings: 0
overall_risk: LOW
critical_count: 0
high_count: 0
medium_count: 0
low_count: 0
informational_count: 2
owasp_categories: [A05, A06]
cwe_ids: [CWE-22, CWE-209]
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

# SECURITY REVIEW REPORT — BRANCH: 006-conformance-corpus (8919130) vs origin/main

**Method note**: post-impl branch review on the main thread (the review-subagent path has been hitting the
`tool_uses:0` channel glitch this session). flash-mem/memory-hub MCP tools are unavailable; markdown-only
flow — governance layer + feature memory already read. Scope = the 22 code/config files in the diff
(`specs/` docs excluded). The plan-time security review (`security-review-report.md`) predicted LOW with
SEC-001/SEC-002 as the carry-forward guards; this confirms they were implemented.

## Executive Summary

**Overall risk: LOW.** This branch adds a test corpus, three test runners, and a CI gate — **no production
attack surface**. The diff introduces no I/O in the shipped library, no network calls, no authn/authz, no
secrets, no new runtime dependencies, no new public API. The two SEC findings raised at plan time
(SEC-001 path confinement, SEC-002 assert-on-error-type) were **implemented** in the runners and are
confirmed present. No new findings; 0 Critical/High/Medium/Low.

## Branch Diff Reviewed

- Target: `006-conformance-corpus` @ `8919130` · Base: `origin/main`
- 22 code/config files: `conformance/` corpus (fixtures + README + moon.yml), `crates/prompting-press/tests/`
  (support module + 3 test files), `packages/python/tests/` + `packages/typescript/test/` runners,
  `scripts/ci/conformance.sh`, `ci/moon.yml`, `.github/workflows/ci.yml`, `.moon/workspace.yml`.

## Vulnerability Findings

**None at any actionable severity.** Domain-by-domain over the diff:

- **Injection (SQL/command/template):** none. `conformance.sh` uses no `eval`/`curl`/`sudo`; the only
  command substitutions are the standard `$(cd … && pwd)` / `dirname` idiom and a `mktemp -d` venv. No
  user/network input reaches a shell.
- **Hardcoded secrets/credentials:** none. A scan of the full diff for password/secret/key/token/private-key
  patterns returns only the SEC-004 scrub *documentation* in comments — no actual secrets. Fixtures contain
  only non-sensitive test data by construction.
- **Access control / authz:** N/A — no auth surface.
- **Cryptographic failures:** N/A — the only crypto is the kernel's existing SHA-256 provenance (unchanged);
  the corpus only *asserts* those hashes, adds none.
- **Input validation:** the runners read only repo-committed fixture files (no network/env/stdin input —
  confirmed: no `requests`/`urllib`/`fetch`/`os.environ`/`process.env`/`std::env` in any runner). The one
  external input shape (the manifest `path`) is confined (see SEC-001 below).
- **Supply chain / new deps:** none. No new shipped-library dependency; `maturin==1.14.1` is exact-pinned
  (matches `test-python.sh`); no JS decimal lib or second YAML parser added. The advisory gates
  (`ci:check-advisories{,-py,-node}`) remain authoritative and unchanged.

## Informational (confirmations of implemented guards — no action)

### SEC-001 (Informational, IMPLEMENTED) — manifest path confinement
- **OWASP** A05:2025 · **CWE-22** · the plan-time finding is implemented: all three schema runners resolve
  the manifest `path` WITHIN the repo root and reject absolute paths / `..` segments before any FS read.
  Python `_safe_resolve` additionally double-checks resolved-path containment under the repo root. Rust
  `resolve_in_repo` and the TS runner mirror it. Defense-in-depth (the manifest is repo-committed). ✓

### SEC-002 (Informational, IMPLEMENTED) — reject asserts on error type, not message
- **OWASP** A05 (mapped) · **CWE-209** · all three schema runners assert a `reject` via the error TYPE
  (`ConsumerError`/`LoadError`/`PromptingPressError`) only, never on free-text detail — so the tests cannot
  couple to or pressure-weaken the SEC-004 scrub. ✓

## Confirmed Secure Patterns

- **Boundary held (Principle III):** no I/O / network / LLM / token surface / new public API in the library;
  runners read fixtures as test harnesses, explicitly distinct from the library.
- **FFI isolation (C-02):** `ci:check-ffi` green; no engine logic in any binding/runner; the `RawVars`
  newtype is test-only.
- **Least-privilege CI (SEC-005, plan-time):** the new `conformance` ci.yml job inherits the workflow's
  top-level `permissions: contents: read` — no `packages:write`, no token escalation. Confirmed.
- **Failure-output hygiene (FR-014):** runner failure messages name binding+case+divergence-kind and do not
  add raw bound-value content beyond the (non-secret) fixture.
- **Deterministic, hermetic test runtime:** `conformance.sh` builds the Python extension into a `mktemp`
  venv with an `EXIT` trap (no repo artifacts leak); exact-pinned maturin.

## Action Plan

**No remediation required** (0 Critical/High/Medium/Low). SEC-001/SEC-002 are implemented and confirmed.
No `/speckit.security-review.followup` needed. No durable security memory warranted (no new
vulnerability/boundary decision — the SEC-004 lineage is already recorded in the as-built spec memories).

## Memory Hub INDEX.md Row

```text
| specs/006-conformance-corpus/security-review-branch-report.md | branch | 2026-06-28 | LOW | C:0 H:0 M:0 L:0 | A05,A06 |
```
