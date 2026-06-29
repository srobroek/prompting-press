# Memory Synthesis — 010 Documentation site (Astro/Starlight)

_(Direct-read fallback: no SQLite/MCP. Stacked on 009 → docs reflect the FINAL Prompt + origin surface.)_

## Current Scope

A published end-user docs site (Astro + Starlight): overview, getting-started, how-to, FAQ, a per-language API
reference (Rust/Python/TS) derived from the codebase + JSON Schema, and the template-feature-set page
(MiniJinja; interpolation/conditionals/loops IN, includes/macros/inheritance OUT). Slim the repo README to
overview + getting-started + a pointer. CI build/deploy (GitHub Pages assumed). Doc-only — NO library behavior
change.

## Active Constraints (governance)

- **C-07 / Principle VII** — the JSON Schema is the single source the API-ref shape docs derive from (don't
  hand-fork the shape).
- **C-05 / Principle V** — document the git-canonical model + the provenance (template_hash/render_hash) story
  truthfully.
- **Principle III** — document the minimal boundary honestly (no I/O, no LLM, the guard is advisory — mirror
  009's honest framing).
- **No floating versions** — Astro 7.0.3, @astrojs/starlight 0.41.1 pinned EXACT (verified 2026-06-29). The
  docs-site package is a SEPARATE dev/build tool — its deps do NOT enter the published library packages.
- **C-08** — don't build a docs *framework* beyond Starlight; no versioned/multi-release docs (later concern).

## Key facts to document (must be ACCURATE to the post-008/009 code)

- **The surface is the `Prompt` object** (008): construct (`new Prompt`/`Prompt(...)`/`Prompt::new`) +
  `fromYaml`/`fromJson`/`fromToml`; `render`/`getSource`/`check`/`with`; immutable; NO Registry.
- **`origin`** is the per-variable trust tag (renamed from provenance in 008); render-result provenance
  (template_hash/render_hash) keeps its name — document BOTH without conflating (the exact trap the rename
  created).
- **`validation_required`** per-variable; validators bound at construction; asymmetric enforcement (TS/Python
  throw, Rust compile-time) — constitution v1.2.0.
- **`variables[].type`** is carried-metadata-for-codegen, NOT runtime-enforced; the variant/default
  shared-variables guarantee.
- The guard is advisory (C-09), not a sanitizer — document honestly (009's framing).

## Decisions to make at specify (roadmap defaults; log them)

- **API-ref: hand-curated + schema-derived, NOT fully auto-generated** (the roadmap says narrative is
  hand-written; the API ref is "generated/derived" — lean: schema-driven shape tables + per-language signature
  pages, curated, not a heavyweight doc-gen toolchain — C-08).
- **Hosting: GitHub Pages** (roadmap default; no bikeshedding).
- **Node floor for the docs package: ≥22.12** (Astro 7 requires it) — HIGHER than the library's Node 20; fine
  because the docs site is a separate build-time package, not shipped. Flag it.

## Conflict Warnings

- No hard conflict. Watch: docs MUST describe the post-reshape surface (Prompt/origin), not the old
  Registry/provenance — same mid-amendment-sweep discipline as 008's README work. Since 010 is stacked on
  008+009, the code it documents is the final shape.

## Retrieval Notes

Governance read direct (constitution v1.2.0, roadmap 010, C-05/C-07/C-08). Astro/Starlight versions verified
main-thread (not a subagent — fabrication risk). Branched on 009.
