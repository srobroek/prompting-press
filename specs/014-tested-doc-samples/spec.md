# Feature Specification: Tested Documentation Samples

**Feature Branch**: `014-tested-doc-samples`

**Created**: 2026-06-29

**Status**: Draft

**Input**: User description: "014 — Every code sample in the docs site must have a passing
automated test, gated as part of docs publishing, so examples can't rot. VERSION-AGNOSTIC:
each docs version's samples are tested against THAT version of prompting-press."

## Overview

The docs site (~108 fenced code blocks across getting-started, guides, reference, and templates
pages; Rust ~33, Python ~37, TypeScript ~38) contains runnable code samples with inline
expected-output comments (`// => "..."`, `# => ...`). Today those samples are not automatically
verified — they can silently drift from the real library surface with each refactor or new spec.

This spec makes every doc code sample verifiable: real, tested example source lives in the repo,
injected into the MDX pages at build time (architecture A, source-canonical), and the tests run
as a gated step in the docs-publish workflow. Because each docs version snapshot (spec 012) freezes
its source-tree state, per-version library-pinning falls out structurally — the injected samples
for a v1.1 frozen docs tree are the same files the v1.1 library tests covered.

The deliverable is a developer-facing trust guarantee: every snippet shown in the docs compiles,
runs, and produces the documented output against the library version that docs version ships with.

## User Scenarios & Testing *(mandatory)*

### User Story 1 — A broken sample is caught before it reaches the docs (Priority: P1)

A developer changes the library's public API (e.g. renames a method, changes a constructor
signature). One or more doc code samples use the old surface. The docs-publish CI gate runs the
sample tests and fails before the stale doc is published.

**Why this priority**: this is the core anti-rot guarantee — the gate must exist and must block
publication of a broken sample. Without this, the feature delivers nothing. It is the MVP slice:
one language's gate catching one broken sample is already valuable and independently deliverable.

**Independent Test**: introduce a deliberate breakage in one doc sample (e.g. call a method with
the wrong signature), run the sample-test gate, and confirm CI fails with an error that points to
the broken sample file and line.

**Acceptance Scenarios**:

1. **Given** a doc sample that calls `greet.render(vars)` and the library changes `render` to
   require a second argument, **When** the sample-test gate runs, **Then** CI fails with a
   compiler/runtime error citing the example file and **does not** publish the docs.
2. **Given** all doc samples correctly reflect the current API, **When** the sample-test gate runs,
   **Then** CI passes and the docs-publish step proceeds.
3. **Given** a sample test gate failure, **When** a developer reads the error output, **Then** the
   error message identifies the sample file (e.g. `examples/rust/getting_started.rs:L14`) and the
   specific failure, not an opaque harness error.

---

### User Story 2 — All doc code samples across all three languages are covered (Priority: P2)

Every fenced code block that contains runnable code in the docs site — across Rust, Python, and
TypeScript, across getting-started, guides, and reference pages — is backed by a tested source
file. No page has untested samples that could drift silently.

**Why this priority**: P1 establishes the gate; P2 extends coverage to the full surface. Partial
coverage (e.g. only Rust) already blocks rot in that language; full coverage is the complete
guarantee. Stacked on P1.

**Independent Test**: run the coverage audit tool (`moon run docs:check-sample-coverage` or
equivalent) against the docs site and confirm it reports 0 uncovered fenced runnable code blocks
across all three languages.

**Acceptance Scenarios**:

1. **Given** the full docs site MDX tree, **When** the coverage audit runs, **Then** it reports
   100% of fenced runnable code blocks (those with a language tag other than pure config/YAML/TOML
   definition blocks) are backed by a corresponding example source file.
2. **Given** a new docs page is added with a Rust code block but no corresponding test file,
   **When** the coverage audit runs, **Then** it reports the missing test file by name and the gate
   fails.
3. **Given** the three language test suites, **When** each runs in isolation (`cargo test --doc`,
   `pytest examples/`, `node:test` / vitest on TS examples), **Then** all pass without requiring
   the other two languages' environments to be present.

---

### User Story 3 — Expected-output comments in samples are verified as assertions (Priority: P3)

Doc samples include inline expected-output annotations (e.g. `result.text; // => "Hi Ada, you
have 3 messages."`, `# => "default"`). These are currently illustrative. This user story makes
them real assertions: a mismatch between the computed value and the annotated expected output
fails the test.

**Why this priority**: P1/P2 ensure samples compile and run; P3 ensures they produce the
documented result. A sample that runs but produces the wrong output is still a broken doc.
Stacked on P2 — assertions are only meaningful once coverage is full. SHA-256 hash fields
(`template_hash`, `render_hash`) are exempt from exact-match assertion (they are verified by
format/length only).

**Independent Test**: change the expected-output annotation in a sample (`// => "Hi Ada, you have
3 messages."` → `// => "Hello Ada"`) without changing the code, run the gate, and confirm it fails
with an assertion mismatch pointing to that annotation.

**Acceptance Scenarios**:

1. **Given** a Rust sample `result.text; // => "Hi Ada, you have 3 messages."`, **When** the
   test runs, **Then** `assert_eq!(result.text, "Hi Ada, you have 3 messages.")` (or equivalent)
   is verified.
2. **Given** a Python sample `result.text  # => "Hi Ada, you have 3 messages."`, **When** the
   test runs, **Then** the equality is asserted (via doctest `==` form or pytest `assert ==`).
3. **Given** a TypeScript sample `result.text; // => "Hi Ada, you have 3 messages."`, **When** the
   test runs, **Then** `expect(result.text).toBe("Hi Ada, you have 3 messages.")` is verified.
4. **Given** a sample that asserts a hash field (`result.template_hash; // => "9f2c…"`), **When**
   the test runs, **Then** the test checks `templateHash.length === 64` and `/^[0-9a-f]+$/.test(...)`,
   **not** an exact-value match (the exact hash is an implementation detail that changes with the
   template source).

---

### Edge Cases

- **Config/definition-only blocks**: fenced blocks tagged `yaml`, `json`, `toml` that are prompt
  definitions or config snippets (not executable code) are NOT required to have a backing test —
  they are data, not code. The coverage audit MUST distinguish runnable code blocks from
  definition-only blocks.
- **Tabs components**: many samples appear inside `<Tabs syncKey="..."><TabItem>` MDX structures.
  The injection and coverage tooling MUST handle samples nested inside Tabs without
  mis-attributing them to another page or skipping them.
- **Incomplete fragments**: doc pages may show intentional fragment snippets (e.g. a struct
  declaration without a `main`) that are designed to be read in context, not run standalone. Such
  fragments SHOULD be wrapped into a compile-check-only test (type-checks, no execute) rather than
  skipped entirely.
- **SHA-256 hash assertions**: `template_hash` and `render_hash` values are content-addressed over
  the template source; they change if the sample's template text changes. Expected-output
  assertions for hash fields MUST use format/length checks, not exact-match.
- **Version-agnostic pinning across docs versions**: a frozen v1.1 docs tree (spec 012) MUST run
  its samples against the v1.1 library, not the latest. The per-version mechanism MUST be decided
  (see [NEEDS CLARIFICATION] Q3).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Every fenced code block in the docs site whose language tag identifies runnable code
  (i.e., `rust`, `python`/`py`, `typescript`/`ts`, `javascript`/`js`) MUST be backed by a
  tested source file in the repo.
- **FR-002**: The sample-test gate MUST run as part of the docs-publish CI workflow — docs MUST NOT
  be published if any sample test fails (gate is non-optional, same model as the conformance corpus
  gate from spec 006).
- **FR-003**: The source-canonical architecture MUST be used: real, tested example source files
  live in the repo and are injected into the MDX pages at docs build time — the MDX pages are
  never the source of truth for sample code. This is the same pattern as `gen-shape-table.mjs`
  (spec 011) and extends it to all doc samples. [NEEDS CLARIFICATION: Q1 — confirm source-canonical
  vs hybrid is the agreed architecture before implementing the injection harness]
- **FR-004**: Per-language testing MUST use each language's native testing idiom:
  - **Rust**: `cargo test --doc` (rustdoc doctests in `///` doc comments) or `cargo test` on
    example files under `examples/` — whichever fits the sample's completeness level.
  - **Python**: `pytest` on example files under `examples/python/`; docstring doctests via
    `doctest` module are acceptable for simple inline assertions.
  - **TypeScript**: example files under `examples/typescript/`, type-checked with `tsc --noEmit`
    and executed with `node:test` or vitest (TypeScript has no native doctest facility).
- **FR-005**: Sample tests MUST be **build-time/dev-time only** — no new runtime dependency is
  introduced into any published library package (`prompting-press`, `prompting-press-py`,
  `prompting-press-node`). The test harness and example files are dev-only artifacts. (Principle
  II/III: FFI isolation and minimal boundary are not affected.)
- **FR-006**: The feature MUST be **version-agnostic**: a frozen docs version (spec 012) MUST
  run its sample tests against the matching version of the library, not the latest. [NEEDS
  CLARIFICATION: Q3 — per-version library-pin mechanism]
- **FR-007**: Expected-output annotations in sample files (`// => "..."`, `# => ...`) SHOULD be
  promoted to real assertions in the test. SHA-256 hash fields (`template_hash`, `render_hash`,
  `templateHash`, `renderHash`) MUST be verified by format and length only (64-char lowercase hex),
  never by exact value. [NEEDS CLARIFICATION: Q2 — whether to promote all `// =>` annotations or
  only a curated subset]
- **FR-008**: A coverage-audit step MUST report which fenced runnable code blocks in the MDX pages
  lack a corresponding tested source file. The audit MUST distinguish runnable code blocks from
  definition-only blocks (YAML/JSON/TOML prompt definitions and config snippets).
- **FR-009**: The injection step (source → MDX) MUST coordinate with `gen-shape-table.mjs` (spec
  011) to run in the same `prebuild` / `pregenerate` pipeline stage so there is one consistent
  build entry point. The injected content in MDX pages MUST be marked with a generator comment
  (e.g. `{/* AUTO-INJECTED from examples/rust/... */}`) so it is not hand-edited.
- **FR-010**: Samples nested inside `<Tabs>` / `<TabItem>` MDX components MUST be covered by the
  injection and audit tooling; they MUST NOT be silently skipped due to MDX nesting.
- **FR-011**: The gate MUST produce error output that identifies the failing sample by its source
  file and line number (not just the MDX page), so a developer can fix the right file.
- **FR-012**: Fragment-only samples (intentional partial code snippets not runnable as a standalone
  program) SHOULD receive a compile-check-only test (type checking, no execution) rather than being
  excluded from coverage.

### Key Entities

- **Sample source file**: a real, tested code file (`.rs` doctest / example, `.py` example,
  `.ts` example) that lives in the repo under `examples/{rust,python,typescript}/` (or equivalent
  lang-specific location). It is the source of truth for the doc sample; the MDX block is injected
  from it.
- **Injection marker**: an MDX comment pair (`{/* INJECT: examples/rust/greet.rs#render */}` …
  `{/* END INJECT */}`) that the build-time injector replaces with the fenced code block content
  from the referenced source file and anchor.
- **Coverage audit**: a script (moon task `docs:check-sample-coverage` or equivalent) that walks
  the MDX tree, identifies all fenced runnable code blocks, and reports which lack injection
  markers (i.e. are not backed by a tested source file).
- **Sample-test gate**: a CI step (`moon run docs:test-samples` or equivalent) that runs all three
  language sample suites and fails the build if any sample test fails.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of fenced runnable code blocks in the docs site MDX tree (tagged `rust`,
  `python`/`py`, `typescript`/`ts`, `javascript`/`js`) are backed by a tested source file. The
  coverage audit reports zero uncovered blocks.
- **SC-002**: A deliberately broken doc sample (wrong method name, wrong argument count) causes the
  sample-test gate to fail CI and halts the docs-publish step.
- **SC-003**: Each per-language sample suite runs independently without requiring the other
  languages' toolchains to be present (Rust suite: `cargo test`; Python suite: `pytest`; TS suite:
  `tsc` + vitest / `node:test`).
- **SC-004**: Each docs version's sample tests run against the matching version of the library —
  the v1.1 frozen docs tree tests against the v1.1 library, not against a later release.
- **SC-005**: Expected-output annotations (`// => "..."`, `# => ...`) for all three languages are
  promoted to real assertions; the assertions pass on all three sample suites. Hash fields
  (`template_hash`, `render_hash`) are verified by format/length (64-char hex), not exact value.
- **SC-006**: No new runtime dependency is added to any published library package; all example
  files and test harness code are confined to `examples/` and dev-only tooling (verifiable by
  inspecting published package manifests).
- **SC-007**: Injected MDX blocks carry a generator comment; the coverage audit confirms no
  hand-edited sample blocks exist in the MDX tree without a corresponding injection marker.

## Assumptions

- **Spec 011 (generate-from-source)**: a gen-shape-table–style injection pipeline already exists
  (spec 011, `prebuild` / `pregenerate` step in `docs/site/scripts/`). This spec extends that
  pipeline for sample injection; it does not create a parallel build path. If spec 011 has not
  landed, the injection harness for samples stands alone and spec 011 integration is a follow-up.
- **Spec 012 (versioned docs)**: a frozen docs version is a git-tagged snapshot of the
  `docs/site/` tree plus its source dependencies. The per-version library-pin mechanism is decided
  at spec 012 specify time; spec 014 defers the exact mechanism to that spec but requires the gate
  to be version-aware (SC-004, FR-006).
- **~108 fenced code blocks in scope**: approximate count based on the docs site MDX tree at
  spec-010-delivery time (Rust ~33, Python ~37, TypeScript ~38), spanning getting-started, guides,
  reference, and templates pages. The exact count will shift as the site evolves; the coverage
  audit is the authoritative check, not a static number.
- **Definition-only blocks are excluded**: fenced blocks tagged `yaml`, `json`, `toml` containing
  prompt definitions (not executable code) are not runnable and are explicitly excluded from the
  coverage requirement.
- **No new published runtime deps**: the sample-test harness is entirely dev-only tooling; no
  changes to `Cargo.toml` (prompting-press / prompting-press-core) or the Python/TS package
  manifests that affect published artifacts.
- **Architecture A (source-canonical)** is assumed but gated on Q1 clarification — if a hybrid
  approach is preferred (source-canonical for new samples + extract-and-test for legacy blocks),
  FR-003 applies to new samples and a migration path applies to existing ones.
- **Node 22.12 floor for docs tooling** is already established by spec 010 (Astro 7); the TS
  sample test runner operates under the same floor.
- **Moon is the build orchestrator**: moon tasks (`moon run docs:test-samples`,
  `moon run docs:check-sample-coverage`) are the entry points, consistent with the rest of the
  project's build system.

## Dependencies

- **Depends on**: 010 (the docs site to add samples to; the MDX tree and Starlight structure that
  the injection and audit tooling targets).
- **Coordinates with**: 011 (generate-from-source pattern — shares the `prebuild` pipeline and
  the injection-marker convention) and 012 (versioned docs — the per-version library-pin mechanism
  required by FR-006/SC-004).
- **Should land before**: 007 (v1 release publish) — so the publish gate already enforces
  sample correctness on day one.

## Out of Scope

- **Re-implementing or replacing `gen-shape-table.mjs`** (spec 011 deliverable) — this spec
  extends the same pipeline, not replaces it.
- **Testing non-code content** (prose, tables, diagrams, YAML/JSON/TOML prompt definition blocks).
- **Full narrative prose auto-generation** — out of scope for this spec (spec 010 Assumption).
- **A docs-specific linter for prose quality** (grammar, style) — out of scope.
- **Adding runtime behavior to the library** — this spec is build-time/dev-time only; no library
  behavior changes (Principle III / spec 010 FR-011 parity).
- **Cross-language render-parity testing via doc samples** — that is the conformance corpus's job
  (spec 006); doc sample tests assert the documented expected output, not cross-language identity.
