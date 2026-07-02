/**
 * Derive guide — re-validation on overlay: overlaying a body that references an undeclared
 * variable throws `PromptRenderError` (agreement failure over the merged whole).
 *
 * Standalone — the docs page displays this file verbatim; run it directly to check.
 */

import assert from "node:assert/strict";
import { test } from "node:test";
import { Prompt, PromptRenderError } from "prompting-press";

const assistantYaml = `
name: assistant
role: system
body: "You are a support assistant for {{ company }}. Keep your replies under {{ max_words }} words."
variables:
  company: { type: string, trusted: true }
  max_words: { type: integer, trusted: true }
`;

test("derive re-validates the merged whole and rejects an undeclared variable", () => {
	const assistant = Prompt.fromYaml(assistantYaml);

	try {
		const bad = assistant.derive({ body: "You help {{ ghost }}." });
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
