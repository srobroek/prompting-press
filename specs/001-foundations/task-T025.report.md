# T025 Report — Generate the Rust serde struct (committed, wired, freshness-gated)

**Task:** Generate the committed Rust serde shape module from
`schemas/jsonschema/prompt-definition.schema.json` via the cargo-typify pipeline pinned in T022,
place it in a marked-generated segregated module, and wire it into the `prompting-press` consumer
crate.

**Date:** 2026-06-25
**Depends on:** T015 (schema) ✓, T022 (tool pin + spike) ✓
**Tooling:** `mise exec --` (cargo-typify 0.7.0, rustfmt 1.9.0 from rust 1.95.0, jq 1.8.1)

---

## TL;DR

| Question | Answer |
|---|---|
| Generated path | `crates/prompting-press/src/generated/prompt_definition.rs` |
| Codegen command | `bash crates/prompting-press/scripts/codegen.sh` (jq-strip → `cargo typify --no-builder` → `rustfmt`) |
| `propertyNames`-strip workaround used? | **YES** — `jq 'del(.properties.variants.propertyNames)'` on a scratch copy only |
| Schema modified? | **NO** — on-disk schema byte-for-byte untouched |
| Twice-run byte-identical (determinism)? | **YES** — SHA-256 identical across two runs |
| `cargo build -p prompting-press`? | **YES** — clean (serde/serde_json pulled in) |
| `cargo build --workspace` links all 4 crates? | **YES** — exit 0 |
| FFI isolation intact (`pyo3` not in tree)? | **YES** — `did not match any packages` (also re-checked `napi`) |
| Generated file committed (not gitignored)? | **YES** — `git check-ignore` confirms tracked |

---

## 1. The codegen script

`crates/prompting-press/scripts/codegen.sh` (consistent with `packages/python/scripts/codegen.sh`
and `packages/typescript/scripts/codegen.mjs`). It resolves the repo root from its own location,
runs the pipeline, and prepends a static do-not-edit header. Full pipeline:

```bash
SCHEMA="schemas/jsonschema/prompt-definition.schema.json"
OUT="crates/prompting-press/src/generated/prompt_definition.rs"

# 1. Strip ONLY the typify-incompatible validation key onto a scratch copy (schema untouched).
TMP_SCHEMA="$(mktemp -t pp-typify-schema.XXXXXX.json)"
mise exec -- jq 'del(.properties.variants.propertyNames)' "${SCHEMA}" > "${TMP_SCHEMA}"

# 2. typify does not create the parent dir; ensure the segregated generated dir exists (FR-016).
mkdir -p "$(dirname "${OUT}")"

# 3. Generate plain serde types (no builder boilerplate — T022 F2: 754 vs 1165 lines).
mise exec -- cargo typify --no-builder --output "${OUT}" "${TMP_SCHEMA}"

# 4. Determinism guard (verified no-op on this schema; kept against future typify fmt drift).
mise exec -- rustfmt --edition 2021 "${OUT}"

# 5. Prepend the STATIC header (// line comments — legal before typify's #![allow(...)] inner attrs).
```

Determinism contract baked into the script: the header is static (no version, no timestamp, no host
data); typify emits no time/host-varying bytes; rustfmt re-normalizes. Two extra mechanical points
the spike did not cover but were needed in practice:
- typify does **not** create the output's parent directory → added `mkdir -p` (the TS script does the
  equivalent `mkdir`).
- `mktemp` scratch files for the stripped schema and the header-prepend, cleaned via `trap`.

### Why the `propertyNames` strip is sound (workaround used, as sanctioned)

cargo-typify 0.7.0 panics on `properties.variants.propertyNames = { "not": { "const": "default" } }`
(T022 Finding F1 — unimplemented `not`-subschema). The schema is the cross-language source of truth
and is **not** modified. The script strips that one key from a scratch copy fed to typify only.
`propertyNames` is a pure VALIDATION constraint — no generated type in any language can encode "map
key must not equal `'default''`". The reserved-`default` rule (FR-011b) is enforced by the US2
validation gate (the `variant-named-default.json` reject fixture), not by the Rust type. So the
strip removes nothing the type could have carried.

---

## 2. Generated module — header + key types

### Header (placed BEFORE the `#![allow(...)]` inner attributes)

typify emits crate-level `#![allow(clippy::...)]` **inner** attributes that must be the first
non-comment items. The header is therefore written as `//` line comments (legal before inner attrs;
`//!` doc comments are NOT — they are themselves inner items). Verbatim top of the file:

```rust
// GENERATED FILE — DO NOT EDIT.
//
// This module is code-generated from the single source of truth:
//   schemas/jsonschema/prompt-definition.schema.json
// by cargo-typify (pinned via mise: "cargo:cargo-typify" = "0.7.0").
//
// Regenerate with: crates/prompting-press/scripts/codegen.sh  (re-run on schema change).
// Hand edits are overwritten and will fail the US4 freshness gate. Edit the schema.
//
// NOTE: the schema's `variants.propertyNames` (reserved-"default" rejection,
// FR-011b) is a VALIDATION constraint with no representable Rust type; it is
// stripped before typify (which cannot parse its `not`/`const` form) and is
// enforced by the US2 validation gate, not by the types below. See codegen.sh.

#![allow(clippy::redundant_closure_call)]
#![allow(clippy::needless_lifetimes)]
#![allow(clippy::match_single_binding)]
#![allow(clippy::clone_on_copy)]
```

The `#![allow(...)]` attrs scope to the `prompt_definition` module they live in (it is a `mod`-
declared file, never `include!`d — F3 resolved). No timestamp/version in the file → byte-deterministic.

### String enums (`role`, `provenance`) — per-variant `#[serde(rename)]`

```rust
pub enum PromptDefinitionRole {
    #[serde(rename = "system")]    System,
    #[serde(rename = "user")]      User,
    #[serde(rename = "assistant")] Assistant,
}
pub enum VariableDeclProvenance {
    #[serde(rename = "trusted")]   Trusted,
    #[serde(rename = "untrusted")] Untrusted,
    #[serde(rename = "external")]  External,
}
```

### Sealed objects → `#[serde(deny_unknown_fields)]` (3 occurrences: lines 115, 377, 761)

On `PromptDefinition`, `VariableDecl`, `Variant` — the schema's `additionalProperties: false` sealing
survives into Rust.

### Open objects (`metadata`, `meta`) → `serde_json::Map<String, Value>`

```rust
// PromptDefinition.meta, PromptDefinition.metadata, Variant.meta:
#[serde(default, skip_serializing_if = "::serde_json::Map::is_empty")]
pub meta: ::serde_json::Map<::std::string::String, ::serde_json::Value>,
#[serde(default, skip_serializing_if = "::serde_json::Map::is_empty")]
pub metadata: ::serde_json::Map<::std::string::String, ::serde_json::Value>,
```

### Untagged `oneOf` for `VariableDecl.type`

```rust
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
#[serde(untagged)]
pub enum VariableDeclType {
    String(VariableDeclTypeString),                    // "string"|"integer"|… (6 variants)
    Array(::std::vec::Vec<VariableDeclTypeArrayItem>), // items incl. Null     (7 variants)
}
```
`VariableDeclTypeString` (6) and `VariableDeclTypeArrayItem` (7, incl. `Null`) are separate generated
enums — redundant but correct, exactly as the T022 spike verified round-trips.

### Other notable types

- `PromptDefinition.name` → newtype `PromptDefinitionName(String)` (minLength:1 validating newtype, F4).
- `Variant` carries only `body: String` + the open `meta` map (FR-011a sealing enforced by `deny_unknown_fields`).
- `variables` → `HashMap<String, VariableDecl>` with `#[serde(default, skip_serializing_if = ...is_empty)]`.

File length: **768 lines** (754 typify + 14-line header). rustfmt `--check` clean.

---

## 3. lib.rs wiring diff

`crates/prompting-press/src/lib.rs` — added module declaration + curated re-exports (the consumer
re-exports but NEVER hand-edits the generated module, critique E1):

```diff
 /// Re-export of the kernel, so consumers can reach core types through one entry point.
 pub use prompting_press_core as core;
+
+/// Code-generated shape modules, emitted from the JSON Schema single source of truth
+/// by `cargo-typify` (FR-016 / constitution C-07). Marked-generated, segregated, and
+/// freshness-gated in CI (US4); never hand-edited. Regenerate via
+/// `crates/prompting-press/scripts/codegen.sh`.
+pub mod generated;
+
+/// Re-export the generated `PromptDefinition` shape and its supporting types so consumers
+/// reach them through this crate's public surface rather than the generated module path.
+/// This crate re-exports but NEVER hand-edits the generated module.
+pub use generated::prompt_definition;
+pub use generated::prompt_definition::PromptDefinition;
```

New hand-written wrapper `crates/prompting-press/src/generated.rs` (the `mod` declaration so the
generated file stays a clean module-file carrying its own inner attrs):

```rust
//! Segregated home for code-generated shape modules (FR-016).
//! … never hand-edit … Regenerate with crates/prompting-press/scripts/codegen.sh.
pub mod prompt_definition;
```

`crates/prompting-press/Cargo.toml` — the generated module needs serde derives + serde_json maps:

```diff
 [dependencies]
 prompting-press-core = { path = "../prompting-press-core" }
+serde = { workspace = true }
+serde_json = { workspace = true }
```

`Cargo.lock` auto-updated: only `serde` + `serde_json` added to the `prompting-press` package's dep
list (both already present in the lock from sibling crates — no new package entries).

---

## 4. Determinism proof (US4 freshness-gate prerequisite)

Script run twice; SHA-256 of the committed file compared:

```
=== RUN 1 ===  Generated crates/prompting-press/src/generated/prompt_definition.rs
=== RUN 2 ===  Generated crates/prompting-press/src/generated/prompt_definition.rs
SHA run1: 22d48213d14b6ae748b31d6e3d1933bb65bd11edfefeea8d00a7facf76436208
SHA run2: 22d48213d14b6ae748b31d6e3d1933bb65bd11edfefeea8d00a7facf76436208
DETERMINISTIC: YES (byte-identical)
```

`rustfmt --edition 2021 --check` on the committed file: **CLEAN (no diff)**.

---

## 5. Build + link + FFI re-check

```
$ mise exec -- cargo build -p prompting-press
   Compiling serde … serde_json … prompting-press-core … prompting-press
    Finished `dev` profile … in 7.45s                       # CLEAN

$ mise exec -- cargo build --workspace                       # exit 0
    Finished `dev` profile … in 15.14s
  → Compiled: prompting-press-core, prompting-press, prompting-press-py, prompting-press-node

$ cargo metadata --no-deps → 4 members:
    prompting-press  prompting-press-core  prompting-press-node  prompting-press-py

$ mise exec -- cargo tree -p prompting-press -i pyo3
    error: package ID specification `pyo3` did not match any packages
  → FFI ISOLATION INTACT (pyo3 not reachable from the consumer crate)
$ mise exec -- cargo tree -p prompting-press -i napi  (belt-and-suspenders)
    error: … `napi` did not match any packages   → also intact
```

Note: `cargo tree -i` returns a non-zero exit when the inverted package is absent — that non-zero IS
the pass signal (the package is correctly not in the dependency graph), not a failure.

---

## 6. Scope / files

Modified (T025 scope only — `crates/prompting-press/` + this report):
- `crates/prompting-press/scripts/codegen.sh` (new)
- `crates/prompting-press/src/generated.rs` (new, hand-written mod wrapper)
- `crates/prompting-press/src/generated/prompt_definition.rs` (new, GENERATED, committed)
- `crates/prompting-press/src/lib.rs` (mod + re-export wiring)
- `crates/prompting-press/Cargo.toml` (serde/serde_json deps)
- `Cargo.lock` (auto: serde/serde_json on the prompting-press package)
- `specs/001-foundations/task-T025.report.md` (this report)

Untouched: the schema, `packages/`, other crates, other specs. `.gitignore` already exempts
`crates/*/src/generated/` from ignoring (it only ignores build output); confirmed via
`git check-ignore` that the generated file is **tracked, not ignored**.

---

## Carry-forward notes for T026 (freshness gate)

- Canonical regen command: `bash crates/prompting-press/scripts/codegen.sh`. The gate should run it,
  then assert `git diff --exit-code crates/prompting-press/src/generated/prompt_definition.rs` is empty.
- The gate must have `jq`, `cargo-typify`, and rustfmt available under `mise` (all pinned).
- The `propertyNames` strip is intrinsic to the script, so the gate inherits it automatically — no
  separate handling needed.
