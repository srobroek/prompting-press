# Contract: API-doc Intermediate Representation (IR)

The single contract that decouples the three language **extractors** from the one shared
**renderer**. Each extractor (`extract-rust-api.mjs`, `extract-ts-api.mjs`, `extract-python-api.py`)
produces this same JSON shape; the renderer (`render-api-ref.mjs`) consumes ONLY this shape, so the
three pages render in parallel structure (FR-009) without per-language renderer branches.

This is the load-bearing design artifact. The extractors absorb each toolchain's idiosyncrasies
(rustdoc JSON, TypeDoc JSON, griffe) and normalize to this; the renderer stays language-agnostic.

## Top-level: `ApiDoc`

```jsonc
{
  "language": "rust" | "python" | "typescript",  // drives only the page title + code-fence lang
  "package": "prompting-press",                   // display name of the package/crate
  "version": "latest",                            // from gen-api-refs --version (default "latest")
  "generatedFrom": "rustdoc-json 0.x | typedoc 0.28.19 | griffe 2.1.0",  // provenance line, informational
  "groups": [ Group, ... ]                        // ordered; the renderer emits them in array order
}
```

## `Group` — a section of related symbols

```jsonc
{
  "title": "Prompt" | "RenderResult" | "Errors" | "Composition" | ...,
  "anchor": "prompt",            // stable slug for in-page links
  "blurb": "string | null",      // optional group-level doc text (jargon-stripped); null if none
  "symbols": [ Symbol, ... ]     // sorted deterministically (kind, then name) by the EXTRACTOR
}
```

The extractor is responsible for assigning each public symbol to a group and for the deterministic
ordering (R5). The canonical group set + order is fixed across languages so the three pages line up:
`Prompt` → `RenderResult` → `GuardConfig` → `CheckReport` / `Finding` → `Composition` / `Message`
→ `Errors` (hierarchy + code vocabulary) → `Shape types` (links to the shape page, not re-rendered).

## `Symbol` — one public API item

```jsonc
{
  "name": "render",
  "kind": "class" | "struct" | "enum" | "interface" | "function" | "method" |
          "constructor" | "accessor" | "field" | "variant" | "const" | "type",
  "signature": "fn render<V>(&self, vars: &V, variant: Option<&str>, guard: &GuardConfig) -> Result<RenderResult, ConsumerError>",
                                 // language-native, rendered verbatim in a code fence
  "doc": "string | null",        // the symbol's doc-comment text, jargon-stripped + MDX-escaped;
                                 //   null ⇒ undocumented ⇒ the GATE FAILS (FR-008 / R6)
  "members": [ Symbol, ... ],    // nested (e.g. struct fields, enum variants, class methods); may be []
  "shapeRef": "string | null",   // when set (e.g. "PromptDefinition"), the renderer emits a LINK to
                                 //   the shape page instead of rendering members (FR-010); else null
  "deprecated": "string | null"  // optional deprecation note; null if not deprecated
}
```

## Invariants the renderer relies on

1. **`doc: null` is a hard error.** A public symbol with no doc comment makes the extractor mark
   `doc: null`; the orchestrator/gate fails and names `language` + `name` (R6 / FR-008). The renderer
   never silently omits it.
2. **`signature` is pre-formatted, language-native, and verbatim.** The renderer wraps it in a code
   fence tagged by `language`; it does not parse or reformat it.
3. **`doc` is already jargon-stripped + MDX-escaped** by the extractor via the shared
   `scripts/lib/strip-jargon.mjs` (so no `C-NN`/`FR-`/`SC-`/`SEC-`/Principle/spec-number strings reach
   the page — FR-006 / SC-005 — and pipes/braces/backticks are safe — FR-007).
4. **Ordering is deterministic and set by the extractor** (kind, then name within a group; groups in
   the fixed canonical order). The renderer preserves array order, so two runs are byte-identical (R5).
5. **`shapeRef` short-circuits member rendering** — a re-exported shape type links to
   `prompt-definition.mdx`, never duplicates it (FR-010).
6. **Public surface only.** The extractor emits only symbols on the language's public surface (Rust
   lib.rs re-exports / TS index.ts exports / Python `__all__` — R4); internal symbols never enter the IR.

## Rendered output (per language)

The renderer writes `reference/<language>.mdx` with:
- AUTO-GENERATED frontmatter marker + a brief "generated from source doc comments" note (matching the
  shape page's convention).
- One `##` section per `Group` (title + optional blurb), each symbol as a `###` with its signature
  code fence, doc prose, and nested members; `shapeRef` symbols render as a link line.

The page is byte-stable given a fixed IR; the freshness gate regenerates and diffs (SC-003).
