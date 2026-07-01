// The one-variable `ask` prompt used throughout the guard guide: `topic` is
// declared untrusted (trusted: false). Standalone — run under `node --test`.

import assert from "node:assert/strict";
import { test } from "node:test";
import { Prompt } from "prompting-press";
import { z } from "zod";

// The prompt: `topic` is declared untrusted (trusted: false).
const ask = Prompt.fromYaml(`
name: ask
role: user
body: "Tell me about {{ topic }}."
variables:
  topic:
    type: string
    trusted: false
`);

// The typed vars schema handed to render().
const Ask = z.object({ topic: z.string().min(1) });

test("the ask prompt renders plainly without the guard", () => {
	const result = ask.render(Ask, { topic: "rivers" });
	assert.equal(result.text, "Tell me about rivers.");
	assert.equal(result.guard, null);
});
