import assert from "node:assert/strict";
import { test } from "node:test";
import { z } from "zod";
import { Composition, Prompt } from "prompting-press";

const SysVars = z.object({ instructions: z.string().min(1) });
const UserVars = z.object({ query: z.string().min(1) });

// Build the two prompts inline from their shape, so the content is explicit.
const sysPrompt = new Prompt({
  name: "system-preamble",
  role: "system",
  body: "{{ instructions }}",
  variables: { instructions: { type: "string", trusted: true } },
});
const userPrompt = new Prompt({
  name: "user-turn",
  role: "user",
  body: "{{ query }}",
  variables: { query: { type: "string", trusted: false } },
});

test("compose with fromMessages", () => {
  // fromMessages: all validation runs before any Composition state is created.
  const comp = Composition.fromMessages([
    { prompt: sysPrompt,  schema: SysVars, data: { instructions: "Be concise." } },
    { prompt: userPrompt, schema: UserVars, data: { query: "What is Rust?" } },
  ]);

  const messages = comp.resolve();
  for (const m of messages) {
    console.log(m.role, m.text);
    // "system" "Be concise."
    // "user"   "What is Rust?"
  }

  assert.deepEqual(
    messages.map((m) => [m.role, m.text]),
    [
      ["system", "Be concise."],
      ["user", "What is Rust?"],
    ],
  );
});
