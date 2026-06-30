# docs/site/src/versions/

Frozen, per-minor documentation snapshots. **Never hand-edit** files inside this directory.

Each subdirectory (`v1.0/`, `v1.1/`, …) is created and overwritten exclusively by the
`docs:snapshot` moon task:

```bash
moon run docs:snapshot -- --version X.Y.Z
```

A minor release creates a new `vX.Y/` tree; a patch release overwrites the existing `vX.Y/`
tree (rollup). The manifest at `../data/versions.json` is updated in the same run.

The dynamic Astro route at `src/pages/v/[version]/[...slug].astro` serves this content
under `/v/X.Y/…`. Latest docs stay at the unprefixed canonical paths in `src/content/docs/`.

See `specs/012-docs-versioning/` for the full specification.
