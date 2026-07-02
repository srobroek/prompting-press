/**
 * Derive guide — the starting pair: an `assistant` prompt (a `company` + `max_words` system
 * body) and a matching `AssistantVars`. Every later example on the page derives from this.
 *
 * Standalone — the docs page displays this file verbatim; run it directly to check.
 */

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

test("assistant + AssistantVars is the starting pair", () => {
	// The pair parses and validates: the body's {{ company }}/{{ max_words }} agree with AssistantVars.
	const assistant = Prompt.fromYaml(readFileSync(defFile("assistant.yaml"), "utf8"));
	assert.equal(assistant.name, "assistant");

	// AssistantVars is a plain Zod schema — parse a value to prove the shape.
	const vars = AssistantVars.parse({ company: "Acme Robotics", max_words: 50 });
	assert.equal(vars.company, "Acme Robotics");
	assert.equal(vars.max_words, 50);
});
