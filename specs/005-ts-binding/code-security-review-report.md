# Code Review + Security Review — Spec 005 (TypeScript binding `prompting-press-node`)

**Date**: 2026-06-28 · **Steps**: 12 (code-review) + 13 (security-review), run in parallel.
**Scope**: `git diff main..HEAD` — the napi crate + TS facade + tests + CI + the Python C-11 conformance change (made this cycle).

> **Provenance**: 2 subagents (code-reviewer, security-auditor). Both passed integrity checks (correct
> line counts, real file:line citations); findings re-verified main-thread before acting. (Note: the
> security agent flagged a *display-layer* `rg` rendering glitch that stripped substrings like `node`→`n`
> in its terminal echo — it verified the real on-disk names via `sed`/`cat`; conclusions unaffected.)

## Verdict

**Code-review: APPROVE for merge.** **Security-review: no CRITICAL/HIGH.** No merge blockers. The C-11
options-object/keyword-only refactor is verified conformant across TS + Python (+ Rust correctly stays
positional, below its threshold); SEC-004 holds in the final code, tested on both leak paths.

## Findings (triaged + dispositioned)

### Fixed this cycle
| ID | Sev | Source | Finding | Disposition |
|----|-----|--------|---------|-------------|
| M1 | MEDIUM | security | `decodeAddonError` validated the payload envelope (`code` string, `errors` array) but not each row's `{field,code,message}` shape (`index.ts:185-194`) — rows flow out on `.errors` + into the summary. Not exploitable today (the Rust producer is the sole, exhaustively-typed source), but the function's contract is "nothing non-conforming escapes". | **FIXED**: added a per-row `typeof` guard before the cast; a malformed-rows payload now falls through to the base-class defensive wrapper. |
| M2 | MEDIUM | security | `check-advisories-node.sh` comments said `pnpm audit --prod` but the command has no `--prod` — actual behavior is *safer* (scans the full tree incl. dev toolchain), but the comment could mislead a maintainer into adding `--prod` and silently narrowing coverage. | **FIXED**: corrected both comments to state the deliberate full-tree scan; command unchanged. |
| CR-S1 | SUGGESTION | code | Stale "spec-001 skeleton / no runtime code" docstrings now that real code ships (`package.json` description, crate `README.md`). | **FIXED**: package.json description + crate README rewritten to the real (post-005) state. (tsconfig header comment left — purely internal.) |

### Accepted / deferred (not blocking)
| ID | Sev | Source | Finding | Disposition |
|----|-----|--------|---------|-------------|
| CR-S2 | SUGGESTION | code | `isSchema` duck-typing (`safeParse` sniff) survives in `render`'s schema-vs-static dispatch (`index.ts:331`). Latent, unreachable in tests; a static-form data object exposing a `safeParse` method would be misrouted. | **ACCEPTED**: the Composition refactor dissolved it there; `render` keeps schema+data positional (both required, below C-11's optional-tail rule). Contrived collision; documented risk. |
| CR-S3 | SUGGESTION | code | `render` static-form `opts` is cast not validated (`index.ts:436`). | **ACCEPTED**: TS overloads guard the typed path; bites only untyped JS callers. |
| L1 | LOW | security | `--audit-level high` lets `moderate` through (deliberate, matches Rust/Python gates). | **ACCEPTED** (documented posture). |
| L2 | LOW | security | `pnpm audit` needs outbound HTTPS; air-gap fallback documented. | **ACCEPTED** (CI-reliability note; same as cargo-deny/pip-audit). |

## Confirmed controls (security — positive evidence)
- **SEC-004 scrub HOLDS** end-to-end: Rust routes `KernelError` through the consumer's tested scrubber
  first (`error.rs:157-160`), never reads raw `.detail`; the TS facade surfaces only scrubbed rows; the
  `ZodError`→rows mapper copies `issue.message`+`path` only, never `issue.input` (structurally
  unreachable via `ZodLikeIssue`). Tested at both layers + both leak paths (`render.test.mjs:184-250`,
  `error.rs:190-259`).
- **FFI boundary**: no `unsafe`, no panic-across-napi on the production path, name-absent → structured
  error not panic. Marshal recursion is napi-owned (upstream of this crate); napi pinned exact mitigates
  any future depth CVE.
- **Supply chain**: napi 3.9.4 / napi-derive 3.5.7 + all npm deps pinned exact; `--frozen-lockfile` in
  CI; the new advisory gate is wired (script + moon task + CI step). No floating versions.
- **Boundary (Principle III/F4)**: no I/O, network, eval, dynamic require, env, subprocess, or token
  surface in either binding layer.

## Compliance (code-review — verified)
- **C-11 / Principle VI (v1.1.0)**: TS `render`/`getSource`/`Composition` are options-object form;
  Python `render`/`get_source`/`Composition.append`/`GuardConfig` are keyword-only (`*,`); Rust keeps a
  single `Option<&str>` positional (correct, below the Rust threshold). No public fn retains a
  positional-optional tail.
- **C-01/C-02** (zero engine logic, FFI isolation), **C-06** (error normalization, no native leak),
  **C-07** (codegen'd TS shape) — all verified at call sites.

## Recommended disposition
M1, M2, CR-S1 fixed. Proceed to cleanup (14) → sync (15/16) → retro (17). The deferred items
(CR-S2/S3, L1/L2) + the cross-cutting audit follow-ups (py.typed/.pyi, etc.) carry to roadmap-debrief.
