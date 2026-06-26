// @ts-check
/**
 * Deterministic TypeScript codegen for Prompting Press (constitution C-07).
 *
 * Generates `src/generated/prompt-definition.ts` from the single source of
 * truth JSON Schema (`schemas/jsonschema/prompt-definition.schema.json`,
 * Draft 2020-12). The emitted file is COMMITTED and freshness-gated in CI
 * (US4); it is NOT a build artifact. Run via `pnpm -C packages/typescript codegen`.
 *
 * Determinism contract (SEC-001/002, US4):
 *   - json-schema-to-typescript is exact-pinned (15.0.4); prettier is
 *     exact-pinned (3.8.4) and is the formatter json2ts uses internally.
 *     Both are locked in pnpm-lock.yaml with integrity hashes.
 *   - The banner below is STATIC: no version, no timestamp, no host data.
 *   - Output is normalized to LF line endings with a single trailing newline.
 *   - Given the same schema + pinned deps, output is byte-identical across
 *     machines and repeated runs (verified by the twice-run check in CI/T026).
 *
 * Equivalent CLI invocation (recorded for T026; this script is the canonical
 * form because it also owns the banner + newline normalization):
 *   json2ts \
 *     --input  ../../schemas/jsonschema/prompt-definition.schema.json \
 *     --output src/generated/prompt-definition.ts \
 *     --bannerComment '' \
 *     --no-additionalProperties \
 *     --unknownAny
 */

import { mkdir, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { compileFromFile } from "json-schema-to-typescript";

const HERE = dirname(fileURLToPath(import.meta.url));
const PKG_ROOT = resolve(HERE, "..");
const REPO_ROOT = resolve(PKG_ROOT, "..", "..");

const SCHEMA_PATH = resolve(
  REPO_ROOT,
  "schemas/jsonschema/prompt-definition.schema.json",
);
const OUT_PATH = resolve(PKG_ROOT, "src/generated/prompt-definition.ts");

/**
 * Stable do-not-edit banner. Intentionally carries NO version or timestamp so
 * the output stays byte-stable (the whole point of the US4 freshness gate).
 */
const BANNER = [
  "/**",
  " * GENERATED — DO NOT EDIT.",
  " *",
  " * Source of truth: schemas/jsonschema/prompt-definition.schema.json",
  " * Regenerate with: pnpm -C packages/typescript codegen",
  " *",
  " * This file is committed and freshness-gated in CI (constitution C-07).",
  " * Edit the JSON Schema and regenerate; never hand-edit this file.",
  " */",
].join("\n");

async function main() {
  // json2ts uses the pinned prettier internally to format the output.
  // `additionalProperties: false` is the default for keys without an explicit
  // setting; the schema already sets it per-object, so sealed objects stay
  // closed (no `[k: string]: unknown`) and only the open `meta`/`metadata`
  // (additionalProperties: true) get an index signature.
  const compiled = await compileFromFile(SCHEMA_PATH, {
    bannerComment: "",
    additionalProperties: false,
    unknownAny: true,
    cwd: dirname(SCHEMA_PATH),
  });

  // Normalize: strip any leading blank lines json2ts may leave from the empty
  // banner, force LF, and guarantee exactly one trailing newline.
  const body = compiled.replace(/^\s+/, "").replace(/\r\n/g, "\n").trimEnd();
  const output = `${BANNER}\n\n${body}\n`;

  await mkdir(dirname(OUT_PATH), { recursive: true });
  await writeFile(OUT_PATH, output, "utf8");

  process.stdout.write(`Wrote ${OUT_PATH}\n`);
}

main().catch((err) => {
  process.stderr.write(`${err instanceof Error ? err.stack : String(err)}\n`);
  process.exit(1);
});
