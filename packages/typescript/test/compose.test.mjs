/**
 * US4 multi-message composition tests for the TypeScript facade (`prompting-press`) — spec 005, T021.
 *
 * US4 lands the ordered-composition surface (FR-012 / FR-013): an explicit, ordered array of
 * `(prompt, vars, variant?)` entries that resolves to a `Message[]` in append order. There is NO
 * fluent `.chain()` (FR-013) — composition is built with `new Composition()` + `.append(...)` or
 * the `Composition.fromMessages([...])` static factory.
 *
 * The PUBLIC `Composition` is the TS-facade class (critique E2): it runs Zod validation on each
 * entry BEFORE handing the validated value to the low-level addon composition.
 *
 * What these pin (all TS-observable; none re-verify cross-language render parity, which Principle I
 * makes structural):
 *   - order + roles (SC-008): N entries → exactly N `Message` in input order, each `.role` matching
 *     its prompt definition's role and `.text` rendered with that entry's own vars.
 *   - one bad entry → no partial (FR-013 / SC-008): (a) vars failing Zod validation throw at
 *     append/fromMessages with NOTHING stored; (b) an unknown prompt name surfaces at `resolve` as
 *     an `UnknownPromptError` and NO partial array is returned.
 *   - empty composition → [].
 *   - no `.chain()` on the class or an instance (FR-013).
 *   - a `[name, schema, data, variant]` entry selects the variant; `[name, schema, data]` defaults
 *     to the reserved `default` arm; an unknown variant fails at `resolve` as `PromptRenderError`.
 */

import assert from "node:assert/strict";
import { test } from "node:test";

import { z } from "zod";

import {
  Registry,
  Composition,
  PromptingPressError,
  PromptRenderError,
  PromptValidationError,
  UnknownPromptError,
} from "prompting-press";

// --------------------------------------------------------------------------------------
// Zod Vars schemas (the per-language idiom; Principle VI).
// --------------------------------------------------------------------------------------

/** A single `name` whose refine rejects an empty string (so a bad entry is genuinely rejected). */
const Named = z.object({ name: z.string().refine((s) => s.length > 0, "name must be non-empty") });

/** No declared variables — for variable-free, role-carrying prompts. */
const EmptyVars = z.object({});

// --------------------------------------------------------------------------------------
// Registry helper + prompt definitions
// --------------------------------------------------------------------------------------

function registry(...definitions) {
  const reg = new Registry();
  for (const def of definitions) reg.insert(def);
  return reg;
}

const SYS_PREAMBLE = { name: "sys_preamble", role: "system", body: "You are helpful.", variables: {} };
const GREET = {
  name: "greet",
  role: "user",
  body: "Hi {{ name }}",
  variables: { name: { type: "string", provenance: "trusted" } },
};
const FAREWELL = {
  name: "farewell",
  role: "user",
  body: "Bye {{ name }}",
  variables: { name: { type: "string", provenance: "trusted" } },
};
const WITH_VARIANT = {
  name: "salute",
  role: "user",
  body: "Hi {{ name }}",
  variants: { formal: { body: "Good day, {{ name }}" } },
  variables: { name: { type: "string", provenance: "trusted" } },
};

// --------------------------------------------------------------------------------------
// 1. Order + roles (SC-008) — both construction paths
// --------------------------------------------------------------------------------------

test("the append path preserves order, roles, and per-entry text (SC-008)", () => {
  const reg = registry(SYS_PREAMBLE, GREET);

  const comp = new Composition();
  assert.equal(comp.append({ name: "sys_preamble", schema: EmptyVars, data: {} }), undefined, "append is non-fluent (void)");
  assert.equal(comp.append({ name: "greet", schema: Named, data: { name: "Ada" } }), undefined);
  assert.equal(comp.length, 2, "length reflects the two stored entries");

  const messages = comp.resolve(reg);

  assert.equal(messages.length, 2, "exactly N messages, one per entry");
  assert.equal(messages[0].role, "system");
  assert.equal(messages[0].text, "You are helpful.");
  assert.equal(messages[1].role, "user");
  assert.equal(messages[1].text, "Hi Ada");
});

test("the fromMessages path preserves order, roles, and per-entry text (SC-008)", () => {
  const reg = registry(SYS_PREAMBLE, GREET);

  const comp = Composition.fromMessages([
    { name: "sys_preamble", schema: EmptyVars, data: {} },
    { name: "greet", schema: Named, data: { name: "Bo" } },
  ]);
  assert.ok(comp instanceof Composition);
  assert.equal(comp.length, 2);

  const messages = comp.resolve(reg);

  assert.equal(messages.length, 2);
  assert.deepEqual(
    messages.map((m) => m.role),
    ["system", "user"],
  );
  assert.deepEqual(
    messages.map((m) => m.text),
    ["You are helpful.", "Hi Bo"],
  );
});

test("the two construction paths produce identical ordered messages", () => {
  const reg = registry(SYS_PREAMBLE, GREET);
  const entries = [
    { name: "sys_preamble", schema: EmptyVars, data: {} },
    { name: "greet", schema: Named, data: { name: "Cy" } },
  ];

  const viaAppend = new Composition();
  for (const entry of entries) viaAppend.append(entry);
  const viaFactory = Composition.fromMessages(entries);

  const appended = viaAppend.resolve(reg).map((m) => [m.role, m.text]);
  const factoried = viaFactory.resolve(reg).map((m) => [m.role, m.text]);
  assert.deepEqual(appended, factoried);
  assert.deepEqual(appended, [
    ["system", "You are helpful."],
    ["user", "Hi Cy"],
  ]);
});

// --------------------------------------------------------------------------------------
// 2. One invalid entry → no partial (FR-013 / SC-008)
// --------------------------------------------------------------------------------------

test("invalid vars at append throw PromptValidationError and store nothing (no partial)", () => {
  const reg = registry(GREET);

  const comp = new Composition();
  comp.append({ name: "greet", schema: Named, data: { name: "ok" } }); // one good entry
  assert.equal(comp.length, 1);

  // An empty name fails the Zod refine; append eager-validates, so this is rejected.
  assert.throws(
    () => comp.append({ name: "greet", schema: Named, data: { name: "" } }),
    (err) => {
      assert.ok(err instanceof PromptValidationError);
      assert.ok(err.errors.some((row) => row.field === "name"));
      assert.ok(err.errors.filter((r) => r.field === "name").every((r) => r.code === "validation"));
      return true;
    },
  );

  // Nothing stored from the failed append — length is exactly the prior good count.
  assert.equal(comp.length, 1, "a rejected append must store nothing (no partial state)");

  // The composition still resolves cleanly to just the one good entry.
  assert.deepEqual(
    comp.resolve(reg).map((m) => m.text),
    ["Hi ok"],
  );
});

test("the first invalid entry in fromMessages throws and yields no Composition (no partial)", () => {
  const reg = registry(GREET);

  assert.throws(
    () =>
      Composition.fromMessages([
        { name: "greet", schema: Named, data: { name: "ok" } },
        { name: "greet", schema: Named, data: { name: "" } }, // invalid second entry
      ]),
    PromptValidationError,
  );

  // The all-or-nothing factory hands back nothing on failure. A clean build still works.
  const good = Composition.fromMessages([{ name: "greet", schema: Named, data: { name: "ok" } }]);
  assert.deepEqual(
    good.resolve(reg).map((m) => m.text),
    ["Hi ok"],
  );
});

test("an unknown prompt name at resolve throws UnknownPromptError and returns no partial", () => {
  const reg = registry(SYS_PREAMBLE);

  const comp = new Composition();
  comp.append({ name: "sys_preamble", schema: EmptyVars, data: {} }); // valid + present
  comp.append({ name: "does_not_exist", schema: EmptyVars, data: {} }); // name not in the registry
  assert.equal(comp.length, 2, "append does not resolve the name; both entries are stored");

  // resolve RAISES (does not return the one successfully-rendered prefix).
  const sentinel = Symbol("not-set");
  let result = sentinel;
  assert.throws(
    () => {
      result = comp.resolve(reg);
    },
    (err) => {
      assert.ok(err instanceof UnknownPromptError);
      return true;
    },
  );
  assert.equal(result, sentinel, "resolve must RAISE, not return a partial list");
});

// --------------------------------------------------------------------------------------
// 3. Empty composition → []
// --------------------------------------------------------------------------------------

test("an empty composition resolves to []", () => {
  const reg = registry(SYS_PREAMBLE);
  const empty = new Composition();
  assert.equal(empty.length, 0);
  assert.deepEqual(empty.resolve(reg), []);
});

// --------------------------------------------------------------------------------------
// 4. No .chain() (FR-013)
// --------------------------------------------------------------------------------------

test("there is no fluent .chain() on the class or an instance (FR-013)", () => {
  assert.equal(Composition.prototype.chain, undefined, "no chain on the prototype");
  const comp = new Composition();
  assert.equal(comp.chain, undefined, "no chain on an instance");
  assert.equal(typeof comp.chain, "undefined");
  // append returns undefined (void), so accidental chaining is impossible.
  assert.equal(comp.append({ name: "greet", schema: Named, data: { name: "x" } }), undefined);
});

// --------------------------------------------------------------------------------------
// 5. Variant selection via the entry tuple
// --------------------------------------------------------------------------------------

test("a [name, schema, data, variant] entry selects the named variant arm", () => {
  const reg = registry(WITH_VARIANT);

  const viaFactory = Composition.fromMessages([{ name: "salute", schema: Named, data: { name: "Di" }, variant: "formal" }]);
  assert.deepEqual(
    viaFactory.resolve(reg).map((m) => m.text),
    ["Good day, Di"],
  );

  // The same selection via append.
  const viaAppend = new Composition();
  viaAppend.append({ name: "salute", schema: Named, data: { name: "Di" }, variant: "formal" });
  assert.equal(viaAppend.resolve(reg)[0].text, "Good day, Di");
});

test("a [name, schema, data] entry defaults to the reserved default arm", () => {
  const reg = registry(WITH_VARIANT);

  const comp = Composition.fromMessages([{ name: "salute", schema: Named, data: { name: "Eli" } }]);
  // The root body is the default arm — "Hi {{ name }}", not the `formal` arm.
  assert.deepEqual(
    comp.resolve(reg).map((m) => m.text),
    ["Hi Eli"],
  );
});

test("an unknown variant fails at resolve as PromptRenderError", () => {
  const reg = registry(WITH_VARIANT);

  const comp = new Composition();
  comp.append({ name: "salute", schema: Named, data: { name: "Fa" }, variant: "nonexistent" });

  assert.throws(() => comp.resolve(reg), PromptRenderError);
});

// --------------------------------------------------------------------------------------
// 6. Mixed — system + two user entries resolve to 3 correctly-ordered messages
// --------------------------------------------------------------------------------------

test("a mixed system + two user composition resolves to 3 ordered messages (SC-008)", () => {
  const reg = registry(SYS_PREAMBLE, GREET, FAREWELL);

  const comp = Composition.fromMessages([
    { name: "sys_preamble", schema: EmptyVars, data: {} },
    { name: "greet", schema: Named, data: { name: "Ada" } },
    { name: "farewell", schema: Named, data: { name: "Bo" } },
  ]);
  assert.equal(comp.length, 3);

  const messages = comp.resolve(reg);

  assert.equal(messages.length, 3);
  assert.deepEqual(
    messages.map((m) => m.role),
    ["system", "user", "user"],
  );
  assert.deepEqual(
    messages.map((m) => m.text),
    ["You are helpful.", "Hi Ada", "Bye Bo"],
  );
});

// --------------------------------------------------------------------------------------
// 7. Static (no-schema) entries are accepted too (Q4 parity with render)
// --------------------------------------------------------------------------------------

test("static (no-schema) entries are marshaled directly (Q4)", () => {
  const reg = registry(SYS_PREAMBLE, GREET);

  // { name, data } and { name, data, variant } forms, no schema.
  const comp = Composition.fromMessages([
    { name: "sys_preamble", data: {} },
    { name: "greet", data: { name: "Zed" } },
  ]);
  assert.deepEqual(
    comp.resolve(reg).map((m) => m.text),
    ["You are helpful.", "Hi Zed"],
  );
});

// --------------------------------------------------------------------------------------
// 8. Surface smoke
// --------------------------------------------------------------------------------------

test("the US4 composition surface is exposed", () => {
  assert.equal(typeof Composition, "function");
  assert.equal(typeof Composition.fromMessages, "function");
  assert.equal(typeof new Composition().append, "function");
  assert.equal(typeof new Composition().resolve, "function");
  assert.ok(UnknownPromptError.prototype instanceof PromptingPressError);
  assert.ok(PromptRenderError.prototype instanceof PromptingPressError);
});
