/**
 * US2 dual-input loader tests for the TypeScript facade (`prompting-press`) — spec 005, T015.
 *
 * US2 lands three entry points into the ONE consumer loader (Q3 / FR-005):
 *   - `Registry.loadYaml(text)`  — marshal an already-read YAML document to the consumer.
 *   - `Registry.loadJson(text)`  — marshal an already-read JSON document to the consumer.
 *   - `Registry.insert(obj)`     — re-serialize a constructed shape and feed the SAME load path.
 *
 * These prove the three TS surfaces all reach the shared core, that the four ways to describe one
 * logical prompt render byte-identically with identical provenance (SC-003), that malformed input
 * is loud and leaves nothing partially loaded (FR-007 → LoadError), and that the consumer's
 * YAML-1.2 / Norway-safe parsing is inherited across FFI (research D2).
 *
 * Parity itself is structural (one Rust core renders for every language — Principle I); what is
 * verified here is that each TS entry point routes into that core and normalizes to one
 * representation. Real `schemas/jsonschema/fixtures/valid/*.json` files are used as JSON inputs.
 */

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { test } from "node:test";

import { z } from "zod";

import { Registry, render, LoadError, UnknownPromptError, PromptingPressError } from "prompting-press";

const HEX64 = /^[0-9a-f]{64}$/;

// Repo-root-relative path to the shared schema fixtures (this file lives at
// packages/typescript/test/, so the repo root is four `..` up).
const repoRoot = fileURLToPath(new URL("../../../", import.meta.url));
function readFixture(relPath) {
  return readFileSync(`${repoRoot}${relPath}`, "utf8");
}

// --------------------------------------------------------------------------------------
// The ONE logical prompt, expressed three ways. Each form must normalize to the same internal
// representation and therefore render byte-identically with identical hashes (SC-003).
// --------------------------------------------------------------------------------------

const GREET_OBJ = {
  name: "greet",
  role: "user",
  body: "Hi {{ name }}, you have {{ count }} messages",
  variables: {
    name: { type: "string", provenance: "trusted" },
    count: { type: "integer", provenance: "trusted" },
  },
};

const GREET_JSON = JSON.stringify(GREET_OBJ);

// Hand-authored YAML (no JS YAML dependency — the consumer parses it across FFI).
const GREET_YAML = `
name: greet
role: user
body: "Hi {{ name }}, you have {{ count }} messages"
variables:
  name:
    type: string
    provenance: trusted
  count:
    type: integer
    provenance: trusted
`;

const Greeting = z.object({ name: z.string(), count: z.number().int() });
const GREET_INPUTS = { name: "Ada", count: 3 };
const GREET_TEXT = "Hi Ada, you have 3 messages";

const Empty = z.object({});

// --------------------------------------------------------------------------------------
// 1. Three-input parity — SC-003 / FR-005
// --------------------------------------------------------------------------------------

test("loadYaml, loadJson, and insert reach one core with identical render + provenance (SC-003)", () => {
  const regYaml = new Registry();
  regYaml.loadYaml(GREET_YAML);

  const regJson = new Registry();
  regJson.loadJson(GREET_JSON);

  const regObj = new Registry();
  regObj.insert(GREET_OBJ);

  const results = {
    yaml: render(regYaml, "greet", Greeting, GREET_INPUTS),
    json: render(regJson, "greet", Greeting, GREET_INPUTS),
    insert: render(regObj, "greet", Greeting, GREET_INPUTS),
  };

  // Each individually renders the expected body with hex hashes.
  for (const [label, res] of Object.entries(results)) {
    assert.equal(res.text, GREET_TEXT, label);
    assert.equal(res.variant, "default", label);
    assert.match(res.templateHash, HEX64, `${label}: ${res.templateHash}`);
    assert.match(res.renderHash, HEX64, `${label}: ${res.renderHash}`);
  }

  // And the three surfaces agree on text + BOTH provenance hashes.
  const texts = new Set(Object.values(results).map((r) => r.text));
  const tHashes = new Set(Object.values(results).map((r) => r.templateHash));
  const rHashes = new Set(Object.values(results).map((r) => r.renderHash));
  assert.equal(texts.size, 1, [...texts].join(" | "));
  assert.equal(tHashes.size, 1, [...tHashes].join(" | "));
  assert.equal(rHashes.size, 1, [...rHashes].join(" | "));
});

test("insert(obj) equals loadJson(JSON.stringify(obj)) of the same data (FR-005)", () => {
  const regObj = new Registry();
  regObj.insert(GREET_OBJ);

  const regTxt = new Registry();
  regTxt.loadJson(JSON.stringify(GREET_OBJ));

  const viaObj = render(regObj, "greet", Greeting, GREET_INPUTS);
  const viaTxt = render(regTxt, "greet", Greeting, GREET_INPUTS);

  assert.equal(viaObj.text, viaTxt.text);
  assert.equal(viaObj.text, GREET_TEXT);
  assert.equal(viaObj.templateHash, viaTxt.templateHash);
  assert.equal(viaObj.renderHash, viaTxt.renderHash);
});

// --------------------------------------------------------------------------------------
// 2. Real shared fixtures load via every text path (the canonical valid corpus)
// --------------------------------------------------------------------------------------

test("a shared valid JSON fixture loads via loadJson and via insert (the canonical corpus)", () => {
  const fixtureText = readFixture("schemas/jsonschema/fixtures/valid/single-body.json");
  const fixtureObj = JSON.parse(fixtureText);

  // load via JSON text
  const regJson = new Registry();
  regJson.loadJson(fixtureText);
  // load the same parsed object via insert
  const regObj = new Registry();
  regObj.insert(fixtureObj);

  // The fixture is `greeting` with a `{{date}}` variable. Render both and compare provenance.
  const Vars = z.object({ date: z.string() });
  const viaJson = render(regJson, "greeting", Vars, { date: "2026-06-27" });
  const viaObj = render(regObj, "greeting", Vars, { date: "2026-06-27" });

  assert.equal(viaJson.text, "You are a helpful assistant. Today is 2026-06-27.");
  assert.equal(viaJson.text, viaObj.text);
  assert.equal(viaJson.templateHash, viaObj.templateHash);
  assert.equal(viaJson.renderHash, viaObj.renderHash);
});

test("the multi-variant valid fixture loads and resolves the default arm via loadJson", () => {
  const fixtureText = readFixture("schemas/jsonschema/fixtures/valid/multi-variant.json");
  const reg = new Registry();
  reg.loadJson(fixtureText);

  const Vars = z.object({ article: z.string(), max_words: z.number().int(), style: z.string() });
  const res = render(reg, "content-summariser", Vars, {
    article: "An article.",
    max_words: 50,
    style: "prose",
  });
  // The root body IS the default arm (FR-011).
  assert.ok(res.text.startsWith("Summarise the following article"));
  assert.match(res.templateHash, HEX64);
});

// --------------------------------------------------------------------------------------
// 3. Malformed input → LoadError, and NOTHING is partially loaded — FR-007
// --------------------------------------------------------------------------------------

test("malformed YAML raises LoadError and loads nothing (FR-007)", () => {
  const reg = new Registry();
  assert.throws(() => reg.loadYaml("name: [unterminated"), LoadError);

  // The failed load inserts nothing — that name is not in the registry, so a render is unknown.
  assert.throws(() => render(reg, "unterminated", Empty, {}), UnknownPromptError);
});

test("malformed JSON raises LoadError and loads nothing (FR-007)", () => {
  const reg = new Registry();
  assert.throws(() => reg.loadJson("{ not valid json "), LoadError);
  assert.throws(() => render(reg, "greet", Empty, {}), UnknownPromptError);
});

test("a shape violation (missing required body) raises LoadError on every surface (FR-007)", () => {
  const bad = { name: "noBody", role: "user" }; // no `body`

  const regJson = new Registry();
  assert.throws(() => regJson.loadJson(JSON.stringify(bad)), LoadError);

  const regInsert = new Registry();
  assert.throws(() => regInsert.insert(bad), LoadError);

  const regYaml = new Registry();
  assert.throws(() => regYaml.loadYaml("name: noBody\nrole: user\n"), LoadError);

  // None of the three left a usable entry behind.
  for (const reg of [regJson, regInsert, regYaml]) {
    assert.throws(() => render(reg, "noBody", Empty, {}), UnknownPromptError);
  }
});

test("a known invalid fixture (bad role) is rejected as LoadError on the JSON path", () => {
  const badRole = readFixture("schemas/jsonschema/fixtures/invalid/bad-role.json");
  const reg = new Registry();
  assert.throws(
    () => reg.loadJson(badRole),
    (err) => {
      assert.ok(err instanceof LoadError);
      assert.ok(err instanceof PromptingPressError);
      assert.ok(err.errors.every((row) => row.code === "load"));
      return true;
    },
  );
});

test("a failed re-load does not corrupt an existing entry (atomic load, FR-007)", () => {
  const reg = new Registry();
  reg.insert({
    name: "keep",
    role: "user",
    body: "Hi {{ n }}",
    variables: { n: { type: "string", provenance: "trusted" } },
  });

  const V = z.object({ n: z.string() });
  assert.equal(render(reg, "keep", V, { n: "Ada" }).text, "Hi Ada");

  // A malformed document keyed by the same logical name fails and changes nothing.
  assert.throws(() => reg.loadYaml("keep: [bad"), LoadError);

  // The original entry still renders unchanged.
  assert.equal(render(reg, "keep", V, { n: "Bo" }).text, "Hi Bo");
});

// --------------------------------------------------------------------------------------
// 4. Norway-safe YAML — research D2: `no` / `off` / `yes` stay STRINGS, never booleans
// --------------------------------------------------------------------------------------

for (const literal of ["no", "off", "yes", "on", "true", "false"]) {
  test(`unquoted YAML \`${literal}\` round-trips as the string, not a bool (Norway-safe)`, () => {
    const reg = new Registry();
    reg.loadYaml(`name: norway\nrole: user\nbody: ${literal}\n`);

    const result = render(reg, "norway", Empty, {});

    assert.equal(
      result.text,
      literal,
      `unquoted YAML \`${literal}\` should round-trip as the string, got ${JSON.stringify(result.text)}`,
    );
    // Defensive: never a coerced boolean stringification.
    assert.ok(result.text !== "True" && result.text !== "False");
  });
}

// --------------------------------------------------------------------------------------
// 5. Loader surface smoke — return contract and module exposure
// --------------------------------------------------------------------------------------

test("the loaders return undefined and key the entry by the document's own name", () => {
  const reg = new Registry();
  assert.equal(reg.loadJson(GREET_JSON), undefined);
  assert.equal(reg.loadYaml(GREET_YAML), undefined); // same name ⇒ replaces, still undefined
  assert.equal(reg.insert(GREET_OBJ), undefined);

  assert.equal(render(reg, "greet", Greeting, GREET_INPUTS).text, GREET_TEXT);
  assert.throws(() => render(reg, "absent", Greeting, GREET_INPUTS), UnknownPromptError);
});

test("the US2 loader surface is exposed where the binding promises it", () => {
  const reg = new Registry();
  assert.equal(typeof reg.loadYaml, "function");
  assert.equal(typeof reg.loadJson, "function");
  assert.equal(typeof reg.insert, "function");
  assert.ok(LoadError.prototype instanceof PromptingPressError);
});
