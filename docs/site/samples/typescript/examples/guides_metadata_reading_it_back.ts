/**
 * Reading prompt-level and per-variant metadata back out. The library stores the
 * opaque `metadata` maps and echoes them through accessors; it never interprets
 * them. The accessors return the maps as-is; application code interprets them.
 *
 * Standalone — the doc-sample harness type-checks and runs this file; its
 * assertions are in-program.
 */

import assert from "node:assert/strict";
import { Prompt } from "prompting-press";

// A real consumer would read this from a file; inlined here so the sample is standalone.
const yamlText = `
name: summary
role: user
body: "Summarise {{ article }}."
variables:
  article:
    type: string
    trusted: false
metadata:
  model_hint: claude-sonnet-4-6
  max_tokens: 512
  owner: team-content
variants:
  terse:
    body: "TL;DR of {{ article }}."
    metadata:
      weight: 0.2
      group: experiment-q4
`;

const p = Prompt.fromYaml(yamlText);

p.metadata;          // => { model_hint: "claude-sonnet-4-6", max_tokens: 512, owner: "team-content" }
p.metadata.model_hint;     // application code decides what to do with it

// per-variant metadata (p.variants is undefined when there are no named variants):
p.variants?.["terse"]?.metadata;  // => { weight: 0.2, group: "experiment-q4" }

// The accessors return the maps as-is; nothing is interpreted or mutated.
assert.equal(p.metadata.model_hint, "claude-sonnet-4-6");
assert.equal(p.metadata.max_tokens, 512);
assert.equal(p.metadata.owner, "team-content");
assert.deepEqual(p.variants?.["terse"]?.metadata, {
  weight: 0.2,
  group: "experiment-q4",
});
