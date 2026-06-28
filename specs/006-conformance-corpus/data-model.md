# Phase 1 Data Model — 006 Conformance corpus

The corpus has no runtime data model in the library sense (it adds no types to any shipped package). Its
"data model" is the **fixture-file schema** and the **logical-type mapping** the runners share. These are
test artifacts, not library API.

---

## Entity: Marshaling fixture

One JSON file per logical case under `conformance/marshaling/`.

| Field | Type | Required | Meaning |
|---|---|---|---|
| `case` | string | yes | Stable fixture id (e.g. `"date"`, `"decimal"`, `"nested-model"`, `"null-undefined-none"`, `"int-vs-float"`). |
| `description` | string | no | Human note on what the case exercises. |
| `definition` | object | yes | A spec-001 prompt definition (name, role, body, `variables`, optional variants, `meta`). The body's template references the input fields so marshaling differences show up in the render. |
| `variant` | string \| null | no | Variant selector passed to render; `null`/absent → default arm. |
| `input` | object | yes | Map of Vars field name → **typed value descriptor** (see *Typed value descriptor*). |
| `expected` | object | yes | The golden outcome (see *Expected outcome*). |

### Typed value descriptor

Each `input` field is an object `{ "type": <logical-type>, "value": <json> }` so a runner knows which
**native** value to construct.

| `type` | `value` JSON form | Python construction | TS construction | Rust construction |
|---|---|---|---|---|
| `string` | string | `str` | `string` | `String` (in the serde value) |
| `int` | number (integer) | `int` | `number` (integer) | serde integer |
| `float` | number | `float` | `number` | serde float |
| `bool` | boolean | `bool` | `boolean` | serde bool |
| `null` | `null` | `None` | `null` | serde null |
| `absent` | (field omitted from constructed Vars) | key not set | key not set / `undefined` | key absent |
| `datetime` | ISO-8601 string | the string verbatim | the string verbatim | the string verbatim |
| `date` | `YYYY-MM-DD` string | the string verbatim | the string verbatim | the string verbatim |
| `decimal` | decimal-as-string | the string verbatim | the string verbatim (no JS decimal lib) | the string verbatim |
| `object` | object (recursively typed) | nested dict/model | nested object | nested serde map |
| `array` | array (recursively typed) | list | array | serde seq |

The `absent` and `null` rows encode the fixed FR-008 contract: `null`/`None` → JSON `null`; `absent`
(and JS `undefined`) → field-not-present → kernel strict-undefined if referenced.

> **As-built note (canonical serialized form — memory [[D1]]):** the `datetime`/`date`/`decimal` rows were
> originally drafted to construct a *native* object (`datetime.fromisoformat`, `new Date`, `Decimal`), but
> implementation proved native objects do NOT reproduce the golden — each ecosystem's serializer
> recanonicalizes (Pydantic `mode="json"` emits a `Z` suffix and `1E-17`; JS `Date.toISOString()` emits
> `.000Z`). All three runners therefore feed the **canonical serialized string verbatim**, which the
> kernel renders identically. See `conformance/README.md` and `research.md` D1; the TS runner's
> `assertDateDiverges()` proves the divergence at runtime. Do not "fix" a runner to build a native object.

### Expected outcome (the golden)

| Field | Type | Meaning |
|---|---|---|
| `text` | string | The exact rendered output. |
| `template_hash` | string (64-hex) | `SHA256(resolved variant template source)`. |
| `render_hash` | string (64-hex) | `SHA256(rendered text)`. |

The runner asserts: its binding's `RenderResult` text == `expected.text`, `.template_hash`/`.templateHash`
== `expected.template_hash`, `.render_hash`/`.renderHash` == `expected.render_hash`. (Cross-binding parity
is transitive — all three assert against the one committed golden.)

---

## Entity: Schema round-trip fixture

A single `conformance/schema/manifest.json` referencing the existing fixtures plus minimal YAML twins.

| Field | Type | Meaning |
|---|---|---|
| `fixtures` | array | One entry per document. |
| `fixtures[].path` | string | Repo-relative path (e.g. `schemas/jsonschema/fixtures/valid/single-body.json`, or a new `conformance/schema/yaml/…`). |
| `fixtures[].form` | `"json"` \| `"yaml"` | Which binding loader entry to use (`load_json`/`loadJson` vs `load_yaml`/`loadYaml`). |
| `fixtures[].verdict` | `"accept"` \| `"reject"` | Expected outcome across all three bindings. |
| `fixtures[].note` | string (optional) | Why it's valid/invalid (e.g. `"variant named default"`). |

Coverage (reusing the as-built fixtures):
- **accept**: `valid/single-body.json`, `valid/multi-variant.json`, `valid/variant-with-meta.json`,
  plus one **YAML twin** of a valid doc (new, under `conformance/schema/yaml/`).
- **reject**: `invalid/missing-required.json`, `invalid/extra-root-key.json`, `invalid/bad-role.json`,
  `invalid/bad-provenance.json`, `invalid/variant-redefines-role.json`, `invalid/not-json.txt`, plus one
  **YAML twin** of an invalid doc.

> **As-built note (loader vs schema-validator layers — memory [[A1]]):** `invalid/variant-named-default.json`
> was originally listed as a loader `reject` but is **EXCLUDED** from the loader round-trip set. The
> loaders do serde *shape* deserialization, not full JSON-Schema validation; the "no variant named
> `default`" rule is a JSON-Schema `propertyNames` constraint serde cannot model, so the loader ACCEPTS
> it (it is flagged downstream by `check()` as a `ReservedVariantName` finding, not at load).
> `variant-redefines-role` IS loader-rejected (the `Variant` struct is `additionalProperties:false`, so a
> `role` key fails serde). The corpus tests the **loader's** verdict; spec-001's `validate-fixtures` gate
> tests the stricter schema-validator verdict.

Each runner asserts: an `accept` doc loads without error in its binding; a `reject` doc raises the
binding's normalized structured error (`LoadError`/equivalent) — no partial load, no crash (FR-010).

---

## Entity: Logical-type mapping (shared knowledge, per runner)

Not a file — the per-language constructor table above (D2), implemented once in each runner as a small
`type` → native-value switch. The single source of truth for "the same logical input across languages."

---

## Relationships & invariants

- A marshaling fixture's `definition.variables` field names MUST match the `input` keys (the three-sets
  invariant from specs 003/004/005) — otherwise the kernel's strict-undefined fires (which a fixture may
  *intentionally* exercise via an `absent`/`null` field).
- The golden in `expected` is produced by the **Rust reference binding** (D3); the runners only *assert*
  against it, never regenerate it at test time.
- All hashes are lowercase SHA-256 hex (64 chars), stable across OS/arch (FR-004) because they are taken
  over canonical strings (the ISO/decimal canonical forms remove locale/float-format variance).
- No fixture contains secret data (FR-014) — failure output may echo fixture content safely.
