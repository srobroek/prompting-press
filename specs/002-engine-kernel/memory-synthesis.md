# Memory Synthesis — Spec 002 (Engine kernel)

> Generated markdown-first (SQLite optimizer cache absent; no `speckit_memory` MCP in session).
> Draws on the **governance layer** (constitution v1.0.0 Principles I–VII + roadmap C-01…C-10),
> `docs/research/feature-scope.md` §4.2/§4.3/§4.4/§4.5, the spec-001 worklog, and the generated
> Rust shape `crates/prompting-press/src/generated/prompt_definition.rs`. Phase: **Specify → Plan**.

## Current Scope

Spec 002 — the **engine kernel** (`prompting-press-core`): the binding-agnostic, validation-blind
Rust engine that turns *already-validated values + a prompt definition* into *rendered text +
provenance*. Capabilities: MiniJinja render path (interpolation/conditionals/loops only); the sound
agreement analysis (required-root-vars via `Template::undeclared_variables(nested=false)` − globals
allowlist); variant resolution (caller-owned selection, implicit/explicit default per C-05);
`template_hash`/`render_hash`; var-provenance plumbing + opt-in additive guard expansion; a small
engine-regression render-fixture set. Consumes the generated Rust shape from 001 (the single source
of truth). **No FFI, no validation, no I/O, no LLM/request-body/token/output concerns.**

## Relevant Decisions

- **C-01 Shared core / structural parity** (active, constitution): rendering/agreement/variant/
  hashing happen **once, in Rust, here**. Parity is structural — 002 owns the one place behavior
  lives; later bindings must not reimplement it.
- **C-02 FFI isolation** (active, constitution; CI-enforced by spec 001's `check-ffi`): the kernel
  MUST NOT depend on `pyo3`/`napi`. The existing gate already guards this — 002 must keep it green.
- **C-03 Minimal boundary (non-negotiable)** (active, constitution): no I/O, no LLM calls, no
  request-body assembly, no token counting, no output parsing. The kernel is validation-blind: it
  receives already-validated values.
- **C-04 Sound agreement check** (active, constitution — THE 002 headline): referenced-root-vars via
  MiniJinja stable `undeclared_variables(nested=false)` − globals/filters allowlist; excludes loop
  locals, `{% set %}` targets, block locals; pure analysis, MUST NOT mutate. Soundness holds because
  includes/imports/extends/macros/inheritance are excluded from v1 templates. The kernel exposes the
  **referenced-vars set**; the "referenced ⊆ declared" comparison is the consumer's (003) lint.
- **C-05 Variants / hashing** (active, constitution): caller-owned selection; the root `body` is always
  the default arm (schema makes `body` required), so a no-variant render always resolves to it for
  single- and multi-variant prompts alike — the only variant error is unknown-variant (spec FR-010,
  ratified 2026-06-26; superseded feature-scope §4.3's "missing-default error" which the 001 schema made
  unreachable). Provenance = data on return value; per resolved variant `template_hash = SHA256(variant
  source)` + `render_hash = SHA256(output)`, each over a string. **No `vars_hash`.**
- **C-09 Var provenance = metadata + lint + opt-in guard, never silent mutation** (active): 3-way tag
  `trusted|untrusted|external`; the kernel plumbs the tags (data, not behavior) and supports the
  opt-in, additive, configurable guard expansion. Sanitization/stripping rejected.

## Active Architecture Constraints

- **Behavior lives once, in the kernel** (C-01): 002 is the single implementation site for render,
  agreement analysis, variant resolution, hashing. Bindings (004/005) marshal only.
- **Kernel dependency direction** (C-01/C-02): kernel depends on neither binding nor FFI; the Rust
  consumer (003) and bindings depend on the kernel. The kernel may depend on the **generated Rust
  shape** crate (001) — that is its input contract.
- **Agreement check soundness is preserved by template-feature exclusion** (C-04): v1 templates =
  interpolation/conditionals/loops ONLY. Includes/imports/extends/macros/inheritance MUST be rejected
  (or never parsed as such) so the stable `undeclared_variables` API stays sound — no
  `unstable_machinery`, no include-graph walker.
- **Provenance over strings, per variant, no `vars_hash`** (C-05): the materialize-defaults / JCS /
  RFC-8785 problem was deliberately designed out — do not reintroduce a structured-vars hash.

## Relevant Security Constraints

- The provenance tag is **declarative metadata only** in this version (the generated shape's docstring
  says so explicitly): there is NO runtime enforcement that makes `untrusted`/`external` safe. 002
  delivers the *plumbing* + the *opt-in additive guard expansion* (C-09), not a sanitizer. The guard
  expansion is additive and visible; it MUST NOT mutate the template body. The agreement/provenance
  **lint** is pure analysis (no mutation).

## Accepted Deviations

- None.

## Related Historical Lessons (from spec 001 worklog)

- **MiniJinja pin must be RE-CONFIRMED at plan time** (roadmap Q3): soundness of
  `undeclared_variables(nested=false)` was verified against MiniJinja **2.21** source. 002 must pin a
  concrete MiniJinja version and re-confirm the stable API + the globals/filters allowlist against
  *that* version before relying on it.
- **typify strips `propertyNames`**: the reserved-`default` rule (FR-011b) is NOT encoded in the
  generated Rust type — it was a validation-gate concern in 001. 002's variant resolution must treat
  `"default"` as reserved by its own logic; do not assume the type forbids a variant literally named
  `default`.
- **"Green CI" can mask an unrun step** (moon cache hits): when 002 adds kernel tests/gates, ensure
  task inputs are complete so a cache-hit never hides a gate that never executed.
- **Self-referential-string false positives**: lint/grep gates that scan their own fixtures/docs bit
  001 three times. 002's agreement-check fixtures (templates containing `{{ ... }}`) are a prime
  candidate — design fixture scanning to avoid matching its own corpus.
- **mise rust backend ignores `rust-toolchain.toml` components**: keep `components="rustfmt,clippy"`
  in `mise.toml` + the CI `rustup component add`. Don't regress.

## Conflict Warnings

- **No hard conflicts.** 002 is the spec that makes C-04/C-05/C-09 executable, fully consistent with
  the constitution. Soft watch items: (a) the MiniJinja allowlist must be derived from the *pinned*
  version, not assumed from 2.21; (b) keep the kernel FFI-free (the spec-001 `check-ffi` gate already
  enforces, but 002 adds the first real dependencies — MiniJinja, sha2 — so verify they pull no FFI);
  (c) the agreement check is **pure analysis** — any temptation to "auto-fix" a missing var is a
  C-04/C-09 violation (never mutates).

## Retrieval Notes

- Sources: governance layer (constitution + roadmap C-01..C-10), `docs/research/feature-scope.md`
  §4.1–4.7 + §6, `docs/memory/worklog/001-us1-followups.md`, generated Rust shape. Durable
  `decisions/bugs/architecture/` still empty (fresh project). Within budget. MCP unavailable →
  markdown-first. Full-memory read not required.
