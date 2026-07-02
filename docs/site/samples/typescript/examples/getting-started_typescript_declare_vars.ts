import assert from "node:assert/strict";
import { test } from "node:test";
import { z } from "zod";

// The render values are a caller-owned Zod schema. Its keys match the prompt's
// `variables` (`company`, `max_words`), and `safeParse` runs before the kernel is touched.
const AssistantVars = z.object({
  company: z.string().min(1),
  max_words: z.number().int().min(1),
});

test("the vars schema validates matching data", () => {
  const parsed = AssistantVars.safeParse({ company: "Acme Robotics", max_words: 50 });
  assert.ok(parsed.success);
});
