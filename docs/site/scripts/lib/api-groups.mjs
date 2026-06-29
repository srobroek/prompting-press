/**
 * api-groups.mjs
 *
 * Canonical API-doc group set and order, shared across all three language
 * extractors (extract-rust-api.mjs, extract-ts-api.mjs, extract-python-api.py
 * via the renderer render-api-ref.mjs).
 *
 * The group ORDER here is the single source of truth for page layout (spec 011,
 * contracts/api-doc-ir.md). Every extractor assigns symbols to one of these
 * group titles and the renderer emits groups in the order they appear in
 * API_GROUPS. A group with no symbols for a given language is still emitted
 * (empty section) to keep the three pages structurally parallel (FR-009).
 *
 * Authority: specs/011-autogen-api-refs/contracts/api-doc-ir.md
 */

/**
 * @typedef {Object} GroupDef
 * @property {string} title   - Display heading (used as the `##` section title on the page).
 * @property {string} anchor  - Stable URL slug (lowercase, hyphens); must be unique.
 * @property {string} blurb   - One-line description for maintainers (NOT rendered on the page;
 *                              group-level blurb for the page comes from the extractor, which
 *                              may set it to null).
 */

/**
 * Canonical group definitions in display order.
 *
 * Prompt → RenderResult → GuardConfig → CheckReport / Finding →
 * Composition / Message → Errors → Shape types
 *
 * @type {GroupDef[]}
 */
export const API_GROUPS = [
	{
		title: "Prompt",
		anchor: "prompt",
		blurb: "The core Prompt type and its construction / rendering methods.",
	},
	{
		title: "RenderResult",
		anchor: "render-result",
		blurb: "The value returned by render(): rendered text + provenance hashes.",
	},
	{
		title: "GuardConfig",
		anchor: "guard-config",
		blurb:
			"Configuration for the origin-guard check (untrusted-variable detection).",
	},
	{
		title: "CheckReport",
		anchor: "check-report",
		blurb:
			"The report returned by check(): agreement-check results for a registry.",
	},
	{
		title: "Finding",
		anchor: "finding",
		blurb:
			"A single agreement-check finding (variable/template mismatch, guard violation, etc.).",
	},
	{
		title: "Composition",
		anchor: "composition",
		blurb:
			"Multi-message prompt composition: an ordered sequence of (prompt-ref, vars) pairs.",
	},
	{
		title: "Message",
		anchor: "message",
		blurb:
			"A resolved role+text message produced by composing a multi-message prompt.",
	},
	{
		title: "Errors",
		anchor: "errors",
		blurb: "Error hierarchy, error codes, and structured FieldError.",
	},
	{
		title: "Shape types",
		anchor: "shape-types",
		blurb:
			"Re-exported shape types (PromptDefinition, PromptVariable, PromptVariant) — link to prompt-definition.mdx.",
	},
];

/**
 * Map from group title to its zero-based index in API_GROUPS.
 * Extractors can use this for fast lookup when assigning a symbol to its group.
 *
 * @type {Map<string, number>}
 */
export const GROUP_ORDER = new Map(API_GROUPS.map((g, i) => [g.title, i]));

/**
 * All valid group titles, in canonical order.
 * Useful for validation: an extractor should warn/fail if it tries to emit a
 * symbol under a title not in this set.
 *
 * @type {string[]}
 */
export const GROUP_TITLES = API_GROUPS.map((g) => g.title);

// ---------------------------------------------------------------------------
// IR field-shape reminder (informational — see contracts/api-doc-ir.md)
// ---------------------------------------------------------------------------
//
// ApiDoc (top-level IR object)
//   language:      "rust" | "python" | "typescript"
//   package:       string               — display name (e.g. "prompting-press")
//   version:       string               — from --version flag (default "latest")
//   generatedFrom: string               — provenance line, informational
//   groups:        Group[]              — in API_GROUPS order
//
// Group
//   title:   string                     — must match a GROUP_TITLES entry
//   anchor:  string                     — stable URL slug
//   blurb:   string | null              — jargon-stripped group blurb; null if none
//   symbols: Symbol[]                   — sorted by extractor: kind asc, then name asc
//
// Symbol
//   name:       string
//   kind:       "class"|"struct"|"enum"|"interface"|"function"|"method"|
//               "constructor"|"accessor"|"field"|"variant"|"const"|"type"
//   signature:  string                  — language-native, verbatim, for code fence
//   doc:        string | null           — jargon-stripped + MDX-escaped; null ⇒ gate FAILS
//   members:    Symbol[]                — nested items (fields, methods, variants); [] if none
//   shapeRef:   string | null           — when set, renderer emits a link to prompt-definition.mdx
//   deprecated: string | null           — deprecation note; null if not deprecated
