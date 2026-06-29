# Feature Specification: Documentation site (Astro/Starlight)

**Feature Branch**: `010-docs-site`

**Created**: 2026-06-29

**Status**: Draft

**Input**: User description: "010 — A published end-user documentation site (Astro + Starlight): overview, getting-started, how-to, FAQ, a per-language API reference, and the template feature-set page; slim the README to a pointer; CI build/deploy."

## Overview

A real, published **end-user documentation site** for the library, built with Astro + the Starlight docs theme.
It is **doc-only** — it changes no library behavior (the kernel/bindings are untouched) — and it documents the
**final post-008/009 surface**: the immutable `Prompt` object (no registry), the `origin` per-variable tag, the
`validation_required` model, the advisory guard, and the supported template feature set. It subsumes the
spec-007 "user docs" scope and closes the template-feature-set documentation gap.

The "users" are **developers evaluating and adopting the library** in any of the three languages: the value is
a single authoritative site that explains what the library is, how to get started, how to do common tasks, and
exactly what each language's API surface is — so the repo README can shrink to an overview + a pointer.

The docs site is a **separate build-time package** (its Astro/Starlight toolchain is dev-only and never enters
the published library packages), so its higher Node floor (Astro 7 needs Node ≥22.12) does not affect the
library's Node 20+ support.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A developer evaluates and gets started (Priority: P1)

A developer lands on the site, reads a clear overview of what the library is (and isn't), and follows a
getting-started guide to install the package for their language and render their first prompt.

**Why this priority**: the overview + getting-started is the front door; without it the site delivers no
adoption value. It is the MVP slice — a site with just these two pages is already useful.

**Independent Test**: Build the site; confirm the overview and a per-language getting-started page render, and
that following the getting-started steps verbatim (install + the first `Prompt` render snippet) works against
the actual published-shape API.

**Acceptance Scenarios**:

1. **Given** the built site, **When** a developer opens the landing/overview page, **Then** it states the
   library's purpose, the typed-input + provenance value, and the explicit non-goals (no I/O, no LLM call, the
   guard is advisory).
2. **Given** the getting-started guide, **When** a developer follows the install + first-render steps for their
   language, **Then** the snippet uses the **current** API (`Prompt` construction + `render`), not a removed
   surface (no `Registry`), and produces the described result.

### User Story 2 - A developer looks up the exact API for their language (Priority: P1)

A developer opens the **API reference** for Rust, Python, or TypeScript and finds the `Prompt` construction
forms, `render`/`getSource`/`check`/`with`, the prompt-definition shape (fields incl. `origin` and
`validation_required`), and the error types — accurate to the shipped code/schema.

**Why this priority**: the API reference is the load-bearing reference content and the roadmap's headline
deliverable; it is what a developer returns to repeatedly.

**Independent Test**: For each language, open the API-reference section; confirm the documented shape fields
match the JSON Schema (`origin`, `validation_required`, `type`, variants, metadata) and the documented methods
match the actual public surface.

**Acceptance Scenarios**:

1. **Given** the API reference, **When** a developer reads the prompt-definition shape, **Then** the fields and
   their meanings are **derived from the JSON Schema** (C-07) and include `origin` (per-variable trust tag) and
   `validation_required`.
2. **Given** the per-language API pages, **When** a developer compares them, **Then** each shows that language's
   native idiom (Rust `Result` / Python raise / TS throw; `with` vs `with_`) for the same uniform capability.
3. **Given** the provenance documentation, **When** a developer reads it, **Then** the **render-result
   provenance** (`template_hash`/`render_hash`) is documented as distinct from the per-variable `origin` tag —
   the two are not conflated.

### User Story 3 - A developer learns the template feature set + does common tasks (Priority: P2)

A developer reads the template-feature-set page (what MiniJinja features are supported) and how-to guides for
common tasks (variants, the agreement check in CI, multi-message composition, the advisory guard), plus a FAQ.

**Why this priority**: this converts a started user into a productive one and closes the known template-feature
documentation gap; P2 because it builds on the overview/getting-started/API-ref being present.

**Independent Test**: Open the template-feature page and confirm it states interpolation/conditionals/loops are
**in** and includes/macros/inheritance are **out**; open a how-to guide and follow it against the real API.

**Acceptance Scenarios**:

1. **Given** the template-feature-set page, **When** a developer reads it, **Then** it lists supported features
   (interpolation, conditionals, loops) and explicitly-excluded ones (`include`/`import`/`extends`/macros/
   inheritance) and explains why (the sound agreement check).
2. **Given** the how-to guides, **When** a developer follows the "lint prompts in CI", "add a variant", and
   "compose multi-message" guides, **Then** each uses the current `Prompt`/`check`/`with`/`Composition` surface.
3. **Given** the guard how-to + FAQ, **When** a developer reads it, **Then** it states honestly that the guard
   is advisory text, not enforcement, and that `variables[].type` is carried metadata, not runtime-enforced.

### Edge Cases

- **Docs drift**: a documented API name/field that no longer exists in the code/schema is a defect — the API
  reference must reflect the shipped shape (the build should fail or the content review must catch a stale
  `Registry`/`provenance` reference).
- **Cross-language divergence**: where the idiom legitimately differs (Rust `Result` vs TS throw; `with` vs
  `with_`), the docs state it as intended, not as an inconsistency.
- **Build reproducibility**: the site builds deterministically in CI from pinned dependencies.
- **Two "provenance" meanings**: the docs must not let the reader conflate the per-variable `origin` tag with
  the render-result provenance hashes.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The site MUST be built with **Astro + Starlight** (pinned exact: Astro 7.0.3, @astrojs/starlight
  0.41.1) as a **separate package** whose toolchain does NOT enter any published library package's dependencies.
- **FR-002**: The site MUST include an **overview** page stating the library's purpose, its typed-input +
  provenance value, and its explicit non-goals (no I/O, no LLM/request assembly, advisory-only guard).
- **FR-003**: The site MUST include **getting-started** content per language (Rust/Python/TS): install + a first
  `Prompt` construction + render, using the **current** surface (no removed `Registry`).
- **FR-004**: The site MUST include a **per-language API reference** (Rust/Python/TS) covering `Prompt`
  construction forms, `render`/`getSource`/`check`/`with(_)`, the error types, and the prompt-definition shape.
- **FR-005**: The prompt-definition **shape documentation MUST be derived from the JSON Schema** (C-07) — fields
  incl. `origin`, `validation_required`, `type`, variants, metadata — not hand-forked.
- **FR-006**: The site MUST include a **template-feature-set** page: interpolation/conditionals/loops supported;
  `include`/`import`/`extends`/macros/inheritance excluded, with the agreement-check rationale.
- **FR-007**: The site MUST include **how-to guides** (lint in CI, add a variant via `with`, multi-message
  composition, the opt-in guard) and a **FAQ**, all using the current surface.
- **FR-008**: The docs MUST document the **provenance model honestly**: render-result `template_hash`/
  `render_hash` (C-05) as **distinct** from the per-variable `origin` tag; the guard as advisory not enforcement
  (C-09); `variables[].type` as carried metadata, not runtime-enforced.
- **FR-009**: The repo **README MUST be slimmed** to overview + getting-started + a pointer to the site (the
  per-package READMEs stay short and point here).
- **FR-010**: The site MUST **build in CI** from pinned deps, and a **deploy path** (GitHub Pages assumed) MUST
  be wired; the build MUST be deterministic/reproducible.
- **FR-011**: This spec MUST NOT change any library behavior (doc-only): no kernel/binding/schema change. (It
  may *read* the schema to derive shape docs.)
- **FR-012**: All documented API names, fields, and snippets MUST match the **shipped post-008/009 surface** —
  no `Registry`, no per-variable `provenance`; `origin` + the `Prompt` object surface throughout.

### Key Entities

- **Doc page**: a Starlight content page (overview / getting-started / how-to / FAQ / template-features / API
  reference) authored in MDX/Markdown.
- **API-reference entry**: a per-language description of a public symbol or the prompt-definition shape, derived
  from the codebase + JSON Schema.
- **Site build**: the Astro build artifact deployed to the host (GitHub Pages).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: The site builds successfully in CI from pinned dependencies, with **zero** broken internal links
  and zero references to removed surfaces (`Registry`, per-variable `provenance`).
- **SC-002**: A developer can go from the getting-started guide to a working first render in **each** of the
  three languages using only the documented steps.
- **SC-003**: The API reference covers **100%** of the public `Prompt` surface (construction forms, render,
  getSource, check, with) and the full prompt-definition shape, per language.
- **SC-004**: The prompt-definition shape documented on the site **matches the JSON Schema** field-for-field
  (incl. `origin`, `validation_required`).
- **SC-005**: The template-feature page correctly lists every supported and excluded feature, matching the
  kernel's actual MiniJinja configuration.
- **SC-006**: The repo README is reduced to overview + getting-started + a site pointer; it contains no
  full registry-era examples.
- **SC-007**: The docs-site package's dependencies do **not** appear in any published library package
  (crates/PyPI/npm) — it is a separate build-time package.
- **SC-008**: The provenance documentation distinguishes the render-result hashes from the per-variable `origin`
  tag (no conflation) and states the guard is advisory.

## Assumptions

- **Hosting = GitHub Pages** (roadmap default; no bikeshedding). The deploy workflow targets Pages.
- **API reference = hand-curated + schema-derived**, NOT a heavyweight auto-doc-gen toolchain: the
  prompt-definition shape tables are derived from the JSON Schema; the per-language signature/usage pages are
  curated prose+snippets kept accurate by review + the no-stale-surface check. (A full `cargo doc`/TypeDoc
  pipeline is heavier than the roadmap requires and is out — Scope Discipline / C-08.)
- **Narrative pages are hand-written** (overview/getting-started/howto/FAQ) — not auto-generated prose.
- **Docs-site Node floor ≥22.12** (Astro 7) — higher than the library's Node 20+, acceptable because the site is
  a separate build-time package.
- **No versioned/multi-release docs** in v1 (a later concern).
- **Stacked on 008+009** so the documented surface is final (Prompt/origin). Astro 7.0.3 + Starlight 0.41.1
  verified 2026-06-29.

## Dependencies

- **Depends on** 008 (document the final `origin` + `Prompt` surface) and is informed by 002–006 + 009.
- **Should land before/with** spec 007 (docs live at publish).

## Out of Scope

- Auto-generating narrative prose; a docs framework beyond Starlight; versioned/multi-release docs.
- Any library/kernel/schema behavior change (doc-only).
- Hosting-platform bikeshedding (GitHub Pages assumed).
- A heavyweight per-language auto-doc-gen toolchain (cargo doc / TypeDoc / sphinx) as the API-ref source.
