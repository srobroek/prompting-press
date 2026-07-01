/**
 * Constructing a three-arm prompt inline from a typed shape object — the
 * `variants` map mirrors the document form one-to-one and construction runs the
 * same validation as `Prompt.fromYaml`. Standalone program.
 */

import assert from "node:assert/strict";
import { Prompt, type PromptDefinition } from "prompting-press";

const definition: PromptDefinition = {
  name: "summary",
  role: "user",
  body: "Summarise the following article in {{ max_words }} words:\n\n{{ article }}",
  variables: {
    article: { type: "string", trusted: false },
    max_words: { type: "integer", trusted: true },
  },
  variants: {
    concise: { body: "In one sentence, summarise: {{ article }}" },
    structured: {
      body: "Summarise {{ article }} as a title, three bullets, and a one-line conclusion.",
      metadata: { group: "experiment-q4" },
    },
  },
};

const summary = new Prompt(definition); // same validation as Prompt.fromYaml

assert.deepEqual(Object.keys(summary.variants ?? {}).sort(), ["concise", "structured"]);
