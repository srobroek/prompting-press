# Inter-Spec Conflict Audit — Spec 003 vs 001 / 002

**Date**: 2026-06-27 · **Step**: 16 (`sync.conflicts`) · **Verdict**: ✅ NO CONFLICTS

> **Provenance note.** The `speckit-sync-conflicts` subagent reached the correct verdict (NO CONFLICTS)
> but was partially corrupted by the bash-channel glitch (it cited a non-existent `src/loader.rs` and
> claimed a "duplicate `prompting-press-core` dependency at Cargo.toml lines 7 and 11"). Both the
> verdict and the one actionable nit were **re-verified main-thread** against the real tree; the nit is
> a **false positive** (see below). Conclusions below are evidence-backed.

## Category 1 — Shared-API / contract surface (003 consumes 002 as-built): NO CONFLICT

The consumer wraps the kernel's real surface, inventing nothing:
- `render.rs:104` calls `prompting_press_core::render(def, variant, values, guard)` — the kernel's actual free-fn signature (002).
- `render.rs:126` calls `prompting_press_core::get_source(def, variant)`.
- `check.rs` calls `required_roots(def, variant)` and `provenance_view(def)` — both real kernel exports.
- `error.rs:178` maps the **exact** `KernelError` variant set (`UnknownVariant`, `UndefinedVariable`, `Parse`, `Render`, `ExcludedFeature`) via a wildcard-free match — a new kernel variant would be a compile error, so divergence cannot drift in silently.

## Category 2 — Constitutional boundary (Principles I/II/III): NO CONFLICT

- **No re-implementation (Principle I / C-01).** Rendering, agreement analysis, variant resolution, and hashing are all delegated to the kernel (Category 1 evidence). `rg sha2|Sha256` over the consumer → none.
- **FFI-free (Principle II / C-02).** `moon run ci:check-ffi` → PASSED; no pyo3/napi in the consumer tree.
- **No I/O / no token counting (Principle III / C-03).** Loaders take already-read `&str`; no token surface (F4 dropped). Verified in the sync-analyze report.
- **Guard expansion stays in 002.** The consumer only plumbs `GuardConfig` and surfaces `RenderResult.guard` (`render.rs:104`, `compose.rs:214`); it neither expands nor re-tests guard logic (F5).

## Category 3 — Schema / shape ownership (001/002 own it, 003 re-exports): NO CONFLICT

`lib.rs:182-183` re-exports `prompting_press_core::generated::prompt_definition::PromptDefinition`; `lib.rs:187` re-exports `RenderResult`. No parallel/forked shape is defined in the consumer. The crate doc explicitly states it "re-exports but NEVER hand-edits the generated module" (`lib.rs:180-181`).

## Category 4 — Variant-default semantics: NO CONFLICT

The consumer's CR-1 handling is *consistent with*, not contradictory to, the kernel rule: both `None` and `Some("default")` resolve to the root body (kernel 002). `check.rs:60-77` documents this as the kernel's convention and flags a *declared* `variants["default"]` arm as unreachable (`ReservedVariantName`) precisely because the kernel shadows it — deferring to the kernel rule rather than adding a second default concept.

## Re-verified subagent nit — FALSE POSITIVE

Claimed "duplicate `prompting-press-core` dependency". Evidence: `rg prompting-press-core crates/prompting-press/Cargo.toml` → **one** dependency entry (line 18); the second hit (line 21) is a **comment**. `cargo metadata` parses the manifest with no duplicate-key error, and clippy/tests pass. No duplicate exists.

## Overall verdict

✅ **NO CONFLICTS** across all four categories. Spec 003 consumes the 002 kernel exactly as-built and
re-exports rather than redefines the 001/002 schema shapes. No BLOCKING or MINOR inter-spec
contradiction. Proceed.
