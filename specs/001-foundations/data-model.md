# Phase 1 Data Model: Foundations

The **only** data model in spec 001 is the **prompt-definition shape** — authored as the JSON Schema
(the contract; see `contracts/prompt-definition.schema.json`) and code-generated into per-language
shapes. No runtime entities, no persistence (the library does no I/O). This document describes the
shape's fields and rules; the schema file is the normative source.

> Scope note: 001 *defines and generates* this shape. It does **not** consume it — no rendering,
> validation, or variant resolution (specs 002+). Fields like provenance and output-model ref are
> declared here and plumbed later.

## Entity: PromptDefinition (root)

| Field | Type | Required | Notes |
|---|---|---|---|
| `name` | string | yes | Logical prompt name (the caller's reference key). |
| `role` | enum `system` \| `user` \| `assistant` | yes | The conversational role; first-class metadata the caller reads. |
| `body` | string | yes | The default variant's template source. **The root `body` IS the default arm** (FR-011). |
| `variables` | map<string, VariableDecl> | no (default `{}`) | Declared input variables; see VariableDecl. Shared across all variants. |
| `variants` | map<string, Variant> | no | Named alternative arms; see Variant. Absent ⇒ prompt has only the default arm. |
| `output_model` | string | no | **Opaque** reference (e.g. `"NodeOutput"`); stored + echoed, never resolved/parsed. |
| `metadata` | object (open / free-form) | no | Arbitrary prompt-level metadata; **library-opaque** (incl. uninterpreted model/param hints). Maps to `dict[str,Any]` / `Record<string,unknown>` / `serde_json::Value`. |
| `meta` | object (open / free-form) | no | The **default arm's** selection metadata; library-opaque (see Variant.meta). |

Rules:
- `additionalProperties: false` at the root (sealed shape).
- The default arm is surfaced (in later specs' APIs) under reserved name **`default`** with
  `is_default: true`; this is structural, not a schema field.

## Entity: VariableDecl (a `variables` entry)

| Field | Type | Required | Notes |
|---|---|---|---|
| `type` | JSON-Schema type keyword(s) | yes | `string` \| `integer` \| `number` \| `boolean` \| `array` \| `object`. |
| `provenance` | enum `trusted` \| `untrusted` \| `external` | yes | Per-field provenance tag (plumbed/enforced in later specs). |
| (constraints) | JSON-Schema validation keywords | no | `format`, `pattern`, `minimum`, `maximum`, `enum`, `minLength`, … — carried so a later spec can **generate-then-extend** a typed Vars model. |

Rules:
- Rich enough that spec 003+ can generate a Pydantic/Zod/garde Vars model from it (constitution
  generate-then-extend decision). 001 only fixes the declaration shape; it generates the
  *prompt-definition* shape, not per-prompt Vars models.

## Entity: Variant (a `variants` entry)

| Field | Type | Required | Notes |
|---|---|---|---|
| `body` | string | yes | The variant's template source — **the only field that differs per variant**. |
| `meta` | object (open / free-form) | no | Library-**opaque** selection metadata (weight, group, tags, …). Stored + exposed; never interpreted by the library (caller does round-robin/A-B/grouping). |

Rules:
- `additionalProperties: false` on a Variant — it may carry **only** `body` and `meta`. A variant
  MUST NOT redefine `role`, `variables`, or `output_model` (those are shared) — schema rejects extras
  (FR-011a).
- A `variants` key literally named **`default` MUST be rejected** (collision with the structural
  default arm) — FR-011b. Modeled via schema `propertyNames` / `not` const constraint.
- `meta` carries no schema-enforced selection semantics (FR-011c) — it's an open object.

## Validation fixtures (FR-013)

The schema ships with example documents proving the constraints. Accept/reject matrix:

| Fixture | Expectation | Exercises |
|---|---|---|
| single-body prompt (no `variants`) | accept | default-as-root (FR-011) |
| multi-variant prompt | accept | named variants + shared fields |
| invalid `role` value | reject | role enum (FR-009) |
| invalid `provenance` tag | reject | provenance enum (FR-010a) |
| `variants` entry named `default` | reject | reserved-name collision (FR-011b) |
| variant with a `role`/`variables` key | reject | per-variant extras sealed (FR-011a) |
| valid JSON but extra root key | reject | root `additionalProperties:false` |
| not valid JSON/YAML | reject (parse error) | distinct from schema-invalid (edge case) |

## Generated artifacts (FR-014..016)

From the one schema, codegen produces (committed, marked generated, segregated):
- Python: a Pydantic v2 model (`datamodel-code-generator`).
- TypeScript: a type/interface (`json-schema-to-typescript`).
- Rust: a serde struct (`cargo-typify`).

`metadata`/`meta` map to the language's permissive open-object type; `role`/`provenance` to enums;
the sealed objects to `extra='forbid'` / sealed type / `deny_unknown_fields`.
