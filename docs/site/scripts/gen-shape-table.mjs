/**
 * gen-shape-table.mjs
 *
 * Reads schemas/jsonschema/prompt-definition.schema.json (the single source of truth)
 * and renders the field tables into src/content/docs/reference/prompt-definition.mdx.
 *
 * Run automatically as the "pregenerate" / "prebuild" npm script so `pnpm build`
 * always regenerates this page before Astro touches content.
 *
 * Design:
 *   - Pure Node.js — no external dependencies (stdlib fs/path only).
 *   - Derives every table row from the schema; never hand-forks field descriptions.
 *   - Emits a full MDX page with frontmatter + three tables:
 *       1. Top-level PromptDefinition fields
 *       2. PromptVariable fields (the variables[].* sub-schema)
 *       3. PromptVariant fields (the variants[].* sub-schema)
 *   - The generated file is committed; CI regenerates and the freshness is verified
 *     by `pnpm check-stale-surface` (T005 gate).
 */

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { escapeCell, stripJargon } from "./lib/strip-jargon.mjs";

const __dirname = dirname(fileURLToPath(import.meta.url));

// Paths (relative to repo root)
const REPO_ROOT = resolve(__dirname, "../../..");
const SCHEMA_PATH = resolve(
	REPO_ROOT,
	"schemas/jsonschema/prompt-definition.schema.json",
);
const OUT_PATH = resolve(
	__dirname,
	"../src/content/docs/reference/prompt-definition.mdx",
);

// ---------------------------------------------------------------------------
// Load + parse schema
// ---------------------------------------------------------------------------

const schema = JSON.parse(readFileSync(SCHEMA_PATH, "utf-8"));

// stripJargon and escapeCell are imported from ./lib/strip-jargon.mjs above.

/** Format a JSON-schema type (string or string[]) as a code span. */
function fmtType(type) {
	if (!type) return "—";
	if (Array.isArray(type)) return "`" + type.join(" | ") + "`";
	return "`" + type + "`";
}

/**
 * Render a section of properties to a Markdown table.
 *
 * @param {Record<string, object>} properties - The schema's properties object.
 * @param {string[]} required - Names of required fields.
 * @param {object} [defs] - The schema's $defs for resolving $ref entries.
 * @returns {string} A Markdown table (with a trailing newline).
 */
function renderTable(properties, required = [], defs = {}) {
	const header =
		"| Field | Type | Required | Description |\n" +
		"|-------|------|----------|-------------|";

	const rows = Object.entries(properties).map(([name, prop]) => {
		// Resolve $ref if present
		let resolved = prop;
		if (prop.$ref) {
			const refKey = prop.$ref.replace(/^#\/\$defs\//, "");
			resolved = defs[refKey] ?? prop;
		}

		// Type: prefer oneOf summary, then type field, else "object"
		let typeStr;
		if (resolved.oneOf) {
			// e.g. type field is oneOf [{type:string,...}, {type:array,...}]
			typeStr = resolved.oneOf
				.map((s) => {
					if (s.type === "array") return "array";
					if (s.enum) return s.enum.map((v) => `"${v}"`).join(" \\| ");
					return s.type ?? "any";
				})
				.join(" \\| ");
			typeStr = "`" + typeStr + "`";
		} else if (resolved.enum) {
			typeStr = "`" + resolved.enum.map((v) => `"${v}"`).join(" \\| ") + "`";
		} else if (prop.$ref) {
			const refKey = prop.$ref.replace(/^#\/\$defs\//, "");
			typeStr = "`" + refKey + "`";
		} else {
			typeStr = fmtType(resolved.type);
		}

		const req = required.includes(name) ? "Yes" : "No";
		const desc = escapeCell(stripJargon(resolved.description ?? ""));

		// Show default if present
		const defaultVal =
			resolved.default !== undefined
				? ` Default: \`${JSON.stringify(resolved.default)}\`.`
				: "";

		return `| \`${name}\` | ${typeStr} | ${req} | ${escapeCell(desc + defaultVal)} |`;
	});

	return [header, ...rows].join("\n") + "\n";
}

// ---------------------------------------------------------------------------
// Build the page
// ---------------------------------------------------------------------------

const topLevelTable = renderTable(
	schema.properties,
	schema.required ?? [],
	schema.$defs ?? {},
);

const variableDeclDef = schema.$defs?.PromptVariable ?? {};
const variableDeclTable = renderTable(
	variableDeclDef.properties ?? {},
	variableDeclDef.required ?? [],
	schema.$defs ?? {},
);

const variantDef = schema.$defs?.PromptVariant ?? {};
const variantTable = renderTable(
	variantDef.properties ?? {},
	variantDef.required ?? [],
	schema.$defs ?? {},
);

// (spec-015: origin enum replaced by trusted boolean — no enum list needed)

// Backtick-containing strings that cannot live inside a template literal.
const promptWord = "`Prompt`";

const page = [
	"---",
	"# AUTO-GENERATED — do not edit by hand.",
	"# Source: schemas/jsonschema/prompt-definition.schema.json",
	"# Regenerate: pnpm -C docs/site build  (runs scripts/gen-shape-table.mjs as prebuild).",
	'title: "Prompt Definition"',
	'description: "Complete field reference for the prompt-definition document."',
	"---",
	"",
	"import { Aside } from '@astrojs/starlight/components';",
	"",
	'<Aside type="note">',
	"Automatically generated from the prompt-definition JSON Schema.",
	"</Aside>",
	"",
	`A prompt definition is a YAML, JSON, or TOML document that the ${promptWord} object is constructed from.`,
	"The shape is defined **once** as a JSON Schema; the per-language typed forms (Pydantic v2, TypeScript",
	"types, Rust serde structs) are **code-generated** from it at build time and never hand-maintained.",
].join("\n");

const bodySection = [
	"",
	"",
	"---",
	"",
	"## Top-level fields",
	"",
	"`name`, `role`, and `body` are **required**. All other fields are optional.",
	"",
	topLevelTable,
	"---",
	"",
	"## `variables[*]` — PromptVariable",
	"",
	"Each entry in the `variables` map is a `PromptVariable`. The `type` and `trusted` fields are required.",
	"The optional `description` field is a human-readable annotation; validation constraints belong in the",
	"per-language validator (Zod / Pydantic / garde) and are not part of this shape.",
	"",
	variableDeclTable,
	"### `trusted` flag",
	"",
	"The `trusted` field is the **per-variable input-trust flag**.",
	"It is **declarative metadata** — the kernel does not enforce it at render time. Use `check()` to",
	"detect `trusted: false` variables that lack a declared guard.",
	"",
	'<Aside type="caution">',
	"**`trusted` ≠ render-result content hashes.** The per-variable `trusted` flag (trust classification)",
	"is distinct from the render-result hashes (`template_hash` / `render_hash`). The hashes are",
	"content-addressed fingerprints of the template source and the rendered output; `trusted` is a",
	"per-field trust annotation. The two are not the same thing.",
	"</Aside>",
	"",
	"---",
	"",
	"## `variants[*]` — PromptVariant",
	"",
	"Each entry in the `variants` map is a named alternative arm. A variant carries only a `body` (its",
	"own template source) and an optional `metadata` map. Role, variables, and output model are shared",
	"across all variants.",
	"",
	'The name `"default"` is **reserved** for the root body; declaring a variant with that name is',
	"rejected at construction.",
	"",
	variantTable,
	"---",
	"",
	"## Content hashes (render-result)",
	"",
	"Every call to `render` returns a `RenderResult` that carries two content-addressed hashes (in",
	"addition to the rendered `text`):",
	"",
	"| Field | What it hashes |",
	"|-------|----------------|",
	"| `template_hash` (`templateHash` in TS) | `SHA256(resolved variant template source)` |",
	"| `render_hash` (`renderHash` in TS) | `SHA256(rendered output text)` |",
	"",
	"These hashes are **not** the same as the per-variable `trusted` flag. They are content-addressed",
	"fingerprints that can be stored in a trace to pin exactly which template produced which output —",
	"and because all three bindings share the same Rust engine, the hashes are **byte-identical** across",
	"Rust, Python, and TypeScript for the same inputs.",
].join("\n");

writeFileSync(OUT_PATH, page + bodySection + "\n", "utf-8");
console.log(`[gen-shape-table] wrote ${OUT_PATH}`);
