# Public API export inventory — all three languages (2026-07-01)

Authoritative snapshot after the export-consistency cleanup this session. Sources:
`crates/prompting-press/src/lib.rs` (Rust root re-exports), `packages/python/python/prompting_press/__init__.py`
(`__all__`), `packages/typescript/src/index.ts` (exports).

## The surface, side by side

| Concept | Rust `prompting_press::` | Python `from prompting_press import` | TypeScript `from "prompting-press"` |
|---|---|---|---|
| Primary type | `Prompt` | `Prompt` | `Prompt` |
| Render config | `GuardConfig` | `GuardConfig` | `GuardConfig` (type) |
| Render result | `RenderResult` | `RenderResult` | `RenderResult` |
| Composition | `Composition`, `Message` | `Composition`, `Message` | `Composition`, `Message` (type), `CompositionEntry` |
| Lint report | `CheckReport`, `Finding`, `FindingKind` | `CheckReport`, `Finding` | `CheckReport`, `Finding` (type) |
| Derive overlay | `PromptOverlay` | (plain dict) | (partial object) |
| Errors | `ConsumerError`, `FieldError`, `error::code::*` | `PromptingPressError`, `PromptValidationError`, `PromptRenderError`, `LoadError`, `FieldError` | same 4 classes + `FieldError` |
| Shape types | `PromptDefinition`, `PromptVariable`, `PromptVariant`, `PromptDefinitionRole` | `PromptDefinition`, `PromptVariable`, `PromptVariant` | `PromptDefinition`, `PromptVariable`, `PromptVariant` (types) |
| Version probe | `core_version()` (fn) | `core_version` | `coreVersion` |

## Changes made this session (library public-surface, pre-publish 0.1)

1. **`GuardConfig` re-exported at the Rust consumer ROOT** (`prompting_press::GuardConfig`), matching
   Python + TypeScript, which already exported it top-level. Previously it was reachable only via
   `prompting_press::core::GuardConfig` — an inconsistency that also made 6 hand-written Rust doc
   snippets fail to compile. All docs + the sample app now use the root import.
2. **Removed `pub use prompting_press_core as core;`** from the Rust consumer crate. It re-exported the
   ENTIRE kernel (`engine`, `agreement`, `origin`, `KernelError`, the `render` free fn, …) — implementation
   the consumer crate is meant to encapsulate (Principle II/III). Verified nothing referenced `::core::`
   after (1), it was in no reference page/README, and it's pre-publish. The consumer now exposes only its
   curated public interface; the kernel stays internal. **Breaking change** to any `prompting_press::core::*`
   path — none existed in-repo. **TODO: record in DECISIONS.md / a spec note before publish.**

## Intentional cross-language differences (Principle VI — native idiom, NOT defects)

- **Error model**: Rust = one `ConsumerError` enum + a closed `code` string vocabulary (`error::code::*`);
  Python/TS = an exception-class hierarchy (`PromptValidationError`/`PromptRenderError`/`LoadError`). Idiomatic
  per ecosystem.
- **`FindingKind`**: a data-carrying Rust enum (`UntrustedWithoutGuard { field }`). Data-carrying enums do
  NOT cross PyO3/napi natively, so the bindings expose `finding.kind` as a stable snake_case DISCRIMINANT
  STRING (`"untrusted_without_guard"`) with the inner `field` echoed in `finding.detail`. The binding does an
  EXHAUSTIVE match over the Rust enum (compile-fails if a variant is added), so type-safety holds at the
  boundary. (Only one variant exists today.)
- **`PromptOverlay`**: a named Rust struct for `derive`; Python/TS pass a plain dict / partial object.
- **Version probe casing**: `core_version` (Rust/Python) vs `coreVersion` (TS, napi camelCase convention).

## Verification (all green)

- `cargo build --workspace` → Finished; `cargo test -p prompting-press-greeter-cli` → 13/13.
- `node docs/site/scripts/gen-api-refs.mjs` → deterministic (twice-run identical); `core` no longer a symbol
  in reference/rust.mdx; `GuardConfig` now at the root.
- `node docs/site/scripts/build-versions.mjs` → dist assembled.
- NOTE: the api-ref freshness gate reports "drift" ONLY because the regenerated pages are uncommitted vs HEAD
  (regen is deterministic + correct); it passes once committed. Nothing is committed this session by design.
