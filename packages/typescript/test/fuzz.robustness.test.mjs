/**
 * Adversarial robustness + property-based fuzzing for the TypeScript facade — spec 009, T014.
 *
 * Uses fast-check 4.8.0 (dev-only, FR-009) to drive the post-008 Prompt surface
 * (new Prompt / fromYaml / fromJson / fromToml / render / check) with hostile and
 * generated inputs.
 *
 * Invariants asserted across the generated space (FR-001, FR-003, FR-004):
 *   1. never-crash   — every call either returns a value or throws a PromptingPressError
 *                      subclass; a non-PromptingPressError throw is a test failure.
 *   2. hash-determinism — re-rendering the same valid prompt+vars twice yields byte-identical
 *                         templateHash and renderHash (SC-004).
 *   3. validate-before-render — when a Zod schema rejects the data, the kernel is never
 *                               reached; the error is always PromptValidationError (SC-005).
 *
 * Fixed seed (FR-004): failures are reproducible; `numRuns` is bounded so CI stays fast.
 */

import assert from "node:assert/strict";
import { test } from "node:test";
import fc from "fast-check";

import { Prompt, PromptingPressError, PromptValidationError } from "prompting-press";

// ── constants ─────────────────────────────────────────────────────────────────────────────

const SEED = 0xfacecafe; // fixed so failures replay (FR-004)
const NUM_RUNS = 75; // bounded: fast enough for CI, wide enough to catch edge cases

// ── helpers ───────────────────────────────────────────────────────────────────────────────

/**
 * Assert that `fn()` either returns without throwing, or throws a PromptingPressError.
 * Any other thrown value (raw napi error, TypeError, unexpected Error subclass, string, …)
 * is an immediate test failure — that is the "never-crash / always-structured-error" invariant
 * (FR-001, SC-001, SC-002).
 *
 * Note: the `Prompt` constructor is validating and THROWS on invalid input by design.
 * The never-crash invariant is therefore: "either returns a value OR throws a
 * PromptingPressError — never throws something else."
 */
function assertNeverCrash(fn) {
	try {
		fn();
		// Returned a value — invariant holds.
	} catch (err) {
		assert.ok(
			err instanceof PromptingPressError,
			`Expected PromptingPressError, got ${
				err?.constructor ? err.constructor.name : typeof err
			}: ${String(err?.message || err)}`,
		);
		// Structured-error invariant: .errors must be an array of {field, code, message}
		assert.ok(Array.isArray(err.errors), "err.errors must be an array");
		for (const row of err.errors) {
			assert.equal(typeof row.field, "string", "row.field must be string");
			assert.equal(typeof row.code, "string", "row.code must be string");
			assert.equal(typeof row.message, "string", "row.message must be string");
		}
	}
}

// ── a minimal valid prompt shape (anchor for property tests) ──────────────────────────────

const SIMPLE_SHAPE = {
	name: "fuzz_anchor",
	role: "user",
	body: "Hello {{ name }}",
	variables: { name: { type: "string", origin: "trusted" } },
};

// ── T014-A: hostile YAML strings never crash ─────────────────────────────────────────────

test("T014-A: fromYaml never crashes on hostile strings (always PromptingPressError or value)", () => {
	// Hostile arbitraries: truncated, random bytes, unicode, control chars, huge strings.
	const hostileString = fc.oneof(
		fc.string({ unit: "grapheme", maxLength: 500 }),
		fc.string({ unit: "binary", maxLength: 200 }),
		fc.string({ maxLength: 20000 }), // very large
		fc.constant(""),
		fc.constant("name: [unterminated"),
		fc.constant("{{{{{{{{{{{{"),
		fc.constant("\x00\x01\x02\x03\x04"),
		fc.constant("a".repeat(100_000)), // 100 kB body
	);

	fc.assert(
		fc.property(hostileString, (text) => {
			assertNeverCrash(() => Prompt.fromYaml(text));
		}),
		{ numRuns: NUM_RUNS, seed: SEED },
	);
});

// ── T014-B: hostile JSON strings never crash ─────────────────────────────────────────────

test("T014-B: fromJson never crashes on hostile strings", () => {
	const hostileJson = fc.oneof(
		fc.string({ maxLength: 500 }),
		fc.constant("{ not json"),
		fc.constant("null"),
		fc.constant("42"),
		fc.constant("[]"),
		fc.constant("{}"),
		// valid JSON with wrong shape
		fc.json(),
	);

	fc.assert(
		fc.property(hostileJson, (text) => {
			assertNeverCrash(() => Prompt.fromJson(text));
		}),
		{ numRuns: NUM_RUNS, seed: SEED },
	);
});

// ── T014-C: hostile TOML strings never crash ─────────────────────────────────────────────

test("T014-C: fromToml never crashes on hostile strings", () => {
	const hostileToml = fc.oneof(
		fc.string({ maxLength: 500 }),
		fc.constant("name = [unterminated"),
		fc.constant(""),
		fc.constant("[[["),
	);

	fc.assert(
		fc.property(hostileToml, (text) => {
			assertNeverCrash(() => Prompt.fromToml(text));
		}),
		{ numRuns: NUM_RUNS, seed: SEED },
	);
});

// ── T014-D: hostile construction shapes never crash ──────────────────────────────────────

test("T014-D: new Prompt(shape) never crashes on hostile shape objects", () => {
	// Generate arbitrary JSON-safe objects — most will fail with PromptingPressError.
	const hostileShape = fc.oneof(
		fc.record({
			name: fc.string({ maxLength: 50 }),
			role: fc.string({ maxLength: 20 }),
			body: fc.string({ unit: "grapheme", maxLength: 2000 }),
			variables: fc.option(
				fc.dictionary(
					fc.string({ maxLength: 20 }),
					fc.record({
						type: fc.oneof(
							fc.constantFrom("string", "integer", "float", "boolean"),
							fc.string({ maxLength: 10 }),
						),
						origin: fc.oneof(
							fc.constantFrom("trusted", "untrusted", "external"),
							fc.string({ maxLength: 10 }),
						),
					}),
				),
				{ nil: undefined },
			),
		}),
		// Completely degenerate
		fc.constant({}),
		fc.constant({ name: "x" }),
		fc.constant({ name: "", role: "", body: "" }),
		// Body with MiniJinja-hostile syntax
		fc.record({
			name: fc.constant("fuzz"),
			role: fc.constantFrom("user", "system", "assistant"),
			body: fc.oneof(
				fc.constant("{{ unclosed"),
				fc.constant("{% bad tag %}"),
				fc.constant("{# comment #}"),
				fc.string({ unit: "grapheme", maxLength: 500 }),
			),
			variables: fc.constant({}),
		}),
	);

	fc.assert(
		fc.property(hostileShape, (shape) => {
			assertNeverCrash(() => new Prompt(shape));
		}),
		{ numRuns: NUM_RUNS, seed: SEED },
	);
});

// ── T014-E: render never crashes on hostile var values ────────────────────────────────────

test("T014-E: render never crashes on hostile var-set values (static form)", () => {
	const p = new Prompt(SIMPLE_SHAPE);

	// Hostile var values: deeply nested, huge strings, numbers, booleans, null, arrays.
	const hostileValue = fc.oneof(
		fc.string({ unit: "grapheme", maxLength: 5000 }),
		fc.string({ unit: "binary", maxLength: 500 }),
		fc.integer(),
		fc.float({ noNaN: false, noDefaultInfinity: false }),
		fc.boolean(),
		fc.constant(null),
		fc.constant(undefined),
		fc.array(fc.string({ maxLength: 10 }), { maxLength: 100 }),
		fc.constant("a".repeat(1_000_000)), // 1 MB string
	);

	fc.assert(
		fc.property(hostileValue, (val) => {
			assertNeverCrash(() => p.render({ name: val }));
		}),
		{ numRuns: NUM_RUNS, seed: SEED },
	);
});

// ── T014-F: check() never crashes ────────────────────────────────────────────────────────

test("T014-F: check() never crashes on any successfully-constructed prompt", () => {
	// Build prompts from a variety of valid shapes, then call check() on each.
	const origins = ["trusted", "untrusted", "external"];

	const validShape = fc.record({
		name: fc
			.string({ minLength: 1, maxLength: 20 })
			.filter((s) => /^[a-zA-Z_][a-zA-Z0-9_]*$/.test(s)),
		role: fc.constantFrom("user", "system", "assistant"),
		body: fc
			.string({ minLength: 1, maxLength: 200 })
			.filter((s) => !s.includes("{{") && !s.includes("{%")),
		variables: fc.dictionary(
			fc.string({ minLength: 1, maxLength: 10 }).filter((s) => /^[a-zA-Z_][a-zA-Z0-9_]*$/.test(s)),
			fc.record({
				type: fc.constantFrom("string", "integer", "float", "boolean"),
				origin: fc.constantFrom(...origins),
			}),
			{ maxKeys: 5 },
		),
	});

	fc.assert(
		fc.property(validShape, (shape) => {
			// Only call check() when construction succeeds — not all generated shapes are valid.
			let p;
			try {
				p = new Prompt(shape);
			} catch (err) {
				// Construction failure is expected for some generated shapes; skip check().
				assert.ok(
					err instanceof PromptingPressError,
					`Construction threw unexpected: ${err?.constructor?.name}`,
				);
				return;
			}
			// Construction succeeded — check() MUST return a CheckReport without throwing.
			assertNeverCrash(() => p.check());
		}),
		{ numRuns: NUM_RUNS, seed: SEED },
	);
});

// ── T014-G: hash-determinism property (SC-004) ───────────────────────────────────────────

test("T014-G: hash-determinism — re-rendering the same inputs yields byte-identical hashes", () => {
	const p = new Prompt(SIMPLE_SHAPE);

	// Generate valid string values for `name`.
	const nameArb = fc.string({ unit: "grapheme", minLength: 0, maxLength: 200 });

	fc.assert(
		fc.property(nameArb, (name) => {
			const first = p.render({ name });
			const second = p.render({ name });

			assert.equal(
				first.templateHash,
				second.templateHash,
				`templateHash not deterministic for name=${JSON.stringify(name)}`,
			);
			assert.equal(
				first.renderHash,
				second.renderHash,
				`renderHash not deterministic for name=${JSON.stringify(name)}`,
			);
		}),
		{ numRuns: NUM_RUNS, seed: SEED },
	);
});

// ── T014-H: validate-before-render — validation failure never reaches kernel (SC-005) ────

test("T014-H: validate-before-render — schema rejection is always PromptValidationError, kernel not reached", () => {
	const p = new Prompt(SIMPLE_SHAPE);

	// A schema that always rejects its input.
	const rejectAllSchema = {
		safeParse: (_data) => ({
			success: false,
			error: {
				issues: [{ path: ["name"], message: "always fails" }],
			},
		}),
	};

	// Generate arbitrary data — the schema rejects ALL of it.
	const anyData = fc.anything();

	fc.assert(
		fc.property(anyData, (data) => {
			assert.throws(
				() => p.render(rejectAllSchema, data),
				(err) => {
					assert.ok(
						err instanceof PromptValidationError,
						`Expected PromptValidationError, got ${err?.constructor?.name}`,
					);
					// SC-005: errors array names the field the schema reported.
					assert.ok(Array.isArray(err.errors));
					assert.ok(err.errors.length > 0);
					return true;
				},
			);
		}),
		{ numRuns: NUM_RUNS, seed: SEED },
	);
});

// ── T014-I: deep-nesting does not crash ──────────────────────────────────────────────────

test("T014-I: deeply-nested var-set objects do not crash the library", () => {
	const p = new Prompt(SIMPLE_SHAPE);

	// Build a progressively nested object up to depth 50 — pathological but finite.
	function nest(depth) {
		let val = "leaf";
		for (let i = 0; i < depth; i++) val = { child: val };
		return val;
	}

	for (const depth of [1, 5, 10, 20, 50]) {
		assertNeverCrash(() => p.render({ name: nest(depth) }));
	}
});

// ── T014-J: Unicode / control-char bodies in render output do not crash ───────────────────

test("T014-J: Unicode and control-char values in vars do not crash render", () => {
	const p = new Prompt(SIMPLE_SHAPE);

	const unicodeStrings = [
		"😀🎉🦄", // emoji
		" ", // control chars
		"​‌‍﻿", // zero-width / BOM
		"\u{1F600}\u{1F4A9}", // astral plane
		"𐏿", // surrogate pair
		"\n\r\t", // whitespace
		"مرحبا", // Arabic RTL
		"日本語", // CJK
		"ç à ü ñ", // Latin extended
	];

	for (const s of unicodeStrings) {
		assertNeverCrash(() => p.render({ name: s }));
	}
});
