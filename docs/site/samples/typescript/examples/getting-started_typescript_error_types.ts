import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import {
  Prompt,
  PromptingPressError,
  PromptValidationError,
  PromptRenderError,
  LoadError,
} from "prompting-press";
import { z } from "zod";

// The caller reads the definition; the library does no file I/O itself.
// Resolve the file next to this program (a real app uses its own path).
const defFile = (name: string) => fileURLToPath(new URL(name, import.meta.url));

const AssistantVars = z.object({
  company: z.string().min(1),
  max_words: z.number().int().min(1),
});

const assistant = Prompt.fromYaml(readFileSync(defFile("assistant.yaml"), "utf8"));

test("a rejected render surfaces a structured PromptValidationError", () => {
  let caught = false;
  try {
    assistant.render(AssistantVars, { company: "Acme Robotics", max_words: 0 });
  } catch (err) {
    if (err instanceof PromptValidationError) {
      caught = true;
      for (const row of err.errors) {
        console.error(row.field, row.code, row.message);
        // "max_words"  "validation"  "Too small: expected number to be >=1"
      }
      assert.equal(err.errors[0]?.field, "max_words");
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
