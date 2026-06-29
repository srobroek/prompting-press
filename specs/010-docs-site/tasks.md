---
description: "Task list for spec 010 — Documentation site (Astro/Starlight)"
---

# Tasks: Documentation site (Astro/Starlight)

**Input**: `specs/010-docs-site/`  **Prerequisites**: plan.md, spec.md

**Tests**: doc-only; "gates" are the site build, a schema↔shape match check, and a no-stale-surface grep. No
library behavior change.

## Format: `[ID] [P?] Description`

## Phase 1: Scaffold

- [X] T001 Create the `docs/site/` Astro+Starlight package: `package.json` (`private: true`; exact pins `astro` `7.0.3`, `@astrojs/starlight` `0.41.1`; NOT published), `astro.config.mjs` (Starlight integration, site title, sidebar, GitHub-Pages `base`/`site`), and a minimal `src/content/docs/index.mdx`. Node ≥22.12 noted in `package.json` engines. (FR-001)
- [X] T002 Register the docs package in `.moon/workspace.yml` (explicit entry, no glob) + a `docs/site/moon.yml` with a `build` task (`astro build`); confirm it does NOT inherit the cargo build/test tasks. (FR-001, FR-010)
- [X] T003 GATE: `pnpm -C docs/site install && pnpm -C docs/site build` produces a site with zero errors. Confirm the docs deps appear in NO published package manifest (SC-007). (FR-001; SC-007)

## Phase 2: Schema-derived shape doc

- [ ] T004 Add a small build step (Astro content or a prebuild script) that reads `schemas/jsonschema/prompt-definition.schema.json` and renders the prompt-definition **shape table** (fields incl. `origin`, `validation_required`, `type`, variants, metadata) into `src/content/docs/reference/prompt-definition.mdx`. Derive from the schema; do not hand-fork. (FR-005; SC-004)
- [ ] T005 GATE: the rendered shape table matches the schema field-for-field (a check script or a documented diff). (SC-004)

## Phase 3: Narrative content (hand-written, current surface)

- [ ] T006 [P] `index.mdx` overview: purpose, typed-input + provenance value, explicit non-goals (no I/O, no LLM, advisory guard). (FR-002; SC-001)
- [ ] T007 [P] `getting-started/{rust,python,typescript}.mdx`: install + first `Prompt` construction + render per language, current surface (no `Registry`). (FR-003; SC-002)
- [ ] T008 [P] `templates.mdx`: supported (interpolation/conditionals/loops) vs excluded (include/import/extends/macros/inheritance) + the agreement-check rationale. (FR-006; SC-005)
- [ ] T009 [P] `guides/{lint-in-ci,variants-with,composition,guard}.mdx` + `faq.mdx`: how-tos on the current `check`/`with(_)`/`Composition`/guard surface; FAQ covers the guard-is-advisory + `variables[].type`-is-metadata points. (FR-007, FR-008)

## Phase 4: API reference (per language)

- [ ] T010 [P] `reference/rust.mdx`: `Prompt::new`/`from_*`/`render::<V>`/`get_source`/`check`/`with` + `ConsumerError`; Rust `Result` idiom; `with` carries no runtime validator (compile-time). (FR-004; SC-003)
- [ ] T011 [P] `reference/python.mdx`: `Prompt(...)`/`from_*`/`render`/`get_source`/`check`/`with_` + the exception hierarchy; Pydantic validators; raise-on-invalid. (FR-004; SC-003)
- [ ] T012 [P] `reference/typescript.mdx`: `new Prompt`/`from*`/`render`/`getSource`/`check`/`with` + `PromptingPressError` subclasses; Zod validators; throw-on-invalid. (FR-004; SC-003)
- [ ] T013 Document the provenance model on the reference/overview: render-result `template_hash`/`render_hash` (C-05) **distinct** from the per-variable `origin` tag; no conflation. (FR-008; SC-008)

## Phase 5: README slim-down + CI deploy

- [ ] T014 Slim the repo `README.md` to overview + a short getting-started + a pointer to the site; remove any registry-era examples. Keep the type-safety half prominent (brief R6). (FR-009; SC-006)
- [ ] T015 Add `.github/workflows/docs.yml`: build the site from pinned deps + deploy to GitHub Pages (build on PR, deploy on main). Deterministic build. (FR-010; SC-001)

## Phase 6: Stale-surface + link check

- [ ] T016 Add/run a check that NO doc page references the removed surface (`Registry`, per-variable `provenance`, `render(reg`) and that internal links resolve. (FR-012; SC-001, SC-006)
- [ ] T017 GATE: `pnpm -C docs/site build` green; the stale-surface check passes; the shape table matches the schema; the existing library CI gates remain untouched/green. (FR-010, FR-011; SC-001)

## Dependencies & ordering

- Phase 1 → 2 → {3, 4 in parallel} → 5 → 6. Within 3 and 4 the `[P]` pages are independent files.
- Doc-only: NO crates/ or packages/ library-source changes (the site only reads the schema + mirrors the API).

## Implementation strategy

Scaffold + schema-step + CI (Phases 1/2/5) are mechanical — main-thread or one coder agent. The content
(Phases 3/4) is the bulk and is parallelizable per-page; can delegate to coder agents BUT every API/shape claim
must be verified against the actual code/schema (the docs-accuracy risk + the fabrication lesson). Re-verify the
stale-surface check (T016) main-thread.
