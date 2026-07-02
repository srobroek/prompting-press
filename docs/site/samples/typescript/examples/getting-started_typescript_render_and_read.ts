import assert from "node:assert/strict";
import { test } from "node:test";
import { Prompt, PromptValidationError } from "prompting-press";
import { z } from "zod";

const assistantYaml = `name: assistant
role: system
body: "You are a support assistant for {{ company }}. Keep your replies under {{ max_words }} words."
variables:
  company:
    type: string
    trusted: true
  max_words:
    type: integer
    trusted: true
`;

const AssistantVars = z.object({
  company: z.string().min(1),
  max_words: z.number().int().min(1),
});

const assistant = Prompt.fromYaml(assistantYaml);

test("render, and read the result", () => {
  const result = assistant.render(AssistantVars, { company: "Acme Robotics", max_words: 50 });

  assert.equal(result.text, "You are a support assistant for Acme Robotics. Keep your replies under 50 words."); // => "You are a support assistant for Acme Robotics. Keep your replies under 50 words."
  assert.equal(result.variant, "default"); // => "default"  (same arm assistant.body showed in Step 1)
  assert.match(result.templateHash, /^[0-9a-f]{64}$/); // 64-char lowercase-hex SHA-256 of the template
  assert.match(result.renderHash, /^[0-9a-f]{64}$/); // 64-char lowercase-hex SHA-256 of result.text
  assert.equal(result.guard, null); // => null  (no guard requested)
});

test("render validates the data through the schema — bad data is rejected before the kernel", () => {
  // max_words: 0 violates AssistantVars (.int().min(1)); render throws, nothing is rendered.
  assert.throws(
    () => assistant.render(AssistantVars, { company: "Acme Robotics", max_words: 0 }),
    PromptValidationError,
  );
});
