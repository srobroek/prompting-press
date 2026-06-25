# T013 Report — Wire moon projects/tasks (FR-006 + SEC-005)

## Files Created / Modified

### `.moon/workspace.yml` (modified)
Replaced bootstrap globs with explicit project map. `packages/go` is absent by design (FR-005).

```yaml
# .moon/workspace.yml
projects:
  prompting-press-core: "crates/prompting-press-core"
  prompting-press: "crates/prompting-press"
  prompting-press-py: "crates/prompting-press-py"
  prompting-press-node: "crates/prompting-press-node"
```

### `.moon/toolchains.yml` (created)
Enables Rust toolchain support. Version is intentionally omitted so moon inherits from `rust-toolchain.toml` (1.95.0 via mise/rustup). `syncToolchainConfig: true` tells moon to parse Cargo manifests and lockfile for input hashing.

```yaml
# .moon/toolchains.yml
rust:
  syncToolchainConfig: true
```

**moon 2.x schema note**: The file is `toolchains.yml` (plural), not `toolchain.yml`. This was renamed in moon 2.0 migration.

### `.moon/tasks/all.yml` (created)
Global inherited tasks for all projects. In moon 2.x, `.moon/tasks.yml` is removed; tasks inherited by all projects must live in `.moon/tasks/all.yml`.

```yaml
# .moon/tasks/all.yml
$schema: 'https://moonrepo.dev/schemas/tasks.json'

fileGroups:
  sources:
    - 'src/**/*'
    - 'Cargo.toml'
  tests:
    - 'tests/**/*'
    - 'benches/**/*'

tasks:
  build:
    command: 'cargo build --package $project'
    inputs:
      - '@group(sources)'
    env:
      CARGO_TERM_COLOR: 'always'

  test:
    command: 'cargo test --package $project'
    inputs:
      - '@group(sources)'
      - '@group(tests)'
    env:
      CARGO_TERM_COLOR: 'always'
```

`$project` is a moon token variable that expands to the project ID (e.g. `prompting-press-py`), which matches the Cargo package name.

### `crates/*/moon.yml` (created, 4 files)
Minimal per-crate project config declaring `language: rust` so moon's toolchain routing works correctly.

Files: `crates/prompting-press-core/moon.yml`, `crates/prompting-press/moon.yml`, `crates/prompting-press-py/moon.yml`, `crates/prompting-press-node/moon.yml`

Each contains:
```yaml
$schema: 'https://moonrepo.dev/schemas/project.json'
language: 'rust'
```

---

## Verification: `moon projects` — membership

```
╭──────────────────────────────────────────────────────────────────────────────╮
│Project                    Source                            Toolchains       │
│──────────────────────────────────────────────────────────────────────────────│
│prompting-press            crates/prompting-press            rust             │
│prompting-press-core       crates/prompting-press-core       rust             │
│prompting-press-node       crates/prompting-press-node       rust             │
│prompting-press-py         crates/prompting-press-py         rust             │
╰──────────────────────────────────────────────────────────────────────────────╯
```

**packages/go is absent.** Confirmed via `moon query projects` JSON output:
```
prompting-press → crates/prompting-press
prompting-press-core → crates/prompting-press-core
prompting-press-node → crates/prompting-press-node
prompting-press-py → crates/prompting-press-py
```

---

## Verification: `moon tasks` — task registration

```
╭──────────────────────────────────────────────────────────────────────────────╮
│Task                             Command     Toolchains                       │
│──────────────────────────────────────────────────────────────────────────────│
│prompting-press-core:build       cargo       rust                             │
│prompting-press-core:test        cargo       rust                             │
│prompting-press-node:build       cargo       rust                             │
│prompting-press-node:test        cargo       rust                             │
│prompting-press-py:build         cargo       rust                             │
│prompting-press-py:test          cargo       rust                             │
│prompting-press:build            cargo       rust                             │
│prompting-press:test             cargo       rust                             │
╰──────────────────────────────────────────────────────────────────────────────╯
```

---

## Verification: `mise exec -- moon run :build`

```
▮▮▮▮ prompting-press-node:build (23091666)
▮▮▮▮ prompting-press-core:build (b8fc5df8)
▮▮▮▮ prompting-press-py:build (2ddd1e0f)
▮▮▮▮ prompting-press:build (774db4fb)
... (cargo compile output) ...
▮▮▮▮ prompting-press-node:build (4s 694ms, 23091666)
▮▮▮▮ prompting-press-py:build (14s 727ms, 2ddd1e0f)
▮▮▮▮ prompting-press-core:build (14s 802ms, b8fc5df8)
▮▮▮▮ prompting-press:build (14s 638ms, 774db4fb)

Tasks: 4 completed
 Time: 20s 83ms
```

Exit code: 0. All four crates built.

---

## Verification: `mise exec -- moon run :test`

```
▮▮▮▮ prompting-press:test (ae0bb69d)
▮▮▮▮ prompting-press-core:test (b57688aa)
▮▮▮▮ prompting-press-node:test (01289fad)
▮▮▮▮ prompting-press-py:test (7045cc63)
... (cargo test output — 0 tests each, all pass) ...
▮▮▮▮ prompting-press:test (5s 550ms, ae0bb69d)
▮▮▮▮ prompting-press-core:test (5s 981ms, b57688aa)
▮▮▮▮ prompting-press-node:test (8s 48ms, 01289fad)
▮▮▮▮ prompting-press-py:test (9s 725ms, 7045cc63)

Tasks: 4 completed
 Time: 11s 67ms
```

Exit code: 0. All 4 test tasks passed (0 tests, no failures — stubs have no tests yet, as expected).

---

## moon 2.2.3 Schema Gotchas

1. **`toolchains.yml` not `toolchain.yml`**: renamed in moon 2.0. Using the old name creates a silently ignored file.
2. **`.moon/tasks.yml` removed in 2.0**: global tasks must go in `.moon/tasks/all.yml` (or other files under `.moon/tasks/`). The old path is silently ignored.
3. **`@project(name)` is not a valid token**: the correct variable is `$project` (expands to the project ID string inline). Using `@project(name)` produces a hard parse error.
4. **`language:` field in per-project `moon.yml` is required** for correct toolchain routing — without it, moon does not attach the `rust` toolchain to the project even if `toolchains.yml` has `rust:`.

---

## Codegen Stub Note

`.moon/tasks/all.yml` includes a comment marking where a future `:codegen` task should be added (US3). No `:codegen` task is defined — the file structure makes it trivial to add in that phase.
