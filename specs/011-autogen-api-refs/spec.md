# Feature Specification: Auto-generated language API references

**Feature Branch**: `011-autogen-api-refs`

**Created**: 2026-06-29

**Status**: Draft

**Input**: User description: "011 — Auto-generated language API references. Replace the three hand-written language API reference pages with pages generated from the source's own doc comments, eliminating the doc-drift that required manual sync."

## Clarifications

### Session 2026-06-29

- Q: Full-autogen vs. hybrid (generated signatures + separately-maintained prose)? → A: **Full-autogen from doc comments** — the source doc comment IS the prose; generate signatures + descriptions from it, with no separate curated-prose layer. The doc comment becomes the single source of truth (Principle VII), so reader-facing prose cannot drift from code because it lives in the code. A thin-reading page is fixed by enriching the doc comment, not by adding a sidecar.
- Q: Rust extraction — nightly rustdoc-JSON (unstable format) vs. stable-channel approach? → A: **Pinned nightly rustdoc-JSON** (`rustdoc -Z unstable-options --output-format json`), with the nightly toolchain version PINNED via the project's existing toolchain-pinning mechanism. Build-time only; the unstable-format risk is contained by the pin (no surprise bumps — a nightly upgrade is a deliberate, gated step).
- Q: Coordinate with the docs-versioning spec — ship single-"latest" vs. design version-aware now? → A: **Design version-aware from the start** — the generator accepts a version / output-path parameter so the forthcoming versioning spec invokes it per-version without rework. (Scope-Discipline note: normally generality is earned by a real second consumer; here the versioning spec is a committed next step, so that consumer effectively already exists.)

## Context

The documentation site already generates one reference page — the prompt-definition shape page — from the JSON Schema via a build-time script (`docs/site/scripts/gen-shape-table.mjs`, run as the Astro `prebuild`). That page has never drifted from the source of truth.

The three **language** API reference pages (`reference/rust.mdx`, `reference/python.mdx`, `reference/typescript.mdx`) are hand-maintained. They are the only hand-written reference surface, and they are the proven source of documentation drift: in recent work, the `derive` rename, the `meta`→`metadata` collapse, the removal of dead `FindingKind` variants, the removal of `UnknownPromptError`, and a change to error-detail scrubbing each silently invalidated these three pages until the inaccuracy was caught by manual inspection. Every such change must currently be mirrored by hand, and a missed mirror ships as a factual error.

This feature extends the existing "single source of truth → generated page" pattern to the language API references, so the source code's own doc comments become the authoritative source for each language's reference page.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Reference pages stay accurate without manual mirroring (Priority: P1)

A maintainer changes a public API surface in the library source — renames a method, removes an error type, changes a signature, or edits a doc comment. The corresponding language reference page reflects the change automatically when the docs are next built; no separate hand-edit of the `.mdx` page is required, and an out-of-date page cannot ship unnoticed.

**Why this priority**: This is the entire reason for the feature. It removes the recurring, error-prone manual-mirroring step that has repeatedly shipped factual errors, and it is the minimal viable slice — generation plus a freshness gate already delivers the value even before any presentation polish.

**Independent Test**: Change a public doc comment or signature in one binding's source, rebuild the docs, and confirm the rendered reference page changes accordingly without editing any `.mdx` file. Separately, change the source but do **not** regenerate, and confirm the freshness gate fails.

**Acceptance Scenarios**:

1. **Given** a public method's doc comment is edited in the Rust consumer source, **When** the docs build runs, **Then** the Rust reference page shows the updated description with no hand-edit to `reference/rust.mdx`.
2. **Given** a public API symbol is renamed or removed in a binding's source, **When** the docs build runs, **Then** the generated reference page for that language reflects the rename/removal.
3. **Given** the source changed but the committed generated page was not regenerated, **When** the freshness gate runs in CI, **Then** the gate fails and names the stale page.
4. **Given** no source change since the last regeneration, **When** the generator runs twice, **Then** both runs produce byte-identical output (determinism).

### User Story 2 - A reader gets a complete, consistent per-language reference (Priority: P2)

A developer using one of the three languages opens that language's API reference and finds every public symbol of the stable surface (the `Prompt` type and its constructors/accessors/operations, the result/config/report types, the composition types, and the error hierarchy with its stable code vocabulary), each with an accurate signature and description, presented consistently with the other two languages' pages.

**Why this priority**: Accuracy (P1) is necessary but not sufficient; the generated output must also be a usable, readable reference covering the whole public surface. This depends on US1's machinery existing.

**Independent Test**: Compare each generated reference page against the public surface exported by that binding; confirm every public symbol appears with a signature, and that the three pages cover the same conceptual surface in parallel structure.

**Acceptance Scenarios**:

1. **Given** the generated Rust/Python/TypeScript reference pages, **When** a reader looks up any public symbol, **Then** its signature and description are present and match the source.
2. **Given** a symbol that is intentionally **not** part of the public surface, **When** the pages are generated, **Then** that symbol does **not** appear in the reference.
3. **Given** the three language pages, **When** they are compared, **Then** they cover the same conceptual API surface and use consistent section organization.

### User Story 3 - The reference reads well, not just accurately (Priority: P3)

A reader gets conceptual framing around the generated signatures — the kind of "why/when" prose that pure signature dumps lack — without that prose being a drift risk of its own.

**Why this priority**: Readability is a real quality bar (the hand-written pages read well), but it is subordinate to accuracy and completeness, and the mechanism for it is the main open design question (full-autogen vs. hybrid).

**Independent Test**: Confirm the generated pages include conceptual framing where it adds value, and that any hand-curated prose is structurally separated from generated content so it cannot silently contradict a signature.

**Acceptance Scenarios**:

1. **Given** a generated reference page, **When** a reader reads it, **Then** it includes conceptual context (not only bare signatures) for the primary types.
2. **Given** the curated-prose mechanism (if any), **When** the source API changes, **Then** stale curated prose is either regenerated or surfaced by the freshness gate, never silently wrong.

### Edge Cases

- A public symbol carries **no** doc comment in source → the generator must still emit its signature and either flag the missing description (e.g. fail the gate or emit a visible "undocumented" marker) rather than silently omitting the symbol.
- A doc comment contains **internal-governance jargon** (constitution/Principle/`C-NN`/`FR-`/`SC-`/`SEC-`/spec-number citations) → the generated page must strip it, exactly as the existing shape-page generator already does for schema descriptions, so reader-facing docs carry no internal references.
- The extraction toolchain for one language is **unavailable** in an environment (e.g. a doc extractor requires a toolchain channel not installed) → the build/gate must fail loudly with an actionable message, never emit a partial or empty page silently.
- A language binding exposes a symbol via re-export from a generated shape (e.g. `PromptDefinition`/`PromptVariable`/`PromptVariant`) → the language reference must not duplicate the shape page's content; it references the shape page instead.
- Doc-comment content includes Markdown/MDX-significant characters (pipes, backticks, braces, angle brackets) → the generator must escape them so the rendered page is valid MDX (the existing shape-page generator already handles table-cell escaping).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The three language API reference pages (`reference/rust.mdx`, `reference/python.mdx`, `reference/typescript.mdx`) MUST be generated from each binding's own source (its public symbols and their doc comments), not hand-maintained.
- **FR-002**: Generation MUST run as part of the documentation build pipeline (the same `prebuild` stage that already runs the shape-page generator), so a normal docs build produces up-to-date pages.
- **FR-003**: A CI freshness gate MUST fail when a committed generated reference page differs from a freshly generated one (the same twice-run / diff-against-committed contract the existing codegen determinism gate uses).
- **FR-004**: Generation MUST be deterministic — running it twice over unchanged source produces byte-identical output (stable symbol ordering, stable formatting).
- **FR-005**: Each generated page MUST cover the full **public** surface of that binding and MUST NOT include non-public/internal symbols. The public surface is the set a consumer can reach through the published package (e.g. what the binding exports), not every symbol in the crate/module.
- **FR-006**: The generator MUST strip internal-governance jargon (constitution/Principle references, `C-NN` roadmap-decision IDs, `FR-`/`SC-`/`SEC-` IDs, spec numbers) from doc-comment text before rendering, consistent with the existing shape-page generator's sanitization.
- **FR-007**: The generator MUST escape MDX/Markdown-significant characters from doc-comment content so every generated page is valid MDX and renders correctly.
- **FR-008**: A public symbol that lacks a doc comment MUST be surfaced (the symbol's signature still appears, and the absence of a description is made visible or fails the gate) rather than silently omitted.
- **FR-009**: The three generated pages MUST be mutually consistent in structure (parallel section organization) and MUST cover the same conceptual API surface across languages.
- **FR-010**: Generated language pages MUST NOT duplicate the prompt-definition shape page; where a binding re-exports a generated shape type, the page references the shape page instead of re-rendering the shape.
- **FR-011**: The doc-extraction toolchains MUST be **dev/build-time only** and MUST NOT introduce any new runtime dependency into the shipped library (the kernel, the Rust consumer, or the bindings' published runtime dependency sets stay unchanged).
- **FR-012**: Each doc-extraction toolchain MUST be pinned to an exact version (consistent with the project's strict-pin policy for build tooling), and the chosen versions MUST be recorded.
- **FR-013**: When any extraction toolchain is unavailable or fails, the generator/gate MUST fail loudly with an actionable message and MUST NOT emit a partial or empty reference page.
- **FR-014**: The generated pages MUST be produced by **full-autogen** from each binding's public doc comments — signatures AND descriptions come from the source, with no separately-maintained curated-prose layer. The doc comment is the single source of truth for reader-facing reference prose; a thin-reading page is remediated by enriching the source doc comment, not by adding a sidecar. (This makes reader prose structurally undriftable: it is the same text the freshness gate already checks.)
- **FR-015**: Rust extraction MUST use nightly rustdoc JSON (`rustdoc -Z unstable-options --output-format json`) with the nightly toolchain version **pinned** via the project's existing toolchain-pinning mechanism. The extractor is build-time only (FR-011). The unstable-format risk is contained by the pin — a nightly upgrade is a deliberate, gated change, never an implicit one.
- **FR-016**: The generator MUST be **version-aware** — it accepts a version / output-path parameter so a later docs-versioning feature can invoke it per-version without reworking the generator. (For the single-"latest" build shipped by this feature, the parameter defaults to the current/"latest" target.)
- **FR-017**: This feature MUST NOT change any public API of the library; it only changes how the reference pages are produced.

### Key Entities

- **Public API surface**: the set of symbols a consumer reaches through each published package — for the reference, the `Prompt` type with its constructors, read-only accessors, and operations (`render`, `get_source`, `check`, `derive`); the render result, guard config, and check-report/finding types; the composition and message types; and the error/exception hierarchy with its stable code vocabulary. This is the input the generator extracts.
- **Doc extraction**: the per-language, build-time process that reads a binding's source and produces a structured description of its public symbols (signatures + doc-comment text) for rendering.
- **Reference generator**: the build-time script (sibling to the existing shape-page generator) that renders each extraction into the corresponding `reference/*.mdx` page, applying jargon-stripping and MDX-escaping.
- **Freshness gate**: the CI check that regenerates the pages and fails if the committed pages are stale, mirroring the existing codegen determinism gate.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Zero language-reference pages are hand-maintained after this feature — all three are produced by the generator (verifiable: the pages carry the generated-file marker and editing source changes them).
- **SC-002**: A public-API change that is committed without regenerating the reference pages is caught by CI 100% of the time (the freshness gate fails), so no stale language reference can merge.
- **SC-003**: The generator is deterministic: two consecutive runs over unchanged source produce byte-identical pages (zero diff).
- **SC-004**: Every public symbol of each binding's stable surface appears on its reference page with a signature; no public symbol is missing and no internal symbol leaks in.
- **SC-005**: No reader-facing reference page contains internal-governance jargon (no `C-NN` / `FR-` / `SC-` / `SEC-` / Principle / spec-number strings).
- **SC-006**: The shipped library's runtime dependency sets are unchanged by this feature (the doc extractors appear only in dev/build tooling).
- **SC-007**: After this feature lands, a subsequent unrelated API change requires no manual edit to any `reference/{rust,python,typescript}.mdx` file for those pages to stay correct.

## Assumptions

- The documentation platform remains Astro/Starlight; per-version documentation is handled by a separate, later spec and is out of scope here. If versioned docs land, the generator will need to run per-version — this coordination is an open question (below), not a commitment in this spec.
- The existing shape-page generator (`gen-shape-table.mjs`) and its prebuild wiring + jargon-stripping are the model to mirror; this feature adds sibling generators, it does not change the shape-page generator.
- The public API surface is small and stable (enumerated in Key Entities); the generator targets that surface rather than attempting to document every internal symbol.
- The strict exact-version pinning policy used for existing build/codegen tooling applies to the new doc-extraction toolchains.
- The library's own test/build gates (cargo, pytest, node:test, conformance, codegen determinism) are unaffected; this feature adds a docs-side freshness gate alongside them.
- **Coordination with the docs-versioning spec**: the generator is built **version-aware from the start** (FR-016) — it accepts a version/output-path parameter, defaulting to the single "latest" target this feature ships. The later docs-versioning feature invokes it per-version with no rework. This feature does not itself produce multiple versioned pages; it only ensures the generator can.
