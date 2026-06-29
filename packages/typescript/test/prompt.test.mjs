/**
 * Prompt-object surface tests for the TypeScript facade (`prompting-press`) — spec 008, T047.
 *
 * These exercise the new `Prompt` class (T042–T045) and the reshaped `Composition` (T046):
 *   - T042: construction via new/fromYaml/fromJson/fromToml — valid and invalid (throw)
 *   - T043: validation_required coverage check at construction
 *   - T044: render/getSource/check on the object surface
 *   - T045: with() immutability + merged-validation
 *   - T046: Composition over Prompt objects, no Registry, resolve() with no arg
 *
 * All tests use `origin` (not `provenance` — renamed in spec 008 Phase 1).
 * All tests confirm no Registry symbol on the public surface (SC-001 post-reshape).
 *
 * Model invariants:
 *  - Construction throws PromptValidationError / LoadError / PromptRenderError (never raw napi).
 *  - A constructed Prompt carries zero undeclared-variable errors (agreement enforced at construction).
 *  - render() provenance hashes are 64-char lowercase hex.
 *  - with() leaves the original Prompt untouched (SC-004).
 */

import assert from "node:assert/strict";
import { test } from "node:test";

import { z } from "zod";

import {
  Prompt,
  Composition,
  PromptingPressError,
  PromptValidationError,
  PromptRenderError,
  LoadError,
} from "prompting-press";

const HEX64 = /^[0-9a-f]{64}$/;

// ─── Fixtures ─────────────────────────────────────────────────────────────────────────────

/** A minimal valid PromptDefinition object (using `origin`, not `provenance`). */
const GREET_SHAPE = {
  name: "greet",
  role: "user",
  body: "Hi {{ name }}, you have {{ count }} messages",
  variables: {
    name: { type: "string", origin: "trusted" },
    count: { type: "integer", origin: "trusted" },
  },
};

const GREET_YAML = `
name: greet
role: user
body: "Hi {{ name }}, you have {{ count }} messages"
variables:
  name: { type: string, origin: trusted }
  count: { type: integer, origin: trusted }
`;

const GREET_JSON = JSON.stringify(GREET_SHAPE);

const GREET_TOML = `
name = "greet"
role = "user"
body = "Hi {{ name }}, you have {{ count }} messages"

[variables.name]
type = "string"
origin = "trusted"

[variables.count]
type = "integer"
origin = "trusted"
`;

/** Vars schema for GREET prompts. */
const Greeting = z.object({
  name: z.string().refine((s) => s.length > 0, "name must not be empty"),
  count: z.number().int().refine((n) => n >= 0, "count must be non-negative"),
});

// ─── T042: Construction ────────────────────────────────────────────────────────────────────

test("T042: new Prompt(shape) constructs valid prompt with correct accessors", () => {
  const p = new Prompt(GREET_SHAPE);
  assert.equal(p.name, "greet");
  assert.equal(p.role, "user");
  assert.equal(p.body, "Hi {{ name }}, you have {{ count }} messages");
  assert.ok(p.variables !== undefined);
  assert.ok(p.variants !== undefined);
  assert.equal(p.outputModel, undefined);
});

test("T042: Prompt.fromYaml constructs from valid YAML text", () => {
  const p = Prompt.fromYaml(GREET_YAML);
  assert.equal(p.name, "greet");
  assert.equal(p.body, "Hi {{ name }}, you have {{ count }} messages");
});

test("T042: Prompt.fromJson constructs from valid JSON text", () => {
  const p = Prompt.fromJson(GREET_JSON);
  assert.equal(p.name, "greet");
});

test("T042: Prompt.fromToml constructs from valid TOML text (routed to Rust, no smol-toml)", () => {
  const p = Prompt.fromToml(GREET_TOML);
  assert.equal(p.name, "greet");
  assert.equal(p.body, "Hi {{ name }}, you have {{ count }} messages");
});

test("T042: new Prompt with undeclared template variable throws PromptRenderError", () => {
  const bad = { name: "bad", role: "user", body: "{{ ghost }}", variables: {} };
  assert.throws(
    () => new Prompt(bad),
    (err) => {
      assert.ok(err instanceof PromptRenderError, `expected PromptRenderError, got ${err.constructor.name}`);
      assert.ok(err instanceof PromptingPressError);
      const codes = err.errors.map((r) => r.code);
      assert.ok(codes.includes("undefined_variable"), `expected undefined_variable, got ${codes}`);
      return true;
    },
  );
});

test("T042: new Prompt with reserved variant name throws PromptRenderError", () => {
  const bad = {
    name: "bad",
    role: "user",
    body: "hi",
    variables: {},
    variants: { default: { body: "shadowed" } },
  };
  assert.throws(
    () => new Prompt(bad),
    (err) => {
      assert.ok(err instanceof PromptingPressError);
      return true;
    },
  );
});

test("T042: new Prompt with missing required body throws LoadError", () => {
  assert.throws(
    () => new Prompt({ name: "bad", role: "user" }),
    (err) => {
      assert.ok(err instanceof LoadError, `expected LoadError, got ${err.constructor.name}`);
      assert.ok(err instanceof PromptingPressError);
      return true;
    },
  );
});

test("T042: Prompt.fromYaml with malformed YAML throws LoadError", () => {
  assert.throws(
    () => Prompt.fromYaml("name: [unterminated"),
    (err) => {
      assert.ok(err instanceof LoadError);
      return true;
    },
  );
});

test("T042: Prompt.fromJson with malformed JSON throws LoadError", () => {
  assert.throws(
    () => Prompt.fromJson("{ not valid json"),
    (err) => {
      assert.ok(err instanceof LoadError);
      return true;
    },
  );
});

test("T042: Prompt.fromToml with malformed TOML throws LoadError", () => {
  assert.throws(
    () => Prompt.fromToml("name = [unterminated"),
    (err) => {
      assert.ok(err instanceof LoadError);
      return true;
    },
  );
});

// ─── T043: validation_required coverage check ─────────────────────────────────────────────

test("T043: validation_required variable covered by validators.shape succeeds", () => {
  const shape = {
    name: "strict",
    role: "user",
    body: "Hi {{ name }}",
    variables: {
      name: { type: "string", origin: "trusted", validation_required: true },
    },
  };
  const schema = z.object({ name: z.string() });
  // Should not throw — `name` IS in schema.shape.
  const p = new Prompt(shape, schema);
  assert.equal(p.name, "strict");
});

test("T043: validation_required variable NOT covered throws PromptValidationError at construction", () => {
  const shape = {
    name: "strict",
    role: "user",
    body: "Hi {{ name }}",
    variables: {
      name: { type: "string", origin: "trusted", validation_required: true },
    },
  };
  const missingSchema = z.object({ other_field: z.string() });
  assert.throws(
    () => new Prompt(shape, missingSchema),
    (err) => {
      assert.ok(
        err instanceof PromptValidationError,
        `expected PromptValidationError, got ${err.constructor.name}: ${err.message}`,
      );
      assert.ok(
        err.errors.some((r) => r.field === "name"),
        `expected error naming 'name', got ${JSON.stringify(err.errors)}`,
      );
      return true;
    },
  );
});

test("T043: no validators supplied → validation_required check is skipped (documented limitation)", () => {
  const shape = {
    name: "strict",
    role: "user",
    body: "Hi {{ name }}",
    variables: {
      name: { type: "string", origin: "trusted", validation_required: true },
    },
  };
  // No validators → no coverage check → succeeds.
  const p = new Prompt(shape);
  assert.equal(p.name, "strict");
});

test("T043: validators without .shape → coverage check skipped (non-introspectable schema)", () => {
  const shape = {
    name: "strict",
    role: "user",
    body: "Hi {{ name }}",
    variables: {
      name: { type: "string", origin: "trusted", validation_required: true },
    },
  };
  // A ZodLikeSchema without .shape (e.g. a transform/refined schema) — coverage cannot be asserted.
  const noShapeSchema = {
    safeParse: (data) => ({ success: true, data }),
    // no .shape property
  };
  const p = new Prompt(shape, noShapeSchema);
  assert.equal(p.name, "strict");
});

test("T043: fromYaml with validation_required coverage check", () => {
  const yamlWithRequired = `
name: strict
role: user
body: "Hi {{ name }}"
variables:
  name:
    type: string
    origin: trusted
    validation_required: true
`;
  const schema = z.object({ name: z.string() });
  const p = Prompt.fromYaml(yamlWithRequired, schema);
  assert.equal(p.name, "strict");

  const badSchema = z.object({ other: z.string() });
  assert.throws(
    () => Prompt.fromYaml(yamlWithRequired, badSchema),
    PromptValidationError,
  );
});

// ─── T044: render / getSource / check ─────────────────────────────────────────────────────

test("T044: render(schema, data, opts?) produces correct text and 64-hex hashes", () => {
  const p = new Prompt(GREET_SHAPE);
  const result = p.render(Greeting, { name: "Ada", count: 3 });
  assert.equal(result.text, "Hi Ada, you have 3 messages");
  assert.equal(result.name, "greet");
  assert.equal(result.variant, "default");
  assert.match(result.templateHash, HEX64);
  assert.match(result.renderHash, HEX64);
  assert.equal(result.guard, null);
});

test("T044: render(data) static form (no schema) marshals directly", () => {
  const p = new Prompt(GREET_SHAPE);
  const result = p.render({ name: "Bo", count: 1 });
  assert.equal(result.text, "Hi Bo, you have 1 messages");
});

test("T044: render with bound validators uses them automatically", () => {
  const p = new Prompt(GREET_SHAPE, Greeting);
  const result = p.render({ name: "Cy", count: 2 });
  assert.equal(result.text, "Hi Cy, you have 2 messages");
});

test("T044: render with bound validators throws on invalid data (before templating)", () => {
  const p = new Prompt(GREET_SHAPE, Greeting);
  assert.throws(
    () => p.render({ name: "", count: -1 }),
    (err) => {
      assert.ok(err instanceof PromptValidationError);
      const fields = err.errors.map((r) => r.field);
      assert.ok(fields.includes("name") || fields.includes("count"), `got ${fields}`);
      return true;
    },
  );
});

test("T044: render with variant selector picks the named arm", () => {
  const shape = {
    name: "greetv",
    role: "user",
    body: "Hi {{ name }}",
    variables: { name: { type: "string", origin: "trusted" } },
    variants: { formal: { body: "Good day, {{ name }}" } },
  };
  const V = z.object({ name: z.string() });
  const p = new Prompt(shape);
  const def = p.render(V, { name: "Di" });
  const formal = p.render(V, { name: "Di" }, { variant: "formal" });
  assert.equal(def.text, "Hi Di");
  assert.equal(formal.text, "Good day, Di");
  assert.equal(def.variant, "default");
  assert.equal(formal.variant, "formal");
});

test("T044: getSource() returns the unrendered root body", () => {
  const p = new Prompt(GREET_SHAPE);
  const src = p.getSource();
  assert.equal(src, "Hi {{ name }}, you have {{ count }} messages");
  assert.ok(src.includes("{{"), "getSource must return UNrendered source");
});

test("T044: getSource(variant) returns the named variant source", () => {
  const shape = {
    name: "greetv",
    role: "user",
    body: "Hi {{ name }}",
    variables: { name: { type: "string", origin: "trusted" } },
    variants: { formal: { body: "Good day, {{ name }}" } },
  };
  const p = new Prompt(shape);
  const formal = p.getSource({ variant: "formal" });
  assert.equal(formal, "Good day, {{ name }}");
});

test("T044: check() returns passing report for trusted-only prompt", () => {
  const p = new Prompt(GREET_SHAPE);
  const report = p.check();
  assert.ok(report.passed(), "trusted-only prompt must pass check");
  assert.deepEqual(report.findings, []);
});

test("T044: check() returns advisory finding for unguarded untrusted variable", () => {
  const shape = {
    name: "unguarded",
    role: "user",
    body: "{{ payload }}",
    variables: { payload: { type: "string", origin: "untrusted" } },
  };
  const p = new Prompt(shape);
  const report = p.check();
  assert.ok(!report.passed(), "unguarded untrusted field must produce a finding");
  assert.ok(
    report.findings.some((f) => f.kind === "untrusted_without_guard"),
    `expected untrusted_without_guard, got ${report.findings.map((f) => f.kind)}`,
  );
});

test("T044: check() passes when guard is configured", () => {
  const shape = {
    name: "guarded",
    role: "user",
    body: "{{ payload }}",
    variables: { payload: { type: "string", origin: "untrusted" } },
    meta: { guard: { enabled: true } },
  };
  const p = new Prompt(shape);
  assert.ok(p.check().passed(), "guard configured → check must pass");
});

// ─── T045: with() — sole mutator, immutability ────────────────────────────────────────────

test("T045: with() returns a new Prompt with the overlay applied; original unchanged (SC-004)", () => {
  const original = new Prompt(GREET_SHAPE);
  const originalBody = original.body;
  const originalVariantsCount = Object.keys(original.variants ?? {}).length;

  const derived = original.with({ body: "Hey {{ name }}, you have {{ count }} messages" });

  assert.equal(derived.body, "Hey {{ name }}, you have {{ count }} messages");
  assert.equal(original.body, originalBody, "original body unchanged");
  assert.equal(
    Object.keys(original.variants ?? {}).length,
    originalVariantsCount,
    "original variants unchanged",
  );
});

test("T045: with() on overlay that introduces undeclared variable throws PromptRenderError", () => {
  const original = new Prompt(GREET_SHAPE);
  assert.throws(
    () => original.with({ body: "{{ name }} {{ ghost }}" }),
    (err) => {
      assert.ok(err instanceof PromptRenderError);
      assert.ok(err.errors.some((r) => r.code === "undefined_variable"));
      return true;
    },
  );
});

test("T045: with() carries validators forward from source by default (R6)", () => {
  const p = new Prompt(GREET_SHAPE, Greeting);
  const derived = p.with({ body: "Greetings {{ name }}, you have {{ count }} messages" });
  // Derived inherits bound Greeting validator — bad data must throw PromptValidationError.
  assert.throws(
    () => derived.render({ name: "", count: -1 }),
    PromptValidationError,
  );
});

test("T045: with(overlay, validators) overrides bound validator on derived prompt (R6)", () => {
  const p = new Prompt(GREET_SHAPE, Greeting);
  const NoCheck = z.object({ name: z.string(), count: z.number() });
  const derived = p.with({ body: "Hi {{ name }}, you have {{ count }} messages" }, NoCheck);
  // Derived has NoCheck as its bound validator — the count=-1 should now pass (no refine).
  const result = derived.render({ name: "Eli", count: -1 });
  assert.equal(result.text, "Hi Eli, you have -1 messages");
});

test("T045: with() coverage check on derived definition when validators override", () => {
  const shapeWithRequired = {
    name: "strict",
    role: "user",
    body: "Hi {{ name }}",
    variables: {
      name: { type: "string", origin: "trusted", validation_required: true },
    },
  };
  const schema = z.object({ name: z.string() });
  const original = new Prompt(shapeWithRequired, schema);

  // Overlay that adds a validation_required variable not covered by any validator → should throw.
  const badOverlay = {
    variables: {
      name: { type: "string", origin: "trusted", validation_required: true },
      newfield: { type: "string", origin: "trusted", validation_required: true },
    },
    body: "Hi {{ name }} {{ newfield }}",
  };
  assert.throws(
    () => original.with(badOverlay),
    PromptValidationError,
  );
});

// ─── T046: Composition over Prompt objects, no Registry ────────────────────────────────────

const SYS_SHAPE = {
  name: "sys",
  role: "system",
  body: "You are a helpful assistant.",
  variables: {},
};

const GREET_SIMPLE = {
  name: "greet_simple",
  role: "user",
  body: "Hi {{ name }}",
  variables: { name: { type: "string", origin: "trusted" } },
};

const Named = z.object({ name: z.string().refine((s) => s.length > 0, "name must not be empty") });
const EmptyVars = z.object({});

test("T046: Composition.fromMessages over Prompt objects resolves in order with roles", () => {
  const sysPr = new Prompt(SYS_SHAPE);
  const greetPr = new Prompt(GREET_SIMPLE);

  const comp = Composition.fromMessages([
    { prompt: sysPr, schema: EmptyVars, data: {} },
    { prompt: greetPr, schema: Named, data: { name: "Ada" } },
  ]);
  assert.equal(comp.length, 2);

  const messages = comp.resolve();
  assert.equal(messages.length, 2);
  assert.equal(messages[0].role, "system");
  assert.equal(messages[0].text, "You are a helpful assistant.");
  assert.equal(messages[1].role, "user");
  assert.equal(messages[1].text, "Hi Ada");
});

test("T046: Composition.append stores entries; resolve() takes no registry argument", () => {
  const greetPr = new Prompt(GREET_SIMPLE);
  const comp = new Composition();
  assert.equal(comp.append({ prompt: greetPr, schema: Named, data: { name: "Bo" } }), undefined,
    "append must return void (non-fluent)");
  assert.equal(comp.length, 1);
  const messages = comp.resolve();
  assert.equal(messages.length, 1);
  assert.equal(messages[0].text, "Hi Bo");
});

test("T046: invalid vars at append throw PromptValidationError; nothing stored (no partial)", () => {
  const greetPr = new Prompt(GREET_SIMPLE);
  const comp = new Composition();
  comp.append({ prompt: greetPr, schema: Named, data: { name: "ok" } });
  assert.equal(comp.length, 1);

  assert.throws(
    () => comp.append({ prompt: greetPr, schema: Named, data: { name: "" } }),
    PromptValidationError,
  );
  assert.equal(comp.length, 1, "rejected append must store nothing");
});

test("T046: fromMessages with invalid entry throws; no Composition returned (no partial)", () => {
  const greetPr = new Prompt(GREET_SIMPLE);
  assert.throws(
    () =>
      Composition.fromMessages([
        { prompt: greetPr, schema: Named, data: { name: "ok" } },
        { prompt: greetPr, schema: Named, data: { name: "" } }, // invalid
      ]),
    PromptValidationError,
  );
});

test("T046: empty Composition resolves to []", () => {
  const comp = new Composition();
  assert.equal(comp.length, 0);
  assert.deepEqual(comp.resolve(), []);
});

test("T046: Composition.resolve() an unknown variant throws PromptRenderError", () => {
  const greetPr = new Prompt(GREET_SIMPLE);
  const comp = Composition.fromMessages([
    { prompt: greetPr, data: { name: "X" }, variant: "nonexistent" },
  ]);
  assert.throws(() => comp.resolve(), PromptRenderError);
});

test("T046: static (no-schema) entries in Composition are accepted (Q4)", () => {
  const sysPr = new Prompt(SYS_SHAPE);
  const greetPr = new Prompt(GREET_SIMPLE);
  const comp = Composition.fromMessages([
    { prompt: sysPr, data: {} },
    { prompt: greetPr, data: { name: "Zed" } },
  ]);
  const messages = comp.resolve();
  assert.equal(messages[0].text, "You are a helpful assistant.");
  assert.equal(messages[1].text, "Hi Zed");
});

test("T046: Composition has no .chain() method (FR-013)", () => {
  assert.equal(Composition.prototype.chain, undefined, "no chain on prototype");
  const comp = new Composition();
  assert.equal((comp).chain, undefined, "no chain on instance");
});

// ─── SC-001: No Registry on public surface ────────────────────────────────────────────────

test("SC-001: Registry is NOT exported from the public surface (spec 008 removal)", async () => {
  const mod = await import("prompting-press");
  assert.equal(
    mod.Registry,
    undefined,
    "Registry must not be exported after spec 008 reshape",
  );
});

test("SC-001: render(reg, name, …) free function is NOT exported", async () => {
  const mod = await import("prompting-press");
  // The old render free function took 4-5 positional args; it must be gone.
  // The new surface is Prompt.render() method only.
  // We check that the only callable `render` export is not present as a module-level export.
  assert.equal(mod.render, undefined, "render free function must not be exported");
});

// ─── Surface smoke ────────────────────────────────────────────────────────────────────────

test("surface smoke: Prompt and Composition are exported and callable", () => {
  assert.equal(typeof Prompt, "function");
  assert.equal(typeof Prompt.fromYaml, "function");
  assert.equal(typeof Prompt.fromJson, "function");
  assert.equal(typeof Prompt.fromToml, "function");
  assert.equal(typeof Composition, "function");
  assert.equal(typeof Composition.fromMessages, "function");
  assert.ok(PromptValidationError.prototype instanceof PromptingPressError);
  assert.ok(LoadError.prototype instanceof PromptingPressError);
  assert.ok(PromptRenderError.prototype instanceof PromptingPressError);
});
