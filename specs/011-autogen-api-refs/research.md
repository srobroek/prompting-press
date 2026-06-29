# Phase 0 Research: Auto-generated language API references

All Technical-Context unknowns resolved below. Versions verified 2026-06-29 via the package-version
registry checks; the nightly date is pinned at implementation time (see R1).

## Dependency Pins

| Tool | Version | Scope | Used for |
|------|---------|-------|----------|
| TypeDoc | `0.28.19` | dev (docs) | TypeScript extraction → JSON (`--json`) |
| griffe | `2.1.0` | dev (docs, via uv) | Python extraction → structured object model |
| Rust nightly toolchain | `nightly-<PINNED-DATE>` (set at impl; e.g. `nightly-2026-06-15`) | dev (docs) | `rustdoc -Z unstable-options --output-format json` |
| (existing) Node | `25.3.0` | dev | runs the renderer + TypeDoc |
| (existing) uv | `0.11.8` | dev | runs griffe hermetically |

All three are **dev/build-time only** and MUST NOT appear in any shipped crate or package runtime
dependency set (FR-011 / SC-006; CI-guarded by `ci:check-ffi` for the Rust side and by keeping these
out of the bindings' published manifests).

## R1 — Rust extraction: pinned nightly rustdoc JSON (the second-toolchain wrinkle)

- **Decision**: Extract Rust API docs via `cargo +nightly-<PINNED-DATE> rustdoc -p prompting-press -- -Z unstable-options --output-format json`, reading the emitted `target/doc/prompting_press.json`.
- **Rationale**: rustdoc JSON gives resolved signatures, doc-comment text (already markdown), visibility, and item kinds as structured data — exactly the IR the renderer needs. Clarify chose this over a stable syn-based parser (which loses resolution and needs custom parity work).
- **The wrinkle (load-bearing)**: the repo pins ONE **stable** toolchain (`1.95.0`) in lockstep across `rust-toolchain.toml` + `mise.toml` + `Cargo.toml`, specifically so `rustfmt` is byte-stable for the `schemas:codegen-check` determinism gate. rustdoc JSON is nightly-only, so this feature adds a **second, separate** pinned nightly toolchain used **only** for extraction. It does NOT replace or alter the stable pin; the codegen gate keeps running on stable 1.95.0.
- **How pinned**: a dedicated `rust-toolchain-nightly.toml` (or a `mise.toml` second-toolchain entry) names the exact `nightly-YYYY-MM-DD`. The extractor invokes that toolchain explicitly (`cargo +nightly-YYYY-MM-DD …`), never a floating `+nightly`. A nightly bump is a deliberate, reviewed change (the rustdoc-JSON `format_version` can change between nightlies — see R5).
- **CI reachability**: nightly is installed by `mise`/`rustup` on the CI runner that runs the docs gate; build-time only, so it never ships. Confirmed feasible (same mechanism that installs the stable toolchain).
- **Alternatives considered**: stable syn-based extraction (rejected — clarify); `cargo doc` HTML scraping (rejected — fragile, unstructured).

## R2 — TypeScript extraction: TypeDoc JSON

- **Decision**: `typedoc --json <out> packages/typescript/src/index.ts` (TypeDoc 0.28.19), entry-point = the public facade module; consume TypeDoc's JSON model directly (no `typedoc-plugin-markdown` — the shared renderer owns MDX output).
- **Rationale**: TypeDoc resolves TSDoc comments, exported symbol signatures, and visibility from `index.ts`'s exports — naturally bounding extraction to the public surface (R4). Native `--json` avoids a markdown-plugin formatting layer we'd have to normalize.
- **Alternatives**: `tsc` + the TS compiler API directly (rejected — reimplements what TypeDoc already does); api-extractor (heavier, `.d.ts`-rollup-oriented, less doc-comment-centric).

## R3 — Python extraction: griffe (not pdoc)

- **Decision**: extract via **griffe 2.1.0** as a library (run a small `extract-python-api.py` under `uv`), walking `prompting_press` and emitting the IR.
- **Rationale**: griffe is a structured-API-extraction library (the engine behind mkdocstrings) that yields a serializable object model of modules/classes/functions + docstrings + signatures — exactly an IR source. pdoc (16.0.0) is an HTML *generator*, not an extraction library, so it would mean scraping its output; rejected.
- **Public surface**: griffe can resolve `__all__`; the extractor filters to `prompting_press.__all__` (R4).
- **Alternatives**: pdoc (rejected — generator not extractor); `inspect` + `ast` by hand (rejected — reinvents griffe).

## R4 — Public-surface boundary per language (FR-005)

The generated page MUST cover the public surface and exclude internal symbols. Per language the
"public surface" is defined by that language's own export mechanism:

- **Rust**: the set re-exported from `crates/prompting-press/src/lib.rs` (the crate's intended public API — e.g. `Prompt`, `PromptOverlay`, `RenderResult`, `GuardConfig`, `CheckReport`, `Finding`, `FindingKind`, `Composition`, `Message`, `ConsumerError`, `FieldError`, `error::code`, and the re-exported `PromptDefinition`/`PromptVariable`/`PromptVariant`). rustdoc JSON marks visibility and the crate root's re-exports; the extractor walks from the root's public exports, NOT every `pub` item in a private module (memory-synthesis soft-watch).
- **TypeScript**: the symbols exported from `packages/typescript/src/index.ts` (TypeDoc's entry-point export set).
- **Python**: the names in `prompting_press.__all__`.

Re-exported generated shape types (`PromptDefinition`/`PromptVariable`/`PromptVariant`) are **linked to the shape page**, not re-rendered (FR-010), to avoid duplicating `prompt-definition.mdx`.

## R5 — Determinism (SC-003 / FR-004)

- **Decision**: the renderer sorts symbols by a stable key (kind, then name) and emits byte-stable
  markdown; extraction is pinned (R1–R3) so the IR is reproducible. The gate runs the orchestrator
  twice and diffs (mirrors `codegen-check.sh`).
- **rustdoc-JSON `format_version`**: pinned nightly ⇒ stable `format_version`; the extractor asserts
  the expected `format_version` and fails loudly on mismatch (so a stray nightly can't silently
  change the IR). This is the determinism guard for the Rust path.

## R6 — Missing-doc-comment policy (FR-008)

- **Decision**: **fail the freshness gate** on a public symbol with no doc comment, rather than
  emitting a silent "undocumented" marker page that could ship. Rationale: the whole point is that
  the reference is accurate-by-construction; an undocumented public symbol is a doc defect that
  should block, consistent with the project's other hard gates. The gate message names the symbol +
  language so the fix (add the doc comment in source) is obvious.
- **Alternative**: a visible `*(undocumented)*` marker (rejected as the default — it normalizes
  shipping gaps; the gate-fail is stricter and matches the "single source of truth" intent). A
  per-symbol allowlist escape hatch is explicitly **not** added in v1 (Scope Discipline).

## R7 — Generation wiring (mirror, don't moon-ify)

- **Decision**: the docs site is NOT a moon project; it generates via `package.json` `prebuild`
  (`node scripts/gen-shape-table.mjs`) and is freshness-checked in CI through `check-stale-surface.mjs`
  + `schemas:codegen-check`. So the orchestrator `gen-api-refs.mjs` is added to `prebuild` alongside
  the shape generator, and the freshness gate is a CI step that runs it and diffs.
- **Rationale**: consistency with the established, working pattern; no new task-runner surface.
- **Shared helper**: the jargon-strip + MDX-escape logic currently inside `gen-shape-table.mjs` is
  lifted into `scripts/lib/strip-jargon.mjs` and imported by BOTH (the shape generator keeps working
  identically; this is a refactor-extract, the one allowed touch of shape-page code — re-verify its
  output is byte-identical after the extract).

## R8 — Version-awareness (FR-016, coordination with spec 012)

- **Decision**: `gen-api-refs.mjs` accepts `--version <id>` (default `latest`) and `--out <dir>`
  (default the live `reference/` dir). 011 invokes it with defaults (latest only). Spec 012's
  snapshot task calls it per-version with an explicit `--version`/`--out` into the versioned tree.
- **Rationale**: a single argument with a default; keeps the parameter shape stable for 012 without
  building any multi-version behavior in 011 (that's 012's job).
