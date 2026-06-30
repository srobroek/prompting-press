# Quickstart: Native docs versioning, snapshot-per-released-minor

Runnable validation of the versioned-docs mechanism. References [plan.md](./plan.md),
[research.md](./research.md), and the [contracts](./contracts/). Pre-publish: these exercise the
mechanism locally; the first real snapshot happens at the first released minor.

## Prerequisites

- `docs/site` deps installed; the docs moon project present (`docs/site/moon.yml` declaring `docs:snapshot`).
- Snapshot/route code uses stable Node APIs so it runs under both the deploy toolchain (node 22.12.0 /
  pnpm 10.11.0) and local mise (node 25.3.0) — see research R5.

## Snapshot a version locally (US3)

```bash
moon run docs:snapshot -- --version 1.0.0
# → creates src/versions/v1.0/ (frozen current docs incl. per-version API refs)
#   and updates src/data/versions.json (latest=1.0, versions=[1.0], lastPatch=1.0.0)
```

## Validate US2 — minor creates a bucket, patch rolls up

```bash
# New minor → new bucket + dropdown entry:
moon run docs:snapshot -- --version 1.1.0
git status --short docs/site/src/versions/ docs/site/src/data/versions.json
# → versions/v1.1/ added; versions.json gains 1.1 as latest (1.0 retained)

# Patch on that minor → OVERWRITE the bucket, NO new dropdown entry:
moon run docs:snapshot -- --version 1.1.1
rg '"minor"' docs/site/src/data/versions.json     # still only 1.1 and 1.0 (no 1.1.1 entry)
rg '"lastPatch": "1.1.1"' docs/site/src/data/versions.json   # 1.1 bucket footer now reflects 1.1.1
```

## Validate idempotence (FR-008 / SC-004)

```bash
moon run docs:snapshot -- --version 1.1.1 && git add -A
moon run docs:snapshot -- --version 1.1.1
git diff --stat docs/site/src/versions/ docs/site/src/data/versions.json   # MUST be empty
```

## Validate US1 — reader version switching + routing (FR-002/003/004/017)

```bash
pnpm -C docs/site build
# Then over the built output (dist/):
#  - latest pages exist at the UNPREFIXED canonical paths (e.g. /reference/rust/)
#  - pinned pages exist under /v/1.0/... and /v/1.1/...
#  - the VersionSelect dropdown lists 1.1 (latest) and 1.0
#  - switching to 1.0 on a page that exists in 1.0 lands on the equivalent page;
#    a page added only in 1.1 falls back to 1.0's index (no broken link)
#  - non-latest (/v/1.0/...) pages carry <meta name="robots" content="noindex">
rg -l 'noindex' docs/site/dist/v/1.0/ | head        # non-latest pinned pages are noindexed
```

## Validate the trigger contract (FR-016 / FR-013) — without a real release

```bash
# The minor-vs-patch decision is a pure function of (new version, previous version):
node -e "import('./docs/site/scripts/lib/version.mjs').then(m=>{
  console.log(m.bucketAction('1.1.0','1.0.3'));  // 'new-bucket'  (minor changed)
  console.log(m.bucketAction('1.1.1','1.1.0'));  // 'overwrite'   (patch only)
  try { m.bucketAction('garbage','1.1.0'); } catch(e){ console.log('loud-fail OK'); }  // FR-013
})"
```

## Validate Principle V — no library version axis leaked

```bash
# The library API gains NO version parameter/store. Confirm versioning lives only in docs/site:
rg -n 'version' crates/ packages/ --glob '!**/generated/**' --glob '!**/target/**' \
  | rg -iv 'rust-version|workspace|Cargo|abi3|core_version|__version__|package version' | head
# (expect: no render-time/version-pin API surface — versioning is git/release-please + docs only)
mise exec -- moon run ci:check-ffi    # library boundary unchanged
```

## Validate no publishing (FR-014 / SC-007)

```bash
rg -n 'if: false|publish' .github/workflows/release.yml | head   # publish job still gated off
# The docs snapshot/deploy path does not run any cargo/maturin/pnpm publish.
```

## Done when

- `docs:snapshot` creates a bucket on a new minor, overwrites on a patch, updates the manifest, and is
  idempotent (twice-run zero-diff).
- The built site serves latest unprefixed (canonical) + pinned under `/v/X.Y/`; the dropdown lists minors;
  switching degrades gracefully; non-latest is noindexed.
- The trigger computes minor-vs-patch from the previous tag and fails loudly on ambiguity.
- No library version axis; `ci:check-ffi` passes; no publishing enabled.
