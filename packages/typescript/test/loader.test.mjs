/**
 * US2 dual-input loader tests for the TypeScript facade (`prompting-press`) — spec 005, T015.
 * Updated for spec 008 object surface: uses `Prompt.fromJson`/`fromYaml`/constructor instead of
 * the removed `Registry`.
 *
 * US2 lands three construction paths into the ONE consumer loader (Q3 / FR-005):
 *   - `new Prompt(obj)`          — from a PromptDefinition-shaped object
 *   - `Prompt.fromJson(text)`    — from an already-read JSON document
 *   - `Prompt.fromYaml(text)`    — from an already-read YAML document
 *
 * These prove the three TS surfaces all reach the shared core, that the paths render
 * byte-identically with identical provenance (SC-003), that malformed input is loud and
 * leaves nothing partially loaded (FR-007 → LoadError), and that the consumer's YAML-1.2 /
 * Norway-safe parsing is inherited across FFI (research D2).
 *
 * All fixtures use `origin` (spec 008 rename from `provenance`).
 */

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { test } from "node:test";

import { z } from "zod";

import { Prompt, LoadError, PromptRenderError, PromptingPressError } from "prompting-press";

const HEX64 = /^[0-9a-f]{64}$/;

// Repo-root-relative path to the shared schema fixtures (fixtures moved to tests/ in spec 008 Phase 2).
const repoRoot = fileURLToPath(new URL("../../../", import.meta.url));
function readFixture(relPath) {
  return readFileSync(`${repoRoot}${relPath}`, "utf8");
}

// ─── The ONE logical prompt expressed three ways ─────────────────────────────────────────

const GREET_OBJ = {
  name: "greet",
  role: "user",
  body: "Hi {{ name }}, you have {{ count }} messages",
  variables: {
    name: { type: "string", origin: "trusted" },
    count: { type: "integer", origin: "trusted" },
  },
};

const GREET_JSON = JSON.stringify(GREET_OBJ);

const GREET_YAML = `
name: greet
role: user
body: "Hi {{ name }}, you have {{ count }} messages"
variables:
  name:
    type: string
    origin: trusted
  count:
    type: integer
    origin: trusted
`;

const Greeting = z.object({ name: z.string(), count: z.number().int() });
const GREET_INPUTS = { name: "Ada", count: 3 };
const GREET_TEXT = "Hi Ada, you have 3 messages";

const Empty = z.object({});

// ─── 1. Three-input parity — SC-003 / FR-005 ────────────────────────────────────────────

test("fromYaml, fromJson, and new Prompt(obj) reach one core with identical render + provenance (SC-003)", () => {
  const pYaml = Prompt.fromYaml(GREET_YAML);
  const pJson = Prompt.fromJson(GREET_JSON);
  const pObj = new Prompt(GREET_OBJ);

  const results = {
    yaml: pYaml.render(Greeting, GREET_INPUTS),
    json: pJson.render(Greeting, GREET_INPUTS),
    obj: pObj.render(Greeting, GREET_INPUTS),
  };

  for (const [label, res] of Object.entries(results)) {
    assert.equal(res.text, GREET_TEXT, label);
    assert.equal(res.variant, "default", label);
    assert.match(res.templateHash, HEX64, `${label}: ${res.templateHash}`);
    assert.match(res.renderHash, HEX64, `${label}: ${res.renderHash}`);
  }

  const texts = new Set(Object.values(results).map((r) => r.text));
  const tHashes = new Set(Object.values(results).map((r) => r.templateHash));
  const rHashes = new Set(Object.values(results).map((r) => r.renderHash));
  assert.equal(texts.size, 1, [...texts].join(" | "));
  assert.equal(tHashes.size, 1, [...tHashes].join(" | "));
  assert.equal(rHashes.size, 1, [...rHashes].join(" | "));
});

test("new Prompt(obj) equals fromJson(JSON.stringify(obj)) of the same data (FR-005)", () => {
  const pObj = new Prompt(GREET_OBJ);
  const pTxt = Prompt.fromJson(JSON.stringify(GREET_OBJ));

  const viaObj = pObj.render(Greeting, GREET_INPUTS);
  const viaTxt = pTxt.render(Greeting, GREET_INPUTS);

  assert.equal(viaObj.text, viaTxt.text);
  assert.equal(viaObj.text, GREET_TEXT);
  assert.equal(viaObj.templateHash, viaTxt.templateHash);
  assert.equal(viaObj.renderHash, viaTxt.renderHash);
});

// ─── 2. Real shared fixtures ──────────────────────────────────────────────────────────────

test("a shared valid JSON fixture loads via fromJson and new Prompt() (the canonical corpus)", () => {
  const fixtureText = readFixture("schemas/jsonschema/tests/fixtures/valid/single-body.json");
  const fixtureObj = JSON.parse(fixtureText);

  const pJson = Prompt.fromJson(fixtureText);
  const pObj = new Prompt(fixtureObj);

  const Vars = z.object({ date: z.string() });
  const viaJson = pJson.render(Vars, { date: "2026-06-27" });
  const viaObj = pObj.render(Vars, { date: "2026-06-27" });

  assert.equal(viaJson.text, "You are a helpful assistant. Today is 2026-06-27.");
  assert.equal(viaJson.text, viaObj.text);
  assert.equal(viaJson.templateHash, viaObj.templateHash);
  assert.equal(viaJson.renderHash, viaObj.renderHash);
});

test("the multi-variant valid fixture loads and resolves the default arm via fromJson", () => {
  const fixtureText = readFixture(
    "schemas/jsonschema/tests/fixtures/valid/multi-variant.json",
  );
  const p = Prompt.fromJson(fixtureText);

  const Vars = z.object({ article: z.string(), max_words: z.number().int(), style: z.string() });
  const res = p.render(Vars, {
    article: "An article.",
    max_words: 50,
    style: "prose",
  });
  assert.ok(res.text.startsWith("Summarise the following article"));
  assert.match(res.templateHash, HEX64);
});

// ─── 3. Malformed input → LoadError, and NOTHING is partially loaded — FR-007 ────────────

test("malformed YAML raises LoadError (FR-007)", () => {
  assert.throws(() => Prompt.fromYaml("name: [unterminated"), LoadError);
});

test("malformed JSON raises LoadError (FR-007)", () => {
  assert.throws(() => Prompt.fromJson("{ not valid json "), LoadError);
});

test("a shape violation (missing required body) raises LoadError on every surface (FR-007)", () => {
  const bad = { name: "noBody", role: "user" }; // no `body`

  assert.throws(() => Prompt.fromJson(JSON.stringify(bad)), LoadError);
  assert.throws(() => new Prompt(bad), LoadError);
  assert.throws(() => Prompt.fromYaml("name: noBody\nrole: user\n"), LoadError);
});

test("a known invalid fixture (bad role) is rejected as LoadError on the JSON path", () => {
  const badRole = readFixture("schemas/jsonschema/tests/fixtures/invalid/bad-role.json");
  assert.throws(
    () => Prompt.fromJson(badRole),
    (err) => {
      assert.ok(err instanceof LoadError);
      assert.ok(err instanceof PromptingPressError);
      assert.ok(err.errors.every((row) => row.code === "load"));
      return true;
    },
  );
});

// ─── 4. Norway-safe YAML — research D2 ───────────────────────────────────────────────────

for (const literal of ["no", "off", "yes", "on", "true", "false"]) {
  test(`unquoted YAML \`${literal}\` round-trips as the string, not a bool (Norway-safe)`, () => {
    const p = Prompt.fromYaml(`name: norway\nrole: user\nbody: ${literal}\nvariables: {}\n`);

    const result = p.render(Empty, {});

    assert.equal(
      result.text,
      literal,
      `unquoted YAML \`${literal}\` should round-trip as the string, got ${JSON.stringify(result.text)}`,
    );
    assert.ok(result.text !== "True" && result.text !== "False");
  });
}

// ─── 5. PromptRenderError on agreement violation (undeclared variable) ───────────────────

test("a body that references an undeclared variable fails at construction (agreement at construction)", () => {
  // Post-reshape, agreement is enforced at construction — not silently at render.
  assert.throws(
    () => Prompt.fromJson(JSON.stringify({ name: "bad", role: "user", body: "{{ ghost }}" })),
    PromptRenderError,
  );
});

// ─── 6. Loader surface smoke ──────────────────────────────────────────────────────────────

test("the construction paths return Prompt instances keyed by the document's own name", () => {
  const p = Prompt.fromJson(GREET_JSON);
  assert.equal(p.name, "greet");
  assert.equal(p.render(Greeting, GREET_INPUTS).text, GREET_TEXT);
});

test("the US2 loader surface is exposed where the binding promises it", () => {
  assert.equal(typeof Prompt.fromYaml, "function");
  assert.equal(typeof Prompt.fromJson, "function");
  assert.equal(typeof Prompt.fromToml, "function");
  assert.ok(LoadError.prototype instanceof PromptingPressError);
});
