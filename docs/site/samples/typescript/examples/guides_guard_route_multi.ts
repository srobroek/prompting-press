// Routing the guard text for a multi-message composition: prepend the advisory
// as a system message, then the resolved body messages. Standalone — run under
// `node --test`.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import { Composition, Prompt } from "prompting-press";
import { z } from "zod";

// The caller reads the definition; the library does no file I/O itself.
// Resolve the file next to this program (a real app uses its own path).
const defFile = (name: string) => fileURLToPath(new URL(name, import.meta.url));

const ask = Prompt.fromYaml(readFileSync(defFile("ask.yaml"), "utf8"));

const Ask = z.object({ topic: z.string().min(1) });

test("prepend the guard advisory to the resolved composition body", () => {
	// A single-render result supplies the guard advisory to prepend.
	const result = ask.render(
		Ask,
		{ topic: "rivers" },
		{ guard: { enabled: true } },
	);

	// The body messages come from a composition.
	const comp = new Composition();
	comp.append({ prompt: ask, schema: Ask, data: { topic: "rivers" } });

	const guardMsg = { role: "system", content: result.guard };
	const bodyMessages = comp
		.resolve()
		.map((m) => ({ role: m.role, content: m.text }));
	const requestMessages = [guardMsg, ...bodyMessages];

	assert.equal(requestMessages[0].role, "system");
	assert.ok(requestMessages[0].content?.includes("<untrusted>"));
	assert.equal(requestMessages[1].role, "user");
	// Composition never applies the guard, so its body is the plain render.
	assert.equal(requestMessages[1].content, "Tell me about rivers.");
});
