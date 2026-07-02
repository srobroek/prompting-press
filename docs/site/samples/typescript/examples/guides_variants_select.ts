/**
 * Selecting a variant at render: omit the name for the default (root body),
 * pass `{ variant }` for that arm. The resolved name comes back on
 * `RenderResult.variant` and the text is that arm's rendered body. Standalone.
 */

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { Prompt } from "prompting-press";
import { z } from "zod";

// The caller reads the definition; the library does no file I/O itself.
// Resolve the file next to this program (a real app uses its own path).
const defFile = (name: string) => fileURLToPath(new URL(name, import.meta.url));

const SummaryVars = z.object({
  article: z.string().min(1),
  max_words: z.number().int().min(1),
});

const summary = Prompt.fromYaml(readFileSync(defFile("summary.yaml"), "utf8"));
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
