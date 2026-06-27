# Code Review Report — Spec 003 (Rust consumer)

**Date**: 2026-06-26 · **Reviewer**: `/speckit.code-review` (main-thread, post-fix) · **Verdict**: ✅ MEETS STANDARDS

## Scope reviewed

The implementation diff of branch `003-rust-consumer` vs `main` (merge-base), source + tests only:

- `crates/prompting-press/src/{error,registry,render,check,compose,lib}.rs`
- `crates/prompting-press/tests/{check,check_purity,compose,loader,render,render_validation}.rs`
- `crates/prompting-press/Cargo.toml`

(The `specs/003-*` docs and `Cargo.lock`/`feature.json` are not code; excluded.) Reviewed against the
constitution (Principles I, II, III, VI, VII), `CLAUDE.md`, and the 003 FR/SC set. State anchored on a
fresh run: clippy `-D warnings` clean, 44/44 tests pass, FFI gate PASSED.

## Verified-state evidence

| Check | Result |
|-------|--------|
| `cargo clippy -p prompting-press --all-targets -- -D warnings` | ✅ clean |
| `cargo test -p prompting-press` | ✅ 44 passed, 0 failed |
| `moon run ci:check-ffi` | ✅ PASSED (no pyo3/napi in consumer or kernel) |
| secret scan over `src/*.rs` diff | ✅ only intentional SEC-004 test fixtures |

## Project-guideline compliance (spot-checked against the constitution)

- **Principle I (shared core, no duplication)** — ✅ `render`/`get_source` delegate to
  `prompting_press_core::{render,get_source}`; `check` is set-arithmetic over the kernel's
  `required_roots`/`provenance_view`; `compose::resolve` reuses the same kernel render path. No
  rendering/agreement/variant/hash logic is reimplemented (`render.rs:104`, `check.rs:250`,
  `compose.rs:210`).
- **Principle II / C-02 (FFI isolation)** — ✅ `Cargo.toml` declares no pyo3/napi; the CI gate confirms
  the resolved tree is clean.
- **Principle III / C-03 (minimal boundary)** — ✅ no I/O (loaders take already-read `&str`,
  `registry.rs:90/114`); no LLM/request-body/token logic; the token-count hook was dropped (F4) and the
  crate exposes no token seam (`lib.rs:62`). `output_model` is echoed as metadata, never parsed.
- **Principle VI / C-06 (idiom + error normalization)** — ✅ `ConsumerError`/`FieldError` are the only
  public error type; garde `Report` and `KernelError` are normalized via `From` impls and never leak
  (`error.rs:151/178`). Composition is an explicit ordered `Vec` builder; no `.chain()` (`compose.rs`).
- **Principle VII (schema SSoT)** — ✅ the crate re-exports the generated `PromptDefinition` and never
  hand-edits it (`lib.rs:182`).
- **SEC-004 / FR-015 (scrub)** — ✅ `Parse`/`Render`/`ExcludedFeature` arms discard `detail` and emit a
  fixed message; pinned by three scrub tests (`error.rs:192-208`, tests `error.rs:221+`).

## Correctness & robustness

- **No panics in `src`** — ✅ ER-1 honored: `insert_and_get` uses the `Entry` API, not `.expect()`
  (`registry.rs:127`); every fallible path returns `Result`. `.expect()` appears only in `#[cfg(test)]`.
- **`check()` purity & determinism** — ✅ `&Registry` shared borrow; `BTreeMap`/`BTreeSet` ordering;
  the CR-1 reserved-`default` dedup is correct (root body analyzed once; dead arm flagged, not
  analyzed) (`check.rs:340`).
- **Validate-before-render ordering** — ✅ `render` resolves → `validate()?` → `from_serialize` →
  kernel, so an invalid input never reaches the kernel (`render.rs:84-104`). Composition validates
  eagerly at `append`, so `resolve` only ever sees validated entries (`compose.rs:156`).
- **No partial-as-success** — ✅ `resolve` propagates the first entry failure with `?` and discards the
  partial `Vec` (`compose.rs:216`).

## Issues (confidence ≥ 80)

**None.** No Critical (90-100) and no Important (80-89) issues found. The two prior review passes' fixes
(CR-1 default-variant lint gap, ER-1 no-panic registry, the TY-1/TY-4/ER-2 doc notes, +coverage 36→44)
are present and correct in the committed state.

## Strengths

- Exhaustive (wildcard-free) `From<KernelError>` match — a new kernel variant is a compile error here
  until mapped.
- The three-sets invariant (struct↔`variables` gap) is documented *and* pinned by a test rather than
  left implicit (`lib.rs:66`, `render.rs:24`).
- Documentation density matches the kernel crate's house style; every non-obvious decision cites its
  FR / clarify-Q / roadmap-C anchor.

## Recommendation

✅ Proceed to cleanup (step 14). No fix-findings loop required.
