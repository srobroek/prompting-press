import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import { Prompt } from "prompting-press";

// The caller reads the definition; the library does no file I/O itself.
// Resolve the file next to this program (a real app uses its own path).
const defFile = (name: string) => fileURLToPath(new URL(name, import.meta.url));

test("construct from a file", () => {
  const assistant = Prompt.fromYaml(readFileSync(defFile("assistant.yaml"), "utf8")); // validates here, or throws
  // The same definition in JSON or TOML parses into an identical Prompt:
  // const assistant = Prompt.fromJson(readFileSync(defFile("assistant.json"), "utf8"));
  // const assistant = Prompt.fromToml(readFileSync(defFile("assistant.toml"), "utf8"));

  assert.equal(assistant.name, "assistant");
  assert.equal(
    assistant.body,
    "You are a support assistant for {{ company }}. Keep your replies under {{ max_words }} words.",
  );
});
