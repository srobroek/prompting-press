import assert from "node:assert/strict";
import { test } from "node:test";
import { Prompt } from "prompting-press";

// A real consumer reads this text from a file; the library does no file I/O itself.
const greetYaml = `name: greet
role: user
body: "Hi {{ name }}, you have {{ count }} messages."
variables:
  name:
    type: string
    trusted: true
  count:
    type: integer
    trusted: true
`;

test("construct from text", () => {
  const greet = Prompt.fromYaml(greetYaml); // validates here, or throws
  // fromJson / fromToml parse the JSON / TOML forms into the same Prompt.
  // (TOML is parsed by Rust — no JS TOML dependency.)

  assert.equal(greet.name, "greet"); // => "greet"
  assert.equal(greet.body, "Hi {{ name }}, you have {{ count }} messages.");
});
