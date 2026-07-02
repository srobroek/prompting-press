/**
 * Derive guide — re-validation on overlay: overlaying a body that references an undeclared
 * variable throws `PromptRenderError` (agreement failure over the merged whole).
 *
 * Standalone — the docs page displays this file verbatim; run it directly to check.
 */

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import { Prompt, PromptRenderError } from "prompting-press";

// The caller reads the definition; the library does no file I/O itself.
// Resolve the file next to this program (a real app uses its own path).
const defFile = (name: string) => fileURLToPath(new URL(name, import.meta.url));

test("derive re-validates the merged whole and rejects an undeclared variable", () => {
	const assistant = Prompt.fromYaml(readFileSync(defFile("assistant.yaml"), "utf8"));

	try {
		const bad = assistant.derive({ body: "You help {{ ghost }}." });
		throw new Error(`expected PromptRenderError, got ${JSON.stringify(bad)}`);
	} catch (err) {
		if (err instanceof PromptRenderError) {
			console.error(err.errors[0].code); // "undefined_variable"
			console.error(err.errors[0].field); // "ghost"
			assert.equal(err.errors[0].code, "undefined_variable");
			assert.equal(err.errors[0].field, "ghost");
		} else {
			throw err;
		}
	}
});
