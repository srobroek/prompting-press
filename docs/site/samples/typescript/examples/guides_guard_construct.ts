// The one-variable `ask` prompt used throughout the guard guide: `topic` is
// declared untrusted (trusted: false). Standalone — run under `node --test`.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import { Prompt } from "prompting-press";
import { z } from "zod";

// The caller reads the definition; the library does no file I/O itself.
// Resolve the file next to this program (a real app uses its own path).
const defFile = (name: string) => fileURLToPath(new URL(name, import.meta.url));

// The prompt: `topic` is declared untrusted (trusted: false).
const ask = Prompt.fromYaml(readFileSync(defFile("ask.yaml"), "utf8"));

// The typed vars schema handed to render().
const Ask = z.object({ topic: z.string().min(1) });

test("the ask prompt renders plainly without the guard", () => {
	const result = ask.render(Ask, { topic: "rivers" });
	assert.equal(result.text, "Tell me about rivers.");
	assert.equal(result.guard, null);
});
