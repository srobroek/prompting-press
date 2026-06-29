# Implementation Plan: Documentation site (Astro/Starlight)

**Branch**: `010-docs-site` (stacked on `009-adversarial-fuzzing` → on `008`) | **Date**: 2026-06-29 | **Spec**: [spec.md](./spec.md)

**Input**: `specs/010-docs-site/spec.md`

## Summary

A doc-only deliverable: a new `docs/` Astro+Starlight site documenting the final post-008/009 surface (the
immutable `Prompt` object, `origin`, `validation_required`, the advisory guard, the supported template feature
set), a per-language API reference with the prompt-definition shape derived from the JSON Schema, how-to/FAQ
pages, a slimmed README pointing at the site, and a CI build + GitHub Pages deploy. No library behavior change.

## Technical Context

**Stack**: Astro 7.0.3 + @astrojs/starlight 0.41.1 (pinned exact), in a NEW separate package (proposed
`docs/site/` or a `website/` package). **Node floor ≥22.12** for this package only (Astro 7). pnpm workspace.
**Content**: MDX/Markdown (hand-written narrative; schema-derived shape tables). **Source of truth for the
shape docs**: `schemas/jsonschema/prompt-definition.schema.json` (C-07). **Deploy**: GitHub Pages via a CI
workflow. **No library code touched** (the site only *reads* the schema + mirrors the public API in prose).

## Constitution Check

*v1.2.0. Doc-only.*

| Principle | Verdict |
|-----------|---------|
| I. Shared Core | ✅ no kernel/binding change |
| III. Minimal Boundary | ✅ the site documents the boundary; adds none |
| V. Repo Canonical | ✅ documents the git-canonical + provenance-hash model (C-05) truthfully |
| VII. JSON Schema source | ✅ shape docs derive from the schema (C-07), not a hand-fork |
| C-08 Scope Discipline | ✅ Starlight only; no docs framework; no versioned docs; curated API-ref, not a heavyweight auto-doc pipeline |
| C-09 | ✅ guard documented as advisory, not enforcement |
| SEC-003 floating versions | ✅ Astro/Starlight pinned exact; docs-site deps isolated from published packages |

**No violations.**

## Project structure (new — all under a separate docs package)

```text
docs/site/                      # NEW Astro+Starlight package (name TBD at impl: docs/site or website/)
├── package.json                # astro 7.0.3, @astrojs/starlight 0.41.1 (exact); private, not published
├── astro.config.mjs            # Starlight integration + sidebar + base path for Pages
├── src/content/docs/
│   ├── index.mdx               # overview (FR-002)
│   ├── getting-started/{rust,python,typescript}.mdx   # FR-003
│   ├── guides/{lint-in-ci,variants-with,composition,guard}.mdx  # FR-007
│   ├── reference/{rust,python,typescript}.mdx          # API ref (FR-004)
│   ├── reference/prompt-definition.mdx                 # schema-derived shape (FR-005)
│   ├── templates.mdx           # supported feature set (FR-006)
│   └── faq.mdx                 # FR-007
└── (a small build step that reads the JSON Schema → a shape table partial — FR-005)

.github/workflows/docs.yml      # build + GitHub Pages deploy (FR-010)
README.md                       # slimmed → overview + getting-started + pointer (FR-009)
.moon/workspace.yml + docs moon.yml  # register the docs package + a build task (optional CI wiring)
```

**Structure decision**: a self-contained `docs/site/` pnpm package, explicitly added to the moon project map
(no globs), mirroring how `packages/typescript` is registered. Its deps never enter the library packages
(SC-007).

## Implementation phasing (informs tasks)

1. **Scaffold** — the Astro+Starlight package (pinned deps), `astro.config.mjs` (sidebar + Pages base path),
   register in the moon map; `pnpm build` produces a site. Gate: site builds.
2. **Schema-derived shape doc** — a small build step reads `prompt-definition.schema.json` → a shape table
   partial used by the prompt-definition reference page (C-07; FR-005). Gate: table matches the schema.
3. **Narrative content** — overview, getting-started ×3, template-features, how-to ×4, FAQ (hand-written,
   current surface). Gate: content review; no stale `Registry`/`provenance`.
4. **API reference ×3** — per-language `Prompt` surface + error types + the shape page. Gate: covers the full
   public surface; matches code.
5. **README slim-down + CI deploy** — shrink README to a pointer; add `.github/workflows/docs.yml` (build +
   Pages). Gate: README has no registry-era examples; the workflow builds the site.
6. **Stale-surface check** — a link/grep check that no doc references `Registry` or the per-variable
   `provenance`; internal links resolve. Gate: SC-001/SC-006/SC-012.

## Complexity Tracking

*No violations — empty.*
