# Phase 1 Data Model: Native docs versioning

The "data" is build-time/site artifacts: the version manifest, the frozen per-version trees, and the
freshness footer source. No runtime/persistent data; no library state change (Principle V — the library
gains no version axis).

## Entities

### Version manifest (`src/data/versions.json`)

The enumerated list of documentation versions. Full shape + invariants in
[contracts/version-manifest.md](./contracts/version-manifest.md). Key fields: `latest`, and `versions[]`
each with `minor` (X.Y), `lastPatch` (X.Y.Z), `isLatest`, `label`, `slugs[]`.

- **Owner**: `docs:snapshot` (only writer).
- **Consumers**: `VersionSelect.astro` (dropdown), `pages/v/[version]/[...slug].astro` (`getStaticPaths`),
  the freshness footer partial.
- **Validation**: exactly one `isLatest`; newest-first order; canonical sorted-key JSON (idempotence).

### Frozen version tree (`src/versions/v<X.Y>/…`)

A committed snapshot of the full docs content for one minor line, including that version's API references
(from spec-011's generator) and the shape page. One subtree per kept minor; all kept forever (FR-015).

- **Created** when a new minor is released; **overwritten** when a patch on that minor is released (rollup).
- **Served** by the dynamic route under `/v/<X.Y>/…`; non-latest trees are noindexed (FR-017).

### Latest (canonical) docs (`src/content/docs/…`)

The current/working docs — also the snapshot SOURCE. Served at the unprefixed canonical paths and always
mirrors the highest minor bucket.

### Freshness footer (derived)

A small per-page element rendering the page's version `lastPatch` ("docs current as of X.Y.Z", FR-018).
Reads the manifest; no independent storage.

### Release event (transient input)

The release-please-produced version/tag the trigger inspects. Not stored by this feature; consumed by the
workflow to compute the bucket action (new vs. overwrite) and the `--version` passed to `docs:snapshot`.

## Relationships

```
release-please --(release + version/tag)--> release.yml trigger
     │                                            │ compute minor-vs-patch vs previous tag (loud-fail on ambiguity)
     │                                            ▼
     │                          moon run docs:snapshot -- --version X.Y.Z
     │                                            │ (1) gen-api-refs --version X.Y --out staging   [spec 011 seam]
     │                                            │ (2) freeze current docs → src/versions/vX.Y/  (create | overwrite)
     │                                            │ (3) update src/data/versions.json (manifest)
     │                                            ▼
     └───────────────────────────────> docs.yml build (multi-version) → Pages artifact → deploy

VersionSelect.astro ─reads─> versions.json ─drives─> /v/[version]/[...slug].astro (getStaticPaths)
freshness footer    ─reads─> versions.json (lastPatch)
```

## State transitions

The only stateful element is the manifest + version trees across releases:

```
no versions (pre-1.0)        --first minor (1.0)-->   { latest: 1.0, versions: [1.0] }
{ latest: 1.0 }              --minor (1.1)-------->    { latest: 1.1, versions: [1.1, 1.0] }   (new bucket)
{ latest: 1.1, [1.1,1.0] }   --patch (1.1.1)------->   { latest: 1.1, versions: [1.1*, 1.0] }  (*overwrite 1.1; lastPatch=1.1.1)
{ latest: 1.1 }              --patch (1.0.4)------->   { latest: 1.1, versions: [1.1, 1.0*] }  (*overwrite OLD bucket 1.0; latest unchanged)
```

Each transition is produced solely by `docs:snapshot` and is idempotent (re-applying the same version is a
no-op diff, SC-004).
