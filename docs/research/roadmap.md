# Roadmap: Prompting Press

> **Governance artifact of record is `.specify/memory/roadmap.md`** (the SpecKit
> `roadmap` extension ledger — specs 001–007, decisions C-01…C-09, Deferred/Never lists,
> versioned). This document is the human-readable phase-by-phase narrative behind that
> ledger; when they diverge, the ledger wins.

**Status**: planning narrative (2026-06-25), derived from `feature-scope.md` and the constitution
(`.specify/memory/constitution.md`). Phased plan, not a spec. v1 scope is firm; later phases are
directional and gated on real demand (R1 — generality is earned, not anticipated).

---

## Phase 0 — Foundations (build the spine before features)

Everything in v1 hangs off these; they ship first and are themselves the riskiest verified-but-
unbuilt pieces.

1. **Repo reorganization to the load-bearing crate layout.** The bootstrap scaffold created a flat
   `packages/{python,typescript,go,rust}`; restructure to the constitution's shape:
   `crates/prompting-press-core`, `crates/prompting-press`, `crates/prompting-press-py`,
   `crates/prompting-press-node`, plus `packages/{python,typescript}` (published wrappers) and a
   reserved `packages/go` placeholder. moon workspace wired to orchestrate cross-crate build/test.
2. **JSON Schema for the prompt-definition shape** (`schemas/jsonschema/`): role, template body,
   variant set, metadata, output-model ref, per-field provenance tags. This is the single source of
   truth (Principle VII) — authored before any codegen.
3. **Codegen pipeline** (build-pipeline dependency, all three packages): schema → Pydantic models,
   TS types, Rust structs. *Verification needed at spec time:* pick and pin the three generators
   (e.g. datamodel-code-generator / json-schema-to-typescript / typify or equivalent) — do not
   assume; verify current tooling. CI fails if shapes are stale vs schema.
4. **CI guardrails for the constitution's structural invariants:** assert `pyo3`/`napi` absent from
   `prompting-press-core` and `prompting-press` (Principle II); assert codegen freshness.

## Phase 1 — Engine kernel (`prompting-press-core`)

The binding-agnostic, validation-blind Rust engine. No FFI, no typed-Vars.

5. **MiniJinja integration**, restricted feature set: interpolation, conditionals, loops.
   **Disabled/rejected:** `{% include %}` / `{% import %}` / `{% extends %}`, macros, inheritance
   (Principle IV soundness requirement).
6. **Render path** over already-validated values → rendered text.
7. **Sound agreement analysis**: `Template::undeclared_variables(nested=false)` minus a globals/
   filters allowlist → the set of required root variable names. Pure analysis, no mutation.
8. **Variant resolution**: named lookup; implicit `default` for single-variant; error on
   no-variant render against a multi-variant prompt (Principle V).
9. **Hashing**: `template_hash = SHA256(variant source)`, `render_hash = SHA256(rendered output)`,
   per resolved variant.
10. **Var-provenance plumbing**: accept 3-way tags (`trusted | untrusted | external`) as data
    across the (future) boundary; implement the **configurable, opt-in, additive guard-expansion**
    render mode (default guard template + override; never mutates the body).
11. **Engine regression render fixtures** (small; the demoted render corpus).

## Phase 2 — Rust consumer crate (`prompting-press`)

The first full consumer layer; proves the kernel + consumer split before any FFI.

12. **Typed-Vars facade** via serde + **garde 0.23** (custom validators, one-shot `.validate()`).
13. **Dual-input loader**: serialized YAML/JSON **or** a constructed prompt-definition object →
    one internal representation.
14. **Agreement check as a consumer-layer operation**: kernel returns required roots; consumer
    compares against the garde/serde Vars struct's declared fields. `check(registry)` CI/lint entry.
15. **Provenance-lint** (untrusted-field-in-guard-position) as a CI/lint pass.
16. **Error normalization**: garde `Report` → common `[{field, code, message}]` shape (never leaked).
17. **`render()` / `get_source()`** ergonomic API + composition (`Vec` + `append_*`, **not**
    `.chain()`).
18. **Token-count hook** interface (`count_tokens(text, model) -> int`); no built-in counter.

## Phase 3 — Python binding (`prompting-press-py` → `packages/python`)

Consumer #1's language (Bellwether/claudebroker). PyO3 + Pydantic.

19. **PyO3 marshaling** over the kernel (maturin wheel). Marshaling + Pydantic facade only —
    zero rendering/hashing/analysis logic (Principle II).
20. **Pydantic Vars facade** + custom validators; agreement check + provenance lint wired to the
    Pydantic model's declared fields.
21. **Dual-input loader, composition (`from_messages([...])`)** — Python-idiomatic. (Token hook
    struck — dropped in spec 003 refinement F4; the whole token surface is deferred, see the Deferred
    "Token budgeting / truncation" entry.)
22. **Error normalization** to the common shape (Python exceptions).

## Phase 4 — TypeScript binding (`prompting-press-node` → `packages/typescript`)

Proves the *second* binding pattern — exercises the FFI seam the conformance corpus targets.

23. **napi-rs marshaling** over the kernel (npm package, platform-binary packaging).
24. **Zod Vars facade** + `.refine()` validators; agreement check + provenance lint wired to Zod.
25. **Dual-input loader, composition (array literal / builder)** — TS-idiomatic. (Token hook struck —
    same F4 reason as the Python binding; the token surface is deferred, not a binding concern.)
26. **Error normalization** to the common shape (JS errors).

## Phase 5 — Conformance corpus + cross-language hardening

The corpus's verified-correct scope (Principle VII): FFI boundary + schema round-trip, **not**
render parity.

27. **FFI-marshaling fixtures**: same logical input — `datetime`/`Date`/`chrono`, Decimal, nested
    models, `null`/`undefined`/`None`, int-vs-float — through each binding → identical render +
    identical `template_hash`/`render_hash`.
28. **Schema round-trip fixtures**: schema-valid and schema-invalid docs accepted/rejected
    identically across all three languages; codegen'd shapes construct correctly.
29. **Wire the corpus as a CI gate** across the three packages.

## Phase 6 — v1 release

30. Docs (the tagline's *both* halves: type-safety AND press/provenance), READMEs per package,
    quickstart. Reserve `prompting-press` on crates.io / PyPI / npm. Apache-2.0 + NOTICE. Publish.
31. Bellwether integration validated end-to-end (prompts in-repo, provenance → its traces, output
    models referenced).

---

## Deferred (post-v1, gated on real demand — R1)

| Item | Trigger to build | Notes |
|------|------------------|-------|
| **Go binding** | A concrete Go consumer + a solved binding path | cgo-over-C-ABI or WASM-via-wazero against the **same** core — never an independent reimplementation (Principle I). `packages/go` placeholder + a conformance target reserved now. |
| **Inline source-partials** (`{{> name }}`) | Static-boilerplate fan-out friction proven painful | Source-splice *before* MiniJinja parses → analysis stays sound on the stable API, no unstable dependency. Additive, non-breaking. |
| **Token budgeting / truncation** | Someone wires a real `count_tokens` hook and needs fit-to-budget | Behavioral, depends on the hook; per-vendor tokenizer parity is the hard part. |
| **`nested=true` strict mode** | Demand for the check to verify deep attribute paths | Optional stricter agreement check; partially duplicates the type system; MiniJinja recovers full paths only for trivial chains. |
| **Langfuse delivery backend** | A team wants push-to-SaaS *as delivery* | Repo stays canonical; SaaS is never source of truth. Out of v1 entirely. |
| **Additional pluggable interfaces** | A *second concrete implementation* actually exists | No speculative seams (Scope Discipline). All five brief interfaces stay eliminated until earned. |

## Explicitly never (boundary defense — constitution Governance)

LLM calls · provider request-body assembly · output parsing/coercion · built-in token counting ·
a managed version axis · I/O / storage adapters · sanitization/stripping of untrusted vars · a SaaS
authoring backend as source of truth. Any of these requires a constitution amendment before work.

## Cross-cutting risks to watch

- **Codegen tooling** (Phase 0.3) — three generators must stay current and agree on the schema;
  verify at spec time, don't assume.
- **PyO3/napi receiver constraints** — no owned-`self` builders across the boundary; keep the kernel
  plain data (already designed for, but a real compile-check is the proof).
- **MiniJinja minor-version drift** — we depend only on the *stable* `undeclared_variables` + render
  path (no `unstable_machinery`), so drift risk is bounded; re-confirm on each bump.
