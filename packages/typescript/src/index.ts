/**
 * Prompting Press — the public TypeScript facade (ESM-only).
 *
 * This module is the package's real entry point. It is a **thin facade** layered over the
 * low-level NAPI-RS addon (the package-root `index.js`/`index.d.ts`, generated from the Rust
 * binding crate `crates/prompting-press-node`). The addon does the marshaling and surfaces the
 * shared Rust core 1:1; this facade adds only the two things that cannot live in Rust:
 *
 *  1. **Zod validation at the render boundary (Q1).** Zod is a TypeScript library, so the
 *     validate-at-render step runs here — `schema.safeParse(data)` before any templating — and
 *     the already-validated plain value is passed across the FFI boundary. Mirrors the spec-004
 *     Python facade (validation owned at the binding boundary; Principle VI).
 *  2. **The JS `Error`-subclass hierarchy (research D4 / C-06).** The addon throws a `napi::Error`
 *     whose `message` is a JSON document `{ code, errors: [{ field, code, message }] }` (already
 *     SEC-004-scrubbed in Rust — see `crates/prompting-press-node/src/error.rs`). This facade
 *     decodes that payload into the right `PromptingPressError` subclass so callers get
 *     `instanceof` + a structured `.errors` array, never a raw napi error or a `ZodError`.
 *
 * Everything else (rendering, hashing, variant resolution, the agreement/provenance lint,
 * composition resolution) is performed once in Rust and surfaced here unchanged (Principle I).
 *
 * Layout: this source compiles to `dist/index.js` (+ `.d.ts`); the package `exports`/`main`/
 * `types` point at it, so `import "prompting-press"` resolves to this facade. The facade imports
 * the addon from the package root (`../index.js`) — the addon is the low-level layer, never the
 * public entry.
 */

import {
  // The low-level NAPI addon surface. The classes/functions whose error paths the facade must
  // decode are imported under `Napi*` aliases and re-wrapped below; the inert ones are re-exported
  // 1:1 (`RenderResult`, `CheckReport`, `check`).
  Registry as NapiRegistry,
  RenderResult,
  CheckReport,
  Composition as NapiComposition,
  check as napiCheck,
  getSource as napiGetSource,
  coreVersion,
  render as napiRender,
  type Finding,
  type GuardConfig,
  type Message,
  type MessageEntry,
} from "../index.js";

// The generated, freshness-gated prompt-definition shape (constitution C-07; never hand-edited).
import type { PromptDefinition } from "./generated/prompt-definition.js";

// --------------------------------------------------------------------------------------
// The normalized cross-language error contract (Principle VII / C-06): `[{field, code, message}]`.
// --------------------------------------------------------------------------------------

/**
 * One normalized failure row. The TS mirror of the Rust consumer's `FieldError` and the
 * cross-language error contract. Reconstructed by the facade from the addon's JSON payload (or,
 * for a Zod validation failure, built locally from `issue.message` + `issue.path`).
 */
export interface FieldError {
  /** The offending field or dotted path; `""` when no single field applies. */
  readonly field: string;
  /** A stable code from the consumer's closed vocabulary (e.g. `"validation"`, `"render"`). */
  readonly code: string;
  /** A human-readable, SEC-004-scrubbed message safe to log (never the rejected value). */
  readonly message: string;
}

// --------------------------------------------------------------------------------------
// The Error hierarchy (research D4). One base + four leaves; selected by the payload's
// top-level `code`. Native types (`ZodError`, the raw `napi::Error`) never cross this surface.
// --------------------------------------------------------------------------------------

/**
 * The base class for every failure the library raises. Carries the normalized, already-scrubbed
 * `errors` rows. Every thrown error is an `instanceof PromptingPressError`; callers branch on the
 * concrete subclass (or on `err.errors[*].code`) and never see a `ZodError` or a native napi error.
 */
export class PromptingPressError extends Error {
  /** The structured, SEC-004-scrubbed failure rows (the cross-language contract). */
  readonly errors: readonly FieldError[];

  constructor(message: string, errors: readonly FieldError[]) {
    super(message);
    // `name` defaults to the constructed class name; set explicitly so it survives subclassing
    // and shows correctly in stack traces.
    this.name = new.target.name;
    this.errors = errors;
    // Restore the prototype chain for reliable `instanceof` when compiled to older targets.
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

/**
 * Raised when typed input fails validation — either the facade's Zod `safeParse` (before any
 * templating) or a garde-class validation surfaced by the consumer. Top-level code `"validation"`.
 * It is **not** a `ZodError` (SC-006): the facade copies only the value-free issue message + path.
 */
export class PromptValidationError extends PromptingPressError {}

/**
 * Raised when the kernel rejects a render: an unknown variant, a referenced-but-undefined root
 * variable (the loud three-sets gap — never a silent empty render), a parse/render failure, or an
 * excluded template feature. Maps the kernel codes `unknown_variant` / `undefined_variable` /
 * `parse` / `render` / `excluded_feature`. `parse`/`render`/`excluded_feature` detail is scrubbed.
 */
export class PromptRenderError extends PromptingPressError {}

/** Raised when a prompt name is absent from the registry. Top-level code `"unknown_prompt"`. */
export class UnknownPromptError extends PromptingPressError {}

/**
 * Raised when a document fails to load: malformed YAML/JSON, or a prompt-definition shape
 * violation. Nothing is partially loaded (FR-007). Top-level code `"load"`.
 */
export class LoadError extends PromptingPressError {}

// --------------------------------------------------------------------------------------
// The addon-payload decoder (research D4). The addon throws a `napi::Error` whose `message` is the
// JSON `{ code, errors }` document; we parse it and pick the subclass by the top-level `code`.
// --------------------------------------------------------------------------------------

/** The decoded shape of the addon's JSON error payload (see `crates/prompting-press-node/src/error.rs`). */
interface DecodedPayload {
  code: string;
  errors: FieldError[];
}

/**
 * Map a top-level payload `code` to its `PromptingPressError` subclass constructor.
 *
 * The closed vocabulary the Rust consumer emits (`prompting_press::error::code`):
 *  - `validation`                                   → `PromptValidationError`
 *  - `unknown_prompt`                               → `UnknownPromptError`
 *  - `load`                                         → `LoadError`
 *  - the kernel codes `unknown_variant` / `undefined_variable` / `parse` / `render` /
 *    `excluded_feature`                             → `PromptRenderError`
 *
 * An unrecognized code (not produced by the consumer today) falls back to the base
 * `PromptingPressError` so a future code is never silently given the wrong subclass.
 */
function subclassForCode(
  code: string,
): new (message: string, errors: readonly FieldError[]) => PromptingPressError {
  switch (code) {
    case "validation":
      return PromptValidationError;
    case "unknown_prompt":
      return UnknownPromptError;
    case "load":
      return LoadError;
    case "unknown_variant":
    case "undefined_variable":
    case "parse":
    case "render":
    case "excluded_feature":
      return PromptRenderError;
    default:
      return PromptingPressError;
  }
}

/**
 * Decode a value thrown by the NAPI addon into the matching `PromptingPressError` subclass.
 *
 * The addon's thrown error carries the structured payload as JSON in its `.message` (napi sets
 * `message` from the Rust `reason`). We `JSON.parse` it, read `.code` to choose the subclass, and
 * surface `.errors`. The payload is already SEC-004-scrubbed in Rust, so no value content is read.
 *
 * If the thrown value is not the expected JSON shape (it always should be — every consumer error
 * path is exhaustively mapped in Rust), it is wrapped in the base `PromptingPressError` with a
 * single row rather than re-thrown raw, so a native napi error never escapes the public surface.
 */
function decodeAddonError(thrown: unknown): PromptingPressError {
  // Already one of ours (e.g. a Zod failure thrown by the facade) — pass through unchanged.
  if (thrown instanceof PromptingPressError) {
    return thrown;
  }

  const rawMessage =
    thrown instanceof Error
      ? thrown.message
      : typeof thrown === "string"
        ? thrown
        : String(thrown);

  let payload: DecodedPayload | undefined;
  try {
    const parsed: unknown = JSON.parse(rawMessage);
    if (
      typeof parsed === "object" &&
      parsed !== null &&
      typeof (parsed as { code?: unknown }).code === "string" &&
      Array.isArray((parsed as { errors?: unknown }).errors) &&
      // Validate each row's shape, not just the envelope (security review M1): the rows flow
      // out on `.errors` and into the summary. Today the Rust side is the only producer and is
      // exhaustively typed, so this never fails — it is defense-in-depth so a future/foreign
      // payload with a well-formed `code` but malformed rows cannot surface a non-`FieldError`.
      (parsed as { errors: unknown[] }).errors.every(
        (row) =>
          typeof row === "object" &&
          row !== null &&
          typeof (row as FieldError).field === "string" &&
          typeof (row as FieldError).code === "string" &&
          typeof (row as FieldError).message === "string",
      )
    ) {
      payload = parsed as DecodedPayload;
    }
  } catch {
    // Not JSON — fall through to the defensive wrapper below.
  }

  if (payload === undefined) {
    // Defensive: never let a non-conforming native error cross the public surface raw.
    return new PromptingPressError(rawMessage, [
      { field: "", code: "render", message: rawMessage },
    ]);
  }

  const Subclass = subclassForCode(payload.code);
  // Re-summarize the rows into the JS `Error.message` for human readability, while `.errors`
  // carries the structured rows. Keep the (scrubbed) row messages; never inject value content.
  const summary =
    payload.errors.length > 0
      ? payload.errors.map((row) => row.message).join("; ")
      : payload.code;
  return new Subclass(summary, payload.errors);
}

// --------------------------------------------------------------------------------------
// Registry (US2) — the facade wrapper whose loader methods decode addon errors into LoadError.
//
// The low-level napi `Registry.loadYaml/loadJson/insert` throw a RAW `napi::Error` whose `message`
// is the `{code:"load", ...}` JSON payload — NOT a `LoadError`. Re-exporting the napi class 1:1
// would leak that raw error onto the public surface (a plain `Error`, failing `instanceof
// LoadError`). So the public Registry is this facade class: it owns a napi registry and decodes
// each loader method's error through the same payload decoder used for render/resolve. The napi
// registry instance is read back (via the module-private accessor below) when handed to
// `render`/`check`/`getSource`/`Composition.resolve`.
// --------------------------------------------------------------------------------------

/** Module-private slot holding each facade Registry's underlying napi registry. */
const NAPI_REGISTRY = Symbol("napiRegistry");

/**
 * A library-owned map of prompt name → loaded definition (US2). All three input paths normalize
 * through the **one** Rust consumer loader (Q3); YAML↔JSON↔object parity is structural (Principle I).
 *
 * A malformed document (bad YAML/JSON, or a prompt-definition shape violation) throws a
 * {@link LoadError} and inserts **nothing** (FR-007). The loader methods decode the addon's raw
 * error into the facade hierarchy so callers branch on `instanceof LoadError`, never a native error.
 */
export class Registry {
  /** The wrapped low-level napi registry — the single source of truth for the loaded prompts. */
  readonly [NAPI_REGISTRY]: NapiRegistry;

  constructor() {
    this[NAPI_REGISTRY] = new NapiRegistry();
  }

  /**
   * Load a prompt definition from an already-read **YAML** document, keyed by its `name` (an
   * existing entry with the same name is replaced). The binding parses no YAML itself — the text
   * is marshaled to the Rust consumer, so accept/reject and YAML↔JSON parity are structural (Q3).
   *
   * @throws {LoadError} if `text` is not valid YAML or does not match the prompt-definition shape;
   *   nothing is inserted (FR-007).
   */
  loadYaml(text: string): void {
    try {
      this[NAPI_REGISTRY].loadYaml(text);
    } catch (thrown) {
      throw decodeAddonError(thrown);
    }
  }

  /**
   * Load a prompt definition from an already-read **JSON** document, keyed by its `name` (replace
   * on duplicate name). The binding parses nothing; the text is marshaled to the consumer.
   *
   * @throws {LoadError} if `text` is not valid JSON or does not match the shape (FR-007).
   */
  loadJson(text: string): void {
    try {
      this[NAPI_REGISTRY].loadJson(text);
    } catch (thrown) {
      throw decodeAddonError(thrown);
    }
  }

  /**
   * Insert a constructed prompt-definition object (the {@link PromptDefinition} shape, FR-005 third
   * path), keyed by its `name`. It is re-serialized to JSON and fed to the **same** consumer loader
   * as the text paths — one loader, one representation, no parallel shape (Q3 / FR-008).
   *
   * @throws {LoadError} if `definition` does not match the prompt-definition shape (FR-007).
   */
  insert(definition: PromptDefinition | Record<string, unknown>): void {
    try {
      this[NAPI_REGISTRY].insert(definition);
    } catch (thrown) {
      throw decodeAddonError(thrown);
    }
  }
}

/** Read a facade {@link Registry}'s underlying napi registry for the addon-level calls. */
function napiRegistryOf(reg: Registry): NapiRegistry {
  return reg[NAPI_REGISTRY];
}

// --------------------------------------------------------------------------------------
// The Zod validation boundary (Q1 / research D3 / SEC-004).
// --------------------------------------------------------------------------------------

/**
 * The minimal structural contract this facade needs from a Zod schema: a `safeParse` returning a
 * tagged result. Typing it structurally (rather than importing Zod's concrete `ZodType`) keeps the
 * facade decoupled from a specific Zod minor and lets a caller pass any object exposing the same
 * shape — the library never depends on Zod's identity, only on `safeParse` (Principle VI).
 */
export interface ZodLikeSchema<T = unknown> {
  safeParse(data: unknown): ZodSafeParseResult<T>;
}

/** The structural shape of a Zod `safeParse` result (success | failure with `issues`). */
type ZodSafeParseResult<T> =
  | { success: true; data: T }
  | { success: false; error: { issues: readonly ZodLikeIssue[] } };

/** The structural shape of a single Zod issue — only the value-free fields the facade reads. */
interface ZodLikeIssue {
  /** The dotted path segments to the offending field. */
  path: ReadonlyArray<PropertyKey>;
  /** The human-readable, value-free validation message. */
  message: string;
}

/**
 * Type guard: does `value` expose a `safeParse` method (i.e. is it a schema, not plain data)?
 * Distinguishes the `render(reg, name, schema, data)` form (Q1) from the static
 * `render(reg, name, data)` form (Q4) by structural duck-typing.
 */
function isSchema(value: unknown): value is ZodLikeSchema {
  return (
    typeof value === "object" &&
    value !== null &&
    typeof (value as { safeParse?: unknown }).safeParse === "function"
  );
}

/**
 * Run a schema's `safeParse` and, on failure, throw a `PromptValidationError`.
 *
 * SEC-004 / research D3: each row copies **only** `issue.message` and `issue.path.join(".")` →
 * `field`. The rejected value (`issue.input`) is **never** read or surfaced. `code` is the stable
 * `"validation"` discriminant for every row (Zod carries no machine code; the consumer assigns one
 * — matched here so the cross-language `[{field, code, message}]` contract holds).
 *
 * @returns the validated, plain payload to hand across the FFI boundary.
 */
function validateOrThrow<T>(schema: ZodLikeSchema<T>, data: unknown): T {
  const result = schema.safeParse(data);
  if (result.success) {
    return result.data;
  }
  const rows: FieldError[] = result.error.issues.map((issue) => ({
    field: issue.path.map((segment) => String(segment)).join("."),
    code: "validation",
    message: issue.message,
  }));
  const summary = rows.map((row) => row.message).join("; ") || "validation failed";
  throw new PromptValidationError(summary, rows);
}

// --------------------------------------------------------------------------------------
// render (US1) — the validate-at-render wrapper (Q1 / Q4).
// --------------------------------------------------------------------------------------

/**
 * Render a prompt's resolved variant with typed inputs, validating first (Q1).
 *
 * Two call forms, selected by the third argument:
 *
 *  - **Schema form (Q1):** `render(reg, name, schema, data, opts?)`. `schema.safeParse(data)` runs
 *    here, before any templating; on failure a {@link PromptValidationError} is thrown and the
 *    kernel is **never** reached (no render happens). On success the validated plain value is
 *    marshaled to the addon.
 *  - **Static form (Q4):** `render(reg, name, data, opts?)`. Already-typed plain data with no Zod
 *    schema — marshaled directly. (The third argument is the data; the fourth is `opts`.)
 *
 * `opts` is an optional `{ variant?, guard? }` object — the TS-idiomatic equivalent of Python's
 * `variant=` / `guard=` keyword arguments (constitution Principle VI: uniform capability, native
 * idiom). It carries:
 *  - `variant` — select a named variant arm (Principle V / FR-009; caller-owned). Absent ⇒ the
 *    reserved `default` arm. An unknown name ⇒ a {@link PromptRenderError} with `code:
 *    "unknown_variant"`.
 *  - `guard` — the opt-in {@link GuardConfig}, plumbed straight through: absent / `{ enabled: false }`
 *    ⇒ a plain render and `RenderResult.guard === null`. The facade adds no guard logic; the kernel
 *    populates `RenderResult.guard` when enabled and the prompt declares an untrusted/external field.
 *
 * Any addon error is decoded into the matching {@link PromptingPressError} subclass:
 * `UnknownPromptError` (name absent — thrown before marshaling, nothing rendered), or a
 * {@link PromptRenderError} for a kernel rejection (`unknown_variant` / `undefined_variable` /
 * `parse` / `render` / `excluded_feature`; SEC-004 scrubs parse/render/excluded detail).
 *
 * @param reg    the registry to resolve `name` against.
 * @param name   the prompt name.
 * @param schemaOrData a Zod-like schema (schema form) **or** the already-typed plain data (static form).
 * @param dataOrOpts   the data to validate (schema form) **or** the `opts` object (static form).
 * @param opts   `{ variant?, guard? }` (schema form only).
 */
export interface RenderOptions {
  /** Select a named variant arm; absent ⇒ the reserved `default` arm (FR-009 / Principle V). */
  variant?: string;
  /** Opt-in guard config; absent / `{ enabled: false }` ⇒ a plain render (`RenderResult.guard === null`). */
  guard?: GuardConfig | null;
}
export function render<T>(
  reg: Registry,
  name: string,
  schema: ZodLikeSchema<T>,
  data: unknown,
  opts?: RenderOptions | null,
): RenderResult;
export function render(
  reg: Registry,
  name: string,
  data: unknown,
  opts?: RenderOptions | null,
): RenderResult;
export function render(
  reg: Registry,
  name: string,
  schemaOrData: unknown,
  dataOrOpts?: unknown,
  opts?: RenderOptions | null,
): RenderResult {
  let value: unknown;
  let options: RenderOptions | null | undefined;

  if (isSchema(schemaOrData)) {
    // Schema form (Q1): validate `dataOrOpts` against the schema; `opts` is the 5th arg.
    value = validateOrThrow(schemaOrData, dataOrOpts);
    options = opts;
  } else {
    // Static form (Q4): `schemaOrData` IS the plain data; `dataOrOpts` is the `opts` object.
    value = schemaOrData;
    options = dataOrOpts as RenderOptions | null | undefined;
  }

  try {
    // napi `render(reg, name, value, variant?, guard?)`. Variant + guard are caller-owned via `opts`
    // (the TS analogue of Python's variant=/guard= kwargs); the facade adds no engine logic, it only
    // forwards them. Absent variant ⇒ the kernel's reserved default arm.
    return napiRender(
      napiRegistryOf(reg),
      name,
      value,
      options?.variant ?? undefined,
      options?.guard ?? undefined,
    );
  } catch (thrown) {
    throw decodeAddonError(thrown);
  }
}

// --------------------------------------------------------------------------------------
// getSource (US1) + check (US3) — the two remaining registry-reading entry points.
// --------------------------------------------------------------------------------------

/** Options for {@link getSource}. */
export interface GetSourceOptions {
  /** Select a named variant arm; absent ⇒ the reserved `default` arm. */
  variant?: string;
}

/**
 * Return a prompt variant's **unrendered** template source (FR-010). Pure source lookup: no vars,
 * no validation, no marshaling. `opts.variant` selects an arm (absent ⇒ the reserved `default`).
 *
 * @throws {UnknownPromptError} if `name` is absent from `reg`.
 * @throws {PromptRenderError} for a kernel rejection (e.g. an unknown variant).
 */
export function getSource(reg: Registry, name: string, opts?: GetSourceOptions | null): string {
  try {
    return napiGetSource(napiRegistryOf(reg), name, opts?.variant ?? undefined);
  } catch (thrown) {
    throw decodeAddonError(thrown);
  }
}

/**
 * Run the agreement + provenance lint over `reg` (the headline check, US3) and return the report.
 *
 * **Pure** (FR-019): never mutates the registry, never renders, no side effects. The analysis is
 * performed once in Rust (Principle I/IV); this only unwraps the facade registry and surfaces the
 * consumer's {@link CheckReport} unchanged, preserving its deterministic finding order. An empty
 * registry yields an empty, passing report (`report.passed() === true`).
 */
export function check(reg: Registry): CheckReport {
  return napiCheck(napiRegistryOf(reg));
}

// --------------------------------------------------------------------------------------
// Composition (US4) — the TS facade wrapper that validates each entry before the addon.
// --------------------------------------------------------------------------------------

/**
 * One composition entry, as an **options object** (codebase convention: named fields over positional
 * tuples — this also removes the `[name, schema, data]`-vs-`[name, data]` shape ambiguity that a
 * positional tuple forces a reader/parser to duck-type):
 *
 *  - `name` — the prompt's registry name (resolved at {@link Composition.resolve}, not at append).
 *  - `schema` — an optional Zod-like schema. Present ⇒ `schema.safeParse(data)` runs at append
 *    (schema form, Q1); absent ⇒ `data` is marshaled directly (static form, Q4).
 *  - `data` — the vars value (validated against `schema` when present).
 *  - `variant` — the selected variant arm (absent ⇒ the reserved `default`).
 */
export interface CompositionEntry {
  name: string;
  schema?: ZodLikeSchema;
  data: unknown;
  variant?: string;
}

/**
 * An explicit, ordered composition of `(prompt, vars, variant?)` entries that resolves to an
 * ordered `Message[]` (FR-012). The **public** Composition is this TS-facade class (critique E2):
 * it owns the Zod validation of each entry before handing the validated value to the low-level
 * addon `Composition`. There is **no** fluent `.chain()` (FR-013) — `append` returns `void`.
 *
 * No-partial guarantee (FR-013): if any entry fails validation at construction/append, the whole
 * call throws and **nothing** is stored; if an entry fails at `resolve` (unknown prompt, unknown
 * variant, undefined variable, parse/render), `resolve` throws and the partial result is discarded.
 */
export class Composition {
  /** The wrapped low-level addon composition; entries are appended in order after validation. */
  readonly #inner: NapiComposition;

  constructor() {
    this.#inner = new NapiComposition();
  }

  /**
   * Build a composition from an ordered array of {@link CompositionEntry} objects, validating each
   * in order. The first entry whose `schema.safeParse(data)` fails throws a
   * {@link PromptValidationError} and **no** `Composition` is returned (no partial state — FR-013).
   * The prompt `name` is **not** resolved here; an unknown name surfaces at {@link resolve}.
   */
  static fromMessages(entries: readonly CompositionEntry[]): Composition {
    const composition = new Composition();
    for (const entry of entries) {
      composition.append(entry);
    }
    return composition;
  }

  /**
   * Marshal + store one {@link CompositionEntry}. When `entry.schema` is present, validation runs
   * here (`schema.safeParse(entry.data)`); on failure a {@link PromptValidationError} is thrown and
   * nothing is stored. Returns `void` (not `this`): intentionally **not** fluent/chainable (FR-013).
   */
  append(entry: CompositionEntry): void {
    const value =
      entry.schema === undefined ? entry.data : validateOrThrow(entry.schema, entry.data);
    this.#inner.append(entry.name, value, entry.variant);
  }

  /** The number of appended entries (== the resolved-message count on success). */
  get length(): number {
    return this.#inner.length;
  }

  /**
   * Resolve the composition to an ordered `Message[]` (FR-012), rendering each entry in append
   * order through the kernel. One entry's failure (unknown prompt, unknown variant, undefined
   * variable, parse/render) throws the mapped {@link PromptingPressError} subclass and the partial
   * result is discarded — never returned as success. An empty composition resolves to `[]`.
   */
  resolve(reg: Registry): Message[] {
    try {
      return this.#inner.resolve(napiRegistryOf(reg));
    } catch (thrown) {
      throw decodeAddonError(thrown);
    }
  }
}

// --------------------------------------------------------------------------------------
// Re-exports: the inert addon classes/functions surfaced 1:1, plus the generated shape.
// (`Registry`, `render`, `getSource`, `check`, `Composition` are the facade-wrapped versions
// defined above; `RenderResult`/`CheckReport` are read-only result types surfaced unchanged, and
// `coreVersion` is a trivial callable with no error path.)
// --------------------------------------------------------------------------------------

export {
  // Read-only result classes + the trivial version probe, surfaced unchanged (Principle I).
  RenderResult,
  CheckReport,
  coreVersion,
};

export type { Finding, GuardConfig, Message, MessageEntry, PromptDefinition };
