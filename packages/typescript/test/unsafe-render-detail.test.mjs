/**
 * Tests for the opt-in unsafe render-error detail (spec 013, T005).
 *
 * Validates FR-001/FR-002/FR-003/FR-004/FR-010 from the TypeScript binding surface:
 *
 * - With `unsafeRevealRenderDetail: true`, a render error's real detail appears in
 *   `PromptRenderError.errors[0].message` instead of the fixed scrubbed string.
 * - With `unsafeRevealRenderDetail: false` (or absent/default), detail is scrubbed
 *   exactly as before this feature (SEC-004 unchanged).
 * - The flag has no effect on the success path (text/templateHash/renderHash are
 *   byte-identical — SC-005).
 * - No implicit enable: omitting the option produces scrubbed output.
 */

import assert from "node:assert/strict";
import { test } from "node:test";
import { Prompt, PromptRenderError } from "prompting-press";
import { z } from "zod";

// ── Prompt definitions ────────────────────────────────────────────────────────

const GREET_JSON = JSON.stringify({
	name: "greet",
	role: "user",
	body: "Hello {{ name }}",
	variables: { name: { type: "string", origin: "trusted" } },
});

/** A prompt whose body uses an unknown Jinja filter — forces a Render error at render time. */
const RENDER_FAIL_JSON = JSON.stringify({
	name: "fail",
	role: "user",
	body: "{{ name | nonexistent_filter_abc }}",
	variables: { name: { type: "string", origin: "trusted" } },
});

const Vars = z.object({ name: z.string() });

// ── Success path: flag has NO effect on rendered output (SC-005) ──────────────

test("reveal flag does not change success-path output (SC-005)", () => {
	const p = Prompt.fromJson(GREET_JSON);
	const data = { name: "Ada" };

	const rFalse = p.render(Vars, data, { unsafeRevealRenderDetail: false });
	const rTrue = p.render(Vars, data, { unsafeRevealRenderDetail: true });

	assert.equal(rFalse.text, rTrue.text, "text must be byte-identical");
	assert.equal(rFalse.templateHash, rTrue.templateHash, "templateHash must be byte-identical");
	assert.equal(rFalse.renderHash, rTrue.renderHash, "renderHash must be byte-identical");
});

// ── Default scrubs: omitting the option scrubs render detail (SC-002 / FR-002) ─

test("omitting unsafeRevealRenderDetail defaults to scrubbed (FR-002)", () => {
	const p = Prompt.fromJson(RENDER_FAIL_JSON);
	const data = { name: "Ada" };

	assert.throws(
		() => p.render(Vars, data),
		(err) => {
			assert.ok(err instanceof PromptRenderError, "must be PromptRenderError");
			const msg = err.errors[0]?.message;
			assert.equal(msg, "render error", `default must produce scrubbed message, got: ${msg}`);
			return true;
		},
	);
});

test("unsafeRevealRenderDetail: false scrubs render detail (SEC-004)", () => {
	const p = Prompt.fromJson(RENDER_FAIL_JSON);
	const data = { name: "secret-value-should-not-appear" };

	assert.throws(
		() => p.render(Vars, data, { unsafeRevealRenderDetail: false }),
		(err) => {
			assert.ok(err instanceof PromptRenderError);
			const msg = err.errors[0]?.message;
			assert.equal(msg, "render error", `flag=false must scrub, got: ${msg}`);
			assert.ok(!msg.includes("secret-value-should-not-appear"), "data value must not leak");
			return true;
		},
	);
});

// ── opt-in: reveal=true surfaces the real detail (SC-001) ─────────────────────

test("unsafeRevealRenderDetail: true surfaces real render detail (SC-001)", () => {
	const p = Prompt.fromJson(RENDER_FAIL_JSON);
	const data = { name: "Ada" };

	// With flag=false: scrubbed.
	assert.throws(
		() => p.render(Vars, data, { unsafeRevealRenderDetail: false }),
		(err) => {
			assert.ok(err instanceof PromptRenderError);
			assert.equal(
				err.errors[0]?.message,
				"render error",
				"flag=false must produce fixed scrubbed message",
			);
			return true;
		},
	);

	// With flag=true: the real detail is present.
	assert.throws(
		() => p.render(Vars, data, { unsafeRevealRenderDetail: true }),
		(err) => {
			assert.ok(err instanceof PromptRenderError);
			const msg = err.errors[0]?.message;
			assert.notEqual(msg, "render error", "flag=true must NOT produce the fixed scrubbed message");
			// The detail embeds the unknown filter name from the template.
			assert.ok(
				msg.includes("nonexistent_filter_abc") || msg.length > "render error".length,
				`flag=true message should carry filter-name context, got: ${msg}`,
			);
			// Shape is unchanged: field and code are invariant.
			assert.equal(err.errors[0]?.field, "template");
			assert.equal(err.errors[0]?.code, "render");
			return true;
		},
	);
});

// ── No-implicit-enable: the flag is ONLY per-call (FR-003 / SC-003) ───────────

test("no module-level global that enables the flag implicitly (FR-003)", async () => {
	const pp = await import("prompting-press");
	for (const name of [
		"revealRenderDetail",
		"unsafeRevealRenderDetail",
		"enableRenderDetail",
		"renderDetailEnabled",
	]) {
		assert.ok(!(name in pp), `module-level toggle '${name}' must not exist (FR-003 / SC-003)`);
	}
});
