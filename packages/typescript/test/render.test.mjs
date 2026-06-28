/**
 * US1 render-path tests for the TypeScript facade (`prompting-press`) — spec 005, T010.
 *
 * These exercise the TS-observable render path that the Rust `#[cfg(test)]` suite cannot reach
 * because it needs a real Zod schema: validate-in-TS at the render boundary (Q1), the normalized
 * error contract (FR-014, C-06), the SEC-004 scrub, the three-sets agreement gap (a loud
 * `undefined_variable`, never a silent empty render), and the guard plumb-through (FR-009).
 *
 * The tests run against the BUILT facade (`dist/index.js`, resolved via the package self-reference
 * `prompting-press`) layered over the BUILT napi addon — node:test, zero-dep, ESM.
 *
 * Model invariant under test: a single `render` returns the BODY as `.text` and any guard
 * instruction as the SEPARATE `.guard` field — the library never concatenates the two.
 */

import assert from "node:assert/strict";
import { test } from "node:test";

import { z } from "zod";

import {
  Registry,
  render,
  getSource,
  PromptingPressError,
  PromptValidationError,
  PromptRenderError,
  UnknownPromptError,
} from "prompting-press";

// A lowercase 64-char hex string — the SHA-256 provenance hash shape (FR-012/FR-013).
const HEX64 = /^[0-9a-f]{64}$/;

// --------------------------------------------------------------------------------------
// Zod Vars schemas (the per-language idiom; Principle VI). Each carries a real `.refine()` so
// validation is genuinely exercised, not a no-op pass-through.
// --------------------------------------------------------------------------------------

/** Happy-path Vars: a refine rejects a negative message count. */
const Greeting = z.object({
  name: z.string(),
  count: z.number().int().refine((n) => n >= 0, "count must be non-negative"),
});

/** Two independently-refined fields → a single bad input yields a multi-issue ZodError (SC-002). */
const TwoFields = z.object({
  name: z.string().refine((s) => s.length > 0, "name must not be empty"),
  count: z.number().int().refine((n) => n >= 0, "count must be non-negative"),
});

/**
 * A schema whose refine rejects a value that is itself sensitive (SEC-004). The refine message is
 * fixed and value-free; the rejected `token` must never reach the error surface.
 */
const Secretful = z.object({
  token: z.string().refine((v) => !v.startsWith("sk-"), "token has a forbidden prefix"),
});

/** Validates cleanly so the secret crosses the FFI boundary into the kernel (the kernel-path SEC-004). */
const Secret = z.object({ token: z.string() });

/** Three-sets gap: the schema field is `nam`; the template references `{{ name }}`. */
const Misnamed = z.object({ nam: z.string() });

/** Vars for the guard-plumb prompt: a single (untrusted) `topic` string. */
const Topic = z.object({ topic: z.string() });

// --------------------------------------------------------------------------------------
// Registry helpers + prompt definitions
// --------------------------------------------------------------------------------------

const GREET_YAML = `
name: greet
role: user
body: "Hi {{ name }}, you have {{ count }} messages"
variables:
  name:  { type: string,  provenance: trusted }
  count: { type: integer, provenance: trusted }
`;

/** A prompt whose only declared variable is untrusted, so an enabled guard has a field to name. */
const ASK_YAML = `
name: ask
role: user
body: "Tell me about {{ topic }}."
variables:
  topic: { type: string, provenance: untrusted }
`;

function greetRegistry() {
  const reg = new Registry();
  reg.loadYaml(GREET_YAML);
  return reg;
}

// --------------------------------------------------------------------------------------
// 1. Valid render (SC-001) — schema + data path
// --------------------------------------------------------------------------------------

test("valid render produces text, name, variant, and 64-hex provenance hashes", () => {
  const reg = greetRegistry();

  const result = render(reg, "greet", Greeting, { name: "Ada", count: 3 });

  assert.equal(result.text, "Hi Ada, you have 3 messages");
  assert.equal(result.name, "greet");
  assert.equal(result.variant, "default", "no variant selected ⇒ the reserved default arm");
  assert.match(result.templateHash, HEX64, result.templateHash);
  assert.match(result.renderHash, HEX64, result.renderHash);
  // No guard requested ⇒ the separate guard field is null (model: guard ≠ text).
  assert.equal(result.guard, null);
});

test("static (no-schema) data is accepted and marshaled directly (Q4)", () => {
  const reg = greetRegistry();

  const result = render(reg, "greet", { name: "Bo", count: 1 });

  assert.equal(result.text, "Hi Bo, you have 1 messages");
  assert.equal(result.variant, "default");
  assert.match(result.templateHash, HEX64);
  assert.match(result.renderHash, HEX64);
});

// --------------------------------------------------------------------------------------
// 2. Validation failure (SC-002 / Q1) — caught in TS, before any templating
// --------------------------------------------------------------------------------------

test("invalid input raises PromptValidationError naming the field, before any render", () => {
  const reg = greetRegistry();

  assert.throws(
    () => render(reg, "greet", Greeting, { name: "Ada", count: -1 }),
    (err) => {
      assert.ok(err instanceof PromptValidationError, "must be a PromptValidationError");
      const offending = err.errors.filter((row) => row.field === "count");
      assert.ok(offending.length > 0, `expected a row naming \`count\`, got ${JSON.stringify(err.errors)}`);
      assert.ok(
        offending.every((row) => row.code === "validation"),
        offending.map((row) => row.code).join(","),
      );
      return true;
    },
  );
});

test("validation failure names EVERY offending field (SC-002)", () => {
  const reg = greetRegistry();

  assert.throws(
    () => render(reg, "greet", TwoFields, { name: "", count: -1 }),
    (err) => {
      assert.ok(err instanceof PromptValidationError);
      const fields = new Set(err.errors.map((row) => row.field));
      assert.ok(fields.has("name") && fields.has("count"), `got ${[...fields].join(",")}`);
      assert.ok(err.errors.every((row) => row.code === "validation"));
      return true;
    },
  );
});

// --------------------------------------------------------------------------------------
// 3. No native error type leaks across the boundary (SC-006 / C-06)
// --------------------------------------------------------------------------------------

test("a validation error is a PromptValidationError and NOT a ZodError (SC-006)", () => {
  const reg = greetRegistry();

  assert.throws(
    () => render(reg, "greet", Greeting, { name: "Ada", count: -1 }),
    (err) => {
      assert.ok(err instanceof PromptingPressError, "in the binding hierarchy");
      assert.ok(err instanceof PromptValidationError, "specifically the validation subtype");
      // The native ZodError must not cross the boundary. (Zod errors are named "ZodError"
      // and are instances of Error; ours never are.)
      assert.notEqual(err.constructor.name, "ZodError");
      assert.ok(!(err instanceof z.ZodError), "must not be a ZodError instance");
      return true;
    },
  );
});

// --------------------------------------------------------------------------------------
// 4. SEC-004 — the rejected (sensitive) input never appears on the error surface
// --------------------------------------------------------------------------------------

test("a Zod-rejected sensitive value is not leaked (mapper copies issue message only)", () => {
  const secret = "sk-super-secret-token-9f8a7b6c5d4e";
  const reg = new Registry();
  reg.loadYaml(`
name: leaky
role: user
body: "Using {{ token }}"
variables:
  token: { type: string, provenance: trusted }
`);

  assert.throws(
    () => render(reg, "leaky", Secretful, { token: secret }),
    (err) => {
      assert.ok(err instanceof PromptValidationError);
      // Neither the message, the stack, nor any row may contain the rejected value — only the
      // refine's own value-free message is copied (issue.message; never issue.input).
      assert.ok(!String(err.message).includes(secret), `message leaked: ${err.message}`);
      assert.ok(!String(err.stack).includes(secret), "stack leaked the secret");
      for (const row of err.errors) {
        assert.ok(!row.message.includes(secret), `row message leaked: ${row.message}`);
        assert.ok(!row.field.includes(secret), `row field leaked: ${row.field}`);
      }
      // Positive check: the value-free refine message survives.
      assert.ok(err.errors.some((row) => row.message.includes("forbidden prefix")));
      return true;
    },
  );
});

test("a secret in a real kernel render-error value is not leaked (SEC-004 kernel path)", () => {
  // `Secret` validates cleanly, so the secret crosses the FFI boundary into the kernel, where
  // `{{ token + 1 }}` (string + int) is a genuine KernelError::Render. The consumer's scrubber
  // discards the raw detail (which embeds the bound value) and emits the fixed "render error".
  const secret = "sk-super-secret-token-9f8a7b6c5d4e";
  const reg = new Registry();
  reg.loadYaml(`
name: kernely
role: user
body: "Using {{ token + 1 }}"
variables:
  token: { type: string, provenance: trusted }
`);

  assert.throws(
    () => render(reg, "kernely", Secret, { token: secret }),
    (err) => {
      assert.ok(err instanceof PromptRenderError, "kernel rejection ⇒ PromptRenderError");
      assert.ok(!String(err.message).includes(secret), `message leaked: ${err.message}`);
      assert.ok(!String(err.stack).includes(secret), "stack leaked the secret");
      for (const row of err.errors) {
        assert.ok(!row.message.includes(secret), `row leaked: ${row.message}`);
        assert.ok(!row.field.includes(secret), `row.field leaked: ${row.field}`);
      }
      // The scrub replaces the value-bearing detail with the consumer's fixed message + code.
      assert.deepEqual(
        err.errors.map((row) => row.code),
        ["render"],
      );
      assert.ok(err.errors.some((row) => row.message === "render error"));
      return true;
    },
  );
});

// --------------------------------------------------------------------------------------
// 5. Three-sets gap — a Vars/template field-name mismatch is LOUD, not a silent empty render
// --------------------------------------------------------------------------------------

test("a Vars/template field-name mismatch is a loud undefined_variable (not a silent empty render)", () => {
  // The schema has `nam`; the template references `{{ name }}`.
  const reg = new Registry();
  reg.loadYaml(`
name: greet
role: user
body: "Hi {{ name }}!"
variables:
  name: { type: string, provenance: trusted }
`);

  // Validation passes (Misnamed is internally consistent) — the failure is at render.
  assert.throws(
    () => render(reg, "greet", Misnamed, { nam: "Ada" }),
    (err) => {
      assert.ok(err instanceof PromptRenderError, "a kernel rejection, not a validation error");
      const codes = err.errors.map((row) => row.code);
      assert.ok(
        codes.includes("undefined_variable"),
        `a referenced-but-undefined root must be a loud undefined_variable, got ${codes.join(",")}`,
      );
      return true;
    },
  );
});

// --------------------------------------------------------------------------------------
// 6. Guard plumb-through (FR-009) — guard text is SEPARATE from body text
// --------------------------------------------------------------------------------------

test("an enabled guard is plumbed through and stays separate from text", () => {
  const reg = new Registry();
  reg.loadYaml(ASK_YAML);

  const plain = render(reg, "ask", Topic, { topic: "rivers" });
  const guarded = render(reg, "ask", Topic, { topic: "rivers" }, { guard: { enabled: true } });

  // Default render ⇒ no guard.
  assert.equal(plain.guard, null);
  // Enabled guard on a prompt declaring an untrusted field ⇒ guard text present ...
  assert.notEqual(guarded.guard, null);
  assert.equal(typeof guarded.guard, "string");
  // ... and it names the untrusted field.
  assert.ok(guarded.guard.includes("topic"), guarded.guard);

  // The body text is IDENTICAL in both: the guard is never concatenated into `.text`.
  assert.equal(plain.text, "Tell me about rivers.");
  assert.equal(guarded.text, "Tell me about rivers.");
  // And the guard text is not smuggled into the body.
  assert.ok(!guarded.text.includes(guarded.guard));
});

test("a disabled / absent guard config matches no guard at all", () => {
  const reg = new Registry();
  reg.loadYaml(ASK_YAML);

  const noGuard = render(reg, "ask", Topic, { topic: "rivers" });
  const disabled = render(reg, "ask", Topic, { topic: "rivers" }, { guard: { enabled: false } });

  assert.equal(noGuard.guard, null);
  assert.equal(disabled.guard, null);
  assert.equal(noGuard.text, disabled.text);
});

// --------------------------------------------------------------------------------------
// 7. Unknown prompt — surfaced before marshaling (nothing rendered)
// --------------------------------------------------------------------------------------

test("an unknown prompt name raises UnknownPromptError", () => {
  const reg = greetRegistry();

  assert.throws(
    () => render(reg, "does-not-exist", Greeting, { name: "Ada", count: 1 }),
    (err) => {
      assert.ok(err instanceof UnknownPromptError);
      assert.ok(err instanceof PromptingPressError);
      return true;
    },
  );
});

// --------------------------------------------------------------------------------------
// 7b. Variant selection (FR-009 / Principle V) — opts.variant, caller-owned, parity with Py/Rust
// --------------------------------------------------------------------------------------

const VARIANT_YAML = `
name: greetv
role: user
body: "Hi {{ name }}"
variables:
  name: { type: string, provenance: trusted }
variants:
  formal: { body: "Good day, {{ name }}" }
`;

test("render selects a named variant via opts.variant (default arm when absent)", () => {
  const reg = new Registry();
  reg.loadYaml(VARIANT_YAML);
  const V = z.object({ name: z.string() });

  const def = render(reg, "greetv", V, { name: "Ada" });
  const formal = render(reg, "greetv", V, { name: "Ada" }, { variant: "formal" });

  // Default arm vs the named variant render to DIFFERENT bodies — variant selection works.
  assert.equal(def.text, "Hi Ada");
  assert.equal(formal.text, "Good day, Ada");
  assert.equal(def.variant, "default");
  assert.equal(formal.variant, "formal");
  // Provenance differs because the template source differs.
  assert.notEqual(def.templateHash, formal.templateHash);
});

test("render with an unknown variant raises PromptRenderError code unknown_variant", () => {
  const reg = new Registry();
  reg.loadYaml(VARIANT_YAML);
  const V = z.object({ name: z.string() });

  assert.throws(
    () => render(reg, "greetv", V, { name: "Ada" }, { variant: "nope" }),
    (err) => {
      assert.ok(err instanceof PromptRenderError);
      assert.ok(err.errors.some((row) => row.code === "unknown_variant"));
      return true;
    },
  );
});

// --------------------------------------------------------------------------------------
// 8. getSource (FR-010) — returns the UNRENDERED template; no vars, no validation
// --------------------------------------------------------------------------------------

test("getSource returns the unrendered template source", () => {
  const reg = greetRegistry();

  const source = getSource(reg, "greet");

  assert.equal(source, "Hi {{ name }}, you have {{ count }} messages");
  assert.ok(source.includes("{{"), "getSource must return the unrendered source");
});

test("getSource on an unknown name raises UnknownPromptError", () => {
  const reg = greetRegistry();
  assert.throws(() => getSource(reg, "does-not-exist"), UnknownPromptError);
});

test("getSource on an unknown variant raises PromptRenderError with code unknown_variant", () => {
  const reg = greetRegistry();
  assert.throws(
    () => getSource(reg, "greet", { variant: "nope" }),
    (err) => {
      assert.ok(err instanceof PromptRenderError);
      assert.ok(err.errors.some((row) => row.code === "unknown_variant"));
      return true;
    },
  );
});

// --------------------------------------------------------------------------------------
// 9. Surface smoke — GuardConfig type-only import is a no-op at runtime
// --------------------------------------------------------------------------------------

test("the US1 public surface is importable and callable", () => {
  assert.equal(typeof render, "function");
  assert.equal(typeof getSource, "function");
});
