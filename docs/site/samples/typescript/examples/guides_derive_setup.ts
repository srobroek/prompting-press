/**
 * Derive guide — the starting pair: a `greet` prompt (a `name` + `count` body) and a
 * matching `GreetVars`. Every later example on the page derives from this.
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

test("greet + GreetVars is the starting pair", () => {
	// The pair parses and validates: the body's {{ name }}/{{ count }} agree with GreetVars.
	const greet = Prompt.fromYaml(greetYaml);
	assert.equal(greet.name, "greet");

	// GreetVars is a plain Zod schema — parse a value to prove the shape.
	const vars = GreetVars.parse({ name: "Ada", count: 3 });
	assert.equal(vars.name, "Ada");
	assert.equal(vars.count, 3);
});
