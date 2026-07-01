/**
 * Derive guide — re-validation on overlay: overlaying a body that references an undeclared
 * variable throws `PromptRenderError` (agreement failure over the merged whole).
 *
 * Standalone — the docs page displays this file verbatim; run it directly to check.
 */

import assert from "node:assert/strict";
import { test } from "node:test";
import { Prompt, PromptRenderError } from "prompting-press";

const greetYaml = `
name: greet
role: user
body: "Hi {{ name }}, you have {{ count }} messages."
variables:
  name: { type: string, trusted: true }
  count: { type: integer, trusted: true }
`;

test("derive re-validates the merged whole and rejects an undeclared variable", () => {
	const greet = Prompt.fromYaml(greetYaml);

	try {
		const bad = greet.derive({ body: "Hi {{ ghost }}" });
		throw new Error(`expected PromptRenderError, got ${JSON.stringify(bad)}`);
	} catch (err) {
		if (err instanceof PromptRenderError) {
			console.error(err.errors[0].code); // "undefined_variable"
			console.error(err.errors[0].field); // "ghost"
			assert.equal(err.errors[0].code, "undefined_variable");
			assert.equal(err.errors[0].field, "ghost");
		} else {
			throw err;
		}
	}
});
