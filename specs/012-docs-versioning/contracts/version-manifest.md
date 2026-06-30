# Contract: Version-dropdown manifest (`versions.json`)

The single committed data file that enumerates documentation versions. Written ONLY by `docs:snapshot`
(never hand-edited); read by the `VersionSelect` component and the dynamic route's `getStaticPaths`.

## Location

`docs/site/src/data/versions.json`

## Shape

```jsonc
{
  "latest": "1.2",                 // the minor that the unprefixed/canonical paths serve
  "versions": [                    // ordered newest-first; the dropdown renders in this order
    {
      "minor": "1.2",              // the minor bucket key (drives /v/1.2/... and the versions/v1.2/ tree)
      "lastPatch": "1.2.1",        // exact patch this bucket was last snapshotted from → freshness footer
      "isLatest": true,            // exactly one entry is true; mirrors top-level "latest"
      "label": "1.2 (latest)",     // display string for the dropdown; latest gets the "(latest)" suffix
      "slugs": ["", "getting-started/rust", "reference/rust", ...]  // page slugs present in this version
    },
    {
      "minor": "1.1",
      "lastPatch": "1.1.3",
      "isLatest": false,
      "label": "1.1",
      "slugs": ["", "getting-started/rust", "reference/rust", ...]
    }
  ]
}
```

## Invariants

1. **Exactly one `isLatest: true`**, and it equals the top-level `latest`; it is the highest minor present.
2. **Ordered newest-first** by minor (semver-descending). The dropdown and any "older versions" grouping
   read this order directly.
3. **`minor` is `X.Y`** (no patch); **`lastPatch` is the full `X.Y.Z`** the bucket was last frozen from.
   The freshness footer renders `lastPatch`; the dropdown shows `minor`.
4. **`slugs` lists the pages that exist in that version** — used by `VersionSelect` to avoid linking to a
   slug absent in the target version (graceful degradation, FR-003) and by `getStaticPaths` to enumerate
   pinned routes.
5. **Canonical JSON**: keys sorted, stable array order, trailing newline — so `docs:snapshot` output is
   byte-stable and the idempotence gate (twice-run zero-diff, SC-004) holds.
6. **Snapshot-owned**: only `docs:snapshot` mutates this file. A hand edit is drift and is caught by the
   idempotence gate on the next run.

## Consumers

- `src/components/VersionSelect.astro` — renders the dropdown from `versions[]`, current version
  highlighted; each entry links to the equivalent slug under that version (or its index if absent).
- `src/pages/v/[version]/[...slug].astro` — `getStaticPaths` builds `(minor × slug)` from `versions[]`
  (excluding `latest`, which is served unprefixed).
- The freshness footer partial — reads the current page's version `lastPatch`.
