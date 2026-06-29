### 2026-06-29 — SEC-004 carve-out: opt-in unsafe render-error detail per render call

**ID**: D3

**Status**
Active

**Why this is durable**
SEC-004 ("never leak bound-value content into an error message/stack/trace") is the load-bearing
safety invariant across every binding. This decision records the one sanctioned, explicitly-governed
escape hatch: a per-call boolean that lets a caller opt in to receive the full render-error detail
that SEC-004 would otherwise scrub. The carve-out and its constraints — off-by-default,
per-call-only, never global — are what make the escape hatch safe to exist.

**Decision**
Add an explicit, **off-by-default, per-render-call** boolean that, when `true`, surfaces the
otherwise-scrubbed `KernelError::Render` `detail` verbatim in the returned/raised error's
`message`. When `false` (the default) behavior is **byte-for-byte identical** to the prior
scrubbing path (SEC-004 unchanged). The kernel is untouched.

The option:
- **Rust consumer**: `Prompt::render(vars, variant, guard, reveal_render_detail: bool)` — a 4th
  positional argument; all existing callers pass `false`.
- **Python binding**: `render(..., *, unsafe_reveal_render_detail: bool = False)` — keyword-only.
- **TypeScript binding**: `RenderOptions { ..., unsafeRevealRenderDetail?: boolean }` — optional
  field on the existing options object (C-11).

The names are deliberately risk-signaling (`unsafe_*` / `unsafeReveal*`) and carry a doc-comment
warning at every call site that enabling them may surface bound-value content (untrusted input /
PII / secrets) into the returned error and any log/stack derived from it.

The consumer seam is `ConsumerError::from_kernel_revealing(err: KernelError, reveal: bool) -> Self`
in `crates/prompting-press/src/error.rs`. When `reveal = false` it delegates to the existing
`From<KernelError>` impl byte-for-byte. When `reveal = true` it surfaces the `Render` detail
verbatim; ALL other arms (`Parse`, `ExcludedFeature`, `UnknownVariant`, `UndefinedVariable`)
are unaffected regardless of the flag.

This is a **recorded decision + carve-out note, NOT a constitution amendment**:
- The default scrub-by-default guarantee is **unchanged**. SEC-004 still holds for every caller
  that does nothing.
- The opt-in is a simple boolean on an existing call — NOT a new pluggable interface, NOT new I/O
  or LLM coupling — so it does not trigger the boundary-defense amendment process (Scope Discipline
  R1: no seam, no second implementation required).
- Only the `Render`-scrub half of SEC-004 is touched (by design; the `Parse` refinement is D2).

**Evidence**
- Consumer seam: `crates/prompting-press/src/error.rs` — `ConsumerError::from_kernel_revealing`;
  the existing `From<KernelError>` impl is unchanged.
- Render call sites: `crates/prompting-press/src/prompt.rs` `Prompt::render`; all existing callers
  updated to pass `false` (compile-enforced).
- Python binding: `crates/prompting-press-py/src/prompt.rs` — `unsafe_reveal_render_detail` kwarg.
- Node binding: `crates/prompting-press-node/src/prompt.rs` — `unsafe_reveal_render_detail` param;
  `packages/typescript/src/index.ts` — `RenderOptions.unsafeRevealRenderDetail`.
- Tests: `error::tests::from_kernel_revealing_*` (consumer seam unit tests); `reveal_flag_*`
  (prompt.rs); `test_unsafe_render_detail.py` (Python); `unsafe-render-detail.test.mjs` (Node);
  `fuzz_scrub.rs::default_render_false_scrubs_render_path` (no-implicit-enable proof).
- All gates passed: `cargo --workspace`, Python 137, Node 138.

**Tradeoffs**
- Gained: a deliberate, per-call debug path for operators who control their log destination and
  need the real render-error detail to diagnose failures. Previously this information was
  unreachable from the binding surface.
- Cost: the option exists in the API surface. A caller that passes `true` in a context with a
  shared log sink may inadvertently expose bound-value content. The risk-signaling name + doc-comment
  are the mitigations; the caller is responsible.
- Reconsider: if a caller wants per-field or per-error-kind redaction (not just on/off), that is a
  separate, larger feature outside the current scope discipline.

**Where to look next**
`crates/prompting-press/src/error.rs` (`from_kernel_revealing` + risk doc-comment),
`crates/prompting-press/src/prompt.rs` (`render` 4th arg + Errors section),
`crates/prompting-press-py/src/prompt.rs` (`unsafe_reveal_render_detail` kwarg doc-comment),
`packages/typescript/src/index.ts` (`RenderOptions.unsafeRevealRenderDetail` doc-comment).
