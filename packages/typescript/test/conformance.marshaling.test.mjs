/**
 * Spec 006 — FFI marshaling conformance corpus, TypeScript side.
 *
 * For each `conformance/marshaling/*.json` fixture this asserts that the TS binding marshals the
 * fixture's typed `input` across the FFI boundary and reproduces the Rust golden BYTE-FOR-BYTE:
 * identical rendered `text`, `template_hash`, and `render_hash`. Render parity itself is structural
 * (one Rust core renders for every language — Principle I); what this corpus actually guards is the
 * MARSHALING boundary — that dates, decimals, nested models, null/undefined, and int/float cross
 * `napi` identically (Principle VII), so a marshaling regression that silently reshapes a value
 * (reformatting a date, rounding a decimal, dropping a field, reordering an object) is caught.
 *
 * Each fixture is rendered via the binding's STATIC (no-Zod) render path: `render(reg, name, vars)`.
 * The corpus carries no per-fixture Zod schema (it is language-neutral data); the static path is the
 * spec-005 Q4 form (`render(reg, name, data, opts?)`) where the third arg is the already-typed plain
 * data. The values are constructed already-typed from the fixture's logical-type table below.
 *
 * Test harness only: NO engine logic, NO new deps — Node built-ins (`node:fs`/`node:path`/`node:test`/
 * `node:assert`) + the built `prompting-press` facade, matching the other `test/*.mjs` files.
 */

import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";

import { Registry, render } from "prompting-press";

// --------------------------------------------------------------------------------------
// Repo-root discovery — walk up from this file's dir until a `conformance/` dir is found, so the
// harness does not hard-code a fixed number of `..` segments (robust to package relocation).
// --------------------------------------------------------------------------------------

function findRepoRoot(startDir) {
  let dir = startDir;
  for (;;) {
    try {
      const entries = readdirSync(dir);
      if (entries.includes("conformance")) return dir;
    } catch {
      // Unreadable dir — keep walking up.
    }
    const parent = dirname(dir);
    if (parent === dir) {
      throw new Error(`could not find repo root (no 'conformance/' dir) walking up from ${startDir}`);
    }
    dir = parent;
  }
}

const repoRoot = findRepoRoot(dirname(fileURLToPath(import.meta.url)));
const marshalingDir = join(repoRoot, "conformance", "marshaling");

// --------------------------------------------------------------------------------------
// Construct the native Vars value from a fixture's `{ type, value }` cell, per the logical-type
// table. The `type` tag is a LOGICAL type (language-neutral); we map it to the JS value the kernel
// must receive so the render reproduces the Rust golden.
//
// CANONICAL-SERIALIZED-FORM CHOICE (datetime / date / decimal):
//   - datetime — the Rust golden text is the literal "2026-06-28T12:30:00+00:00". A JS `Date`
//     does NOT reproduce it: `new Date("2026-06-28T12:30:00+00:00").toISOString()` yields
//     "2026-06-28T12:30:00.000Z" (millis + `Z`, not `+00:00`). The `assertDateDiverges()` check
//     below proves that divergence at runtime. So per the corpus's canonical-serialized-form
//     decision we pass the RAW STRING `value` for datetime — the kernel receives exactly the
//     pinned ISO-8601 form and renders it unchanged. (A JS `Date` is the wrong vehicle here: its
//     ISO serialization is lossy w.r.t. the offset/precision the golden pins.)
//   - date — same reasoning: a `Date` round-trips to a datetime string, not "2026-06-28". Pass the
//     raw string.
//   - decimal — JS has no native decimal type and no decimal library is added (per the fixture
//     note); a `Number` would round/lose precision ("0.00000000000000001"). Pass the raw string so
//     the kernel renders the exact decimal characters.
// --------------------------------------------------------------------------------------

/** Prove (once) that a JS Date would NOT reproduce the golden datetime — justifying the string choice. */
function assertDateDiverges(isoString) {
  const viaDate = new Date(isoString).toISOString();
  assert.notEqual(
    viaDate,
    isoString,
    `expected a JS Date to diverge from the pinned form ${isoString} (got ${viaDate}); ` +
      "if it now matches, the string-vehicle workaround can be revisited",
  );
}

function buildValue(cell) {
  switch (cell.type) {
    case "string":
    case "int":
    case "float":
    case "bool":
      // A plain JSON scalar — pass through unchanged.
      return cell.value;
    case "null":
      // Explicit null — marshals to JSON null (distinct from absent).
      return null;
    case "datetime":
      // See CANONICAL-SERIALIZED-FORM CHOICE above. Confirm a Date diverges, then use the string.
      assertDateDiverges(cell.value);
      return cell.value;
    case "date":
    case "decimal":
      // No native JS type reproduces these byte-for-byte; pass the canonical string form.
      return cell.value;
    case "object": {
      // Recurse: each map value is itself a `{ type, value }` cell.
      const out = {};
      for (const [key, child] of Object.entries(cell.value)) {
        out[key] = buildValue(child);
      }
      return out;
    }
    case "array":
      // Recurse: each element is itself a `{ type, value }` cell.
      return cell.value.map((child) => buildValue(child));
    default:
      throw new Error(`unknown logical type tag: ${JSON.stringify(cell.type)}`);
  }
}

/** Build the top-level Vars object from the fixture's `input` map, OMITTING absent fields entirely. */
function buildVars(input) {
  const vars = {};
  for (const [field, cell] of Object.entries(input)) {
    if (cell.type === "absent") continue; // absent ⇒ omit the key (JS field-not-present)
    vars[field] = buildValue(cell);
  }
  return vars;
}

// --------------------------------------------------------------------------------------
// One test per `conformance/marshaling/*.json` fixture.
// --------------------------------------------------------------------------------------

const fixtureFiles = readdirSync(marshalingDir)
  .filter((f) => f.endsWith(".json"))
  .sort();

assert.ok(fixtureFiles.length > 0, `no marshaling fixtures found in ${marshalingDir}`);

for (const file of fixtureFiles) {
  const fixture = JSON.parse(readFileSync(join(marshalingDir, file), "utf8"));
  const { case: caseName, definition, variant, input, expected } = fixture;

  test(`marshaling/${caseName}: TS render reproduces the Rust golden byte-for-byte`, () => {
    const reg = new Registry();
    reg.loadJson(JSON.stringify(definition));

    const vars = buildVars(input);
    const opts = variant === null ? undefined : { variant };

    // Static (no-Zod) render path — the corpus carries no per-fixture schema (Q4 form).
    const result = render(reg, definition.name, vars, opts);

    assert.equal(
      result.text,
      expected.text,
      `case '${caseName}': field 'text' diverged from golden\n` +
        `  expected: ${JSON.stringify(expected.text)}\n` +
        `  actual:   ${JSON.stringify(result.text)}`,
    );
    assert.equal(
      result.templateHash,
      expected.template_hash,
      `case '${caseName}': field 'template_hash' (TS .templateHash) diverged from golden\n` +
        `  expected: ${expected.template_hash}\n` +
        `  actual:   ${result.templateHash}`,
    );
    assert.equal(
      result.renderHash,
      expected.render_hash,
      `case '${caseName}': field 'render_hash' (TS .renderHash) diverged from golden\n` +
        `  expected: ${expected.render_hash}\n` +
        `  actual:   ${result.renderHash}`,
    );
  });
}
