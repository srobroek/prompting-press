import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import { Prompt } from "prompting-press";
import { z } from "zod";

// The caller reads the definition; the library does no file I/O itself.
// Resolve the file next to this program (a real app uses its own path).
const defFile = (name: string) => fileURLToPath(new URL(name, import.meta.url));

const AssistantVars = z.object({
  company: z.string().min(1),
  max_words: z.number().int().min(1),
});

test("complete example", () => {
  // 1. Construct from the definition file (validates here).
  const assistant = Prompt.fromYaml(readFileSync(defFile("assistant.yaml"), "utf8"));

  // 2 + 3. Render with the typed, Zod-validated vars.
  const result = assistant.render(AssistantVars, { company: "Acme Robotics", max_words: 50 });
  console.log(result.text); // You are a support assistant for Acme Robotics. Keep your replies under 50 words.
  console.log(result.templateHash); // 64-char hex

  assert.equal(result.text, "You are a support assistant for Acme Robotics. Keep your replies under 50 words.");
  assert.match(result.templateHash, /^[0-9a-f]{64}$/);
});
