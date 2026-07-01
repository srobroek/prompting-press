import assert from "node:assert/strict";
import { test } from "node:test";
import { Prompt } from "prompting-press";
import { z } from "zod";

// A real consumer reads this text from a file; the library does no file I/O itself.
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

test("complete example", () => {
  // 1. Construct (validates here).
  const greet = Prompt.fromYaml(greetYaml);

  // 2 + 3. Render with the typed, Zod-validated vars.
  const result = greet.render(GreetVars, { name: "Ada", count: 3 });
  console.log(result.text); // Hi Ada, you have 3 messages.
  console.log(result.templateHash); // 64-char hex

  assert.equal(result.text, "Hi Ada, you have 3 messages.");
  assert.match(result.templateHash, /^[0-9a-f]{64}$/);
});
