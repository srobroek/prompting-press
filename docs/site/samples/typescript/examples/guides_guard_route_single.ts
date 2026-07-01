// Routing the guard text for a single render: the advisory goes to a system
// message, the body to a user message. The library never concatenates them.
// Standalone — run under `node --test`.

import assert from "node:assert/strict";
import { test } from "node:test";
import { Prompt } from "prompting-press";
import { z } from "zod";

const ask = Prompt.fromYaml(`
name: ask
role: user
body: "Tell me about {{ topic }}."
variables:
  topic:
    type: string
    trusted: false
`);

const Ask = z.object({ topic: z.string().min(1) });

test("route the guard advisory to a system message beside the user body", () => {
	const result = ask.render(
		Ask,
		{ topic: "rivers" },
		{ guard: { enabled: true } },
	);

	const messages = [
		{ role: "system", content: result.guard },
		{ role: "user", content: result.text },
	];

	assert.equal(messages[0].role, "system");
	assert.ok(messages[0].content?.includes("<untrusted>"));
	assert.equal(messages[1].role, "user");
	assert.equal(
		messages[1].content,
		"Tell me about <untrusted>rivers</untrusted>.",
	);
});
