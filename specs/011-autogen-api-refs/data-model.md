# Phase 1 Data Model: Auto-generated language API references

This feature's "data" is the build-time artifacts that flow extractor → renderer → page, plus the
freshness gate's inputs. There is no runtime/persistent data and no library state change.

## Entities

### ApiDoc IR (the central entity)

The normalized, language-agnostic JSON description of one binding's public API surface. Full shape +
invariants in [contracts/api-doc-ir.md](./contracts/api-doc-ir.md). Produced by each extractor,
consumed by the shared renderer. Key fields: `language`, `package`, `version`, `groups[]` →
`symbols[]` (with `name`, `kind`, `signature`, `doc`, `members[]`, `shapeRef`, `deprecated`).

- **Source per language**:
  - Rust → pinned-nightly `rustdoc … --output-format json` over `crates/prompting-press`, filtered to
    `lib.rs` public re-exports.
  - TypeScript → `typedoc --json` over `packages/typescript/src/index.ts` exports.
  - Python → griffe over `prompting_press`, filtered to `__all__`.
- **Validation rule**: any `symbol.doc == null` (undocumented public symbol) → gate failure (FR-008).
- **Determinism rule**: `groups` in fixed canonical order; `symbols` sorted (kind, then name) by the
  extractor; renderer preserves order ⇒ byte-stable output (SC-003 / FR-004).

### Reference page (generated artifact)

`docs/site/src/content/docs/reference/<language>.mdx` — the rendered output. Carries the
AUTO-GENERATED frontmatter marker; treated as generated (never hand-edited; freshness-gated).
Latest-version pages live at the live path; spec 012 will place versioned copies elsewhere via the
`--out` parameter.

### Canonical group set (shared across the three languages)

A fixed, ordered list that makes the three pages parallel (FR-009 / SC-004):
`Prompt` → `RenderResult` → `GuardConfig` → `CheckReport`/`Finding` → `Composition`/`Message` →
`Errors` (hierarchy + the stable `code` vocabulary) → `Shape types` (links to the shape page).
Each extractor maps its language's public symbols into these groups; a language that legitimately
lacks a symbol in a group simply emits no entry there (it must not invent one, and must not add a
language-only group without the others — consistency is the renderer's contract).

### Toolchain pin set (configuration entity)

The exact versions the extraction depends on (TypeDoc 0.28.19, griffe 2.1.0, nightly-`<date>`),
recorded in [research.md](./research.md) Dependency Pins and enforced via the project's existing
pin mechanisms (`mise.toml`, the new `rust-toolchain-nightly.toml`, `docs/site` dev deps). A pinned
nightly's `rustdoc-json` `format_version` is asserted by the Rust extractor (R5) so a stray nightly
fails loudly rather than silently shifting the IR.

## Relationships

```
crates/prompting-press  --(nightly rustdoc json)-->  extract-rust-api    --\
packages/typescript      --(typedoc --json)-------->  extract-ts-api      ---> ApiDoc IR --> render-api-ref --> reference/<lang>.mdx
packages/python          --(griffe)---------------->  extract-python-api  --/                    ^
                                                                                                 |
                                              scripts/lib/strip-jargon.mjs (shared) -------------+ (also used by gen-shape-table.mjs)
```

The freshness gate re-runs the whole left-to-right flow and diffs the three pages against the
committed copies (twice for determinism), mirroring `schemas/scripts/codegen-check.sh`.

## State transitions

None. Generation is a pure, idempotent build step: same source doc comments + same pinned toolchains
⇒ same IR ⇒ same pages. There is no mutable state, no persistence, no lifecycle.
