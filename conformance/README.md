# Conformance corpus

The **shared, language-neutral conformance corpus** for Prompting Press (spec 006). One fixture set,
read by three thin per-language runners (the Rust consumer, the Python binding, the TypeScript binding),
wired as a CI gate (`moon run ci:conformance`).

## What this corpus guards — and what it does NOT

It guards exactly the two per-binding seams the shared Rust core **cannot** self-verify:

1. **Marshaling parity** — the same logical input pushed through each binding yields **identical rendered
   text** and **identical `template_hash`/`render_hash`**. Exercised over the hard marshaling cases:
   dates, Decimal/high-precision numerics, nested models, `null`/`undefined`/`None`/absent, and
   integer-vs-fractional-float.
2. **Schema round-trip parity** — a schema-valid prompt document is **accepted** identically and a
   schema-invalid one is **rejected** identically across all three bindings, through each binding's own
   loader.

It does **NOT** re-test render parity in general: that is a *structural* property of the single shared
Rust core (constitution Principle I / roadmap C-01), true by construction. The corpus must never grow
comprehensive render-parity fixtures — spec 002's engine-regression set
(`crates/prompting-press-core/tests/fixtures/render/`) is the only render-fixture set, and it is
unchanged and kernel-owned. (See spec `FR-016`.)

## Layout

```text
conformance/
├── README.md                  # this file
├── marshaling/                # marshaling-parity fixtures (one JSON per logical case)
│   ├── date.json
│   ├── decimal.json
│   ├── nested-model.json
│   ├── null-undefined-none.json
│   └── int-vs-float.json
└── schema/
    ├── manifest.json          # maps existing schema fixtures + YAML twins → expected verdict
    └── yaml/                  # minimal YAML twins (1 valid, 1 invalid) for the load_yaml path
```

## Marshaling fixture format

Each `marshaling/<case>.json`:

```jsonc
{
  "case": "date",
  "description": "…",
  "definition": { /* a spec-001 prompt definition; its body references the input fields */ },
  "variant": null,                       // optional variant selector
  "input": {                             // each Vars field tagged with its LOGICAL type
    "when": { "type": "datetime", "value": "2026-06-28T12:00:00+00:00" }
  },
  "expected": {                          // the GOLDEN — generated from the Rust reference binding
    "text": "…rendered output…",
    "template_hash": "<64-hex>",
    "render_hash": "<64-hex>"
  }
}
```

### Typed value descriptor — the `type` tag vocabulary

Each `input` field is `{ "type": <logical-type>, "value": <json> }`. The runner constructs the **native**
value named by `type` (this is what exercises the binding's real marshaling code):

| `type` | `value` form | Python | TypeScript | Rust |
|---|---|---|---|---|
| `string` / `int` / `float` / `bool` / `null` | the JSON value | native | native | serde value |
| `absent` | (field omitted) | key not set | key omitted | key absent |
| `datetime` | ISO-8601 string | **the string** (see note) | **the string** (see note) | the string |
| `date` | `YYYY-MM-DD` | **the string** | **the string** | the string |
| `decimal` | decimal-as-string | **the string** (see note) | string (no JS decimal lib) | the string |
| `object` / `array` | recursively typed | nested | nested | nested serde |

**Canonical serialized form** (clarified Q3): types without a universal native equivalent (date, Decimal)
are defined by the *serialized* form the kernel sees (ISO-8601 string; decimal-as-string), and each runner
feeds **that exact string**. This is the **as-built, empirically-required** choice (not just a convenience):
constructing a *native* object does NOT reproduce the golden, because each ecosystem's serializer
recanonicalizes —
- Python `datetime.fromisoformat("…+00:00")` → Pydantic `mode="json"` dumps `…Z`; `Decimal("0.00000000000000001")` → `1E-17` (scientific). Neither matches the golden.
- JS `new Date("…+00:00").toISOString()` → `….000Z`. Does not match.

So all three runners pass the **raw canonical string** the fixture pins. **Do NOT "fix" a runner to build a
native `datetime`/`Date`/`Decimal` object** — that re-introduces the recanonicalization divergence and
breaks the gate. The corpus is asserting that the *canonical serialized form* round-trips identically; a
binding that cannot reach it from a native object is a known ecosystem limitation, documented here, not a
defect to paper over.

`1.0`-vs-`1` is **excluded** — JavaScript has a single IEEE-754 number type, so `1.0` is indistinguishable
from `1`; the int-vs-float case uses an integer `1` vs a **fractional** float `2.5` instead.

## Schema round-trip fixtures

`schema/manifest.json` lists each document and its expected verdict:

```jsonc
{
  "fixtures": [
    { "path": "schemas/jsonschema/fixtures/valid/single-body.json", "form": "json", "verdict": "accept" },
    { "path": "conformance/schema/yaml/valid-single-body.yaml",      "form": "yaml", "verdict": "accept" },
    { "path": "schemas/jsonschema/fixtures/invalid/bad-role.json",   "form": "json", "verdict": "reject", "note": "role not in enum" }
  ]
}
```

It **reuses** the existing `schemas/jsonschema/fixtures/{valid,invalid}/` set (does not fork it) and adds
one valid + one invalid YAML twin so each binding's `load_yaml` path is exercised (`FR-011`).

## How a runner works (contract)

For each marshaling fixture: load `definition` into a `Registry`, construct the native Vars from the
`type`-tagged `input`, call the binding's **real public render**, and assert text + both hashes equal the
golden. For each schema fixture: feed the doc through the binding's `load_json`/`load_yaml` matching
`form`, and assert accept (loads cleanly) or reject (structured error — assert on error **type/code**,
never on scrubbed detail; no partial load, no crash). Runners **never** regenerate goldens at test time.

The full contract — runner obligations, golden provenance, the CI gate, and the scope guards — is in
[`specs/006-conformance-corpus/contracts/corpus-format.md`](../specs/006-conformance-corpus/contracts/corpus-format.md);
the fixture schema + the logical-type mapping is in
[`specs/006-conformance-corpus/data-model.md`](../specs/006-conformance-corpus/data-model.md); how to run
and extend the corpus is in
[`specs/006-conformance-corpus/quickstart.md`](../specs/006-conformance-corpus/quickstart.md).

## Goldens are generated, not hand-authored

`expected.{text,template_hash,render_hash}` are generated from the **Rust reference binding** by an
`#[ignore]`d generator test (`crates/prompting-press/tests/conformance_goldens.rs`), via
`moon run conformance:regen`. Runners only assert against the committed goldens — so cross-binding parity
is transitive (all three == the one golden), and the golden also trips a lockstep kernel regression. A
golden change is a deliberate, PR-reviewed event; do **not** regenerate to silence a red runner —
investigate the divergence first (it may be the real marshaling bug the corpus exists to catch).
