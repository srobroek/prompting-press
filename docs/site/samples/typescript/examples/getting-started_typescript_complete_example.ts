import assert from "node:assert/strict";
import { test } from "node:test";
import { Prompt } from "prompting-press";
import { z } from "zod";

// A real consumer reads this text from a file; the library does no file I/O itself.
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

test("complete example", () => {
  // 1. Construct (validates here).
  const assistant = Prompt.fromYaml(assistantYaml);

  // 2 + 3. Render with the typed, Zod-validated vars.
  const result = assistant.render(AssistantVars, { company: "Acme Robotics", max_words: 50 });
  console.log(result.text); // You are a support assistant for Acme Robotics. Keep your replies under 50 words.
  console.log(result.templateHash); // 64-char hex

  assert.equal(result.text, "You are a support assistant for Acme Robotics. Keep your replies under 50 words.");
  assert.match(result.templateHash, /^[0-9a-f]{64}$/);
});
