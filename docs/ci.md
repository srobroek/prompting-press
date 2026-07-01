# CI & release pipeline

This repo's automation lives in `.github/workflows/`. Gate *logic* lives in moon
tasks / `scripts/ci/*` so every check is locally runnable via
`mise exec -- moon run <task>`; the workflows are thin callers.

## Workflows

| Workflow | Trigger | Purpose |
|---|---|---|
| `ci.yml` | push, pull_request, **merge_group** | The gate suite (see jobs below). |
| `codeql.yml` | push/PR to main, **merge_group**, weekly | CodeQL SAST across Rust/Python/JS-TS. |
| `docs.yml` | push/PR, manual | Build + publish the docs site. |
| `release.yml` | push to main | release-please PR/tag + OIDC publish (crates.io/PyPI/npm). |

## `ci.yml` jobs

- **`pr-gate`** — FAST, no-compile checks for early failure: floating-version
  lint, FFI isolation, the lint trio (Rust `fmt`, Python `ruff`, Node `biome`),
  the license trio + third-party attribution freshness, and the advisory trio.
- **`gates`** — freshness/determinism checks that need a toolchain (schema +
  codegen freshness, API-ref freshness — builds the node addon + nightly rustdoc).
- **`clippy`** — `cargo clippy … -D warnings` (compiles; needs Python 3.12 for
  the pyo3 `abi3-py312` crate).
- **`test-python` / `test-node` / `conformance` / `samples-test`** — build the
  FFI bindings + run the per-language suites and the conformance corpus.
- **`build`** — `cargo build --workspace` across ubuntu/macos/windows (FFI link
  paths differ per OS).

Speed/cost: `concurrency` cancels superseded PR runs; every job has a
`timeout-minutes`; Rust builds use `Swatinem/rust-cache` and the pnpm store + uv
cache are cached across runs. All third-party actions are pinned to a commit SHA
(Dependabot's `github-actions` updater keeps them current).

## Merge queue (required, one-time repo Settings step)

The `merge_group` trigger on `ci.yml` + `codeql.yml` lets the required checks run
inside GitHub's merge queue, which tests each PR against the prospective
post-merge state of `main` (with other queued PRs) **before** it lands. This is
what prevents individually-green PRs from combining into a red `main` (lockfile
drift, dependency interactions, stale generated files) — a stronger guarantee
than re-verifying at release time.

The workflow triggers are in place; enable the queue in **Settings → Branches →
`main` branch protection rule** (or a repo ruleset):

1. **Require a pull request before merging.**
2. **Require status checks to pass** and mark these as required (names must match
   the job names exactly, or the queue will hang):
   - `PR gate — cheap checks (ubuntu)`
   - `Gates (ubuntu)`
   - `Clippy (ubuntu)`
   - `Python binding tests (ubuntu)`, `Node binding tests (ubuntu)`
   - `Conformance corpus gate (ubuntu)`
   - `Consumer sample apps (ubuntu)`
   - `Build (ubuntu-latest)`, `Build (macos-latest)`, `Build (windows-latest)`
   - `Analyze (rust)`, `Analyze (python)`, `Analyze (javascript-typescript)`
3. **Require merge queue.** Suggested settings: build concurrency 1–5, a status
   check timeout comfortably above the slowest job (the OS matrix), and
   min/max PRs per batch to taste.

> Note on release-please PRs: workflows triggered by `GITHUB_TOKEN` don't fire
> the `pull_request` event, so release-please's PRs (and the Cargo.lock-sync /
> docs-snapshot PRs) get their CI from `ci.yml`'s `on: push` trigger rather than
> the PR event. They still flow through the merge queue on merge.

## Release provenance

`release.yml`'s publish job emits a CycloneDX **SBOM** (Syft) before publishing
and a build-provenance **attestation** for the Python wheel + sdist; the npm
package is published with `--provenance`. crates.io has no OIDC provenance path
yet (tracked as a follow-up).
