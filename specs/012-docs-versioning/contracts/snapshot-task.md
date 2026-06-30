# Contract: `docs:snapshot` task + release trigger

Two coupled contracts: the moon `docs:snapshot` task (the snapshot mechanism) and the release-workflow
trigger that invokes it (the glue). release-please stays the version oracle; this is everything downstream
of "a release happened."

## `docs:snapshot` — moon task

Invocation (local or CI):

```bash
moon run docs:snapshot -- --version <X.Y.Z>
```

- **Input**: `--version <X.Y.Z>` — the full released version. The task derives the minor bucket `X.Y`.
  (No `--out`; the snapshot writes into the repo's `docs/site/src/versions/` + `versions.json`. Spec-011's
  generator, which it calls, gets `--out` pointed at a temp/staging dir.)
- **Steps**:
  1. Regenerate per-version API refs: call `gen-api-refs.mjs --version <X.Y> --out <staging>` (spec-011).
     If 011 is not yet landed, the adapter falls back to copying the current reference pages as-is + logs a
     pending-011 warning (research R7).
  2. Assemble the current docs (content + shape page + the API refs from step 1) and **freeze** them into
     `src/versions/v<X.Y>/`: **create** the tree if `X.Y` is new, **overwrite** it if it already exists
     (patch rollup).
  3. Update `src/data/versions.json` (see [version-manifest.md](./version-manifest.md)): add the `X.Y` entry
     if new; set it `latest`/`isLatest` iff it is the highest minor; always set its `lastPatch = X.Y.Z` and
     refresh its `slugs`.
  4. Stamp the freshness footer source (the bucket's `lastPatch`) so each version's pages can render
     "docs current as of X.Y.Z".
- **Output**: a new/updated `src/versions/v<X.Y>/` tree + an updated canonical `versions.json`. Nothing else.
- **Idempotence (FR-008 / SC-004)**: deterministic copy + canonical (sorted-key) JSON ⇒ re-running for the
  same `X.Y.Z` produces zero git diff. The validation gate runs it twice and asserts a clean tree.
- **Failure modes (FR-013)**: unparseable `--version` ⇒ exit non-zero with an actionable message; a missing
  spec-011 generator when the adapter is in "require 011" mode ⇒ loud failure (default mode is the
  fall-back-with-warning).
- **Purity / no publish**: the task only writes docs files; it performs no package publishing (FR-014).

## Release trigger — in `.github/workflows/release.yml`

After the release-please step yields a release + new version/tag:

1. **Determine bucket action** via `scripts/lib/version.mjs`, comparing the new version to the **previous**
   release version:
   - `major.minor` changed (or no previous release) ⇒ **new bucket**.
   - only `patch` changed ⇒ **overwrite** the current minor bucket (rollup).
   - unparseable / ambiguous ⇒ **fail loudly** (FR-013), do not snapshot.
2. **Invoke** `moon run docs:snapshot -- --version <new X.Y.Z>` (same task for both new-bucket and overwrite;
   the task's step 2 create-vs-overwrite handles the distinction).
3. **Deploy** the multi-version site via the existing GitHub Pages pipeline (`docs.yml` build → artifact →
   deploy). The trigger does not introduce a separate deploy path.

Constraints:
- **No publishing**: the trigger lives alongside, but does not enable, the gated-off `publish` job (FR-014).
- **Action SHA pin**: the release-please action SHA-pin TODO (from the release-please setup) still applies;
  carry it forward when this trigger lands.
- **Every release runs the task**; minor-vs-patch decides *new bucket vs. overwrite*, never *run vs. skip*.
