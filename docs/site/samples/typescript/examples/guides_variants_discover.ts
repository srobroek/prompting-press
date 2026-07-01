/**
 * Discovering the selectable variants: `variants` is the declared variant map;
 * read its keys to see what is selectable (the default arm is not listed — it is
 * the root body, name `"default"`). Standalone program.
 */

import assert from "node:assert/strict";
import { Prompt } from "prompting-press";

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
  structured:
    body: "Summarise {{ article }} as a title, three bullets, and a one-line conclusion."
`;

const summary = Prompt.fromYaml(summaryYaml);
const variants = summary.variants ?? {};

assert.deepEqual(Object.keys(variants).sort(), ["concise", "structured"]);
assert.ok("concise" in variants); // true
