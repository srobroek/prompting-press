# Contributing to Prompting Press

## Build requirements

The repo uses [mise](https://mise.jdx.dev/) for toolchain management and [moon](https://moonrepo.dev/) as the task runner.

```sh
mise install          # installs Rust, Python, Node, and other pinned tools
```

### Running tests

```sh
# Rust (core + consumer crate)
cargo test --workspace

# Python binding
cd packages/python
uv run pytest

# TypeScript binding
cd packages/typescript
pnpm test
```

All three suites plus schema-codegen gates run in CI (`cargo xtask` / moon pipelines). A PR that breaks any gate will not be merged.

### Docs site (Astro / Starlight)

```sh
pnpm -C docs/site dev    # http://localhost:4321/prompting-press/
```

The published docs are at <https://srobroek.github.io/prompting-press/>.

---

## Commit style

This repo uses [Conventional Commits](https://www.conventionalcommits.org/). The PR title is what ends up in the changelog (via release-please on squash-merge), so make it user-facing and accurate.

| Prefix | When to use |
|--------|-------------|
| `feat:` | New public capability |
| `fix:` | Bug fix visible to users |
| `chore:` | Tooling, deps, CI — no user-visible change |
| `docs:` | Documentation only |
| `refactor:` | Internal restructure, no behaviour change |
| `test:` | Tests only |

A breaking change gets `!` after the prefix (e.g. `feat!:`) and a `BREAKING CHANGE:` footer.

---

## Substantial changes: SpecKit workflow

New public API, schema changes, new bindings, or anything that touches multiple crates/packages goes through the SpecKit spec workflow (spec → clarify → plan → tasks → implement → verify). Open a [feature request issue](https://github.com/srobroek/prompting-press/issues/new?template=feature_request.yml) first; a maintainer will initiate the spec if the proposal is accepted.

---

## Publishing

Releases are fully automated via [release-please](https://github.com/googleapis/release-please). Maintainers trigger the release workflow; contributors do not need to bump versions or publish packages manually.

---

## Code of conduct

Be constructive, be kind, be specific. That's it.
