import assert from "node:assert/strict";
import { test } from "node:test";
import { Prompt, type PromptDefinition } from "prompting-press";

test("construct from an object", () => {
  // The constructor takes a typed PromptDefinition — an editor type-checks the
  // shape (field names, the role enum, each variable's `trusted` flag) at author time.
  const definition: PromptDefinition = {
    name: "greet",
    role: "user",
    body: "Hi {{ name }}, you have {{ count }} messages.",
    variables: {
      name: { type: "string", trusted: true },
      count: { type: "integer", trusted: true },
    },
  };

  const greet = new Prompt(definition); // same validation as the from* factories

  assert.equal(greet.name, "greet"); // => "greet"
});
