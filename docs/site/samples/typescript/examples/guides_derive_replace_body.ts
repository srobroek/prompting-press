/**
 * Derive guide — replace only the root body (the default arm) with `derive`.
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

test("derive replaces only the root body", () => {
	const greet = Prompt.fromYaml(greetYaml);

	const briefGreet = greet.derive({ body: "Hi {{ name }}!" });

	const result = briefGreet.render(GreetVars, { name: "Ada", count: 3 });
	assert.equal(result.text, "Hi Ada!");
});
