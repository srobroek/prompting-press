/**
 * Selecting a variant at render: omit the name for the default (root body),
 * pass `{ variant }` for that arm. The resolved name comes back on
 * `RenderResult.variant` and the text is that arm's rendered body. Standalone.
 */

import assert from "node:assert/strict";
import { Prompt } from "prompting-press";
import { z } from "zod";

const summaryYaml = `
name: summary
role: user
body: "Summarise the following article in {{ max_words }} words:\\n\\n{{ article }}"
variables:
  article:
    type: string
    trusted: false
  max_words:
    type: integer
    trusted: true
variants:
  concise:
    body: "In one sentence, summarise: {{ article }}"
`;

const SummaryVars = z.object({
  article: z.string().min(1),
  max_words: z.number().int().min(1),
});

const summary = Prompt.fromYaml(summaryYaml);
const data = { article: "The Nile floods yearly.", max_words: 20 };

const def = summary.render(SummaryVars, data); // default (root body)
const concise = summary.render(SummaryVars, data, { variant: "concise" });

assert.equal(def.variant, "default");
assert.equal(
  def.text,
  "Summarise the following article in 20 words:\n\nThe Nile floods yearly.",
);
assert.equal(concise.variant, "concise");
assert.equal(concise.text, "In one sentence, summarise: The Nile floods yearly.");
