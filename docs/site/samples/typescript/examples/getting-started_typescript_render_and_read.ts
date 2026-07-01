import assert from "node:assert/strict";
import { test } from "node:test";
import { Prompt } from "prompting-press";
import { z } from "zod";

const greetYaml = `name: greet
role: user
body: "Hi {{ name }}, you have {{ count }} messages."
variables:
  name:
    type: string
    trusted: true
  count:
    type: integer
    trusted: true
`;

const GreetVars = z.object({
  name: z.string().min(1),
  count: z.number().int().nonnegative(),
});

const greet = Prompt.fromYaml(greetYaml);

test("render, and read the result", () => {
  const result = greet.render(GreetVars, { name: "Ada", count: 3 });

  assert.equal(result.text, "Hi Ada, you have 3 messages."); // => "Hi Ada, you have 3 messages."
  assert.equal(result.variant, "default"); // => "default"  (same arm greet.body showed in Step 1)
  assert.match(result.templateHash, /^[0-9a-f]{64}$/); // 64-char lowercase-hex SHA-256 of the template
  assert.match(result.renderHash, /^[0-9a-f]{64}$/); // 64-char lowercase-hex SHA-256 of result.text
  assert.equal(result.guard, null); // => null  (no guard requested)
});
