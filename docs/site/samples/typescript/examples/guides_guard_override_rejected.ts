// A non-conforming advisory override (missing the required markers) is rejected
// by the kernel and throws `PromptRenderError` with `code === "render"` and
// `field === "guard"`. Standalone — run under `node --test`.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import { Prompt, PromptRenderError } from "prompting-press";
import { z } from "zod";

// The caller reads the definition; the library does no file I/O itself.
// Resolve the file next to this program (a real app uses its own path).
const defFile = (name: string) => fileURLToPath(new URL(name, import.meta.url));

const ask = Prompt.fromYaml(readFileSync(defFile("ask.yaml"), "utf8"));

const Ask = z.object({ topic: z.string().min(1) });

test("a non-conforming advisory override throws PromptRenderError", () => {
	try {
		ask.render(
			Ask,
			{ topic: "x" },
			{
				guard: { enabled: true, advisory: "Missing the required markers." },
			},
		);
		assert.fail("a non-conforming advisory must be rejected");
	} catch (err) {
		assert.ok(err instanceof PromptRenderError);
		assert.equal(err.errors[0].code, "render");
		assert.equal(err.errors[0].field, "guard");
	}
});
