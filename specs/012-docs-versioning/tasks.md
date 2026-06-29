---
description: "Task list for spec 012 — native docs versioning, snapshot-per-released-minor"
---

# Tasks: Native docs versioning, snapshot-per-released-minor

**Input**: Design documents from `specs/012-docs-versioning/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/{version-manifest,snapshot-task}.md, quickstart.md, memory-synthesis.md

**Organization**: Phased by the plan's four work units — (1) versioned content structure + dynamic route + manifest, (2) the version dropdown, (3) the `docs:snapshot` moon task, (4) the release-trigger wiring — plus the `next`/main refinement (#26) and the spec-011 adapter seam.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: parallelizable (different files, no incomplete-dep)
- **[Story]**: US1 (reader version-switching), US2 (auto-snapshot-on-release), US3 (local runnability)

## Implementation-open items (resolve during execution)

- **IO-1 — `next`/main version** (#26): model `main` as a `next` docs version (auto-deployed by docs.yml), with `latest` = highest released minor as canonical/default. Pre-1.0: `next` is the effective default until the first release. Fold into the manifest shape + dropdown + route.
- **IO-2 — spec-011 adapter seam** (research R7): the snapshot calls 011's `gen-api-refs.mjs --version/--out` for per-version API refs. 011 is built but not yet merged to main. Behind an adapter: use it if present, else copy current reference pages + log a pending-011 warning. 012 must not hard-block on 011.
- **IO-3 — node/pnpm skew** (research R5): docs.yml runs node 22.12.0/pnpm 10.11.0; mise pins node 25.3.0. Snapshot + route code must use stable Node APIs runnable under both.

---

## Phase 1: Setup & Foundations (Blocking Prerequisites)

**Purpose**: the versioned content structure + manifest schema + the version helper — everything else builds on these.

- [ ] T001 Create the versioned-content directory convention `docs/site/src/versions/` (frozen per-minor trees, e.g. `versions/v1.1/…`) with a `.gitkeep` + a README explaining it is snapshot-owned (never hand-edited). Latest stays in `src/content/docs/`.
- [ ] T002 Define the version-dropdown manifest `docs/site/src/data/versions.json` per contracts/version-manifest.md: `{ latest, versions: [{ minor, lastPatch, isLatest, label, slugs[] }] }`, canonical sorted-key JSON. Seed it pre-1.0 with a single `next` entry (IO-1) representing main (no released minor yet). **GATE**: valid JSON matching the contract; `next` present.
- [ ] T003 `docs/site/scripts/lib/version.mjs` — semver helpers: parse `X.Y.Z`, derive the `X.Y` bucket, `bucketAction(newVer, prevVer)` → `'new-bucket' | 'overwrite'` (major.minor changed ⇒ new; patch-only ⇒ overwrite; unparseable/missing-prev ⇒ throw, FR-013/FR-016), and canonical manifest read/write. Stable Node APIs only (IO-3). **GATE**: unit-style checks: `bucketAction('1.1.0','1.0.3')='new-bucket'`, `('1.1.1','1.1.0')='overwrite'`, `('garbage','1.1.0')` throws.

**Checkpoint**: content structure + manifest + version helper exist; the route/dropdown/snapshot can build on them.

---

## Phase 2 — User Story 1 (P1): Reader views + switches doc versions

**Goal**: a multi-version site with a working version dropdown; latest at the bare path, pinned under `/v/X.Y/`. MVP.

**Independent test**: with ≥2 versioned trees + `next`, build the site; confirm the dropdown lists them, switching serves the right version, latest is canonical, missing pages degrade gracefully.

- [ ] T004 [US1] Dynamic Astro route `docs/site/src/pages/v/[version]/[...slug].astro` — `getStaticPaths` enumerates `(version × slug)` from the manifest + the `versions/<vX.Y>/` trees; serves pinned content under `/v/X.Y/…`. Latest stays on the existing unprefixed routes (FR-004). **GATE**: build emits `/v/<ver>/…` pages for each non-latest version.
- [ ] T005 [US1] Graceful page-missing degradation (FR-003): when a slug doesn't exist in a target version, the route/selector lands on that version's nearest valid page or its index — never a broken link. **GATE**: switching to a version lacking a page does not 404.
- [ ] T006 [US1] Non-latest `noindex` (FR-017): pinned `/v/X.Y/` pages (not latest) emit `<meta name="robots" content="noindex">`. **GATE**: built non-latest pages carry noindex; latest does not.
- [ ] T007 [US1] Canonical/default (FR-004): the bare/unprefixed docs path serves `latest`; confirm `next` and pinned versions are reachable only under their prefixes. **GATE**: bare path = latest content.

**Checkpoint (US1)**: the multi-version site serves + routes correctly. (Dropdown component in Phase 3 — the route works without it for testing.)

---

## Phase 3 — User Story 1 cont. (P1): the version dropdown

- [ ] T008 [US1] `docs/site/src/components/VersionSelect.astro` — renders the dropdown from `versions.json` (newest-first; current highlighted; `next` labeled distinctly per IO-1); each entry links to the equivalent slug in that version (or its index — degradation per T005). **GATE**: dropdown lists all manifest versions incl. `next`; selecting navigates correctly.
- [ ] T009 [US1] Wire `VersionSelect` into the Starlight nav (a header/sidebar component override per Starlight's component slots). **GATE**: the selector appears on every page.
- [ ] T010 [US1] Freshness footer (FR-018): each version's pages show "docs current as of `X.Y.Z`" from the manifest's `lastPatch`. **GATE**: footer reflects the bucket's lastPatch.

**Checkpoint (US1 complete / MVP)**: a reader can switch versions via the nav dropdown, land on the right page, see freshness. SC-001/002 satisfied.

---

## Phase 4 — User Story 3 (P3): the `docs:snapshot` moon task (local runnability)

**Goal**: `moon run docs:snapshot -- --version <x.y.z>` freezes current docs → a versioned tree + updates the manifest; idempotent; runnable locally.

**Independent test**: run it locally for a fake 1.0.0/1.1.0/1.1.1; verify bucket-create vs overwrite (rollup), manifest update, idempotence (twice-run zero diff).

- [ ] T011 [US3] Introduce the docs moon project `docs/site/moon.yml` (first docs moon project) declaring the `docs:snapshot` task (cacheable; locally runnable). Justified per research R3 (snapshot logic = moon task, not inline CI). **GATE**: `moon run docs:snapshot` is invokable.
- [ ] T012 [US3] `docs/site/scripts/snapshot-docs.mjs` (the task body): given `--version X.Y.Z`, (a) call spec-011's `gen-api-refs.mjs --version X.Y --out <staging>` behind the adapter (IO-2; fallback + warn if 011 absent); (b) freeze current docs → `src/versions/vX.Y/` (create if new minor, overwrite if patch rollup — uses version.mjs); (c) update `versions.json` (add minor / set latest / refresh lastPatch + slugs); (d) stamp the freshness footer source. Canonical JSON ⇒ idempotent. **GATE**: produces the tree + manifest update for a given version.
- [ ] T013 [US3] Idempotence (FR-008/SC-004): re-running `docs:snapshot` for the same `X.Y.Z` yields zero diff. **GATE**: twice-run `git diff --stat` empty.

**Checkpoint (US3)**: snapshot works locally + idempotently.

---

## Phase 5 — User Story 2 (P2): release-trigger wiring + `next` deploy

**Goal**: a release-please release auto-snapshots (minor=new bucket, patch=rollup); main auto-deploys as `next`.

**Independent test**: simulate a minor + a patch release event; confirm new-bucket vs overwrite; confirm patches add no dropdown entry.

- [ ] T014 [US2] Extend `.github/workflows/release.yml`: after the release-please step yields a version/tag, compute `bucketAction(new, prev-tag)` via version.mjs (FR-016), run `moon run docs:snapshot -- --version <new>`, then trigger the docs deploy. Fail loudly on unparseable/ambiguous (FR-013). Publish stays gated (FR-014). **GATE**: a simulated minor release runs the snapshot + deploy; a patch overwrites; bad version fails.
- [ ] T015 [US2] `next`/main deploy (IO-1 / #26): `docs.yml` (push to main) publishes the current docs into the `next` slot of the multi-version site (the `next` manifest entry). No new CI — docs.yml IS the nightly. **GATE**: a push to main updates `next`; released minors remain frozen.
- [ ] T016 [US2] The multi-version site is published by the existing GitHub Pages deploy (FR-011) — confirm the build emits all versions (latest + next + pinned) into the Pages artifact. **GATE**: deployed artifact contains all version trees.

**Checkpoint (US2)**: releases drive snapshots automatically; main is `next`.

---

## Phase 6: Polish & Cross-cutting

- [ ] T017 [P] Principle V guard: confirm NO library version axis leaked — versioning lives only in docs/site (manifest + route), driven by git/release-please; `ci:check-ffi` green; no `version=` render API. **GATE**: inspection + ci:check-ffi.
- [ ] T018 [P] No publishing enabled (FR-014/SC-007): the snapshot/deploy path runs no cargo/maturin/pnpm publish. **GATE**: grep release.yml — publish job still `if: false`.
- [ ] T019 Quickstart validation: run the full quickstart.md locally (snapshot 1.0/1.1/1.1.1, build multi-version site, verify route/dropdown/noindex/footer/idempotence). **GATE**: all quickstart checks pass.

---

## Dependencies & order

- **Phase 1 blocks all** (structure + manifest + version helper).
- US1 route (Phase 2) + dropdown (Phase 3) need the manifest; the route works for testing before the dropdown.
- The snapshot task (Phase 4) needs version.mjs + manifest; it's what populates versions for the route/dropdown to show.
- Release wiring (Phase 5) needs the snapshot task.
- Phase 6 is final verification.

## MVP scope

**Phase 1 + 2 + 3 (US1)** = a working multi-version site with a dropdown (versions can be hand-seeded for testing). The snapshot task (US3) + release wiring (US2) automate population. Pre-1.0, `next` is the only live version until the first release.

## Parallel opportunities

- T017 + T018 (independent verification checks).
- Within Phase 2, T005/T006/T007 are largely independent of each other once T004's route exists.

## Note on testability pre-publish

The release-trigger half (Phase 5) cannot be END-TO-END verified until a real release exists (post-publish). This feature BUILDS + locally-validates the mechanism (snapshot task, route, dropdown, manifest); the first real snapshot fires at the first released minor.
