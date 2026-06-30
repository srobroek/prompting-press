# Phase 0 Research: Native docs versioning, snapshot-per-released-minor

Resolves the Technical-Context unknowns + the six addressed points. Architecture (moon-task snapshot,
release-please oracle, CI glue, minor=new/patch=rollup) is pre-decided; research fills the mechanics.

## R1 — Versioned content structure + dynamic route

- **Decision**: Latest docs stay where they are (`src/content/docs/…`, unprefixed routes — the canonical
  URLs). Frozen versions live in `src/versions/<vX.Y>/…` (committed subtrees), served by a dynamic Astro
  route `src/pages/v/[version]/[...slug].astro` that reads the matching versioned tree and renders it under
  `/v/<vX.Y>/…`. `getStaticPaths` enumerates `(version × slug)` from the manifest + the per-version trees,
  so the multi-version site is fully static (no SSR), matching the current static build + Pages deploy.
- **Rationale**: keeps the high-traffic latest at clean canonical paths (FR-004/FR-017) and isolates frozen
  content under one prefix; static generation preserves the existing deploy model.
- **Alternatives**: build N independent Starlight sites per version (rejected — duplicates config, breaks one
  shared nav/dropdown); the `starlight-versions` plugin (rejected in clarify — early-development).
- **Graceful degradation (FR-003)**: the `VersionSelect` component, when switching to version V, links to the
  same slug under V if it exists in V's tree, else falls back to V's index. The dynamic route 404s only for a
  truly absent `(version, slug)`; the selector avoids generating such links by checking the manifest's
  per-version slug set.

## R2 — Version-dropdown manifest

- **Decision**: a committed `src/data/versions.json` (the manifest) — see
  [contracts/version-manifest.md](./contracts/version-manifest.md). Lists each minor version, marks `latest`,
  and records each version's last-snapshotted exact patch (for the freshness footer) + its available slugs
  (for degradation). Both `VersionSelect.astro` and the dynamic route's `getStaticPaths` read it; it is the
  single source for "what versions exist."
- **Rationale**: one declarative file drives selector + routing + footer; updated only by `docs:snapshot`
  (never hand-edited), so it stays consistent.

## R3 — docs:snapshot as a moon task (resolve "docs is not a moon project")

- **Decision**: introduce a **docs moon project** (`docs/site/moon.yml`) that declares `docs:snapshot`. The
  snapshot LOGIC lives in `scripts/snapshot-docs.mjs`; the moon task wraps it (cacheable, `moon run
  docs:snapshot -- --version 1.2`). Other docs build steps (gen-shape-table, spec-011 gen-api-refs) stay as
  the Astro `prebuild` for the latest build and are NOT moved into moon by this feature.
- **Rationale**: the decided architecture says "snapshot = moon task"; the docs site lacking a moon project
  today is exactly the gap to close. A docs moon project also gives future docs tasks a home. CI invokes the
  task rather than embedding snapshot logic in YAML.
- **Alternative**: a bare script invoked directly by CI (rejected — violates the decided "moon task, not
  inline CI" layering and loses local runnability/caching parity with the repo's other generators).

## R4 — docs:snapshot behavior (minor=new bucket, patch=rollup) + idempotence

- **Decision**: `docs:snapshot --version <x.y.z>` (full released version in; the task derives the `x.y` bucket):
  1. Run spec-011's `gen-api-refs.mjs --version <x.y> --out <tmp>` so the snapshot captures **per-version**
     API refs (FR-012), plus the current shape page + guides.
  2. Freeze the assembled current docs into `src/versions/v<x.y>/` — **create** if the minor is new, **overwrite**
     if it already exists (patch rollup).
  3. Update `versions.json`: add the minor if new (and set it `latest` when it is the highest), always update
     that bucket's `lastPatch = x.y.z` (drives the freshness footer, FR-018).
  4. Re-point the unprefixed/latest content at the newest minor's frozen tree (latest mirrors the top bucket).
- **Idempotence (FR-008/SC-004)**: deterministic file copy + canonical JSON (sorted keys) ⇒ a second run for
  the same `x.y.z` yields zero diff. Validated twice-run, mirroring `codegen-check.sh`.
- **Local runnability**: the task runs with no CI context (`moon run docs:snapshot -- --version 1.2.0`).

## R5 — Node/pnpm toolchain skew

- **Decision**: align the snapshot task + dynamic route to run under BOTH the deploy toolchain (docs.yml:
  node 22.12.0 / pnpm 10.11.0) and local mise (node 25.3.0). Use only stable Node APIs (fs/path, no
  version-specific features); pin pnpm in the docs project consistently. Prefer bumping `docs.yml` to align
  with mise's node where safe, OR explicitly document the dual-runtime support — decide during implementation
  with a quick compatibility check, but the task code MUST avoid node-25-only APIs so it is portable.
- **Rationale**: the snapshot runs in CI (deploy toolchain) and locally (mise); divergent runtimes are the
  soft-watch from memory-synthesis. Keeping the code to stable APIs removes the risk regardless of which
  alignment is chosen.

## R6 — Minor-vs-patch trigger contract (FR-016 / FR-013)

- **Decision**: the release workflow determines new-bucket-vs-overwrite by **comparing the released version
  to the previous release's version**: if `major.minor` changed ⇒ new bucket; if only `patch` changed ⇒
  overwrite the current minor bucket. It does NOT rely on release-please exposing a bespoke "minor bumped"
  signal (more portable, and release-please's outputs include the new tag/version which is sufficient).
- **Loud-fail (FR-013)**: if the released version is unparseable as `major.minor.patch`, or there is no
  previous release to compare against in an ambiguous case, the workflow FAILS with an actionable message —
  never silently snapshots the wrong bucket or skips. (First-ever release: no previous tag ⇒ treated as a new
  bucket for that minor, which is correct, not an error.)
- **Where gated**: in `release.yml`, after the release-please step produces the release/tag, a small step
  computes the bucket decision (via `scripts/lib/version.mjs`) and invokes `moon run docs:snapshot`. The
  existing gated-off `publish` job is untouched; the SHA-pin TODO on the release-please action still applies
  (carry it forward).

## R7 — Spec 011 dependency (does 012 block on 011?)

- **Decision**: **012 does not hard-block on 011, but coordinates via a stable seam.** `snapshot-docs.mjs`
  calls spec-011's `gen-api-refs.mjs --version/--out`; if 011 has not landed yet, the call is behind a small
  adapter that (a) uses 011's generator when present, else (b) falls back to copying the current
  (hand-or-shape-generated) reference pages as-is, with a logged warning that per-version API-ref regeneration
  is pending 011. This lets 012's structure/route/dropdown/snapshot mechanics be built + tested independently,
  while FR-012's full per-version API-ref capture lights up automatically once 011 ships.
- **Rationale**: 011 is specced + planned but not implemented; hard-blocking 012 on it would stall both. The
  adapter keeps the integration point explicit and small. Recommended implementation order: land 011 first
  (it's independently testable now), then 012 — but the adapter means 012 is not strictly gated.
- **Alternative**: hard-block 012 until 011 merges (rejected — unnecessary coupling; the seam is cheap).

## R8 — SEO / canonical (FR-017)

- **Decision**: latest pages (unprefixed) carry the canonical link; pinned `/v/<vX.Y>/` pages that are not
  latest get `<meta name="robots" content="noindex">` (or the Starlight head-injection equivalent) so search
  engines don't surface/duplicate stale versions. Latest is always the indexed, canonical surface.
- **Rationale**: standard versioned-docs SEO hygiene (matches Docusaurus default behavior) without leaving
  Starlight.

## Decisions summary

| # | Decision | Rationale |
|---|----------|-----------|
| R1 | latest unprefixed + `/v/<vX.Y>/` dynamic static route; versions in `src/versions/` | clean canonical latest, isolated frozen trees, static deploy preserved |
| R2 | committed `versions.json` manifest drives selector + route + footer | one declarative source, snapshot-owned |
| R3 | new docs moon project hosting `docs:snapshot` | honors "snapshot = moon task" + local runnability |
| R4 | minor=create bucket, patch=overwrite; canonical JSON ⇒ idempotent | matches clarify; twice-run zero-diff |
| R5 | snapshot/route use stable Node APIs; align/ document the 22↔25 skew | portable across deploy + local toolchains |
| R6 | compute minor-vs-patch from prev-tag comparison; loud-fail on ambiguity | portable trigger; no silent wrong snapshot |
| R7 | adapter seam to spec-011 generator; soft-coordinate, not hard-block | both specs progress independently |
| R8 | non-latest noindex; latest canonical | versioned-docs SEO hygiene |
