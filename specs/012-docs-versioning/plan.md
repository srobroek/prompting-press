# Implementation Plan: Native docs versioning, snapshot-per-released-minor

**Branch**: `012-docs-versioning` | **Date**: 2026-06-29 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/012-docs-versioning/spec.md`

## Summary

Add per-version documentation to the existing Astro/Starlight site using native content collections +
a dynamic `[version]/[...slug]` route + a custom version-dropdown component (no platform migration, no
third-party plugin). A moon task `docs:snapshot --version <x.y>` freezes the current docs (including
spec-011's per-version API references) into a versioned tree and updates a dropdown manifest; it is
idempotent (twice-run zero-diff). release-please remains the version oracle; the release workflow is
the trigger glue — **every** release snapshots, with **minor** creating a new bucket + dropdown entry
and **patch** overwriting (rolling up) its minor bucket. The dropdown lists minors only; a footer
surfaces the exact patch. Latest serves at the unprefixed/canonical path; pinned versions at `/vX.Y/`
(non-latest noindexed). All minors kept forever. Publishing stays gated off; the GitHub Pages artifact
deploy continues to publish, now multi-version.

## Technical Context

**Language/Version**: Astro/Starlight docs site (`docs/site`); build orchestration in Node. **Toolchain
skew to resolve (R5)**: `docs.yml` deploys with node `22.12.0` / pnpm `10.11.0`, while `mise.toml` pins
node `25.3.0`; the snapshot task + dynamic route MUST run under the deploy toolchain.

**Primary Dependencies**: existing Astro + Starlight (no new versioning plugin). Any small added helper
(manifest read/write, version-route glue) is plain Node/Astro — pinned exactly if introduced. Coordinates
with spec-011's `gen-api-refs.mjs --version/--out` for per-version API refs.

**Storage**: file-based — frozen per-version doc trees under `docs/site` + a version-dropdown manifest
(JSON) committed to the repo; no database.

**Testing**: `docs:snapshot` idempotence via twice-run `git diff --exit-code` (mirrors codegen-check);
Astro build over the multi-version site; route/dropdown validated by building + checking emitted paths
(latest at bare path, pinned at `/vX.Y/`, missing-page degradation).

**Target Platform**: GitHub Pages (the existing `docs.yml` artifact deploy), now publishing all versions.

**Project Type**: documentation-site build + release-automation wiring (no library code touched).

**Performance Goals**: snapshot + build complete within the existing docs CI budget; the static
multi-version site grows linearly per kept minor (acceptable — small static output, all kept).

**Constraints**: deterministic/idempotent snapshot (SC-004); no managed version axis in the library
(Principle V); no publishing enabled (FR-014/SC-007); latest canonical + non-latest noindex (FR-017).

**Scale/Scope**: one new minor bucket per release minor, kept indefinitely; pre-publish, so zero real
snapshots until the first released minor — this feature builds + validates the mechanism.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Principle V (Repo canonical; git owns versioning)** — ✅ **Central and upheld.** The *library* gains
  no managed version axis: there is still no `versions:` store and no `version=` render pin. Only the
  **docs site** is versioned, and its versions are driven by git tags / release-please, not a
  library-internal mechanism. The dynamic route + manifest live entirely in `docs/site`; nothing in the
  kernel/consumer/bindings learns about doc versions. Verified: no FR introduces a library-side version axis.
- **Principle VII (single source of truth; generated artifacts)** — ✅ Consistent. The snapshot freezes
  *already-generated* content (the shape page + spec-011 API refs) deterministically; the snapshot task is
  idempotent and gated like the codegen determinism gate.
- **Principles I–IV, VI** — ✅ N/A / untouched. No library behavior, FFI, boundary, agreement-check, or
  per-language-idiom surface changes; this is docs-site + CI work.
- **Boundary defense / "everything except publish"** — ✅ FR-014 keeps package publishing gated off; this
  feature only snapshots + deploys docs via the existing Pages pipeline.

**Scope Discipline (R1)**: no new *library* pluggable interface. The snapshot task's `--version`/`--out`
mirror spec-011's generator parameter (the coordinating consumer), and the dropdown manifest is a docs-site
data file, not a library seam. **Result: PASS** (no violations; Complexity Tracking empty). Re-checked
post-Phase-1: still PASS.

## Project Structure

### Documentation (this feature)

```text
specs/012-docs-versioning/
├── plan.md              # This file
├── research.md          # Phase 0 output (route model, manifest, trigger contract, skew, 011 dep)
├── data-model.md        # Phase 1 output (manifest + versioned-tree + freshness-footer entities)
├── quickstart.md        # Phase 1 output (run snapshot locally, simulate minor/patch, verify route)
├── contracts/
│   ├── version-manifest.md   # Phase 1 — the dropdown manifest shape + invariants
│   └── snapshot-task.md      # Phase 1 — the docs:snapshot CLI/behavior + trigger contract
└── tasks.md             # Phase 2 output (/speckit-tasks — NOT created here)
```

### Source Code (repository root)

```text
docs/site/
├── src/
│   ├── content/docs/                 # the CURRENT (latest) docs — the snapshot SOURCE
│   ├── versions/                     # NEW — frozen per-version trees (e.g. versions/v1.1/, v1.2/)
│   │   └── <vX.Y>/...                #       one subtree per kept minor (committed)
│   ├── data/versions.json            # NEW — the version-dropdown MANIFEST (versions[] + latest)
│   ├── components/VersionSelect.astro# NEW — the nav dropdown, driven by the manifest
│   └── pages/v/[version]/[...slug].astro  # NEW — dynamic route serving pinned /vX.Y/ content
│                                     #       (latest stays at the existing unprefixed routes)
├── scripts/
│   ├── snapshot-docs.mjs             # NEW — snapshot logic (freeze current → versions/<vX.Y>,
│   │                                 #       update manifest, stamp freshness footer); calls spec-011's
│   │                                 #       gen-api-refs.mjs --version <x.y> --out for per-version API refs
│   └── lib/version.mjs               # NEW — semver parse + minor-vs-patch + manifest read/write helpers
└── moon.yml                          # NEW (first docs moon project) — declares the docs:snapshot task

.github/workflows/release.yml         # EXTEND — on a release event, compute minor-vs-patch vs the previous
                                       #   tag, run moon run docs:snapshot -- --version <x.y>, then deploy
.github/workflows/docs.yml            # keep as the publish mechanism; build now emits the multi-version site
```

**Structure Decision** (resolves the "docs is not a moon project" question, R3): introduce a **docs moon
project** (`docs/site/moon.yml`) declaring `docs:snapshot` (and able to host future docs tasks), so the
snapshot is a first-class, cacheable, locally-runnable moon task per the decided architecture — consistent
with how codegen/schema/conformance are moon tasks. The Astro `prebuild` flow (gen-shape-table +
spec-011's gen-api-refs) is unchanged for the *latest* build; `docs:snapshot` is a separate task that freezes
output, invoked by the release workflow and runnable locally. This keeps "snapshot logic = moon task"
literally true rather than burying it in CI YAML.

## Complexity Tracking

> No constitution violations. (The version-aware coupling to spec 011 is the *reason* 011 was built
> version-aware; it is the realized second consumer, not new speculative generality.)

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|--------------------------------------|
| _(none)_ | — | — |
