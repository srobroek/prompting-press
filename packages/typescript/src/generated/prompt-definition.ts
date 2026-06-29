/**
 * GENERATED — DO NOT EDIT.
 *
 * Source of truth: schemas/jsonschema/prompt-definition.schema.json
 * Regenerate with: pnpm -C packages/typescript codegen
 *
 * This file is committed and freshness-gated in CI (constitution C-07).
 * Edit the JSON Schema and regenerate; never hand-edit this file.
 */

/**
 * The single source of truth for a prompt's shape (constitution Principle VII / C-07). Per-language shapes (Pydantic v2 / TypeScript types / Rust serde structs) are code-generated from THIS document; it is never hand-mirrored. The library authors, generates, and round-trips this shape; it does not render, validate, or resolve it (Principle III — that lives in specs 002+). $id is a stable identity, not a live endpoint (research D5).
 */
export interface PromptDefinition {
  /**
   * Logical prompt name; the caller's reference key.
   */
  name: string;
  /**
   * Conversational role; first-class metadata the caller reads. Shared across all variants.
   */
  role: "system" | "user" | "assistant";
  /**
   * The DEFAULT variant's template source. The root body IS the default arm (FR-011); surfaced under reserved name 'default' with is_default=true.
   */
  body: string;
  /**
   * Declared input variables, shared across all variants. Each entry declares the variable's type and input-trust origin.
   */
  variables?: {
    [k: string]: PromptVariable;
  };
  /**
   * Named alternative arms. Absent => the prompt has only the default (root body) arm. Each arm differs ONLY in body (+ optional opaque meta).
   */
  variants?: {
    [k: string]: PromptVariant;
  };
  /**
   * Optional OPAQUE reference to the caller's output model (e.g. 'NodeOutput'). Stored and echoed; never resolved, loaded, or parsed (Principle III). Shared across variants.
   */
  output_model?: string;
  /**
   * Arbitrary prompt-level metadata; library-OPAQUE (may include uninterpreted model/param hints, selection labels like weight/group/tags, or a `guard` key). Stored and echoed; never interpreted by the library. The prompt and each variant each carry exactly one `metadata` bag.
   */
  metadata?: {
    [k: string]: unknown;
  };
}
/**
 * A declared input variable: type, origin, and an optional human-readable description. Validation constraints belong in the per-language validator (Zod/Pydantic/garde); the kernel is validation-blind.
 */
export interface PromptVariable {
  /**
   * JSON-Schema type keyword(s) for the variable.
   */
  type:
    | ("string" | "integer" | "number" | "boolean" | "array" | "object")
    | ("string" | "integer" | "number" | "boolean" | "array" | "object" | "null")[];
  /**
   * Per-field input-trust tag. DECLARATIVE METADATA ONLY — the library does not enforce this tag at render time; it is not a security guard by itself. Use `check()` to detect `untrusted`/`external` variables that lack a declared guard, and enable the opt-in guard to receive advisory guard text. This per-variable trust tag is distinct from the render-result content hashes (`template_hash`/`render_hash`).
   */
  origin: "trusted" | "untrusted" | "external";
  /**
   * When true, a validator covering this variable MUST be supplied when the Prompt is constructed (spec 008). Orthogonal to `origin` — it MAY mark any variable, not only untrusted/external ones. Declarative metadata; enforcement is per-language (constitution Principle VI v1.2.0): TypeScript (Zod) and Python (Pydantic) introspect the supplied validator and throw/raise at construction if this variable is uncovered, while Rust guarantees coverage structurally at compile time. The kernel never reads this field (validation-blind).
   */
  validation_required?: boolean;
  /**
   * Optional human-readable description of the variable.
   */
  description?: string;
}
/**
 * A named alternative arm. May carry ONLY body and metadata; redefining role/variables/output_model is rejected (FR-011a).
 */
export interface PromptVariant {
  /**
   * The variant's template source — the only field that differs per variant.
   */
  body: string;
  /**
   * Library-OPAQUE per-variant metadata (selection labels like weight/group/tags, or a `guard` key). Stored + exposed; never interpreted by the library (caller selects). No schema-enforced selection semantics (FR-011c). Mirrors the prompt-level `metadata` bag.
   */
  metadata?: {
    [k: string]: unknown;
  };
}
