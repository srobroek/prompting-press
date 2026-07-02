/**
 * Derive guide — add a variant at runtime: spread the current `variants` map (read via the
 * accessor) into the overlay, then add one — so existing arms survive. `derive` is the sole
 * mutator; the original is untouched.
 *
 * Standalone — the docs page displays this file verbatim; run it directly to check.
 */

import assert from "node:assert/strict";
import { test } from "node:test";
import { Prompt } from "prompting-press";

const assistantYaml = `
name: assistant
role: system
body: "You are a support assistant for {{ company }}. Keep your replies under {{ max_words }} words."
variables:
  company: { type: string, trusted: true }
  max_words: { type: integer, trusted: true }
`;

test("derive adds a variant, leaving the original untouched", () => {
	const assistant = Prompt.fromYaml(assistantYaml);

	// READ the current variants (spread), then add one — so existing arms survive.
	const derivedAssistant = assistant.derive({
		variants: {
			...assistant.variants, // keep what's already there
			formal: {
				body: "You are the official support assistant for {{ company }}. Please keep every reply under {{ max_words }} words.",
			},
		},
	});
	// assistant is unchanged; derivedAssistant is a new, fully-validated Prompt.

	assert.deepEqual(assistant.variants ?? {}, {}, "original is untouched");
	assert.ok("formal" in (derivedAssistant.variants ?? {}));
});
