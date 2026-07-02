import assert from "node:assert/strict";
import { test } from "node:test";
import { Prompt } from "prompting-press";

// A real consumer reads this text from a file; the library does no file I/O itself.
const assistantYaml = `name: assistant
role: system
body: "You are a support assistant for {{ company }}. Keep your replies under {{ max_words }} words."
variables:
  company:
    type: string
    trusted: true
  max_words:
    type: integer
    trusted: true
`;

test("construct from text", () => {
  const assistant = Prompt.fromYaml(assistantYaml); // validates here, or throws
  // fromJson / fromToml parse the JSON / TOML forms into the same Prompt.
  // (TOML is parsed by Rust — no JS TOML dependency.)

  assert.equal(assistant.name, "assistant"); // => "assistant"
  assert.equal(
    assistant.body,
    "You are a support assistant for {{ company }}. Keep your replies under {{ max_words }} words.",
  );
});
