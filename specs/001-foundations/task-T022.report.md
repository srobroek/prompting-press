# T022 Report — Pin Rust codegen (cargo-typify) + verify output on a sample

**Task:** De-risk US3 by pinning the Rust JSON-Schema -> serde codegen tool and empirically
verifying it handles `schemas/jsonschema/prompt-definition.schema.json` correctly. Verification
spike only — no generated code committed into crates.

**Date:** 2026-06-25
**Schema under test:** `schemas/jsonschema/prompt-definition.schema.json` (Draft 2020-12)
**Scratch location (deleted after this spike):** `/tmp/typify-sample/`

---

## TL;DR

| Question | Answer |
|---|---|
| cargo-typify 0.7.0 confirmed (via mise)? | **YES** |
| Output deterministic across runs? | **YES** (byte-identical, both modes, pre- and post-rustfmt) |
| String enums handled correctly? | **YES** (clean `enum` + per-variant `#[serde(rename="...")]`) |
| Sealed objects -> `deny_unknown_fields`? | **YES** (emitted on root, VariableDecl, Variant) |
| Open objects (`meta`/`metadata`) handled? | **YES** (`serde_json::Map<String, Value>`) |
| `oneOf` in `VariableDecl.type` usable? | **YES** (`#[serde(untagged)]` enum; round-trips correctly) |
| Generated code compiles? | **YES** (clean build + 6/6 functional tests pass against serde 1) |
| **BLOCKER for T025?** | **YES — typify 0.7.0 PANICS on the current schema as-is.** See Finding F1. |

**The schema cannot be fed to typify 0.7.0 unmodified.** The `variants.propertyNames` constraint
`{ "not": { "const": "default" } }` triggers an `unimplemented!()` panic in typify. Everything
*else* in the schema generates clean, correct, deterministic, compiling Rust. T025 must resolve F1
before it can wire codegen. Recommended path documented below (Finding F1, Option 3).

---

## 1. Tool pin confirmation

```
$ mise exec -- cargo typify --version
cargo-typify 0.7.0
```

Source of truth is `mise.toml`: `"cargo:cargo-typify" = "0.7.0"`. No `cargo install` performed or
needed. The subcommand is `cargo typify` (a cargo plugin), invoked under `mise exec --`.

rustfmt from the pinned toolchain:
```
$ mise exec -- rustfmt --version
rustfmt 1.9.0-stable (59807616e1 2026-04-14)
```
(Component of the pinned `rust = "1.95.0"` rustup toolchain.)

### CLI surface (relevant flags)
`cargo typify [OPTIONS] <INPUT>`
- `-o, --output <OUTPUT>` — output file (or `-` for stdout).
- `-b, --builder` — **DEFAULT**: emit builder-style interface. Adds ~400 lines of builder structs
  for our schema (1165 lines vs 754 without). T025 does not need builders.
- `-B, --no-builder` — suppress the builder interface. **Recommended for T025** (smaller, simpler,
  same types).
- `-d, --additional-derive <derive>` — add a derive to ALL generated types (e.g. `PartialEq`).
- `-a, --additional-attr <attr>` — add an attribute to ALL generated types.
- No timestamp / no version-stamp / no header comment is emitted. No flag affects ordering.

---

## 2. EXACT invocation for T025/T026 to reuse

> **Cannot be run against the schema as committed today — see Finding F1.** Once F1 is resolved,
> the mechanical command is:

```bash
mise exec -- cargo typify \
  --no-builder \
  --output <DEST>/prompt_definition.rs \
  schemas/jsonschema/prompt-definition.schema.json

# Optional normalize step — VERIFIED A NO-OP on this output (typify already emits rustfmt-clean
# code), but include it so the committed file is stable against future typify formatting drift:
mise exec -- rustfmt --edition 2021 <DEST>/prompt_definition.rs
```

Notes for T025:
- typify output is **already rustfmt-clean** for this schema: the SHA-256 was identical before and
  after `rustfmt --edition 2021`. Keep the rustfmt step anyway as a determinism guard (cheap; the
  US4 freshness gate diffs the committed file).
- The generated file `#![allow(...)]`s four clippy lints at the top
  (`redundant_closure_call`, `needless_lifetimes`, `match_single_binding`, `clone_on_copy`). T025
  should expect these inner attributes; if the file is `include!`d rather than a module root, those
  `#![...]` inner attributes will need to live at module/crate scope or be converted.
- Generated code depends on `serde` + `serde_json`. **If F1 is resolved via the `pattern` route
  (Option 3), it additionally depends on `regress` (typify's regex engine) and uses
  `std::sync::LazyLock` (Rust 1.80+; fine on 1.95.0).** See F1.

---

## 3. Determinism check (US4 freshness-gate prerequisite)

Generated **twice** in each mode from an identical input; compared by `diff` and SHA-256.

| Mode | run1 vs run2 | SHA-256 |
|---|---|---|
| builder (default), pre-fmt | **byte-identical** | `733fe06d…aa320f` |
| no-builder, pre-fmt | **byte-identical** | `ac56f1b3…1a9c05` |
| no-builder, post-`rustfmt` | **byte-identical** (and identical to pre-fmt) | `ac56f1b3…1a9c05` |

**Result: deterministic.** No timestamps, no nondeterministic ordering observed. Map/struct fields
are emitted in a stable (sorted) order. This satisfies the property the US4 freshness gate relies
on (regenerate -> `git diff` is empty iff schema unchanged).

(Determinism was verified against the schema with the `propertyNames` blocker stripped — see F1 —
since the unmodified schema cannot generate at all. The determinism property is a function of
typify's emitter, not the specific schema, so this result carries forward once F1 is resolved.)

---

## 4. Verification findings per point

All quotes below are from the `--no-builder` output. The relevant types are byte-identical in
builder mode (builder mode only *adds* builder structs).

### 4a. String enums (`role`, `provenance`) — CORRECT

Clean Rust `enum` with **per-variant** `#[serde(rename = "...")]` (not container `rename_all`),
plus a generated `Display`, `FromStr`, and `TryFrom`. Derives `Copy, Eq, Hash, Ord, PartialOrd`.

```rust
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PromptDefinitionRole {
    #[serde(rename = "system")]
    System,
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
}
```
`provenance` -> `VariableDeclProvenance { Trusted, Untrusted, External }`, same shape. **Usable
as-is.**

### 4b. Sealed objects (`additionalProperties: false`) -> `deny_unknown_fields` — CORRECT (REQUIRED behavior present)

typify **does** emit `#[serde(deny_unknown_fields)]` on every sealed object. Confirmed on all three:

```rust
#[serde(deny_unknown_fields)]
pub struct PromptDefinition { … }

#[serde(deny_unknown_fields)]
pub struct VariableDecl { … }

#[serde(deny_unknown_fields)]
pub struct Variant { … }
```

Functionally verified: deserializing `{"body":"x","role":"system"}` into `Variant` **fails**
(unknown `role`), and an unknown root key is rejected. The schema's sealing survives into Rust.
**No T025 action needed for sealing.**

### 4c. Open objects (`metadata`, `meta`, `additionalProperties: true`) — CORRECT

Become `serde_json::Map<String, Value>` (not bare `Value`, not `HashMap`), with
`#[serde(default, skip_serializing_if = "::serde_json::Map::is_empty")]`:

```rust
#[serde(default, skip_serializing_if = "::serde_json::Map::is_empty")]
pub meta: ::serde_json::Map<::std::string::String, ::serde_json::Value>,
#[serde(default, skip_serializing_if = "::serde_json::Map::is_empty")]
pub metadata: ::serde_json::Map<::std::string::String, ::serde_json::Value>,
```
Open content round-trips correctly (verified `{"weight":3,"tags":["a"]}` survives). **Usable as-is.**

### 4d. The `oneOf` in `VariableDecl.type` — USABLE (untagged enum)

typify renders it as an `#[serde(untagged)]` enum with two arms, generating a distinct enum for
each branch's inline string-enum (so the 6-value and 7-value-with-`null` enums are *separate*
types):

```rust
#[serde(untagged)]
pub enum VariableDeclType {
    String(VariableDeclTypeString),                       // string | integer | … | object  (6)
    Array(::std::vec::Vec<VariableDeclTypeArrayItem>),    // items: …| object | null         (7)
}
```
`VariableDeclTypeString` (6 variants) and `VariableDeclTypeArrayItem` (7, incl. `Null`) are
separate generated enums — slightly redundant but correct and harmless.

**Functionally verified** (untagged discrimination works):
- `"type":"string"` -> `VariableDeclType::String(...)`
- `"type":["string","null"]` -> `VariableDeclType::Array([..., Null])`
- `"type":[]` (empty array) -> `Array([])` (correctly cannot match the string arm)

Untagged-enum caveat (inherent to serde, not a typify bug): serde tries arms in order and reports a
generic "did not match any variant" on failure, losing the specific inner error. Acceptable here
because both arms are structurally disjoint (scalar string vs array). **Usable as-is.**

### 4e. `variants.propertyNames` / reserved-`default` rejection — **DOES NOT SURVIVE (and currently PANICS) — see Finding F1**

This is the one place typify fails. Two sub-results:
- A `propertyNames` expressed as `{ "not": { "const": "default" } }` (the current schema) makes
  typify **panic** (`unimplemented!("unhandled not schema …")`). See F1.
- A `propertyNames` expressed as a **`pattern`** (no `not`/`const`) does NOT get ignored —
  typify generates a key newtype that enforces the regex at deserialize time. (Documented in F1
  Option 3 as the remediation.)
- A `propertyNames` with only `type`/`description` (no `not`/`pattern`/`const`) is structurally
  ignored and produces a plain `HashMap<String, Variant>` key.

So: the reserved-`default` rule does NOT survive into the Rust type for free, and as written it
breaks codegen entirely. Per task guidance this rule can legitimately be enforced at the validation
gate (US2) rather than the generated struct — but the schema must still be made *typify-parseable*.
See F1.

### 4f. `default: {}` on `variables` — `#[serde(default)]` emitted — CORRECT

```rust
#[serde(default, skip_serializing_if = ":: std :: collections :: HashMap::is_empty")]
pub variables: ::std::collections::HashMap<::std::string::String, VariableDecl>,
```
typify infers `default` from the schema's `"default": {}` and also adds `skip_serializing_if` for
the empty map. Verified: omitting `variables` deserializes to an empty map. **Usable as-is.**

### 4g. Bonus: `name` (minLength: 1) -> validating newtype

`name` becomes `PromptDefinitionName(String)` with a `FromStr` that rejects empty strings. Not a
verification point but noted: typify turns string `minLength` into a runtime-validated newtype, not
a bare `String`. T025 should be aware the field type is `PromptDefinitionName`, not `String`.

### 4h. Compile + functional verification

Generated `--no-builder` file dropped into a scratch crate (serde 1 + serde_json 1, edition 2021,
toolchain 1.95.0):
- `cargo build`: **clean, zero errors, zero warnings.**
- `cargo test` (6 hand-written round-trip tests): **6/6 pass** — covers untagged oneOf (both arms +
  empty array), `deny_unknown_fields` rejection on root and Variant, `variables` default, open
  `meta`.

---

## Findings affecting T025

### F1 (BLOCKER): typify 0.7.0 panics on `variants.propertyNames = { not: { const: "default" } }`

**Evidence (verbatim):**
```
$ mise exec -- cargo typify --output … schemas/jsonschema/prompt-definition.schema.json
thread 'main' panicked … not yet implemented: unhandled not schema Object(SchemaObject {
    … const_value: Some(String("default")), … })
  at typify-impl-0.7.0/src/convert.rs:1763
```
Isolated cause: deleting **only** `.properties.variants.propertyNames` makes generation succeed
(exit 0). A minimal probe `{"x":{"not":{"const":"default"}}}` reproduces the same panic. So the
trigger is typify's unimplemented handling of a `not` subschema (here wrapping a `const`), NOT
`propertyNames` per se.

**Impact:** T025/T026 cannot run `cargo typify` against the schema as committed today. This is a
hard stop, not a cosmetic issue.

**The schema is the cross-language source of truth — do NOT change it unilaterally.** Note also that
`not: { const }` is a perfectly valid Draft 2020-12 construct; the Python generator
(datamodel-code-generator, T020/T023) and the validation gate (US2) may handle it fine, so any
change must be evaluated for cross-language parity, not just to appease typify. Options for T025,
in recommended order:

- **Option 1 (accept; lowest risk to the contract): keep the schema as-is, pre-strip for Rust
  codegen only.** In the T025/T026 codegen step, pipe the schema through
  `jq 'del(.properties.variants.propertyNames)'` before handing it to `cargo typify`. The
  reserved-`default` rule is then enforced solely at the US2 validation gate (task guidance
  explicitly permits this — it is a validation constraint, not a structural one). Cost: the codegen
  pipeline gains a documented preprocessing step; the Rust type cannot reject a `default` variant
  key at deserialize time (US2 catches it). Keeps the schema byte-for-byte unchanged across all
  languages. **Recommended** unless the team wants the constraint reflected in the Rust type.

- **Option 2: re-express the constraint as a `pattern` in the schema** (cross-language change —
  requires sign-off). Replace
  `"propertyNames": { "not": { "const": "default" } }` with
  `"propertyNames": { "pattern": "^(?!default$).*$" }`. **Verified:** typify then generates a
  validating key newtype (`PromptDefinitionVariantsKey`) that rejects `"default"` at deserialize
  time — the rule *does* survive into Rust. Costs: (a) the generated newtype pulls in the
  **`regress`** crate (typify's regex engine, supports the `(?!…)` lookahead that the standard
  `regex` crate does not) and uses `std::sync::LazyLock`; (b) `pattern` semantics differ subtly
  from `not:const` and must be validated against the Python/TS generators for parity; (c) edits the
  shared contract. Only take this if reflecting the rule in the Rust type is worth a new transitive
  dependency.

- **Option 3: report upstream / pin-with-workaround.** The panic is an upstream typify limitation
  (unimplemented `not` handling). Not actionable for T025's timeline; track separately. Do not
  block on an upstream fix.

**Recommendation for T025:** Option 1 (strip `propertyNames` in the Rust codegen step only; enforce
reserved-`default` at the US2 validation gate). It is the only option that requires **zero change to
the shared schema** and **zero new runtime dependency**, and the task explicitly sanctions enforcing
this rule at the validation layer. Surface Option 2 to the team only if they want the constraint in
the Rust type and accept the `regress` dependency + cross-language pattern-parity review.

### F2 (advisory): prefer `--no-builder` for T025

Default builder mode emits 1165 lines (vs 754) of mostly builder boilerplate the library does not
need. Use `--no-builder`. Pure addition/subtraction — the core types are identical.

### F3 (advisory): generated file uses crate-level inner attributes

The output begins with four `#![allow(clippy::…)]` inner attributes. If T025 places the generated
code as a non-root module via `include!`, those inner attributes are illegal at that position. Place
the generated file as its own module file (`mod generated;` -> `generated.rs`) or strip/relocate the
`#![…]` lines. Minor, but it will bite if `include!`d into the middle of a file.

### F4 (advisory): field types are validating newtypes, not primitives

`name` -> `PromptDefinitionName` (not `String`); `variants` key may become a newtype under Option 2.
T025 callers/wrappers must account for the newtype wrappers (they `Deref` to `String` and `impl
From`/`TryFrom`, so ergonomics are fine, but the types are not bare `String`).

---

## Cleanup

All artifacts were written under `/tmp/typify-sample/` (scratch). Nothing was added to `crates/`,
`packages/`, or the schema. `mise.toml` unchanged (pin already correct). This report is the only
committed deliverable.
