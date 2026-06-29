/**
 * render-api-ref.mjs
 *
 * Pure function: ApiDoc IR (contracts/api-doc-ir.md) → MDX string.
 *
 * Called by gen-api-refs.mjs (the orchestrator) for each language.
 * Mirrors the AUTO-GENERATED marker + style of gen-shape-table.mjs.
 *
 * Rules (spec 011 T011 / contracts/api-doc-ir.md):
 *   - AUTO-GENERATED frontmatter marker matching gen-shape-table.mjs.
 *   - Groups emitted in IR array order (extractors already sort to API_GROUPS order).
 *   - Empty groups: emitted as "(none)" to keep pages structurally parallel (FR-009).
 *   - shapeRef symbols: rendered as a link to /reference/prompt-definition/. No code
 *     fence, no doc, no doc:null gate-fail (FR-010). shapeRef wins over doc:null.
 *   - Non-shapeRef symbols: ### heading + language-tagged code fence + doc prose.
 *     Nested members (fields, variants, methods) rendered the same way, indented.
 *   - doc:null on a NON-shapeRef symbol → throw Error naming language + symbol (FR-008/R6).
 *   - Deterministic: preserves IR array order (set by extractor, R5).
 *   - Doc text already jargon-stripped + MDX-escaped by the extractor; pass through
 *     stripJargon as a defensive second pass (idempotent).
 */

import { stripJargon } from "./strip-jargon.mjs";

/**
 * Escape curly braces in MDX prose text so JSX does not interpret them as
 * expressions. Pipes and newlines were already handled by the extractor's
 * escapeCell(); braces require a second pass at render time because they are
 * not a concern for table cells (which the extractors target) but ARE a
 * concern for MDX body prose.
 *
 * @param {string} s
 * @returns {string}
 */
function escapeMdxProse(s) {
	// Replace { and } with their HTML entities so MDX does not parse them as
	// JSX expression delimiters.
	return s.replace(/\{/g, "&#123;").replace(/\}/g, "&#125;");
}

// Language display names for the page title.
const LANG_DISPLAY = {
	rust: "Rust",
	python: "Python",
	typescript: "TypeScript",
};

// Language tag for code fences.
const LANG_FENCE = {
	rust: "rust",
	python: "python",
	typescript: "ts",
};

// Shape page link path (stable). MUST include the Astro `base` (/prompting-press) so the
// link resolves under the deployed base path — matching every other internal docs link
// (e.g. /prompting-press/reference/...). A bare /reference/... would 404 on the live site.
const SHAPE_PAGE = "/prompting-press/reference/prompt-definition/";

/**
 * Render a single Symbol to MDX lines.
 *
 * @param {object} sym       - IR Symbol
 * @param {string} language  - "rust" | "python" | "typescript"
 * @param {string} context   - path for error messages (e.g. "rust > Prompt > render")
 * @param {number} depth     - heading depth (3 = ###, 4 = ####, …)
 * @returns {string[]}       - MDX lines (no trailing newline per entry)
 */
function renderSymbol(sym, language, context, depth = 3) {
	const lines = [];
	const heading = "#".repeat(depth);
	const fenceLang = LANG_FENCE[language] ?? language;

	// shapeRef: emit a link line, no code fence, no doc, no gate-fail (FR-010).
	if (sym.shapeRef !== null) {
		lines.push(
			`- [\`${sym.name}\`](${SHAPE_PAGE}) — see the Prompt definition shape.`,
		);
		return lines;
	}

	// Non-shapeRef: doc:null is a hard error (FR-008 / R6).
	if (sym.doc === null) {
		throw new Error(
			`[render-api-ref] FR-008 VIOLATION: undocumented public symbol ` +
				`"${context}" (language: ${language}). ` +
				`Add a doc comment to the source and regenerate.`,
		);
	}

	// Deprecation note prefix.
	const deprecNote =
		sym.deprecated !== null
			? `\n> **Deprecated**: ${escapeMdxProse(stripJargon(sym.deprecated))}\n`
			: "";

	// Heading + code fence + doc.
	lines.push(`${heading} \`${sym.name}\``);
	lines.push("");
	lines.push("```" + fenceLang);
	lines.push(sym.signature);
	lines.push("```");
	if (deprecNote) {
		lines.push("");
		lines.push(deprecNote.trim());
	}
	lines.push("");
	lines.push(escapeMdxProse(stripJargon(sym.doc)));

	// Nested members (fields, variants, methods).
	if (sym.members && sym.members.length > 0) {
		lines.push("");
		for (const member of sym.members) {
			const memberCtx = `${context} > ${member.name}`;
			const memberLines = renderSymbol(member, language, memberCtx, depth + 1);
			lines.push(...memberLines);
			lines.push("");
		}
	}

	return lines;
}

/**
 * Render a full ApiDoc IR object to an MDX string.
 *
 * @param {object} ir - ApiDoc IR (see contracts/api-doc-ir.md)
 * @returns {string}  - Complete MDX page content (trailing newline included)
 * @throws {Error}    - When a non-shapeRef symbol has doc:null (FR-008)
 */
export function renderApiRef(ir) {
	const { language, package: pkg, version, generatedFrom, groups } = ir;
	const langDisplay = LANG_DISPLAY[language] ?? language;
	const fenceLang = LANG_FENCE[language] ?? language;

	const lines = [];

	// ── Frontmatter (AUTO-GENERATED marker, matching gen-shape-table.mjs style) ──
	lines.push("---");
	lines.push("# AUTO-GENERATED — do not edit by hand.");
	lines.push(`# Source: ${generatedFrom}`);
	lines.push(
		`# Regenerate: pnpm -C docs/site build  (runs scripts/gen-api-refs.mjs as prebuild).`,
	);
	lines.push(`title: "${langDisplay} API reference"`);
	lines.push(
		`description: "API reference for the ${pkg} ${language} binding. Generated from source doc comments."`,
	);
	lines.push("---");
	lines.push("");

	// ── Intro note ──
	lines.push(`import { Aside } from '@astrojs/starlight/components';`);
	lines.push("");
	lines.push(`<Aside type="note">`);
	lines.push(
		`Automatically generated from ${langDisplay} source doc comments (${generatedFrom}).`,
	);
	lines.push(`</Aside>`);
	lines.push("");
	lines.push(`Package: \`${pkg}\`. Version: \`${version}\`.`);
	lines.push("");

	// ── Groups ──
	for (const group of groups) {
		lines.push(`## ${group.title}`);
		lines.push("");

		if (group.blurb !== null && group.blurb !== undefined) {
			lines.push(escapeMdxProse(stripJargon(group.blurb)));
			lines.push("");
		}

		if (!group.symbols || group.symbols.length === 0) {
			lines.push("_(none)_");
			lines.push("");
			continue;
		}

		for (const sym of group.symbols) {
			const symCtx = `${language} > ${group.title} > ${sym.name}`;
			const symLines = renderSymbol(sym, language, symCtx, 3);
			lines.push(...symLines);
			lines.push("");
		}
	}

	// Join and ensure a single trailing newline.
	return (
		lines
			.join("\n")
			.replace(/\n{3,}/g, "\n\n")
			.trimEnd() + "\n"
	);
}
