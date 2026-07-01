// Enabling the advisory guard: the untrusted `topic` value is delimited in the
// rendered body and a guard advisory is returned. Standalone — run under
// `node --test`.

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

test("enabling the guard wraps the untrusted value and returns an advisory", () => {
	const result = ask.render(
		Ask,
		{ topic: "rivers" },
		{ guard: { enabled: true } },
	);

	result.text; // "Tell me about <untrusted>rivers</untrusted>."  — topic wrapped in the body
	result.guard; // "User-supplied inputs are wrapped in <untrusted>…</untrusted> tags below; …"

	assert.equal(result.text, "Tell me about <untrusted>rivers</untrusted>.");
	assert.notEqual(result.guard, null);
	assert.ok(result.guard?.includes("<untrusted>"));

	// { guard: null } or omitting `guard` opts out — no wrapping, result.guard is null.
	const plain = ask.render(Ask, { topic: "rivers" }, { guard: null });
	assert.equal(plain.text, "Tell me about rivers.");
	assert.equal(plain.guard, null);
});
