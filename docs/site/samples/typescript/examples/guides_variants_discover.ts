/**
 * Discovering the selectable variants: `variants` is the declared variant map;
 * read its keys to see what is selectable (the default arm is not listed — it is
 * the root body, name `"default"`). Standalone program.
 */

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { Prompt } from "prompting-press";

// The caller reads the definition; the library does no file I/O itself.
// Resolve the file next to this program (a real app uses its own path).
const defFile = (name: string) => fileURLToPath(new URL(name, import.meta.url));

const summary = Prompt.fromYaml(readFileSync(defFile("summary.yaml"), "utf8"));
const variants = summary.variants ?? {};

assert.deepEqual(Object.keys(variants).sort(), ["concise", "structured"]);
assert.ok("concise" in variants); // true
