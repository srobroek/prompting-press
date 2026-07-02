/**
 * Derive guide — replace only the root body (the default arm) with `derive`.
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

test("derive replaces only the root body", () => {
	const assistant = Prompt.fromYaml(readFileSync(defFile("assistant.yaml"), "utf8"));

	const briefAssistant = assistant.derive({ body: "You are a support assistant for {{ company }}." });

	const result = briefAssistant.render(AssistantVars, { company: "Acme Robotics", max_words: 50 });
	assert.equal(result.text, "You are a support assistant for Acme Robotics.");
});
