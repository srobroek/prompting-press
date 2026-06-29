# Feature Specification: Native docs versioning, snapshot-per-released-minor

**Feature Branch**: `012-docs-versioning`

**Created**: 2026-06-29

**Status**: Draft

**Input**: User description: "012 — Native Starlight docs versioning, snapshot-per-released-minor via release-please. Add per-version documentation to the existing Astro/Starlight docs site without leaving the platform, with a version dropdown, where documentation versions track released minor versions automatically."

## Clarifications

### Session 2026-06-29

- Q: Patch releases — skip snapshot entirely, or snapshot into the minor bucket? → A: **Patches re-snapshot, rolling up into their minor bucket.** Every release (minor or patch) runs the snapshot; a **minor** creates a NEW version bucket (e.g. `1.2`) and a new dropdown entry, while a **patch** (e.g. `1.2.1`) **overwrites** the existing `1.2` bucket. The dropdown lists only minor lines (`1.0`, `1.1`, `1.2`, …). Rationale: SemVer guarantees the documented API surface is identical across patches, so a per-patch bucket adds no reader value — but patches can carry doc fixes (corrected examples, typos), and rolling them into the minor bucket gets those fixes to readers pinned to that line without waiting for the next minor. (Supersedes the original "patch MUST NOT snapshot" wording.)
- Q: How many doc versions stay live; are old ones pruned? → A: **Keep all released minors forever** (no pruning). Docs are small static output; readers pinned to old versions keep their docs. Revisit a rolling window only if the count ever becomes unwieldy.
- Q: URL scheme + canonical/default + old-version SEO? → A: **Unprefixed latest + `/vX.Y/` for pinned.** Latest lives at the bare path (e.g. `/reference/rust/`) for clean canonical URLs; pinned versions live under `/vX.Y/…` (e.g. `/v1.1/reference/rust/`). The bare path always serves latest. Older `/vX.Y/` pages are excluded from search-engine indexing (noindex) to avoid duplicate-content dilution.
- Q: A subtle "docs current as of `X.Y.Z`" footer (showing the exact patch within a minor bucket)? → A: **Yes** — a low-cost footer note surfacing the exact patch the bucket was last snapshotted from, so readers can see freshness even though the dropdown is minor-only.

## Context

The documentation site is currently single-"latest" (one set of pages, no version history). The library now releases all crates plus the Python and TypeScript packages in **lockstep from one version** via release-please (configured, with publishing intentionally gated off pre-publish). This feature adds **per-version documentation** so a reader can view the docs that match a specific released library version, and so each released **minor** version gets a frozen documentation snapshot.

The platform stays **Astro/Starlight** — Astro versions natively via content collections and dynamic routing, so no migration to another docs platform is needed, and no third-party versioning plugin is adopted.

### Architecture (decided — encoded here, not re-opened)

A clean separation of responsibilities, mirroring how the repo already structures build logic:

- **Snapshot logic = a moon task** (e.g. `docs:snapshot`). It is cacheable and **locally runnable** (`moon run docs:snapshot -- --version <x.y>`), the same way codegen / schema-validation / conformance live as moon tasks rather than inline CI YAML. It freezes the current docs content into a versioned tree and updates the version-dropdown manifest. It is idempotent.
- **release-please = the version oracle only.** It decides the new version on a release; it does not perform the snapshot.
- **CI/CD = the trigger glue.** On a release-please release where the **minor** version bumped, the workflow invokes the moon snapshot task, then deploys. **Patch** releases do **not** snapshot.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A reader views the docs for a specific library version (Priority: P1)

A developer pinned to a released library version opens the documentation, selects that version from a dropdown in the site navigation, and reads the documentation as it was at that version — the version they are looking at is unambiguous, and switching versions keeps them on the equivalent page.

**Why this priority**: This is the reader-facing point of the whole feature — without a way to view and switch versions, the snapshots have no value. It is the minimal viable slice: a multi-version site with a working dropdown delivers value even before the release automation is wired.

**Independent Test**: With at least two versioned doc trees present, load the site, confirm a version dropdown lists them, select a non-latest version, and confirm the served pages are that version's content; switch back and confirm the equivalent page loads.

**Acceptance Scenarios**:

1. **Given** the site has multiple documentation versions, **When** a reader opens any page, **Then** a version selector in the navigation shows the current version and lists the others.
2. **Given** a reader is on a page in the latest version, **When** they select an older version from the dropdown, **Then** the equivalent page in that version is served (or a clear fallback if that page did not exist in the older version).
3. **Given** a reader visits the canonical/default docs URL, **When** the page loads, **Then** the latest version is shown by default.

### User Story 2 - A released minor automatically gets a frozen docs snapshot (Priority: P2)

When the library cuts a new **minor** release, the documentation as it stands at that release is frozen into a new versioned tree, the version dropdown gains that minor, and the multi-version site is deployed — with no manual snapshot step. A **patch** release refreshes the current minor's docs in place (rolling up) without adding a new dropdown entry.

**Why this priority**: This is the automation that keeps versioned docs in sync with releases without manual effort. It depends on US1's versioned structure existing.

**Independent Test**: Simulate a minor release (drive the release event / tag) and confirm a new versioned tree + dropdown entry appear and deploy; simulate a patch release and confirm the existing minor bucket is overwritten with no new dropdown entry.

**Acceptance Scenarios**:

1. **Given** a release-please release that bumps the minor version, **When** the release workflow runs, **Then** the snapshot task produces a new frozen versioned tree for that minor and the dropdown manifest gains that minor.
2. **Given** a release that bumps only the patch version, **When** the release workflow runs, **Then** the current minor's bucket is overwritten with the refreshed docs and the dropdown gains **no** new entry (the freshness footer reflects the new patch).
3. **Given** a successful snapshot, **When** the deploy step runs, **Then** the published site serves all versions including the new/refreshed one.

### User Story 3 - A maintainer can snapshot/preview versioned docs locally (Priority: P3)

A maintainer runs the snapshot task locally to preview what a versioned snapshot will contain before any release, and re-running it is safe (idempotent).

**Why this priority**: Local runnability and idempotence make the mechanism testable and debuggable independent of CI, matching the repo's existing moon-task conventions. Subordinate to the reader-facing and automation slices.

**Independent Test**: Run `moon run docs:snapshot -- --version <x.y>` locally, confirm it produces the versioned tree + manifest update, and confirm a second run yields no spurious diff.

**Acceptance Scenarios**:

1. **Given** a clean checkout, **When** the snapshot task is run locally for a given version, **Then** it produces that version's frozen tree and updates the dropdown manifest without requiring CI.
2. **Given** a snapshot already produced for a version, **When** the task is run again for the same version, **Then** the result is unchanged (idempotent — no duplicate or drifting output).

### Edge Cases

- A reader is on a page that **does not exist** in the version they switch to (a page added in a later version) → the selector must degrade gracefully (land on that version's nearest valid page or its index), never a broken link or blank page.
- The snapshot runs but the **per-version API reference generation** (from the coordinating auto-generated-API-reference feature) has not run for that version → the snapshot must capture the correct per-version API reference content, not the latest-only content.
- A release event fires but the version **cannot be parsed** as a clean `major.minor.patch` (or the minor-vs-patch determination is ambiguous) → the workflow must fail loudly rather than silently snapshotting or silently skipping.
- The same minor is **released/tagged twice** (re-run, retry) → the snapshot must not create a duplicate or conflicting version (idempotence at the automation layer too).
- A **patch** lands on a minor line that has been superseded by a later minor (e.g. `1.1.5` released after `1.2.0` exists) → the patch must overwrite the `1.1` bucket (not touch latest), and the freshness footer for `1.1` updates while latest stays `1.2`.
- The docs site is **pre-1.0 / unreleased** (everything at `0.0.0`, no real release yet) → the site must present sensible current docs before any frozen snapshot exists.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The docs site MUST support multiple concurrent documentation versions served from one deployed site, staying on the existing Astro/Starlight platform (no migration to another docs platform, no third-party versioning plugin).
- **FR-002**: A version selector MUST appear in the site navigation on every page, showing the current version and listing the other available versions; selecting one navigates to that version of the docs.
- **FR-003**: Switching versions MUST keep the reader on the equivalent page where it exists, and MUST degrade gracefully (nearest valid page or that version's index) where it does not — never a broken link.
- **FR-004**: The canonical/default docs URL MUST resolve to the latest version.
- **FR-005**: A versioned content structure MUST hold each version's frozen docs separately, and a dynamic route MUST serve the correct version's content by URL.
- **FR-006**: A version-dropdown manifest MUST enumerate the available versions and which is latest; the selector is driven from this manifest.
- **FR-007**: Snapshot logic MUST be a moon task (cacheable, locally runnable as `moon run docs:snapshot -- --version <x.y>`), not inline CI scripting. It freezes the current docs content into a new versioned tree and updates the dropdown manifest.
- **FR-008**: The snapshot task MUST be idempotent: re-running it for an already-snapshotted version produces no duplicate or drifting output.
- **FR-009**: release-please MUST remain solely the version oracle (it decides the new version); it MUST NOT perform the snapshot. (No change to the existing lockstep release-please configuration.)
- **FR-010**: The release workflow MUST invoke the snapshot task on **every** release, then deploy. A **minor** release MUST create a **new** version bucket (e.g. `1.2`) and a new dropdown entry; a **patch** release MUST **overwrite** its existing minor bucket (roll up, e.g. `1.2.1` refreshes the `1.2` bucket) and MUST NOT create a new dropdown entry. The dropdown therefore lists only minor lines.
- **FR-011**: The multi-version site MUST be published by the existing documentation deploy pipeline (the snapshot feeds it; the deploy continues to publish to the same target).
- **FR-012**: The versioned snapshot MUST capture the **per-version** auto-generated API reference content (coordinating with the version-aware API-reference generator), not latest-only content, for the version being frozen.
- **FR-013**: A release event whose version cannot be cleanly parsed, or whose minor-vs-patch status is ambiguous, MUST cause the workflow to fail loudly rather than silently snapshot or silently skip.
- **FR-014**: This feature MUST NOT enable any actual package publishing (publishing stays gated off pre-publish); it builds the docs-versioning mechanism. The first real snapshot occurs at the first released minor.
- **FR-015**: All released minor documentation versions MUST be retained and kept live indefinitely; there is no pruning policy in this feature (a rolling window is explicitly out of scope unless revisited later).
- **FR-016**: The snapshot runs on every release; the minor-vs-patch decision selects **new bucket vs. overwrite existing bucket** (not run-vs-skip). The workflow MUST determine minor-vs-patch by comparing the released version to the previous release's version (the major.minor pair changed ⇒ new bucket; only the patch changed ⇒ overwrite the current minor bucket). If the comparison cannot be made cleanly (unparseable or missing previous version), the workflow MUST fail loudly (FR-013).
- **FR-017**: The latest version MUST be served at the unprefixed/bare path (e.g. `/reference/rust/`) as the canonical URL; pinned versions MUST be served under a version-prefixed path (e.g. `/v1.1/reference/rust/`). The bare path always serves latest. Pinned (`/vX.Y/`) pages that are not latest MUST be excluded from search-engine indexing (noindex) to avoid duplicate-content dilution.
- **FR-018**: Each version's pages MUST display a freshness indicator (e.g. a footer note) naming the exact patch version (`X.Y.Z`) the bucket was last snapshotted from, so readers can see how current a minor-bucketed page is even though the dropdown is minor-only.

### Key Entities

- **Documentation version**: a frozen snapshot of the docs content corresponding to a released minor library version (e.g. `1.2`). Has an identifier, a content tree, and a latest/not-latest status.
- **Version-dropdown manifest**: the enumerated list of available documentation versions (and which is latest) that drives the navigation selector and the dynamic route.
- **Snapshot task**: the moon task that, given a version, freezes the current docs (including the per-version API references) into that version's tree and updates the manifest; idempotent and locally runnable.
- **Release event**: the release-please-produced release/tag that the CI glue inspects to decide whether (minor) and at what version to snapshot.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A reader can switch between any two available documentation versions from the nav selector and land on the equivalent (or gracefully-degraded) page 100% of the time — no broken links on version switch.
- **SC-002**: The canonical docs URL resolves to the latest version on every visit.
- **SC-003**: A minor release produces exactly one new frozen docs version / dropdown entry (no duplicates) with zero manual steps; a patch release produces zero new dropdown entries and instead overwrites its minor bucket in place.
- **SC-004**: The snapshot task is idempotent: a second run for the same version yields a byte-identical result (zero diff).
- **SC-005**: Each frozen version's API reference content matches that version's library API (the per-version generator output is captured, not latest-only content).
- **SC-006**: A malformed or ambiguous release version causes a loud, actionable failure rather than a silent wrong/skipped snapshot.
- **SC-007**: No actual package publishing occurs as a result of this feature (publishing remains gated off).

## Assumptions

- The platform stays Astro/Starlight; native content-collection + dynamic-routing versioning is used rather than the third-party `starlight-versions` plugin (rejected as early-development) or a platform migration (rejected).
- The release-please lockstep configuration already in place is the version source; this feature consumes its release events and does not modify it.
- The version-aware auto-generated API-reference generator (coordinating feature, spec 011) exists or lands alongside this work; its version/output-path parameter is what this feature drives per snapshotted version.
- The existing GitHub Pages deploy pipeline remains the publish mechanism; this feature changes what it publishes (multi-version) but not that it publishes there.
- Doc version **buckets are keyed to minor lines**; the dropdown lists only minors. Every release snapshots, but a patch overwrites its minor bucket (roll up) rather than creating a new one — so patch-level doc fixes ship to that line's readers without a new dropdown entry. This matches the lockstep release model and SemVer's same-API-across-patches guarantee.
- The project is pre-publish: this feature builds and validates the mechanism; the first real frozen snapshot is produced at the first released minor. Before any release, the site presents current docs as the default/latest.
- Strict tooling-version pinning (as used elsewhere in the project) applies to any new build/site tooling introduced.
