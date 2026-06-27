# Memory Synthesis

> Markdown-first retrieval: the `speckit_memory` SQLite optimizer / MCP tools are not registered this
> session, so synthesis is drawn directly from the governance layer (constitution v1.0.0 Principles
> I–VII + roadmap C-01…C-10), the as-built spec-002 kernel + spec-003 consumer (on `main`), and the
> verified `crates/prompting-press-py` + `packages/python` scaffolds. This is the documented fallback
> (`.specify/memory/workflow.md`), as used for spec 003. Phase: **Clarify → Plan**.

## Current Scope

Spec 004 — the **Python binding** `prompting-press-py` (→ `packages/python`): the **first FFI binding**,
a PyO3 extension packaged as a maturin wheel. It reproduces the spec-003 Rust consumer surface in
Python idiom — a **Pydantic v2** typed-Vars facade (validation owned at render), a dual-input loader
(reused from the Rust consumer via FFI), `check(registry)` agreement + provenance lint, `render`/
`get_source`, `from_messages` composition, and errors normalized to `[{field, code, message}]` raised
as a `PromptingPressError` hierarchy. **Marshaling + Pydantic facade only — zero engine logic.**
Affected: `crates/prompting-press-py/` (build out the binding), `packages/python/` (Python package +
generated shape + tests + wheel build). Kernel + Rust consumer are **not** modified.

## Relevant Decisions

- **C-02 FFI Isolation** (active, constitution Principle II; CI-enforced `ci:check-ffi`): `pyo3` ONLY in
  `prompting-press-py`; the kernel + Rust consumer MUST stay FFI-free. THE governing 004 decision.
- **C-06 Per-language idiom** (active, Principle VI): typed Vars via **Pydantic v2**; errors normalized
  to `[{field, code, message}]` then raised as Python exceptions; native types never cross FFI.
  Composition = explicit `from_messages` array, NEVER `.chain()`.
- **C-07 JSON Schema SSoT** (active, Principle VII): the Python `PromptDefinition` shape is
  **code-generated** from the JSON Schema (`datamodel-code-generator`), never hand-maintained;
  dual-input (YAML/JSON/object) into one representation. Codegen freshness is a build gate.
- **C-01 Shared core / structural parity** (active, Principle I): render/agreement/variant/hash live
  ONCE in Rust. 004 MARSHALS to them — render byte-parity is structural, NOT re-tested in Python.
- **C-04 / C-09** (active, surfaced via `check`): the agreement check (`referenced ⊆ declared`, declared
  authority = the definition's `variables` block) + the provenance lint (untrusted/external field with
  no `meta.guard` configured). Kernel returns the sets; the binding reuses the consumer's comparison.

## Active Architecture Constraints

- **Dependency direction** (C-01/C-02): kernel ← Rust consumer ← Python binding. The binding depends on
  both Rust crates (path deps already declared); nothing here may invert that or add engine logic.
- **No logic duplication** (C-01): `render`/`get_source`/`check`/compose are FFI calls into the Rust
  core. The binding adds ONLY the Pydantic facade, the marshaling bridge, exception normalization, and
  packaging — exactly what the core deliberately omits for Python.
- **Codegen, not hand-authoring** (C-07): `packages/python/python/prompting_press/generated/` is
  generated; `schemas:codegen-check` fails the build on drift. Never hand-edit it.
- **Minimal boundary** (C-03, Principle III): no I/O, no model calls, no request-body assembly, no
  output parsing, **no token counting**. `output_model` is metadata only.

## Accepted Deviations

- None. (004 introduces no boundary-expanding capability and no new pluggable interface.)

## Relevant Security Constraints

- **SEC-004 scrub (carryover from 002/003)**: `KernelError::Parse`/`Render`/`ExcludedFeature` detail may
  carry bound-value content (untrusted input / PII / secrets). The binding's exception normalization
  MUST emit a fixed message for those codes and never copy raw detail into an exception message or a log
  derived from it. The Rust consumer already scrubs in `From<KernelError>`; the binding must not
  re-introduce the leak when surfacing to Python.
- **Provenance is declarative + lint-only** (C-09): the provenance tags are metadata; `check` flags
  untrusted/external-without-guard. Pure analysis — no sanitization, no mutation, no runtime enforcement.

## Related Historical Lessons

- **Verify-at-spec-time** (corrected MiniJinja in 002, garde in 003): confirm current **PyO3 / maturin /
  Pydantic v2 / datamodel-code-generator** versions + APIs against crates.io / PyPI **directly** at plan
  time. Subagent-reported versions have been **fabricated** before — do not trust them.
- **Subagent fabrication is SYSTEMIC** (escalated in 003): treat every audit/research subagent verdict
  as UNTRUSTED; re-verify each load-bearing finding against `rg`/`cargo`/`git`/PyPI before accepting,
  incl. that cited files/APIs exist.
- **`KernelError` / `ConsumerError` are closed enums**: keep the binding's exception mapping exhaustive
  so a new Rust variant is a compile/translation error, not a silent fallthrough.
- **Reserved `default` + analysis-error finding kinds** (003 CR-1 / F7): preserve `check`'s
  `ReservedVariantName` + `AnalysisError` semantics and deterministic (BTreeMap/BTreeSet) order.
- Process: `rm` blocked (use `git mv`/`git rm`); single-quote `git commit -m` with backticks;
  `dgit push`; `Closes #N` one per line; cite "roadmap decision C-NN" (never "constitution C-NN").

## Conflict Warnings

- **Soft (roadmap drift — must reconcile, not silently carry)**: the roadmap 004 (and 005) `Scope (in)`
  still lists a **"token hook"**. This was DROPPED in spec 003 (refinement F4) and deferred to the
  Deferred "Token budgeting / truncation" entry. Spec 004 drops it (consistent with F4); propose
  amending the roadmap 004/005 lines at plan / roadmap-sync time. No hard conflict.
- **Resolved scaffold inconsistency**: crate `abi3-py39` vs `requires-python >=3.10` vs codegen 3.10
  target → reconciled to **abi3-py310 / floor 3.10** (clarified Q4). Bump the crate feature at plan time.
- **No hard conflicts.** 004 is fully consistent with the constitution + the merged 002/003 surface.

## Retrieval Notes

- Sources: governance layer (constitution Principles I–VII, roadmap 004 entry + C-01..C-10), as-built
  kernel (`crates/prompting-press-core/src/`) + consumer (`crates/prompting-press/src/{lib,error,render,
  check,compose}.rs`), verified scaffolds (`crates/prompting-press-py/`, `packages/python/`),
  auto-memory (spec-002-engine-kernel, speckit-workflow-gotchas), 003 memory-synthesis. Durable
  decisions/bugs/architecture dirs empty (fresh). MCP optimizer unavailable → markdown-first. Within the
  900-word budget; ≤5 decisions, ≤5 constraints, ≤3 security, ≤2 worklog observed.
