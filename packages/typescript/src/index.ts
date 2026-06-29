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
 *  3. **The `Prompt` class (spec 008, T042–T045).** A validated, immutable object wrapping a
 *     `NapiPrompt` handle. Construction is validating (`new Prompt()` throws on invalid),
 *     read-only accessors are getters, and `with(overlay)` is the sole mutator (re-validates
 *     the merged whole). The optional `validators` argument enables `validation_required`
 *     coverage checking at construction via `ZodObject.shape` (R2 / T043).
 *  4. **The `Composition` class (spec 008, T046).** Updated to aggregate `Prompt` objects
 *     (not names against a Registry). `resolve()` takes no arguments; each entry holds an owned
 *     `Prompt` and its own already-validated value.
 *
 * **Removed (spec 008 reshape):** `Registry`, `render(reg, name, …)`, `getSource(reg, …)`,
 * `check(reg)`. The `Prompt` object is now the primary surface (T046).
 *
 * Everything else (rendering, hashing, variant resolution, the agreement lint) is performed
 * once in Rust and surfaced here unchanged (Principle I).
 *
 * Layout: this source compiles to `dist/index.js` (+ `.d.ts`); the package `exports`/`main`/
 * `types` point at it, so `import "prompting-press"` resolves to this facade. The facade imports
 * the addon from the package root (`../index.js`) — the addon is the low-level layer, never the
 * public entry.
 */

import {
	CheckReport,
	coreVersion,
	type Finding,
	type GuardConfig,
	type Message,
	// Prompt-level addon surface (spec 008 Phase 5 additions).
	type NapiPrompt,
	promptFromJson,
	promptFromToml,
	promptFromYaml,
	promptNew,
	// Result types — surfaced unchanged.
	RenderResult,
} from "../index.js";

// The generated, freshness-gated prompt-definition shape (constitution C-07; never hand-edited).
import type {
	PromptDefinition,
	PromptVariable,
	PromptVariant,
} from "./generated/prompt-definition.js";

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

/**
 * Raised when a document fails to load: malformed YAML/JSON/TOML, or a prompt-definition shape
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
 *  - `load`                                         → `LoadError`
 *  - the kernel codes `unknown_variant` / `undefined_variable` / `parse` / `render` /
 *    `excluded_feature`                             → `PromptRenderError`
 *
 * An unrecognized code (not produced by the consumer today) falls back to the base
 * `PromptingPressError` so a future code is never silently given the wrong subclass.
 */
function subclassForCode(
	code: string,
): new (
	message: string,
	errors: readonly FieldError[],
) => PromptingPressError {
	switch (code) {
		case "validation":
			return PromptValidationError;
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
// The Zod validation boundary (Q1 / research D3 / SEC-004).
// --------------------------------------------------------------------------------------

/**
 * The minimal structural contract this facade needs from a Zod schema: a `safeParse` returning a
 * tagged result. Typing it structurally (rather than importing Zod's concrete `ZodType`) keeps the
 * facade decoupled from a specific Zod minor and lets a caller pass any object exposing the same
 * shape — the library never depends on Zod's identity, only on `safeParse` (Principle VI).
 *
 * The optional `shape` field supports the `validation_required` coverage check at construction
 * (T043 / R2): if present (it is on a `ZodObject`), `field in schema.shape` proves that a
 * `validation_required` variable is covered. If absent, the coverage check is skipped with a
 * documented "cannot assert coverage" limitation.
 */
export interface ZodLikeSchema<T = unknown> {
	safeParse(data: unknown): ZodSafeParseResult<T>;
	/**
	 * Optional: `ZodObject.shape` — a record of field name → ZodType (Zod 4.4.3 API, research R2).
	 * Present on a `ZodObject`; absent on other schema types. When absent, `validation_required`
	 * coverage cannot be introspected and the check is skipped (documented limitation).
	 */
	shape?: Record<string, unknown>;
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
 * Type guard: does `value` expose a `safeParse` method — i.e. is it a schema, not plain data?
 * Used in `Prompt.render()` overload dispatch.
 */
function isZodLikeSchema(value: unknown): value is ZodLikeSchema {
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
	const summary =
		rows.map((row) => row.message).join("; ") || "validation failed";
	throw new PromptValidationError(summary, rows);
}

// --------------------------------------------------------------------------------------
// ValidatorMap — the optional Zod validator bound at Prompt construction.
// --------------------------------------------------------------------------------------

/**
 * A validator bound at `Prompt` construction. In practice this is a Zod schema (a `ZodObject`)
 * whose `shape` property the construction-time `validation_required` coverage check introspects.
 * Typed as `ZodLikeSchema` so callers aren't forced to import Zod types.
 */
export type ValidatorMap = ZodLikeSchema;

// --------------------------------------------------------------------------------------
// RenderOptions — options object for Prompt.render().
// --------------------------------------------------------------------------------------

/**
 * Options for {@link Prompt.render} (C-11: named-field options object over positional args).
 */
export interface RenderOptions {
	/** Select a named variant arm; absent ⇒ the reserved `default` arm (FR-009 / Principle V). */
	variant?: string;
	/** Opt-in guard config; absent / `{ enabled: false }` ⇒ a plain render (`RenderResult.guard === null`). */
	guard?: GuardConfig | null;
}

// --------------------------------------------------------------------------------------
// coverage-check helpers (T043 / R2 — validation_required + ZodObject.shape).
// --------------------------------------------------------------------------------------

/**
 * Check that every variable with `validation_required: true` in `variables` is covered by
 * `validators.shape`. Throws a {@link PromptValidationError} naming the first uncovered variable.
 *
 * Skipped when `validators` is absent or `validators.shape` is absent (R2 documented limitation:
 * "cannot assert coverage" when the schema does not expose `.shape`).
 */
function assertValidatorCoverage(
	variables: Record<string, unknown> | undefined,
	validators: ValidatorMap | undefined,
): void {
	if (validators === undefined || validators.shape === undefined) {
		return; // no introspectable schema → skip (R2 documented limitation)
	}
	if (variables === undefined) {
		return; // no declared variables → nothing to check
	}
	for (const [fieldName, decl] of Object.entries(variables)) {
		const variableDecl = decl as Partial<PromptVariable>;
		if (
			variableDecl.validation_required === true &&
			!(fieldName in validators.shape)
		) {
			const msg = `validation_required variable "${fieldName}" is not covered by the supplied validators schema`;
			throw new PromptValidationError(msg, [
				{ field: fieldName, code: "validation", message: msg },
			]);
		}
	}
}

// --------------------------------------------------------------------------------------
// Module-private symbols — defined before Prompt to avoid temporal dead zone errors.
// --------------------------------------------------------------------------------------

/**
 * Module-private Symbol used to access a `Prompt`'s underlying `NapiPrompt` handle from
 * `Composition` (which lives in the same module). Not exported.
 */
const PROMPT_HANDLE_KEY = Symbol("promptHandle");

/**
 * Module-private runtime key for the internal construction token object.
 * Combined with the type-brand below, only code in this module can construct an
 * `InternalCtorArg` value, making the third Prompt constructor parameter opaque to callers.
 */
const _CTOR_KEY = Symbol("promptInternalCtor");

/** Type brand that prevents external code from forming a valid `InternalCtorArg`. */
declare const _CTOR_BRAND: unique symbol;

/**
 * The internal construction token passed as the third argument to `new Prompt(...)` by the
 * static factories and `with()`. Its type uses a `unique symbol` brand so TypeScript rejects
 * any attempt to construct it outside this module.
 */
type InternalCtorArg = { readonly [_CTOR_BRAND]: true; handle: NapiPrompt };

/** Construct an `InternalCtorArg` — module-private; the type is opaque to callers. */
function makeInternalArg(handle: NapiPrompt): InternalCtorArg {
	return { [_CTOR_KEY]: true, handle } as unknown as InternalCtorArg;
}

// --------------------------------------------------------------------------------------
// Prompt (spec 008, T042–T045) — the primary public type post-reshape.
// --------------------------------------------------------------------------------------

/**
 * An immutable, fully-validated prompt.
 *
 * Wraps a `NapiPrompt` handle; all construction invariants (shape-valid, template-parseable,
 * agreement-sound, reserved-name clean) are enforced by the Rust consumer at construction time
 * (Principle I / T042). There are no setters; the sole mutator is {@link Prompt.derive} (T045).
 *
 * ## Construction — four entry points, all throwing on invalid input (Q6)
 *
 * ```ts
 * const p = new Prompt(shape, validators?);
 * const p = Prompt.fromYaml(text, validators?);
 * const p = Prompt.fromJson(text, validators?);
 * const p = Prompt.fromToml(text, validators?);   // TOML routed to Rust, no smol-toml dep
 * ```
 *
 * ## validators? — validation_required coverage check (T043 / R2)
 *
 * When supplied, any variable with `validation_required: true` must appear in `validators.shape`.
 * Construction throws a {@link PromptValidationError} if a required variable is uncovered.
 * When `validators.shape` is absent (non-`ZodObject` schema), the check is skipped.
 *
 * ## render() — validate-then-render (T044 / Q1)
 *
 * ```ts
 * p.render(schema, data, opts?);   // schema form: safeParse before templating
 * p.render(data, opts?);           // static form (or uses bound validators when present)
 * ```
 *
 * ## derive(overlay, validators?) — sole mutator (T045 / R6)
 *
 * Shallow-replaces top-level fields; re-validates the merged whole. Validators carry forward
 * from the source by default (R6); pass `validators` to override.
 */
export class Prompt {
	/** The underlying napi handle. Private — never exposed outside this class. */
	readonly #handle: NapiPrompt;
	/** The bound validator (if any) stored for render() and derive(). */
	readonly #validators: ValidatorMap | undefined;

	/**
	 * Primary public constructor — constructs a `Prompt` from a `PromptDefinition`-shaped object.
	 *
	 * The optional third parameter `_internal` is a module-private token (type `InternalCtorArg`)
	 * that can only be formed by code within this module. External callers cannot construct a valid
	 * `InternalCtorArg` value (its type uses a `unique symbol` brand), so `new Prompt(shape, v?)` is
	 * effectively the only publicly-callable form. The static factories (`fromYaml`, `fromJson`,
	 * `fromToml`) and `with()` use the internal path to avoid re-running the Rust validator on an
	 * already-validated handle.
	 */
	constructor(
		shape: PromptDefinition,
		validators?: ValidatorMap,
		_internal?: InternalCtorArg,
	) {
		if (_internal !== undefined) {
			// Internal path: handle already constructed externally (fromYaml / fromJson / fromToml /
			// with). Coverage check already ran at the call site.
			this.#handle = (_internal as unknown as { handle: NapiPrompt }).handle;
			this.#validators = validators;
			return;
		}

		// Public path: shape is a PromptDefinition-shaped object. Run coverage check BEFORE
		// calling Rust so coverage failures surface as PromptValidationError, not LoadError.
		const vars = shape.variables as Record<string, unknown> | undefined;
		assertValidatorCoverage(vars, validators);

		try {
			this.#handle = promptNew(shape as unknown as Record<string, unknown>);
		} catch (thrown) {
			throw decodeAddonError(thrown);
		}
		this.#validators = validators;
	}

	// ── static factories ─────────────────────────────────────────────────────────────────────

	/**
	 * Construct a `Prompt` from already-read **YAML** text.
	 *
	 * The text is routed to the Rust consumer's `Prompt::from_yaml` — no JS YAML parsing (Q3 /
	 * Principle I). Error semantics mirror `new Prompt()`.
	 *
	 * @throws {LoadError}             malformed YAML or shape violation.
	 * @throws {PromptRenderError}     template/agreement error.
	 * @throws {PromptValidationError} uncovered `validation_required` variable.
	 */
	static fromYaml(text: string, validators?: ValidatorMap): Prompt {
		let handle: NapiPrompt;
		try {
			handle = promptFromYaml(text);
		} catch (thrown) {
			throw decodeAddonError(thrown);
		}
		assertValidatorCoverage(
			handle.variables as Record<string, unknown> | undefined,
			validators,
		);
		return new Prompt(
			{} as PromptDefinition,
			validators,
			makeInternalArg(handle),
		);
	}

	/**
	 * Construct a `Prompt` from already-read **JSON** text.
	 *
	 * @throws {LoadError}             malformed JSON or shape violation.
	 * @throws {PromptRenderError}     template/agreement error.
	 * @throws {PromptValidationError} uncovered `validation_required` variable.
	 */
	static fromJson(text: string, validators?: ValidatorMap): Prompt {
		let handle: NapiPrompt;
		try {
			handle = promptFromJson(text);
		} catch (thrown) {
			throw decodeAddonError(thrown);
		}
		assertValidatorCoverage(
			handle.variables as Record<string, unknown> | undefined,
			validators,
		);
		return new Prompt(
			{} as PromptDefinition,
			validators,
			makeInternalArg(handle),
		);
	}

	/**
	 * Construct a `Prompt` from already-read **TOML** text.
	 *
	 * TOML parsing is done by the Rust consumer (`toml@1.1.2` via `Prompt::from_toml`). Raw text
	 * is routed to the addon — no `smol-toml` or other JS TOML library needed (Q3 / Principle I).
	 *
	 * @throws {LoadError}             malformed TOML or shape violation.
	 * @throws {PromptRenderError}     template/agreement error.
	 * @throws {PromptValidationError} uncovered `validation_required` variable.
	 */
	static fromToml(text: string, validators?: ValidatorMap): Prompt {
		let handle: NapiPrompt;
		try {
			handle = promptFromToml(text);
		} catch (thrown) {
			throw decodeAddonError(thrown);
		}
		assertValidatorCoverage(
			handle.variables as Record<string, unknown> | undefined,
			validators,
		);
		return new Prompt(
			{} as PromptDefinition,
			validators,
			makeInternalArg(handle),
		);
	}

	// ── read-only accessors ───────────────────────────────────────────────────────────────────

	/** The prompt's name (the `name` field of the underlying definition). */
	get name(): string {
		return this.#handle.name;
	}

	/** The conversational role (`"system"` / `"user"` / `"assistant"`). */
	get role(): string {
		return this.#handle.role;
	}

	/** The root body template source (the default arm's unrendered template text). */
	get body(): string {
		return this.#handle.body;
	}

	/**
	 * The declared variables map (`{ [name]: PromptVariable }`). Read-only metadata. Each entry
	 * carries the variable's `type`, `origin`, and optional `validation_required`.
	 */
	get variables(): PromptDefinition["variables"] {
		return this.#handle.variables as PromptDefinition["variables"];
	}

	/**
	 * The named variants map (`{ [name]: PromptVariant }`). Empty object when the prompt has no named
	 * variants (only the implicit `default` arm).
	 */
	get variants(): PromptDefinition["variants"] {
		return this.#handle.variants as PromptDefinition["variants"];
	}

	/**
	 * The `output_model` reference, if declared. Carried as metadata only — the library never
	 * parses against it (Principle III).
	 */
	get outputModel(): string | undefined {
		return this.#handle.outputModel ?? undefined;
	}

	/**
	 * The `metadata` opaque map (library-defined top-level annotations, if any).
	 */
	get metadata(): Record<string, unknown> {
		return (this.#handle.metadata as Record<string, unknown>) ?? {};
	}

	// ── operations ────────────────────────────────────────────────────────────────────────────

	/**
	 * Render this prompt's resolved variant with typed inputs, validating first (Q1).
	 *
	 * Three call forms, selected at runtime:
	 *  - **Schema form:** `render(schema, data, opts?)` — `schema.safeParse(data)` runs here,
	 *    before any templating; on failure a {@link PromptValidationError} is thrown and the
	 *    kernel is **never reached**.
	 *  - **Static form:** `render(data, opts?)` — already-typed plain data, marshaled directly
	 *    (no Zod check at render time).
	 *  - **Bound-validator form:** `render(data, opts?)` when the prompt was constructed with
	 *    `validators` — the bound schema's `safeParse(data)` runs automatically.
	 *
	 * `opts` carries `{ variant?, guard? }` (C-11: named fields over positionals; Principle VI).
	 *
	 * @throws {PromptValidationError} validation failed (schema form or bound-validator form).
	 * @throws {PromptRenderError}     the kernel rejected the render.
	 */
	render<T>(
		schema: ZodLikeSchema<T>,
		data: unknown,
		opts?: RenderOptions | null,
	): RenderResult;
	render(data: unknown, opts?: RenderOptions | null): RenderResult;
	render(
		schemaOrData: unknown,
		dataOrOpts?: unknown,
		opts?: RenderOptions | null,
	): RenderResult {
		let value: unknown;
		let options: RenderOptions | null | undefined;

		if (isZodLikeSchema(schemaOrData)) {
			// Schema form: validate dataOrOpts against the schema; opts is the third arg.
			value = validateOrThrow(schemaOrData, dataOrOpts);
			options = opts;
		} else if (this.#validators !== undefined) {
			// Bound-validator form: the first arg IS the data; run the bound validator.
			value = validateOrThrow(this.#validators, schemaOrData);
			options = dataOrOpts as RenderOptions | null | undefined;
		} else {
			// Static form: schemaOrData IS the already-typed data.
			value = schemaOrData;
			options = dataOrOpts as RenderOptions | null | undefined;
		}

		try {
			return this.#handle.renderPrompt(
				value as Record<string, unknown>,
				options?.variant ?? undefined,
				options?.guard ?? undefined,
			);
		} catch (thrown) {
			throw decodeAddonError(thrown);
		}
	}

	/**
	 * Return a variant's **unrendered** template source (the exact string the kernel hashes into
	 * `templateHash`). Pure: no vars, no validation. `opts.variant` selects an arm (absent ⇒
	 * the reserved `default`).
	 *
	 * @throws {PromptRenderError} unknown variant name.
	 */
	getSource(opts?: { variant?: string } | null): string {
		try {
			return this.#handle.getSourcePrompt(opts?.variant ?? undefined);
		} catch (thrown) {
			throw decodeAddonError(thrown);
		}
	}

	/**
	 * Pure advisory lint: returns a {@link CheckReport} containing only the origin/guard finding
	 * class (`"untrusted_without_guard"`). Construction already enforces agreement, parse, and
	 * reserved-name invariants; those are structurally unreachable here (R7 / Q4).
	 *
	 * Pure (FR-019): never renders, never mutates.
	 */
	check(): CheckReport {
		return this.#handle.checkPrompt();
	}

	/**
	 * The sole mutator: shallow-replace top-level fields from `overlay` onto a clone of this
	 * prompt's definition, then re-validate the merged whole via the Rust consumer. The original
	 * `Prompt` is untouched (SC-004).
	 *
	 * Validators carry forward from the source by default (R6); pass `validators` to
	 * override/augment. Coverage is re-checked against the merged definition.
	 *
	 * @param overlay    A partial `PromptDefinition` object — any subset of top-level fields to replace.
	 * @param validators Optional new validator. If omitted, the source's bound validator is inherited.
	 * @throws {LoadError}             overlay causes a shape violation.
	 * @throws {PromptRenderError}     merged template/agreement error.
	 * @throws {PromptValidationError} uncovered `validation_required` variable after merge.
	 */
	derive(
		overlay: Partial<PromptDefinition>,
		validators?: ValidatorMap,
	): Prompt {
		// Effective validator: overlay's (if explicitly provided) else inherit from this.
		const effectiveValidators =
			validators !== undefined ? validators : this.#validators;

		let derivedHandle: NapiPrompt;
		try {
			derivedHandle = this.#handle.derivePrompt(
				overlay as Record<string, unknown>,
			);
		} catch (thrown) {
			throw decodeAddonError(thrown);
		}

		// Coverage check on the derived handle's merged variables.
		assertValidatorCoverage(
			derivedHandle.variables as Record<string, unknown> | undefined,
			effectiveValidators,
		);

		return new Prompt(
			{} as PromptDefinition,
			effectiveValidators,
			makeInternalArg(derivedHandle),
		);
	}

	/**
	 * Module-private accessor: expose the underlying `NapiPrompt` handle to sibling code in this
	 * module (e.g. `Composition.append`). Not exported — the Symbol ensures it cannot be called
	 * from outside this module.
	 */
	[PROMPT_HANDLE_KEY](): NapiPrompt {
		return this.#handle;
	}
}

// --------------------------------------------------------------------------------------
// Composition (spec 008, T046) — aggregates Prompt objects, no Registry.
// --------------------------------------------------------------------------------------

/**
 * One composition entry, as an **options object** (C-11: named fields over positional tuples).
 *
 *  - `prompt`  — the `Prompt` object to render (owned; Principle V: caller-owned selection).
 *  - `schema`  — optional Zod-like schema. Present ⇒ `schema.safeParse(data)` runs at append.
 *  - `data`    — the vars value (validated against `schema` when present).
 *  - `variant` — the selected variant arm (absent ⇒ the reserved `default`).
 */
export interface CompositionEntry {
	prompt: Prompt;
	schema?: ZodLikeSchema;
	data: unknown;
	variant?: string;
}

/** Internal representation of a stored composition entry (after validation and handle extraction). */
interface StoredEntry {
	/** The already-validated value for this entry. */
	value: unknown;
	/** The selected variant (`undefined` ⇒ the reserved `default`). */
	variant: string | undefined;
	/** The NapiPrompt handle (extracted at append time for kernel-direct render at resolve). */
	handle: NapiPrompt;
	/** The role from the prompt definition (needed for Message construction). */
	role: string;
}

/**
 * An explicit, ordered composition of `(Prompt, vars, variant?)` entries that resolves to an
 * ordered `Message[]` (FR-012). Built with `new Composition()` + `append()` or
 * `Composition.fromMessages([...])`. No fluent `.chain()` (FR-013).
 *
 * **No Registry** (spec 008 T046): each entry holds an owned `Prompt` object. `resolve()`
 * takes no arguments — it renders using each entry's stored `Prompt` handle directly.
 *
 * No-partial guarantee (FR-013): if any entry fails validation at `append`/`fromMessages`, the
 * whole call throws and **nothing** is stored. If an entry fails at `resolve` (unknown variant,
 * undefined variable, parse/render), `resolve` throws and the partial result is discarded.
 */
export class Composition {
	/** Entries in append order — the resolved-message order (FR-012). */
	readonly #entries: StoredEntry[] = [];

	constructor() {}

	/**
	 * Build a composition from an ordered array of {@link CompositionEntry} objects, validating
	 * each in order. The first entry whose `schema.safeParse(data)` fails throws a
	 * {@link PromptValidationError} and **no** `Composition` is returned (no partial state — FR-013).
	 */
	static fromMessages(entries: readonly CompositionEntry[]): Composition {
		const composition = new Composition();
		for (const entry of entries) {
			composition.append(entry);
		}
		return composition;
	}

	/**
	 * Marshal + store one {@link CompositionEntry}. When `entry.schema` is present, validation
	 * runs here (`schema.safeParse(entry.data)`); on failure a {@link PromptValidationError} is
	 * thrown and nothing is stored. Returns `void` (not `this`): intentionally **not** fluent
	 * (FR-013).
	 */
	append(entry: CompositionEntry): void {
		const value =
			entry.schema === undefined
				? entry.data
				: validateOrThrow(entry.schema, entry.data);
		this.#entries.push({
			value,
			variant: entry.variant,
			handle: entry.prompt[PROMPT_HANDLE_KEY](),
			role: entry.prompt.role,
		});
	}

	/** The number of appended entries (== the resolved-message count on success). */
	get length(): number {
		return this.#entries.length;
	}

	/**
	 * Resolve the composition to an ordered `Message[]` (FR-012), rendering each entry in append
	 * order through the kernel (via `NapiPrompt.renderPrompt` on each stored handle).
	 *
	 * **No Registry** — each entry holds its own `NapiPrompt` handle (T046).
	 *
	 * One entry's failure (unknown variant, undefined variable, parse/render error) throws the
	 * mapped {@link PromptingPressError} subclass and the partial result is discarded — never
	 * returned as success. An empty composition resolves to `[]`.
	 *
	 * @throws {PromptRenderError} kernel rejection for any entry.
	 */
	resolve(): Message[] {
		const messages: Message[] = [];

		for (const entry of this.#entries) {
			let result: RenderResult;
			try {
				result = entry.handle.renderPrompt(
					entry.value as Record<string, unknown>,
					entry.variant,
					undefined, // composition uses no guard expansion
				);
			} catch (thrown) {
				throw decodeAddonError(thrown);
			}
			messages.push({ role: entry.role, text: result.text });
		}

		return messages;
	}
}

// ─────────────────────────────────────────────────────────────────────────────────────────
// Re-exports: the inert addon classes/functions surfaced 1:1, plus the generated shape.
// (`Prompt`, `Composition`, and the error hierarchy are the primary surface above;
// `RenderResult`/`CheckReport` are read-only result types surfaced unchanged, and
// `coreVersion` is a trivial callable with no error path.)
// ─────────────────────────────────────────────────────────────────────────────────────────

export type {
	Finding,
	GuardConfig,
	Message,
	PromptDefinition,
	PromptVariable,
	PromptVariant,
};
export {
	CheckReport,
	coreVersion,
	// Read-only result classes + the trivial version probe, surfaced unchanged (Principle I).
	RenderResult,
};
