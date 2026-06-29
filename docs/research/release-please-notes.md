# Release Please: Setup Notes

**Status**: wired (2026-06-29). Publishing intentionally disabled — pre-publish per project owner.

---

## Lockstep design

All three publishable units — Cargo workspace root (`.`), `packages/python`, and
`packages/typescript` — are versioned together via the `linked-versions` plugin. When any
component receives a conventional commit that would bump its version, `linked-versions` takes
the highest proposed version across the group and applies it to all three. A single release PR
covers all three packages (`"separate-pull-requests": false`).

The `cargo-workspace` plugin is included with `"merge": false` so the Cargo workspace members
(all sharing `version.workspace = true` from the root `Cargo.toml`) are updated together through
the workspace root package, rather than release-please trying to treat each crate as an
independent component.

Config files:

- `release-please-config.json` — component definitions, plugin wiring, release types
- `.release-please-manifest.json` — current tracked version per path (all `"0.0.0"` until first release)
- `.github/workflows/release.yml` — the GitHub Actions workflow

## Action version and SHA pin

The workflow uses `googleapis/release-please-action@v5.0.0` (released 2026-04-22).

**SHA to pin**: `45996ed1f6d02564a971a2fa1b5860e934307cf7`

The workflow currently references `@v5` (mutable tag). Before enabling for production use, pin to
the SHA above — matching the pattern used in `ci.yml` — by replacing `@v5` with `@45996ed...` and
updating the inline comment. Verify the SHA matches
<https://github.com/googleapis/release-please-action/releases/tag/v5.0.0> before pinning.

## Publishing: intentionally disabled

The `publish` job in `release.yml` has `if: false`. All publish steps (cargo publish, maturin
publish, pnpm publish) are present but commented out. Remove `if: false` and uncomment the
appropriate steps when the project is ready to publish (post spec 010 pre-publish gates).

Required secrets when publishing:

- `CARGO_REGISTRY_TOKEN` — crates.io API token
- `MATURIN_PYPI_TOKEN` — PyPI API token (maturin publish)
- `NODE_AUTH_TOKEN` / `NPM_TOKEN` — npm token (pnpm publish)

## Docs versioning on MINOR releases

On each MINOR release a snapshot of the docs site should be captured into the versioned docs
archive (e.g. `docs/site/v0.x/` or equivalent). This is **not built here** — it depends on the
native-docs-versioning spec (future work, referenced in roadmap spec 010). When that spec lands,
add a conditional step to the `publish` job gated on `startsWith(github.ref, 'refs/tags/')` and
a semver minor-bump check.

## Schema references

- release-please config schema: <https://raw.githubusercontent.com/googleapis/release-please/main/schemas/config.json>
- release-please manifest schema: <https://raw.githubusercontent.com/googleapis/release-please/main/schemas/versions.json>
