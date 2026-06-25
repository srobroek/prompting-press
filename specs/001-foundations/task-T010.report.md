# Report — T010 (`packages/typescript/` published-package skeleton)

Spec: 001 "Foundations" · Branch: `001-foundations` · FR-004 · US1

## What was created

Exactly two files, both under `packages/typescript/`. No runtime code, no `src/`,
no `node_modules`, nothing outside the package directory touched.

1. `/Users/sjors/personal/dev/prompting-press/packages/typescript/package.json`
2. `/Users/sjors/personal/dev/prompting-press/packages/typescript/tsconfig.json`

This is a SKELETON: spec 001 ships no runtime code (FR-004). The package is
structured to *eventually* wrap the Rust napi cdylib crate
`crates/prompting-press-node/`, which is declared by T008 and does not exist yet
(the workspace `members = ["crates/*"]` matches nothing today — by design).

## File 1 — `packages/typescript/package.json`

```json
{
  "name": "prompting-press",
  "version": "0.0.0",
  "description": "Node.js distribution of Prompting Press — a thin wrapper around the Rust napi-rs binding crate (crates/prompting-press-node). Skeleton only: no runtime code ships in spec 001.",
  "license": "Apache-2.0",
  "type": "module",
  "author": "Sjors Robroek",
  "repository": {
    "type": "git",
    "url": "https://github.com/srobroek/prompting-press.git",
    "directory": "packages/typescript"
  },
  "private": true,
  "packageManager": "pnpm@10.33.2",
  "engines": {
    "node": ">=20"
  },
  "main": "index.js",
  "types": "index.d.ts",
  "files": [
    "index.js",
    "index.d.ts",
    "*.node"
  ],
  "napi": {
    "binaryName": "prompting-press",
    "packageName": "prompting-press"
  },
  "scripts": {
    "build": "napi build --release --platform --esm --manifest-path ../../crates/prompting-press-node/Cargo.toml --output-dir .",
    "build:debug": "napi build --platform --esm --manifest-path ../../crates/prompting-press-node/Cargo.toml --output-dir .",
    "prepublishOnly": "napi prepublish -t npm"
  },
  "dependencies": {},
  "devDependencies": {
    "@napi-rs/cli": "3.7.2",
    "typescript": "5.9.2"
  }
}
```

Notes / decisions:

- **name** = `"prompting-press"` — the simple unscoped name, per the task's
  preference between that and `@prompting-press/node`. Matches the Rust crate
  family name and the `binaryName`/`packageName` below.
- **version** = `"0.0.0"` — nothing is published in spec 001; matches the Rust
  `[workspace.package].version`.
- **license** = `"Apache-2.0"` — verified against the repo `LICENSE` file
  (Apache License 2.0); SPDX id matches the Rust workspace.
- **type** = `"module"` — ESM. The build scripts pass `--esm` so the
  napi-generated binding loader is ESM, consistent with this field.
- **packageManager** = `"pnpm@10.33.2"` — matches `mise.toml`
  (`"npm:pnpm" = "10.33.2"`); corepack reads this field. Exact pin, no range.
- **private** = `true` — defensive: prevents an accidental `npm publish` of a
  0.0.0 skeleton that has no built `.node` artifact. Removed when the package is
  ready to publish in a later spec.
- **engines.node** = `">=20"` — a floor, not a dependency pin (SEC-003 governs
  *dependency* versions, not engine ranges). `@napi-rs/cli@3.7.2` itself only
  requires `node >= 16`; the repo runs Node 25.3.0 (`mise.toml`).
- **dependencies** = `{}` — empty, as required. The runtime artifact is the
  compiled `.node` binary plus the napi-generated `index.js`/`index.d.ts`, not
  an npm dependency.
- **devDependencies** — exact-pinned only (see version table below).
- **napi** config uses the **v3 schema** (`binaryName` + `packageName`), which
  matches the task wording and the installed CLI major (3.x). In napi v2 these
  were `name` and `package.name`; v3 renamed them (confirmed via napi-rs docs,
  v2→v3 migration guide).
- **scripts.build** points `napi build` at the Rust crate via
  `--manifest-path ../../crates/prompting-press-node/Cargo.toml` (relative to
  this package dir). This is the wiring that makes the TS package a thin wrapper
  over the cdylib. `--platform` adds the target triple to the `.node` filename;
  `--output-dir .` drops the binding next to `package.json`. Flags use the v3
  CLI surface (`--manifest-path` replaced v2's removed `--cargo-cwd`).
  **Not run** in this task — the crate stub does not exist yet and building is
  T008/US1 work; T010 is wiring, not building.
- **files** allowlist (`index.js`, `index.d.ts`, `*.node`) describes the future
  publishable surface; harmless today since none exist.

## File 2 — `packages/typescript/tsconfig.json`

```jsonc
{
  // Minimal strict TS config for the Prompting Press Node wrapper skeleton.
  // Logic-free in spec 001: this exists so `tsc` can later type-check the thin
  // wrapper and emit `.d.ts` declarations alongside the napi-generated binding.
  "compilerOptions": {
    "target": "ES2022",
    "module": "NodeNext",
    "moduleResolution": "NodeNext",
    "lib": ["ES2022"],

    // Type safety: strict everything.
    "strict": true,
    "noUncheckedIndexedAccess": true,
    "noImplicitOverride": true,
    "noFallthroughCasesInSwitch": true,
    "exactOptionalPropertyTypes": true,
    "forceConsistentCasingInFileNames": true,

    // Declaration emit (the wrapper publishes `.d.ts` types for consumers).
    "declaration": true,
    "declarationMap": true,
    "sourceMap": true,
    "outDir": "./dist",

    // Interop / module hygiene.
    "esModuleInterop": true,
    "isolatedModules": true,
    "verbatimModuleSyntax": true,
    "skipLibCheck": true
  },
  // No `src/` yet — spec 001 ships no runtime code. `include` is intentionally
  // empty so `tsc` is a no-op until the wrapper source lands in a later spec.
  "include": [],
  "exclude": ["node_modules", "dist"]
}
```

Notes / decisions:

- **target / module / moduleResolution** = `ES2022` / `NodeNext` / `NodeNext` —
  modern Node ESM, consistent with `"type": "module"`. `NodeNext` is the correct
  resolution mode for an ESM package targeting current Node.
- **strict: true** plus the extra safety flags (`noUncheckedIndexedAccess`,
  `exactOptionalPropertyTypes`, etc.) — enterprise-grade strictness; cheap to set
  now and avoids relaxing later.
- **declaration: true** + `outDir: "./dist"` — the wrapper will publish types;
  `tsc` emits `.d.ts`. (At publish time the napi binding's own `index.d.ts` lives
  at the package root via `--output-dir .`; the hand-written wrapper, when it
  exists, compiles to `dist/`. No conflict today.)
- **include: []** — intentional. There is no source yet, so `tsc` is a no-op.
  This keeps the config logic-free and prevents `tsc` from erroring on an empty
  package. Populated when wrapper source lands.
- `skipLibCheck: true` — standard pragma to avoid type-checking third-party
  `.d.ts` (including the future generated napi binding) during wrapper builds.

## Pinned versions chosen — and how I verified they are real

All versions queried against the live npm registry on 2026-06-25.

| Package        | Pinned   | Verification                                                                 |
|----------------|----------|------------------------------------------------------------------------------|
| `@napi-rs/cli` | `3.7.2`  | `npm view @napi-rs/cli@3.7.2 version` → `3.7.2`; `dist.shasum` = `4a43d7bc7703159da0de1a7e3b1cd94141132b10`. It is the registry `latest` dist-tag (current stable). `engines.node` = `>= 16`; `bin` = `{ napi: dist/cli.js }`. |
| `typescript`   | `5.9.2`  | `npm view typescript@5.9.2 version` → `5.9.2` (real published).              |

Both pins are **exact** strings (no `^`/`~`/`latest`/`*`), satisfying SEC-003 /
T030a. Verified programmatically against `package.json`:

```
OK: all dependency versions are exact-pinned
```

### Why `@napi-rs/cli@3.7.2` (not the suggested `2.18.4`)

The task suggested `2.18.4` *or* "whatever the current stable is — pick a real
published version." The current `latest` dist-tag is `3.7.2`, and the task's own
config wording (`binaryName`, `packageName`) is the **v3** schema — in v2 these
fields were named `name` and `package.name`. Pinning 3.7.2 makes the config and
the CLI major consistent. `2.18.4` is also verified-real as a fallback, but
mixing a v3-shaped config with a v2 CLI would be incoherent.

### Why `typescript@5.9.2` (not `6.0.3`, which is `latest`)

Deliberate, lower-risk choice — flagged as an assumption. `6.0.3` is `latest` but
TS 6.0 is a brand-new major (an `rc` for `7.0.1` already exists in dist-tags).
The sibling codegen task T021 pins `json-schema-to-typescript@15.0.4`, whose
toolchain expectations were established against the TS 5.x line. A skeleton that
only needs `tsc` for `.d.ts` emission gains nothing from adopting a fresh major.
`5.9.2` is the latest 5.9 patch and is verified-published. Trivially bumpable.

## napi-rs CLI availability finding (critique E3 / CHK026)

The task requires confirming the CLI situation. Findings:

1. **Registry availability (real published version):**
   `npm view @napi-rs/cli@3.7.2 version` → `3.7.2`. The pinned version resolves
   on the public registry. ✅
2. **Local resolution without install (expected to fail):**
   `mise exec -- npx --no-install @napi-rs/cli --version` →
   `npm error could not determine executable to run`. ✅ **This failure is
   expected and correct** — `@napi-rs/cli` is NOT globally installed, and it is
   not supposed to be. It is declared as a per-package **devDependency** and will
   be resolved by `pnpm install` (then runnable via `pnpm exec napi …` /
   `pnpm build`). `test -d node_modules/@napi-rs/cli` → `NOT installed`
   (no `pnpm install` has been run yet).
3. **No `napi build` was run.** Per the task, building is out of scope: the
   `prompting-press-node` crate stub does not exist yet (T008/US1), so a build
   would have nothing to compile. T010 is wiring, not building.

**Conclusion:** the acceptance bar is met — the CLI is correctly declared as a
concrete-pinned devDependency (`3.7.2`), the package is structured to invoke it
(`napi build … --manifest-path ../../crates/prompting-press-node/Cargo.toml`),
and the chosen version is a real, currently-published release.

## How this wraps `crates/prompting-press-node` in US3 (and beyond)

- **The wrapper relationship (build-time, US1 → later):** the Rust crate
  `crates/prompting-press-node/` is a `cdylib` that depends on `napi`/`napi-derive`
  (it is the only crate permitted to dep `napi`, per FR-003 / constitution
  Principle II). The `napi build` script in this `package.json` compiles that
  crate via `--manifest-path ../../crates/prompting-press-node/Cargo.toml` and
  emits a `<binaryName>.<triple>.node` binding plus `index.js`/`index.d.ts` into
  the package root. The published npm package `prompting-press` is therefore a
  **thin loader** over that compiled binding — no business logic lives in JS/TS
  (it lives in `prompting-press-core`, surfaced through the node crate).
- **US3 (codegen) intersection:** US3 (T021, T024) later adds a TS codegen
  toolchain (`json-schema-to-typescript@15.0.4` + pinned Prettier) as additional
  dev deps in *this* package, and generates the shared JSON-Schema type shape
  into a marked-generated, segregated path under `packages/typescript/`
  (e.g. `generated/prompt-definition.ts`). This T010 skeleton deliberately leaves
  room for that: `dependencies` stays empty, `devDependencies` will gain the
  codegen tools, and the `tsconfig.json` `include` (empty now) will pick up the
  generated + wrapper sources. The napi binding's runtime types and the codegen'd
  schema types are complementary surfaces of the same package.
- **Lockfile note for later (T021 / SEC-001/002):** when deps are installed, the
  pnpm lockfile (`pnpm-lock.yaml`, carrying integrity hashes) must be committed
  and CI must install with `--frozen-lockfile`. Out of scope for T010 (no install
  performed), noted for traceability.

## Validation performed (proof both files parse + pins are clean)

- `package.json` parses as strict JSON via `require()`:
  `name=prompting-press version=0.0.0 type=module packageManager=pnpm@10.33.2`,
  `napi={"binaryName":"prompting-press","packageName":"prompting-press"}`,
  `devDeps={"@napi-rs/cli":"3.7.2","typescript":"5.9.2"}`, `deps={}`.
- Floating-version scan over all deps: `OK: all dependency versions are exact-pinned`.
- `tsconfig.json` parses as JSONC (comments stripped → `JSON.parse`):
  `target=ES2022 module=NodeNext strict=true declaration=true outDir=./dist`.
- No `tsc` and no `napi build` invoked (no source, no crate, no install — all by
  design for a US1 skeleton).
```
