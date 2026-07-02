/**
 * Derive guide — render a named variant: after adding a variant with `derive`, select it by
 * name at render time. Variant selection is caller-owned.
 *
 * Standalone — the docs page displays this file verbatim; run it directly to check.
 */

import assert from "node:assert/strict";
import { test } from "node:test";
import { Prompt } from "prompting-press";
import { z } from "zod";

const assistantYaml = `
name: assistant
role: system
body: "You are a support assistant for {{ company }}. Keep your replies under {{ max_words }} words."
variables:
  company: { type: string, trusted: true }
  max_words: { type: integer, trusted: true }
`;

const AssistantVars = z.object({
	company: z.string().min(1),
	max_words: z.number().int().min(1),
});

test("a derived named variant renders by name", () => {
	const assistant = Prompt.fromYaml(assistantYaml);
	const derivedAssistant = assistant.derive({
		variants: {
			...assistant.variants,
			formal: {
				body: "You are the official support assistant for {{ company }}. Please keep every reply under {{ max_words }} words.",
			},
		},
	});

	const result = derivedAssistant.render(
		AssistantVars,
		{ company: "Acme Robotics", max_words: 50 },
		{ variant: "formal" },
	);
	console.log(result.text); // "You are the official support assistant for Acme Robotics. Please keep every reply under 50 words."
	console.log(result.variant); // "formal"
	assert.equal(
		result.text,
		"You are the official support assistant for Acme Robotics. Please keep every reply under 50 words.",
	);
	assert.equal(result.variant, "formal");
});
