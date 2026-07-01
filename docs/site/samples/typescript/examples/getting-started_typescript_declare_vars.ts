import assert from "node:assert/strict";
import { test } from "node:test";
import { z } from "zod";

// The render values are a caller-owned Zod schema. Its keys match the prompt's
// `variables` (`name`, `count`), and `safeParse` runs before the kernel is touched.
const GreetVars = z.object({
  name: z.string().min(1),
  count: z.number().int().nonnegative(),
});

test("the vars schema validates matching data", () => {
  const parsed = GreetVars.safeParse({ name: "Ada", count: 3 });
  assert.ok(parsed.success);
});
