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

const greetYaml = `
name: greet
role: user
body: "Hi {{ name }}, you have {{ count }} messages."
variables:
  name: { type: string, trusted: true }
  count: { type: integer, trusted: true }
`;

const GreetVars = z.object({
	name: z.string().min(1),
	count: z.number().int().nonnegative(),
});

test("a derived named variant renders by name", () => {
	const greet = Prompt.fromYaml(greetYaml);
	const derivedGreet = greet.derive({
		variants: {
			...greet.variants,
			formal: { body: "Good day, {{ name }}. You have {{ count }} messages." },
		},
	});

	const result = derivedGreet.render(
		GreetVars,
		{ name: "Ada", count: 3 },
		{ variant: "formal" },
	);
	console.log(result.text); // "Good day, Ada. You have 3 messages."
	console.log(result.variant); // "formal"
	assert.equal(result.text, "Good day, Ada. You have 3 messages.");
	assert.equal(result.variant, "formal");
});
