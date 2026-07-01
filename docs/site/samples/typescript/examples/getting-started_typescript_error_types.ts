import assert from "node:assert/strict";
import { test } from "node:test";
import {
  Prompt,
  PromptingPressError,
  PromptValidationError,
  PromptRenderError,
  LoadError,
} from "prompting-press";
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

test("a rejected render surfaces a structured PromptValidationError", () => {
  let caught = false;
  try {
    greet.render(GreetVars, { name: "Ada", count: -1 });
  } catch (err) {
    if (err instanceof PromptValidationError) {
      caught = true;
      for (const row of err.errors) {
        console.error(row.field, row.code, row.message);
        // "count"  "validation"  "Too small: expected number to be >=0"
      }
      assert.equal(err.errors[0]?.field, "count");
      assert.equal(err.errors[0]?.code, "validation");
    }
  }
  assert.ok(caught, "expected a PromptValidationError");

  // The error hierarchy: every type extends the base, which extends Error.
  assert.ok(PromptValidationError.prototype instanceof PromptingPressError);
  assert.ok(PromptRenderError.prototype instanceof PromptingPressError);
  assert.ok(LoadError.prototype instanceof PromptingPressError);
  assert.ok(PromptingPressError.prototype instanceof Error);
});
