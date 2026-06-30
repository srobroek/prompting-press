/**
 * US1 render-path tests for the TypeScript facade (`prompting-press`) — spec 005, T010.
 * Updated for spec 008 object surface: uses `Prompt.render()` instead of `render(reg, …)`.
 *
 * These exercise the TS-observable render path: validate-in-TS at the render boundary (Q1),
 * the normalized error contract (FR-014, C-06), the SEC-004 scrub, the three-sets agreement
 * gap (a loud `undefined_variable`, never a silent empty render), and the guard plumb-through
 * (FR-009).
 *
 * All fixtures use `trusted` (spec 015 replaced the `origin` enum with a boolean).
 */

import assert from "node:assert/strict";
import { test } from "node:test";
import {
	Prompt,
	PromptingPressError,
	PromptRenderError,
	PromptValidationError,
} from "prompting-press";
import { z } from "zod";

// A lowercase 64-char hex string — the SHA-256 provenance hash shape (FR-012/FR-013).
const HEX64 = /^[0-9a-f]{64}$/;

// ── Zod Vars schemas ──────────────────────────────────────────────────────────────────────

const Greeting = z.object({
	name: z.string(),
	count: z
		.number()
		.int()
		.refine((n) => n >= 0, "count must be non-negative"),
});

const TwoFields = z.object({
	name: z.string().refine((s) => s.length > 0, "name must not be empty"),
	count: z
		.number()
		.int()
		.refine((n) => n >= 0, "count must be non-negative"),
});

const Secretful = z.object({
	token: z.string().refine((v) => !v.startsWith("sk-"), "token has a forbidden prefix"),
});

const Secret = z.object({ token: z.string() });

const Misnamed = z.object({ nam: z.string() });

const Topic = z.object({ topic: z.string() });

// ── Prompt helpers ────────────────────────────────────────────────────────────────────────

const GREET_YAML = `
name: greet
role: user
body: "Hi {{ name }}, you have {{ count }} messages"
variables:
  name:  { type: string,  trusted: true }
  count: { type: integer, trusted: true }
`;

const ASK_YAML = `
name: ask
role: user
body: "Tell me about {{ topic }}."
variables:
  topic: { type: string, trusted: false }
`;

// ── 1. Valid render (SC-001) ──────────────────────────────────────────────────────────────

test("valid render produces text, name, variant, and 64-hex provenance hashes", () => {
	const p = Prompt.fromYaml(GREET_YAML);
	const result = p.render(Greeting, { name: "Ada", count: 3 });

	assert.equal(result.text, "Hi Ada, you have 3 messages");
	assert.equal(result.name, "greet");
	assert.equal(result.variant, "default", "no variant selected ⇒ the reserved default arm");
	assert.match(result.templateHash, HEX64, result.templateHash);
	assert.match(result.renderHash, HEX64, result.renderHash);
	assert.equal(result.guard, null);
});

test("static (no-schema) data is accepted and marshaled directly (Q4)", () => {
	const p = Prompt.fromYaml(GREET_YAML);
	const result = p.render({ name: "Bo", count: 1 });

	assert.equal(result.text, "Hi Bo, you have 1 messages");
	assert.equal(result.variant, "default");
	assert.match(result.templateHash, HEX64);
	assert.match(result.renderHash, HEX64);
});

// ── 2. Validation failure (SC-002 / Q1) ──────────────────────────────────────────────────

test("invalid input raises PromptValidationError naming the field, before any render", () => {
	const p = Prompt.fromYaml(GREET_YAML);

	assert.throws(
		() => p.render(Greeting, { name: "Ada", count: -1 }),
		(err) => {
			assert.ok(err instanceof PromptValidationError, "must be a PromptValidationError");
			const offending = err.errors.filter((row) => row.field === "count");
			assert.ok(
				offending.length > 0,
				`expected a row naming \`count\`, got ${JSON.stringify(err.errors)}`,
			);
			assert.ok(
				offending.every((row) => row.code === "validation"),
				offending.map((row) => row.code).join(","),
			);
			return true;
		},
	);
});

test("validation failure names EVERY offending field (SC-002)", () => {
	const p = Prompt.fromYaml(GREET_YAML);

	assert.throws(
		() => p.render(TwoFields, { name: "", count: -1 }),
		(err) => {
			assert.ok(err instanceof PromptValidationError);
			const fields = new Set(err.errors.map((row) => row.field));
			assert.ok(fields.has("name") && fields.has("count"), `got ${[...fields].join(",")}`);
			assert.ok(err.errors.every((row) => row.code === "validation"));
			return true;
		},
	);
});

// ── 3. No native error type leaks (SC-006 / C-06) ────────────────────────────────────────

test("a validation error is a PromptValidationError and NOT a ZodError (SC-006)", () => {
	const p = Prompt.fromYaml(GREET_YAML);

	assert.throws(
		() => p.render(Greeting, { name: "Ada", count: -1 }),
		(err) => {
			assert.ok(err instanceof PromptingPressError);
			assert.ok(err instanceof PromptValidationError);
			assert.notEqual(err.constructor.name, "ZodError");
			assert.ok(!(err instanceof z.ZodError));
			return true;
		},
	);
});

// ── 4. SEC-004 ───────────────────────────────────────────────────────────────────────────

test("a Zod-rejected sensitive value is not leaked (mapper copies issue message only)", () => {
	const secret = "sk-super-secret-token-9f8a7b6c5d4e";
	const p = Prompt.fromYaml(`
name: leaky
role: user
body: "Using {{ token }}"
variables:
  token: { type: string, trusted: true }
`);

	assert.throws(
		() => p.render(Secretful, { token: secret }),
		(err) => {
			assert.ok(err instanceof PromptValidationError);
			assert.ok(!String(err.message).includes(secret), `message leaked: ${err.message}`);
			assert.ok(!String(err.stack).includes(secret), "stack leaked the secret");
			for (const row of err.errors) {
				assert.ok(!row.message.includes(secret), `row message leaked: ${row.message}`);
				assert.ok(!row.field.includes(secret), `row field leaked: ${row.field}`);
			}
			assert.ok(err.errors.some((row) => row.message.includes("forbidden prefix")));
			return true;
		},
	);
});

test("a secret in a real kernel render-error value is not leaked (SEC-004 kernel path)", () => {
	const secret = "sk-super-secret-token-9f8a7b6c5d4e";
	const p = Prompt.fromYaml(`
name: kernely
role: user
body: "Using {{ token + 1 }}"
variables:
  token: { type: string, trusted: true }
`);

	assert.throws(
		() => p.render(Secret, { token: secret }),
		(err) => {
			assert.ok(err instanceof PromptRenderError);
			assert.ok(!String(err.message).includes(secret));
			assert.ok(!String(err.stack).includes(secret));
			for (const row of err.errors) {
				assert.ok(!row.message.includes(secret));
				assert.ok(!row.field.includes(secret));
			}
			assert.deepEqual(
				err.errors.map((row) => row.code),
				["render"],
			);
			assert.ok(err.errors.some((row) => row.message === "render error"));
			return true;
		},
	);
});

// ── 5. Three-sets gap ────────────────────────────────────────────────────────────────────

test("a Vars/template field-name mismatch is a loud undefined_variable (not a silent empty render)", () => {
	const p = Prompt.fromYaml(`
name: greet
role: user
body: "Hi {{ name }}!"
variables:
  name: { type: string, trusted: true }
`);

	assert.throws(
		() => p.render(Misnamed, { nam: "Ada" }),
		(err) => {
			assert.ok(err instanceof PromptRenderError);
			const codes = err.errors.map((row) => row.code);
			assert.ok(
				codes.includes("undefined_variable"),
				`expected loud undefined_variable, got ${codes.join(",")}`,
			);
			return true;
		},
	);
});

// ── 6. Guard plumb-through (FR-009) ─────────────────────────────────────────────────────

test("spec-015: enabled guard wraps untrusted value in <untrusted>…</untrusted>; advisory is separate", () => {
	// spec-015: when guard is enabled, untrusted variable values are wrapped in
	// <untrusted>…</untrusted> delimiters in the rendered body.
	const p = Prompt.fromYaml(ASK_YAML);

	const plain = p.render(Topic, { topic: "rivers" });
	const guarded = p.render(Topic, { topic: "rivers" }, { guard: { enabled: true } });

	// Unguarded render: no guard advisory, value verbatim.
	assert.equal(plain.guard, null);
	assert.equal(plain.text, "Tell me about rivers.");

	// Guarded render: advisory field is a non-empty static instruction string.
	assert.notEqual(guarded.guard, null);
	assert.equal(typeof guarded.guard, "string");
	// spec-015: guard advisory is a static instruction, not a per-field enumeration.
	assert.ok(guarded.guard.length > 0, `guard advisory must be non-empty, got: ${guarded.guard}`);

	// spec-015 delimiting: the body wraps the untrusted value.
	assert.ok(
		guarded.text.includes("<untrusted>rivers</untrusted>"),
		`expected <untrusted>rivers</untrusted> in body, got: ${guarded.text}`,
	);

	// The body IS altered by the guard (body !== plain body).
	assert.notEqual(
		guarded.text,
		plain.text,
		"guard-enabled body must differ from plain body (spec-015 delimiting)",
	);

	// Guard advisory is a SEPARATE string — not embedded in the body.
	assert.ok(
		!guarded.text.includes(guarded.guard),
		"guard advisory must not be embedded in the body",
	);
});

test("a disabled / absent guard config matches no guard at all", () => {
	const p = Prompt.fromYaml(ASK_YAML);

	const noGuard = p.render(Topic, { topic: "rivers" });
	const disabled = p.render(Topic, { topic: "rivers" }, { guard: { enabled: false } });

	assert.equal(noGuard.guard, null);
	assert.equal(disabled.guard, null);
	assert.equal(noGuard.text, disabled.text);
});

// ── 7. Variant selection (FR-009 / Principle V) ──────────────────────────────────────────

const VARIANT_YAML = `
name: greetv
role: user
body: "Hi {{ name }}"
variables:
  name: { type: string, trusted: true }
variants:
  formal: { body: "Good day, {{ name }}" }
`;

test("render selects a named variant via opts.variant (default arm when absent)", () => {
	const p = Prompt.fromYaml(VARIANT_YAML);
	const V = z.object({ name: z.string() });

	const def = p.render(V, { name: "Ada" });
	const formal = p.render(V, { name: "Ada" }, { variant: "formal" });

	assert.equal(def.text, "Hi Ada");
	assert.equal(formal.text, "Good day, Ada");
	assert.equal(def.variant, "default");
	assert.equal(formal.variant, "formal");
	assert.notEqual(def.templateHash, formal.templateHash);
});

test("render with an unknown variant raises PromptRenderError code unknown_variant", () => {
	const p = Prompt.fromYaml(VARIANT_YAML);
	const V = z.object({ name: z.string() });

	assert.throws(
		() => p.render(V, { name: "Ada" }, { variant: "nope" }),
		(err) => {
			assert.ok(err instanceof PromptRenderError);
			assert.ok(err.errors.some((row) => row.code === "unknown_variant"));
			return true;
		},
	);
});

// ── 8. getSource (FR-010) ────────────────────────────────────────────────────────────────

test("getSource returns the unrendered template source", () => {
	const p = Prompt.fromYaml(GREET_YAML);
	const source = p.getSource();
	assert.equal(source, "Hi {{ name }}, you have {{ count }} messages");
	assert.ok(source.includes("{{"));
});

test("getSource on an unknown variant raises PromptRenderError with code unknown_variant", () => {
	const p = Prompt.fromYaml(GREET_YAML);
	assert.throws(
		() => p.getSource({ variant: "nope" }),
		(err) => {
			assert.ok(err instanceof PromptRenderError);
			assert.ok(err.errors.some((row) => row.code === "unknown_variant"));
			return true;
		},
	);
});

// ── 9. Surface smoke ─────────────────────────────────────────────────────────────────────

test("the render surface is on the Prompt object (not a free function)", () => {
	const p = Prompt.fromYaml(GREET_YAML);
	assert.equal(typeof p.render, "function");
	assert.equal(typeof p.getSource, "function");
});
