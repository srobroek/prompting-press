/**
 * Spec 006 — schema round-trip conformance corpus, TypeScript side.
 *
 * For each entry in `conformance/schema/manifest.json` this feeds the document at `path` through the
 * binding's LOADER matching `form` (`Registry.loadJson` for `json`, `Registry.loadYaml` for `yaml`)
 * and asserts the entry's `verdict`:
 *   - accept ⇒ the loader does NOT throw (the document loads cleanly).
 *   - reject ⇒ the loader throws a normalized `LoadError` (an `instanceof PromptingPressError`), with
 *     nothing partially loaded (FR-007). The assertion is on the error TYPE, never message text
 *     (SEC-002): a scrubbed message is an implementation detail and must not be load-bearing.
 *
 * Accept/reject parity across the three bindings is the property under test (Principle VII); the
 * loaders do serde SHAPE deserialization, not full JSON-Schema validation, so the verdicts here are
 * the binding-observable round-trip (see the manifest's NOTE ON LAYERS).
 *
 * SEC-001: each `path` is repo-relative and MUST resolve within the repo root. The harness REJECTS
 * any absolute path or any path containing a `..` segment before reading, so a malicious/garbled
 * manifest cannot make the test read outside the corpus.
 *
 * Test harness only: NO engine logic, NO new deps — Node built-ins + the built `prompting-press`
 * facade, matching the other `test/*.mjs` files.
 */

import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import { dirname, isAbsolute, join, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";

import { Prompt, LoadError, PromptingPressError } from "prompting-press";

// --------------------------------------------------------------------------------------
// Repo-root discovery — walk up from this file's dir until a `conformance/` dir is found.
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
const manifestPath = join(repoRoot, "conformance", "schema", "manifest.json");
const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));

assert.ok(Array.isArray(manifest.fixtures), "manifest.fixtures must be an array");
assert.ok(manifest.fixtures.length > 0, "manifest.fixtures must be non-empty");

// --------------------------------------------------------------------------------------
// SEC-001 — resolve a manifest `path` to an absolute path only if it is repo-relative and contained.
// Reject absolute paths and any `..` segment; resolve from the repo root.
// --------------------------------------------------------------------------------------

function resolveContainedPath(relPath) {
  assert.ok(typeof relPath === "string" && relPath.length > 0, `bad manifest path: ${JSON.stringify(relPath)}`);
  assert.ok(!isAbsolute(relPath), `SEC-001: absolute path rejected: ${relPath}`);
  const segments = relPath.split(/[\\/]/);
  assert.ok(!segments.includes(".."), `SEC-001: path traversal ('..') rejected: ${relPath}`);
  const resolved = join(repoRoot, relPath);
  // Defense in depth: the resolved path must still be inside the repo root.
  assert.ok(
    resolved === repoRoot || resolved.startsWith(repoRoot + sep),
    `SEC-001: resolved path escapes repo root: ${resolved}`,
  );
  return resolved;
}

// --------------------------------------------------------------------------------------
// One test per manifest entry.
// --------------------------------------------------------------------------------------

for (const entry of manifest.fixtures) {
  const { path: relPath, form, verdict, note } = entry;
  const label = `schema/${verdict}/${form}: ${relPath}${note ? ` (${note})` : ""}`;

  test(label, () => {
    const absPath = resolveContainedPath(relPath);
    const text = readFileSync(absPath, "utf8");

    const load = () => {
      if (form === "json") Prompt.fromJson(text);
      else if (form === "yaml") Prompt.fromYaml(text);
      else throw new Error(`unknown manifest form: ${JSON.stringify(form)}`);
    };

    if (verdict === "accept") {
      assert.doesNotThrow(load, `expected '${relPath}' to load cleanly via ${form}`);
    } else if (verdict === "reject") {
      // Assert on the error TYPE only (SEC-002): it must be the normalized LoadError, which is an
      // instanceof PromptingPressError. Never assert on the (scrubbed) message text.
      assert.throws(
        load,
        (err) => {
          assert.ok(
            err instanceof LoadError,
            `expected a LoadError for '${relPath}', got ${err?.constructor?.name ?? typeof err}`,
          );
          assert.ok(err instanceof PromptingPressError, "LoadError must be in the PromptingPressError hierarchy");
          return true;
        },
        `expected '${relPath}' to be rejected via ${form}`,
      );
    } else {
      throw new Error(`unknown manifest verdict: ${JSON.stringify(verdict)}`);
    }
  });
}
