# Implementation Plan: Opt-in unsafe render-error detail

**Branch**: `013-unsafe-render-detail` | **Date**: 2026-06-29 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/013-unsafe-render-detail/spec.md`

## Summary

Add an explicit, **off-by-default, per-render-call** option that surfaces the otherwise-scrubbed
`Render`-error detail in the normalized error message, for callers who control their own log sink and
accept the bound-value/PII exposure. The default stays scrubbed everywhere (SEC-004 unchanged). The
kernel is untouched (it already produces the detail); the change lives entirely in the consumer's
`KernelError → ConsumerError` normalization, threaded from a per-call render option, and inherited by
all three bindings. Governance is a recorded decision (D3) + a SEC-004 carve-out note, not a
constitution amendment.

## Technical Context

**Language/Version**: Rust (consumer crate `prompting-press` + the two binding crates), Python (PyO3
facade), TypeScript (napi facade). No kernel (`prompting-press-core`) change.

**Primary Dependencies**: none new. Uses the existing error-normalization path
(`crates/prompting-press/src/error.rs`, `From<KernelError> for ConsumerError`) and the existing
per-render options surface.

**Storage**: N/A.

**Testing**: cargo tests (consumer + both binding crates), pytest, node:test; the existing
`fuzz_scrub` corpus + `render_detail_secret_is_scrubbed` MUST stay green (default scrub unchanged);
new tests prove opt-in surfaces detail and is never implicitly on.

**Target Platform**: the library's three published bindings.

**Project Type**: library error-handling feature (consumer + bindings).

**Performance Goals**: none (an error-path branch; no hot-path impact).

**Constraints**: off-by-default non-negotiable (FR-002/003); render-detail-only (FR-010); normalized
`{field,code,message}` shape preserved, no native error leak (FR-005/006); no I/O/logging (FR-007);
no kernel change (FR-007/Principle I); success path byte-identical (FR-004/SC-005).

**Scale/Scope**: a single boolean option threaded through one consumer seam + three render call sites.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Principle I (Shared core / structural parity)** — ✅ Kernel unchanged. `KernelError::Render { detail }`
  already carries the detail; the scrub/surface choice is purely in the consumer. Cross-language behavior
  stays structural (all bindings inherit the consumer seam).
- **Principle II (FFI isolation)** — ✅ No FFI dep change; the option is plain data crossing the boundary.
- **Principle III (Minimal boundary)** — ✅ No I/O, no logging, no LLM, no request-body assembly. "Surface"
  = put detail on the returned error value; the caller owns logging. Verify nothing in the feature writes
  anywhere.
- **Principle IV** — N/A (no agreement/template behavior).
- **Principle VI (per-language idiom; normalized errors)** — ✅ Reinforced. The opt-in is set the same way
  per binding (a per-call option in each idiom), and detail is delivered through the shared
  `{field,code,message}` contract; native error types never leak.
- **SEC-004 (value scrub)** — ⚠️ **Touched (the Render half), by design.** This is the spec's whole point.
  Handled per the Clarifications as a **recorded decision (D3) + SEC-004 carve-out note**, NOT a redefinition:
  default stays scrubbed; the carve-out is explicit, off-by-default, per-call. **Boundary-defense trigger
  check**: the opt-in is a boolean option on an existing call, NOT a new pluggable interface and NOT new
  I/O/LLM/etc. — so it does not require the amendment process; a decision record + carve-out note suffices.

**Scope Discipline (R1)**: no new pluggable interface; one boolean on the existing options surface. **PASS.**

**Result: PASS** (the one security-sensitive surface, the SEC-004 Render carve-out, is governed by the
recorded-decision path the spec clarified). Re-check post-design: still PASS.

## Project Structure

### Documentation (this feature)

```text
specs/013-unsafe-render-detail/
├── plan.md              # this file
├── research.md          # Phase 0 (the consumer-seam design, naming, per-binding option shape)
├── data-model.md        # Phase 1 (the option + the two error-normalization paths)
├── quickstart.md        # Phase 1 (prove: default scrubs, opt-in surfaces, never implicit)
├── contracts/
│   └── unsafe-detail-option.md   # Phase 1 — the per-binding option + behavior contract
├── memory-synthesis.md  # written (direct-read fallback)
└── tasks.md             # Phase 2 (/speckit-tasks — NOT created here)
```

### Source Code (repository root)

```text
crates/prompting-press/src/
├── error.rs             # the SEAM: add a detail-preserving normalization path for Render alongside the
│                        #   scrubbing `From<KernelError>` (e.g. `ConsumerError::from_kernel(err, reveal_render_detail)`),
│                        #   the existing `From` keeps scrubbing as the default for every other call site.
└── prompt.rs            # render gains the per-call option; passes reveal flag into the normalization on the Render path.

crates/prompting-press-py/src/prompt.rs     # render gains a keyword-only option (off-by-default); threads to the consumer
crates/prompting-press-node/src/{prompt.rs} # render gains an option (in the options object); threads to the consumer
packages/typescript/src/index.ts            # RenderOptions gains the flag; passed through to the addon

docs/site/src/content/docs/                  # error-reference pages + the relevant guide: document the tradeoff + risk warning
docs/memory/decisions/<date>-unsafe-render-detail-optin.md  # D3 decision record + SEC-004 carve-out note
```

**Structure Decision**: the load-bearing design is the **consumer seam**. `From<KernelError> for
ConsumerError` is a parameterless `From` impl that scrubs Render detail. Adding a per-call flag cleanly
means: keep that `From` impl as the scrubbing default (used everywhere), and add an explicit
`ConsumerError::from_kernel(err, reveal_render_detail: bool)` (or equivalent) that `Prompt::render` calls
with the caller's per-render flag — surfacing the real `Render` detail only when true, scrubbing otherwise,
and never touching Parse/ExcludedFeature/UnknownVariant/UndefinedVariable. Both binding crates route kernel
errors through this same consumer path, so they inherit the behavior rather than re-deciding the scrub.

## Complexity Tracking

> No constitution violations requiring justification. The SEC-004 Render carve-out is handled by the
> recorded-decision path the spec clarified (D3 + carve-out note), explicitly within Scope Discipline
> (a boolean option, not a pluggable seam).

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|--------------------------------------|
| _(none)_ | — | — |
