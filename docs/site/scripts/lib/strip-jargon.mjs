/**
 * strip-jargon.mjs
 *
 * Shared helpers for stripping internal governance jargon and escaping MDX
 * cell content. Extracted from gen-shape-table.mjs (the one allowed touch of
 * that file per spec 011 T005) so that the three API-ref extractors
 * (extract-rust-api.mjs, extract-ts-api.mjs, extract-python-api.py via the
 * renderer) and the shape-table generator all apply exactly the same
 * sanitisation rules.
 *
 * Pure functions — no I/O, no side-effects.
 */

/**
 * Strip internal-governance jargon from a description string before it is
 * shown to end users. Schema descriptions and source doc-comments are written
 * for library maintainers and may cite constitution principles, roadmap
 * decisions (C-NN), FR-/SC-/SEC- IDs, and spec numbers — none of which mean
 * anything to a docs reader. We sanitize at render time rather than editing
 * the source (the source is the published contract / single source of truth).
 *
 * @param {string | null | undefined} str
 * @returns {string}
 */
export function stripJargon(str) {
	let s = String(str ?? "");
	// Drop parenthetical citation clusters: "(FR-010a)", "(constitution Principle VI v1.2.0)",
	// "(spec 008)", "(Principle III)", "(roadmap decision C-09 ...)", "(C-07)".
	s = s.replace(
		/\s*\((?:constitution\s+|roadmap\s+decision\s+)?(?:Principle\s+[IVXLC]+|FR-[0-9A-Za-z]+|SC-[0-9]+|SEC-[0-9]+|C-[0-9]+|spec\s+[0-9]+)[^)]*\)/gi,
		"",
	);
	// Drop inline " (renamed from `provenance` in spec 008)" style notes that name specs.
	s = s.replace(/\s*\(renamed from `provenance` in spec [0-9]+\)/gi, "");
	// Drop bare trailing/inline references like "per roadmap decision C-09" / "constitution Principle IV".
	s = s.replace(
		/\s*(?:per\s+)?(?:roadmap decision\s+C-[0-9]+|constitution Principle\s+[IVXLC]+)\b[^.]*/gi,
		"",
	);
	// Collapse any doubled spaces the removals leave behind.
	return s.replace(/\s{2,}/g, " ").trim();
}

/**
 * Escape content for use inside a Markdown table cell:
 *   - pipes would break column boundaries
 *   - newlines would break the row
 *
 * @param {string | null | undefined} str
 * @returns {string}
 */
export function escapeCell(str) {
	return String(str ?? "")
		.replace(/\|/g, "\\|")
		.replace(/\n/g, " ");
}
