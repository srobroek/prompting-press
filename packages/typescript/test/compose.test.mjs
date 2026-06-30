/**
 * US4 multi-message composition tests for the TypeScript facade (`prompting-press`) — spec 005, T021.
 * Updated for spec 008 object surface: uses `Prompt` objects (not names+Registry), and
 * `Composition.resolve()` takes no registry argument.
 *
 * US4 lands the ordered-composition surface (FR-012 / FR-013): an explicit, ordered array of
 * `(Prompt, vars, variant?)` entries that resolves to a `Message[]` in append order. There is
 * NO fluent `.chain()` (FR-013) — composition is built with `new Composition()` + `.append(…)`
 * or the `Composition.fromMessages([…])` static factory.
 *
 * What these pin (all TS-observable; none re-verify cross-language render parity):
 *   - order + roles (SC-008): N entries → exactly N `Message` in input order.
 *   - one bad entry → no partial (FR-013 / SC-008): vars failing validation throw at append
 *     with NOTHING stored; an unknown variant surfaces at `resolve` as a `PromptRenderError`.
 *   - empty composition → [].
 *   - no `.chain()` on the class or an instance (FR-013).
 *   - a `variant` entry field selects the variant arm.
 *
 * All fixtures use `origin` (spec 008 rename from `provenance`).
 */

import assert from "node:assert/strict";
import { test } from "node:test";

import { z } from "zod";

import {
	Prompt,
	Composition,
	PromptingPressError,
	PromptRenderError,
	PromptValidationError,
} from "prompting-press";

// ── Zod Vars schemas ──────────────────────────────────────────────────────────────────────

const Named = z.object({ name: z.string().refine((s) => s.length > 0, "name must be non-empty") });
const EmptyVars = z.object({});

// ── Prompt helpers ────────────────────────────────────────────────────────────────────────

function makePrompt(def) {
	return new Prompt(def);
}

const SYS_PREAMBLE = makePrompt({
	name: "sys_preamble",
	role: "system",
	body: "You are helpful.",
	variables: {},
});
const GREET = makePrompt({
	name: "greet",
	role: "user",
	body: "Hi {{ name }}",
	variables: { name: { type: "string", origin: "trusted" } },
});
const FAREWELL = makePrompt({
	name: "farewell",
	role: "user",
	body: "Bye {{ name }}",
	variables: { name: { type: "string", origin: "trusted" } },
});
const WITH_VARIANT = makePrompt({
	name: "salute",
	role: "user",
	body: "Hi {{ name }}",
	variants: { formal: { body: "Good day, {{ name }}" } },
	variables: { name: { type: "string", origin: "trusted" } },
});

// ── 1. Order + roles (SC-008) — both construction paths ──────────────────────────────────

test("the append path preserves order, roles, and per-entry text (SC-008)", () => {
	const comp = new Composition();
	assert.equal(
		comp.append({ prompt: SYS_PREAMBLE, schema: EmptyVars, data: {} }),
		undefined,
		"append is non-fluent (void)",
	);
	assert.equal(comp.append({ prompt: GREET, schema: Named, data: { name: "Ada" } }), undefined);
	assert.equal(comp.length, 2);

	const messages = comp.resolve();
	assert.equal(messages.length, 2);
	assert.equal(messages[0].role, "system");
	assert.equal(messages[0].text, "You are helpful.");
	assert.equal(messages[1].role, "user");
	assert.equal(messages[1].text, "Hi Ada");
});

test("the fromMessages path preserves order, roles, and per-entry text (SC-008)", () => {
	const comp = Composition.fromMessages([
		{ prompt: SYS_PREAMBLE, schema: EmptyVars, data: {} },
		{ prompt: GREET, schema: Named, data: { name: "Bo" } },
	]);
	assert.ok(comp instanceof Composition);
	assert.equal(comp.length, 2);

	const messages = comp.resolve();
	assert.equal(messages.length, 2);
	assert.deepEqual(
		messages.map((m) => m.role),
		["system", "user"],
	);
	assert.deepEqual(
		messages.map((m) => m.text),
		["You are helpful.", "Hi Bo"],
	);
});

test("the two construction paths produce identical ordered messages", () => {
	const entries = [
		{ prompt: SYS_PREAMBLE, schema: EmptyVars, data: {} },
		{ prompt: GREET, schema: Named, data: { name: "Cy" } },
	];

	const viaAppend = new Composition();
	for (const entry of entries) viaAppend.append(entry);
	const viaFactory = Composition.fromMessages(entries);

	const appended = viaAppend.resolve().map((m) => [m.role, m.text]);
	const factoried = viaFactory.resolve().map((m) => [m.role, m.text]);
	assert.deepEqual(appended, factoried);
	assert.deepEqual(appended, [
		["system", "You are helpful."],
		["user", "Hi Cy"],
	]);
});

// ── 2. One invalid entry → no partial (FR-013 / SC-008) ──────────────────────────────────

test("invalid vars at append throw PromptValidationError and store nothing (no partial)", () => {
	const comp = new Composition();
	comp.append({ prompt: GREET, schema: Named, data: { name: "ok" } });
	assert.equal(comp.length, 1);

	assert.throws(
		() => comp.append({ prompt: GREET, schema: Named, data: { name: "" } }),
		(err) => {
			assert.ok(err instanceof PromptValidationError);
			assert.ok(err.errors.some((row) => row.field === "name"));
			return true;
		},
	);
	assert.equal(comp.length, 1, "a rejected append must store nothing");

	assert.deepEqual(
		comp.resolve().map((m) => m.text),
		["Hi ok"],
	);
});

test("the first invalid entry in fromMessages throws and yields no Composition (no partial)", () => {
	assert.throws(
		() =>
			Composition.fromMessages([
				{ prompt: GREET, schema: Named, data: { name: "ok" } },
				{ prompt: GREET, schema: Named, data: { name: "" } },
			]),
		PromptValidationError,
	);

	const good = Composition.fromMessages([{ prompt: GREET, schema: Named, data: { name: "ok" } }]);
	assert.deepEqual(
		good.resolve().map((m) => m.text),
		["Hi ok"],
	);
});

test("an unknown variant at resolve throws PromptRenderError and returns no partial", () => {
	const comp = new Composition();
	comp.append({ prompt: SYS_PREAMBLE, schema: EmptyVars, data: {} });
	comp.append({ prompt: GREET, data: { name: "X" }, variant: "nonexistent" });
	assert.equal(comp.length, 2);

	const sentinel = Symbol("not-set");
	let result = sentinel;
	assert.throws(
		() => {
			result = comp.resolve();
		},
		(err) => {
			assert.ok(err instanceof PromptRenderError);
			return true;
		},
	);
	assert.equal(result, sentinel, "resolve must RAISE, not return a partial list");
});

// ── 3. Empty composition → [] ─────────────────────────────────────────────────────────────

test("an empty composition resolves to []", () => {
	const empty = new Composition();
	assert.equal(empty.length, 0);
	assert.deepEqual(empty.resolve(), []);
});

// ── 4. No .chain() (FR-013) ───────────────────────────────────────────────────────────────

test("there is no fluent .chain() on the class or an instance (FR-013)", () => {
	assert.equal(Composition.prototype.chain, undefined);
	const comp = new Composition();
	assert.equal(comp.chain, undefined);
	assert.equal(comp.append({ prompt: GREET, schema: Named, data: { name: "x" } }), undefined);
});

// ── 5. Variant selection via the entry object ─────────────────────────────────────────────

test("a variant entry field selects the named variant arm", () => {
	const viaFactory = Composition.fromMessages([
		{ prompt: WITH_VARIANT, schema: Named, data: { name: "Di" }, variant: "formal" },
	]);
	assert.deepEqual(
		viaFactory.resolve().map((m) => m.text),
		["Good day, Di"],
	);

	const viaAppend = new Composition();
	viaAppend.append({
		prompt: WITH_VARIANT,
		schema: Named,
		data: { name: "Di" },
		variant: "formal",
	});
	assert.equal(viaAppend.resolve()[0].text, "Good day, Di");
});

test("no variant field defaults to the reserved default arm", () => {
	const comp = Composition.fromMessages([
		{ prompt: WITH_VARIANT, schema: Named, data: { name: "Eli" } },
	]);
	assert.deepEqual(
		comp.resolve().map((m) => m.text),
		["Hi Eli"],
	);
});

// ── 6. Mixed system + two user entries ────────────────────────────────────────────────────

test("a mixed system + two user composition resolves to 3 ordered messages (SC-008)", () => {
	const comp = Composition.fromMessages([
		{ prompt: SYS_PREAMBLE, schema: EmptyVars, data: {} },
		{ prompt: GREET, schema: Named, data: { name: "Ada" } },
		{ prompt: FAREWELL, schema: Named, data: { name: "Bo" } },
	]);
	assert.equal(comp.length, 3);

	const messages = comp.resolve();
	assert.equal(messages.length, 3);
	assert.deepEqual(
		messages.map((m) => m.role),
		["system", "user", "user"],
	);
	assert.deepEqual(
		messages.map((m) => m.text),
		["You are helpful.", "Hi Ada", "Bye Bo"],
	);
});

// ── 7. Static (no-schema) entries ─────────────────────────────────────────────────────────

test("static (no-schema) entries are marshaled directly (Q4)", () => {
	const comp = Composition.fromMessages([
		{ prompt: SYS_PREAMBLE, data: {} },
		{ prompt: GREET, data: { name: "Zed" } },
	]);
	assert.deepEqual(
		comp.resolve().map((m) => m.text),
		["You are helpful.", "Hi Zed"],
	);
});

// ── 8. resolve() takes no registry argument (T046) ────────────────────────────────────────

test("resolve() takes no argument — there is no registry in the post-008 surface (T046)", () => {
	const comp = Composition.fromMessages([{ prompt: SYS_PREAMBLE, data: {} }]);
	// Calling resolve() with no argument must succeed (not crash on missing registry).
	const messages = comp.resolve();
	assert.equal(messages.length, 1);
});

// ── 9. Surface smoke ──────────────────────────────────────────────────────────────────────

test("the US4 composition surface is exposed", () => {
	assert.equal(typeof Composition, "function");
	assert.equal(typeof Composition.fromMessages, "function");
	assert.equal(typeof new Composition().append, "function");
	assert.equal(typeof new Composition().resolve, "function");
	assert.ok(PromptRenderError.prototype instanceof PromptingPressError);
	assert.ok(PromptValidationError.prototype instanceof PromptingPressError);
});
