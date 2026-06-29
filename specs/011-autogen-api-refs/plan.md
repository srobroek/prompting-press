# Implementation Plan: Auto-generated language API references

**Branch**: `011-autogen-api-refs` | **Date**: 2026-06-29 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/011-autogen-api-refs/spec.md`

## Summary

Replace the three hand-written language API reference pages (`docs/site/src/content/docs/reference/{rust,python,typescript}.mdx`) with pages **generated from each binding's own public doc comments**, mirroring the existing `gen-shape-table.mjs` (JSON Schema → `prompt-definition.mdx`) pattern. Three build-time extractors (pinned-nightly rustdoc JSON, TypeDoc JSON, griffe) each emit a common intermediate JSON shape; one shared renderer turns each into an `.mdx` page (full-autogen: signatures + doc-comment prose, jargon-stripped, MDX-escaped, public-surface-only). A CI freshness gate (twice-run determinism + diff-against-committed) mirrors `schemas:codegen-check`. The generator is version-aware (accepts `--version`/output-path defaulting to `latest`) so spec 012 can drive it per-version. No library behavior change, no public API change, no new runtime dependency.

## Technical Context

**Language/Version**: Build tooling in Node 25.3.0 (the renderer + TypeDoc) and Python 3.14.4 (griffe), orchestrated alongside the existing `docs/site` Astro build. Extraction inputs: Rust `crates/prompting-press` (nightly rustdoc JSON), TypeScript `packages/typescript/src/index.ts` (TypeDoc), Python `packages/python/python/prompting_press` (griffe).

**Primary Dependencies** (dev/build-time ONLY — never shipped; Principle II/III):
- TypeDoc `0.28.19` (native `--json` emit; no markdown plugin — the renderer is owned here).
- griffe `2.1.0` (structured Python API extraction as a serializable object model; chosen over pdoc which is an HTML generator, not an extraction library).
- A pinned **nightly** Rust toolchain for `rustdoc -Z unstable-options --output-format json` (see Dependency Pins for the exact date + the second-toolchain wrinkle).

**Storage**: N/A — pure file generation into `docs/site/src/content/docs/reference/`.

**Testing**: freshness gate (regenerate + `git diff --exit-code` over the three pages, twice-run determinism), mirroring `schemas/scripts/codegen-check.sh`; Astro build must succeed over the generated pages.

**Target Platform**: the documentation site build (CI + local), publishing to GitHub Pages via the existing `docs.yml`.

**Project Type**: documentation build tooling (sibling scripts under `docs/site/scripts/`), not a library crate/package.

**Performance Goals**: generation is a build step; target is "fast enough not to dominate the docs build" — each extractor + render completes in seconds; no hard latency SLA.

**Constraints**: deterministic byte-stable output (SC-003); zero runtime-dependency impact (FR-011/SC-006); public-surface-only (FR-005); no internal-jargon leakage (FR-006/SC-005).

**Scale/Scope**: a small, stable public surface (one `Prompt` type + ~6 operations, result/config/report types, composition, the error hierarchy + code vocabulary) per language — three pages total.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

This feature is **dev/build-time documentation tooling**. Per-principle:

- **Principle I (Shared core, structural parity)** — ✅ Untouched. The extractors READ each binding's public surface; no kernel/consumer/binding behavior changes. The rendered prose is *derived from* the bindings, so cross-language consistency is a generation property, not a re-implementation.
- **Principle II (FFI isolation)** — ✅ Upheld and CI-guarded. TypeDoc/griffe/nightly-rustdoc live ONLY in docs/dev tooling. FR-011 + the existing `ci:check-ffi` keep `pyo3`/`napi`/extractor deps out of `-core` and the consumer.
- **Principle III (Minimal boundary)** — ✅ No I/O/LLM/token/request-body capability added to the library; the extractors run against source at build time, outside the shipped artifacts.
- **Principle IV (Typed input / agreement check)** — ✅ N/A to generation; the reference pages *document* the agreement guarantee but do not alter it. (Care: the prose must not misstate `check()`/loader semantics — guarded by full-autogen sourcing the corrected doc comments.)
- **Principle V (Repo canonical; git owns versioning)** — ✅ Consistent. The generator is version-*aware* (a parameter) but owns no managed version axis; spec 012 drives versions off git/release-please, not a library-internal store.
- **Principle VI (Per-language idiom)** — ✅ Reinforced. Each language's reference is extracted from that language's native doc-comment convention (rustdoc `///`, TSDoc, Python docstrings).
- **Principle VII (JSON Schema single source of truth)** — ✅ **EXTENDED, not violated.** The schema remains the single source for the *shape* page; this feature makes each binding's **source doc comments the single source for that language's API reference**, exactly mirroring schema→shape-page. Generated pages carry the AUTO-GENERATED marker + a freshness gate; never hand-edited.

**Scope Discipline (R1)**: one accepted, documented relaxation — the generator is built **version-aware before a second consumer runs** (normally generality is earned by a real second consumer). Justified because spec 012 is already specced + clarified and consumes the version parameter; recorded in the spec's Clarifications and the Complexity Tracking table below. No new pluggable *library* interface is introduced (the version param is a build-tool argument, not a library seam).

**Result: PASS** (no unjustified violations). Re-checked post-Phase-1: still PASS (the intermediate-contract + renderer design adds no library surface).

## Project Structure

### Documentation (this feature)

```text
specs/011-autogen-api-refs/
├── plan.md              # This file
├── research.md          # Phase 0 output (toolchain choices, pins, the nightly wrinkle)
├── data-model.md        # Phase 1 output (the intermediate API-doc JSON contract)
├── quickstart.md        # Phase 1 output (how to run + verify generation/gate locally)
├── contracts/
│   └── api-doc-ir.md    # Phase 1 output (the extractor→renderer intermediate contract)
└── tasks.md             # Phase 2 output (/speckit-tasks — NOT created here)
```

### Source Code (repository root)

```text
docs/site/
├── scripts/
│   ├── gen-shape-table.mjs        # EXISTING — the reference implementation to MIRROR, not modify
│   ├── lib/
│   │   ├── strip-jargon.mjs       # NEW — extracted shared helper (jargon strip + MDX escape),
│   │   │                          #       reused by gen-shape-table.mjs's logic + the new renderer
│   │   └── render-api-ref.mjs     # NEW — shared renderer: API-doc IR → reference/*.mdx
│   ├── extract-rust-api.mjs       # NEW — drives nightly rustdoc JSON → API-doc IR (rust)
│   ├── extract-ts-api.mjs         # NEW — drives TypeDoc --json → API-doc IR (typescript)
│   ├── extract-python-api.py      # NEW — drives griffe → API-doc IR (python); run via uv
│   ├── gen-api-refs.mjs           # NEW — orchestrator: for each language, extract → render →
│   │                              #       write reference/<lang>.mdx (version-aware: --version, --out)
│   └── check-stale-surface.mjs    # EXISTING — extend (or sibling) to cover the 3 new pages
├── src/content/docs/reference/
│   ├── prompt-definition.mdx      # EXISTING generated (schema) — untouched by this feature
│   ├── rust.mdx                   # BECOMES generated (was hand-written)
│   ├── python.mdx                 # BECOMES generated
│   └── typescript.mdx             # BECOMES generated
└── package.json                   # prebuild: add gen-api-refs.mjs alongside gen-shape-table.mjs

rust-toolchain-nightly.toml        # NEW — pins the nightly used ONLY for rustdoc-JSON extraction
                                   #       (the stable 1.95.0 pin stays primary; see research.md)
mise.toml                          # add the pinned nightly as a second rust toolchain entry
.github/workflows/ci.yml           # add an api-refs freshness gate step (mirrors codegen-check)
```

**Structure Decision**: The docs site is **not** a moon project (it builds via `package.json` scripts + the Astro `prebuild`, and its existing freshness is enforced by `check-stale-surface.mjs` + `schemas:codegen-check` in CI). Therefore this feature adds **sibling scripts under `docs/site/scripts/`** and wires the orchestrator into `prebuild`, rather than creating a moon task — matching how the shape page is already generated. The freshness gate is a CI step that runs the orchestrator and diffs, mirroring `schemas/scripts/codegen-check.sh`.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|--------------------------------------|
| Version-aware generator before a second consumer exists (mild Scope-Discipline relaxation) | Spec 012 (docs versioning) is already specced + clarified and will call the generator per-version; designing the `--version`/`--out` parameter now avoids a guaranteed near-term refactor | A "latest-only" generator would have to be reworked the moment 012 lands; the parameter is a single argument with a `latest` default, not a speculative pluggable seam |
| A second (nightly) Rust toolchain alongside the pinned stable 1.95.0 | rustdoc JSON (`--output-format json`) is nightly-only; the stable pin exists for rustfmt byte-stability and cannot be swapped to nightly without breaking the codegen-determinism gate | Stable-channel Rust API extraction (syn-based) was considered and rejected in clarify (loses rustdoc's resolved doc-comment + signature structure, needs a custom parser to reach parity); a pinned nightly used ONLY for extraction contains the unstable-format risk |
