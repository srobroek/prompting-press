/**
 * gen-api-refs.mjs
 *
 * Orchestrator for spec 011 auto-generated language API references (T012).
 *
 * For each language (rust, python, typescript):
 *   1. Run the extractor to produce the API-doc IR (stdout JSON).
 *   2. Parse the IR.
 *   3. Call render-api-ref.mjs to produce an MDX string.
 *   4. Write <out>/<lang>.mdx.
 *
 * CLI:
 *   node docs/site/scripts/gen-api-refs.mjs
 *   node docs/site/scripts/gen-api-refs.mjs --version 0.1.0 --out /tmp/ref
 *
 * Options:
 *   --version <id>   Version label embedded in the IR / page header. Default: "latest".
 *   --out <dir>      Output directory. Default: docs/site/src/content/docs/reference.
 *
 * The --version / --out param shape is stable for spec 012 to call (R8).
 *
 * Extractor commands (verbatim — verified working):
 *   TS:     node docs/site/scripts/extract-ts-api.mjs [--version <id>]
 *   Python: uv run --with griffe==2.1.0 --with pydantic --project packages/python
 *             python3 docs/site/scripts/extract-python-api.py [--version <id>]
 *   Rust:   node docs/site/scripts/extract-rust-api.mjs [--version <id>]
 */

import { execFileSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { renderApiRef } from "./lib/render-api-ref.mjs";

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, "../../..");

// ---------------------------------------------------------------------------
// CLI args
// ---------------------------------------------------------------------------

const args = process.argv.slice(2);

function getFlag(flag, defaultValue) {
	const idx = args.indexOf(flag);
	return idx !== -1 ? (args[idx + 1] ?? defaultValue) : defaultValue;
}

const VERSION = getFlag("--version", "latest");
const OUT_DIR = getFlag(
	"--out",
	resolve(__dirname, "../src/content/docs/reference"),
);

// ---------------------------------------------------------------------------
// Extractor definitions
// ---------------------------------------------------------------------------

const EXTRACTORS = [
	{
		lang: "typescript",
		label: "TS",
		command: () =>
			execFileSync(
				process.execPath,
				[resolve(__dirname, "extract-ts-api.mjs"), "--version", VERSION],
				{ cwd: REPO_ROOT, encoding: "utf8", maxBuffer: 16 * 1024 * 1024 },
			),
	},
	{
		lang: "python",
		label: "Python",
		command: () => {
			// uv must be on PATH (installed via mise). We run uv as a subprocess.
			// The project flag makes uv pick up packages/python's uv.lock so the
			// compiled extension is available to the extractor.
			return execFileSync(
				"uv",
				[
					"run",
					"--with",
					"griffe==2.1.0",
					"--with",
					"pydantic",
					"--project",
					"packages/python",
					"python3",
					resolve(__dirname, "extract-python-api.py"),
					"--version",
					VERSION,
				],
				{ cwd: REPO_ROOT, encoding: "utf8", maxBuffer: 16 * 1024 * 1024 },
			);
		},
	},
	{
		lang: "rust",
		label: "Rust",
		command: () =>
			execFileSync(
				process.execPath,
				[resolve(__dirname, "extract-rust-api.mjs"), "--version", VERSION],
				{ cwd: REPO_ROOT, encoding: "utf8", maxBuffer: 16 * 1024 * 1024 },
			),
	},
];

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function run() {
	process.stderr.write(`[gen-api-refs] version=${VERSION} out=${OUT_DIR}\n`);

	// Ensure the output directory exists.
	mkdirSync(OUT_DIR, { recursive: true });

	let allPassed = true;

	for (const { lang, label, command } of EXTRACTORS) {
		process.stderr.write(
			`\n[gen-api-refs] ── ${label} ─────────────────────\n`,
		);

		// Step 1: run extractor, capture stdout as IR JSON.
		let rawIr;
		try {
			rawIr = command();
		} catch (err) {
			process.stderr.write(
				`[gen-api-refs] FATAL: ${label} extractor failed:\n${err.message}\n`,
			);
			allPassed = false;
			continue;
		}

		// Step 2: parse IR.
		let ir;
		try {
			ir = JSON.parse(rawIr);
		} catch (err) {
			process.stderr.write(
				`[gen-api-refs] FATAL: ${label} extractor emitted invalid JSON: ${err.message}\n`,
			);
			allPassed = false;
			continue;
		}

		// Step 3: render to MDX.
		let mdx;
		try {
			mdx = renderApiRef(ir);
		} catch (err) {
			process.stderr.write(
				`[gen-api-refs] FATAL: ${label} render failed: ${err.message}\n`,
			);
			allPassed = false;
			continue;
		}

		// Step 4: write <out>/<lang>.mdx.
		const outPath = resolve(OUT_DIR, `${lang}.mdx`);
		writeFileSync(outPath, mdx, "utf-8");
		process.stderr.write(`[gen-api-refs] wrote ${outPath}\n`);
	}

	if (!allPassed) {
		process.stderr.write(
			`\n[gen-api-refs] One or more languages FAILED. See errors above.\n`,
		);
		process.exit(1);
	}

	process.stderr.write(
		`\n[gen-api-refs] All three reference pages generated.\n`,
	);
}

run().catch((err) => {
	process.stderr.write(`[gen-api-refs] Unhandled error: ${err.message}\n`);
	process.exit(1);
});
