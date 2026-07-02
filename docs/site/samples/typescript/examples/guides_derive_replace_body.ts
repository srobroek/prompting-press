/**
 * Derive guide — replace only the root body (the default arm) with `derive`.
 *
 * Standalone — the docs page displays this file verbatim; run it directly to check.
 */

import assert from "node:assert/strict";
import { test } from "node:test";
import { Prompt } from "prompting-press";
import { z } from "zod";

const assistantYaml = `
name: assistant
role: system
body: "You are a support assistant for {{ company }}. Keep your replies under {{ max_words }} words."
variables:
  company: { type: string, trusted: true }
  max_words: { type: integer, trusted: true }
`;

const AssistantVars = z.object({
	company: z.string().min(1),
	max_words: z.number().int().min(1),
});

test("derive replaces only the root body", () => {
	const assistant = Prompt.fromYaml(assistantYaml);

	const briefAssistant = assistant.derive({ body: "You are a support assistant for {{ company }}." });

	const result = briefAssistant.render(AssistantVars, { company: "Acme Robotics", max_words: 50 });
	assert.equal(result.text, "You are a support assistant for Acme Robotics.");
});
