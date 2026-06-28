### 2026-06-28 — The binding loaders do serde shape-validation, NOT full JSON-Schema validation

**ID**: A1

**Status**
Active

**Why this is durable**
This is a load-bearing layer distinction that resurfaces whenever someone reasons about "is this prompt
document valid?" The answer differs by layer, and conflating them produces wrong test expectations (it
already did, once — see Evidence). It governs all future loader work and any test that drives the
`load_json`/`load_yaml` path.

**Decision / fact**
A document's validity is enforced at THREE different layers, and they are NOT equivalent:
1. **JSON-Schema validation** (`schemas/jsonschema/scripts/validate_fixtures.py`, spec 001) — the strict
   layer; enforces `propertyNames`, structural rules, the full schema.
2. **Binding loaders** (`Registry::load_json`/`load_yaml` and the Python/TS equivalents) — do **serde
   shape deserialization** into the generated `PromptDefinition` struct. They reject anything serde can't
   shape (missing required field, unknown root key via `additionalProperties:false`, bad enum value,
   a `Variant` carrying an unknown key like `role`), but they **accept** constraints serde cannot model.
3. **`check()`** (spec 003) — flags the semantic rules the loader can't, e.g. a variant named `default`
   (a `ReservedVariantName` finding).

So a `default`-named variant is **schema-invalid but loader-accepted**: the JSON-Schema `propertyNames`
rule forbidding it has no serde equivalent (variants are a map; serde happily deserializes a `default`
key), and it's caught downstream by `check()`, not at load.

**Evidence**
The spec-006 conformance schema runner initially listed `variant-named-default.json` as a loader `reject`
(because it lives in `schemas/jsonschema/fixtures/invalid/`); the Rust runner failed, surfacing that the
loader ACCEPTS it. An empirical probe of all 7 invalid fixtures confirmed: 6 are loader-rejected,
`variant-named-default` is the lone loader-accept. The conformance manifest now excludes it from the
loader round-trip set with a documented note. (`variant-redefines-role` IS loader-rejected — the `Variant`
struct is `additionalProperties:false` over `{body,meta}`, so a `role` key fails serde.)

**Tradeoffs**
- Gained: a correct, binding-observable definition of "schema round-trip" for the conformance corpus.
- Made harder: "is X rejected?" must specify *at which layer*. A test of the loader must encode the
  loader's verdict, not the schema-validator's (which is stricter).
- Reconsider: if the loaders ever gain full JSON-Schema validation (none planned), the layer collapses and
  the manifest exclusion can be revisited.

**Where to look next**
`conformance/schema/manifest.json` (the description documents the layer distinction + the exclusion),
`crates/prompting-press/src/registry.rs` (`load_json`/`load_yaml` = serde only), `crates/prompting-press/
src/check.rs` (`ReservedVariantName`), `schemas/jsonschema/scripts/validate_fixtures.py` (the strict layer).
