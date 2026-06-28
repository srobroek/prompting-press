# Memory Synthesis

> Markdown-first retrieval: the `speckit_memory` SQLite optimizer / MCP tools are not registered this
> session (same as 003/004), so synthesis is drawn directly from the governance layer (constitution
> v1.0.0 Principles I–VII + roadmap C-01…C-10), the as-built spec-002 kernel + spec-003 consumer + the
> merged spec-004 Python binding (all on `main`), and the verified `crates/prompting-press-node` +
> `packages/typescript` scaffolds. Documented fallback (`.specify/memory/workflow.md`). Phase:
> **Clarify → Plan**.

## Current Scope

Spec 005 — the **TypeScript binding** `prompting-press-node` (→ `packages/typescript`): the **second FFI
binding**, a napi-rs native addon packaged for npm (per-platform `optionalDependencies`). It mirrors the
merged spec-004 Python binding in TS idiom — a **Zod v4** typed-Vars facade (validation owned at render,
`.refine()` validators), a dual-input loader (reused from the Rust consumer via FFI), `check(registry)`
agreement + provenance lint, `render`/`getSource`, `fromMessages` composition, and errors normalized to
`[{field, code, message}]` thrown as a `PromptingPressError` `Error`-subclass hierarchy. **Marshaling +
Zod facade only — zero engine logic.** Affected: `crates/prompting-press-node/` (build out the binding),
`packages/typescript/` (npm package + generated shape + tests + native build). Kernel + Rust consumer
are **not** modified. This is the binding that makes the FFI boundary real for spec 006.

## Relevant Decisions

- **C-02 FFI Isolation** (active, constitution Principle II; CI-enforced `ci:check-ffi`): `napi` ONLY in
  `prompting-press-node`; the kernel + Rust consumer MUST stay FFI-free. THE governing 005 decision — and
  the gate must be **extended to assert `napi`** (it currently checks `pyo3`).
- **C-06 Per-language idiom** (active, Principle VI): typed Vars via **Zod v4**; errors normalized to
  `[{field, code, message}]` then thrown as JS `Error` subclasses; native types (`ZodError`, Rust errors)
  never cross FFI. Composition = explicit `fromMessages` array, NEVER `.chain()` (also can't cross napi).
- **C-07 JSON Schema SSoT** (active, Principle VII): the TS `PromptDefinition` shape is **code-generated**
  from the JSON Schema (`json-schema-to-typescript` → `src/generated/prompt-definition.ts`), never
  hand-maintained; dual-input (YAML/JSON/object) into one representation. Codegen freshness is a build gate.
- **C-01 Shared core / structural parity** (active, Principle I): render/agreement/variant/hash live ONCE
  in Rust. 005 MARSHALS to them — render byte-parity (incl. provenance hashes matching the Python binding
  + Rust consumer) is structural, NOT re-tested in TS.
- **C-04 / C-09** (active, surfaced via `check`): agreement (`referenced ⊆ declared`, declared authority =
  the definition's `variables` block) + provenance lint (untrusted/external field with no `meta.guard`).
  Kernel returns the sets; the binding reuses the consumer's comparison.

## Active Architecture Constraints

- **Dependency direction** (C-01/C-02): kernel ← Rust consumer ← TS binding. The `prompting-press-node`
  crate depends on both Rust crates (path deps already declared); nothing here inverts that or adds engine
  logic.
- **No logic duplication** (C-01): `render`/`getSource`/`check`/compose are FFI calls into the Rust core.
  The binding adds ONLY the Zod facade, the napi marshaling bridge, error normalization, and packaging.
- **Codegen, not hand-authoring** (C-07): `packages/typescript/src/generated/` is generated;
  `schemas:codegen-check` fails the build on drift. Never hand-edit it.
- **Minimal boundary** (C-03, Principle III): no I/O, no model calls, no request-body assembly, no output
  parsing, **no token counting**. `outputModel` is metadata only.
- **Marshaling fidelity is first-class** (enables spec 006): `null`/`undefined`/absent rule fixed
  (clarified Q6: undefined/absent → field-not-present; null → JSON null), matched to the Python binding's
  None/absent handling; `number`/`bigint`, nested objects, dates marshal losslessly.

## Accepted Deviations

- None. (005 introduces no boundary-expanding capability and no new pluggable interface.)

## Relevant Security Constraints

- **SEC-004 scrub (carryover from 002/003/004)**: `KernelError::Parse`/`Render`/`ExcludedFeature` detail
  may carry bound-value content (untrusted input / PII / secrets). The binding's error normalization MUST
  emit a fixed message for those codes and never copy raw detail into a thrown `Error` message, `.stack`,
  or a derived log. The Rust consumer already scrubs in `From<KernelError>`; the binding must route
  through it and not re-introduce the leak when surfacing to JS. The `ZodError` mapper MUST copy only the
  issue `message` + `path`, never the rejected input value (the 004 M-1 lesson — the degenerate fallback
  must withhold detail, never surface the raw native error).
- **Provenance is declarative + lint-only** (C-09): tags are metadata; `check` flags
  untrusted/external-without-guard. Pure analysis — no sanitization, no mutation, no runtime enforcement.
- **Supply chain**: a Node advisory gate (FR-025) mirrors the Rust (`ci:check-advisories`) + Python
  (`ci:check-advisories-py`) gates over the pnpm lockfile.

## Related Historical Lessons

- **Verify-at-spec-time** (corrected MiniJinja in 002, garde in 003, PyO3/maturin in 004): confirm current
  **napi-rs / @napi-rs/cli / Zod v4 / json-schema-to-typescript** versions + APIs against crates.io / npm
  **directly** at plan time. Subagent-reported versions have been **fabricated** before — do not trust them.
- **Subagent fabrication is SYSTEMIC and was TOTAL for audit agents in 004** (verify-tasks + both sync
  agents returned tool_uses:0 / fabricated): treat every audit/research subagent verdict as UNTRUSTED;
  bake an **integrity check** into each subagent prompt and re-verify load-bearing findings main-thread
  against `rg`/`cargo`/`git`/npm. `tool_uses:0` or a failed integrity check ⇒ discard + redo main-thread.
- **CI gate for the language package** (004 review I1): a new language package needs its tests wired into
  CI explicitly (the OS-matrix `cargo build` does NOT run pytest/binding tests). Plan a `ci:test-node`
  (or equivalent) gate so the TS-observable guarantees are gated, not rot-prone.
- **napi link path** (parallels the 004 maturin/libpython lesson): the native build + any `cargo test`
  on the node crate must resolve its runtime link deps in CI; verify the napi build path works on the
  Linux runner, not just locally.
- **`KernelError` / `ConsumerError` / `FindingKind` are closed enums**: keep the binding's error + finding
  mapping exhaustive (no wildcard) so a new Rust variant is a compile/translation error, not a silent
  fallthrough. Preserve `check`'s `ReservedVariantName` + `AnalysisError` kinds + deterministic order.
- Process: `rm` blocked (use `git mv`/`git rm`); single-quote `git commit -m` with backticks; `dgit push`;
  `Closes #N` one per line in the PR body; user-facing PR title; cite "roadmap decision C-NN" (never
  "constitution C-NN"). `detect-changed-files.sh` is at the wrong path — review/security-review skills
  fall back to a merge-base diff.

## Conflict Warnings

- **No hard conflicts.** 005 is fully consistent with the constitution + the merged 002/003/004 surface.
- **Token hook (RESOLVED for 005, unlike 004)**: the roadmap 005 `Scope (in)` token-hook line was struck
  during the 004 cycle (T027, per F4); no stale reference remains. No action needed.
- **Soft (plan-time, not a spec conflict)**: the `prompting-press-node` crate declares `napi = "3"` /
  `napi-derive = "3"` — *floating* major-ranges the `ci:check-floating-versions` gate may flag. Plan
  decides pin-vs-allow; same for the npm deps if the gate covers `package.json`.

## Retrieval Notes

- Sources: governance layer (constitution Principles I–VII, roadmap 005 entry + C-01..C-10), as-built
  kernel (`crates/prompting-press-core/src/`) + consumer (`crates/prompting-press/src/`), the merged
  spec-004 binding (`crates/prompting-press-py/`, `packages/python/`) as the mirror, verified scaffolds
  (`crates/prompting-press-node/Cargo.toml` napi 3.x, `packages/typescript/` @napi-rs/cli 3.7.2 +
  json-schema-to-typescript 15.0.4), auto-memory (spec-002, spec-004-python-binding,
  speckit-workflow-gotchas), 004 memory-synthesis as template. Durable decisions/bugs/architecture dirs
  empty (fresh). MCP optimizer unavailable → markdown-first. Within the 900-word budget; ≤5 decisions,
  ≤5 constraints, ≤3 security, ≤2 worklog observed.
