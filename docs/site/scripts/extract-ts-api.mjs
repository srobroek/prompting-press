/**
 * extract-ts-api.mjs
 *
 * TypeScript API-doc extractor for spec 011.
 *
 * Runs TypeDoc 0.28.19 --json over packages/typescript/src/index.ts, consumes
 * the TypeDoc JSON, and emits the API-doc IR (contracts/api-doc-ir.md) to
 * stdout.
 *
 * Public surface (FR-005): the symbols present in the top-level TypeDoc
 * `groups` array (== the explicit export set of index.ts).
 *
 * Re-exported shape types (PromptDefinition / PromptVariable / PromptVariant)
 * get `shapeRef` set; their members are NOT expanded (FR-010).
 *
 * A public symbol with no doc comment → `doc: null` (FR-008).
 *
 * Jargon-stripping and MDX-escaping via lib/strip-jargon.mjs (FR-006 / FR-007).
 *
 * Usage:
 *   node docs/site/scripts/extract-ts-api.mjs
 *   node docs/site/scripts/extract-ts-api.mjs --version 0.1.0
 *
 * NOTE for Python extractor (extract-python-api.py): replicate the SAME
 * stripping rules as strip-jargon.mjs. The regexes to match are:
 *   1. Principle/FR/SC/SEC/C-NN/spec citation parentheticals (see strip-jargon.mjs line 29-32)
 *   2. "(renamed from 'provenance' in spec NNN)" notes (strip-jargon.mjs line 34)
 *   3. Bare "per roadmap decision C-NN" / "constitution Principle X" references (strip-jargon.mjs line 36-39)
 *   4. Collapse doubled spaces + trim
 * Shell out to node/this script or re-implement those four substitutions in Python.
 */

import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, unlinkSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { API_GROUPS } from "./lib/api-groups.mjs";
import { escapeCell, stripJargon } from "./lib/strip-jargon.mjs";

// ---------------------------------------------------------------------------
// Extended jargon stripping — handles patterns not covered by strip-jargon.mjs
//
// strip-jargon.mjs removes *parenthetical* citation clusters like "(FR-010a)"
// or "(constitution Principle VI)". Source doc comments also carry inline
// (non-parenthetical) references that need sanitizing before they reach the
// rendered page:
//
//   - "SEC-004-scrubbed"  → "scrubbed"  (SEC-NNN- prefix before a word)
//   - "(Q3 / Principle I)"  → ""        (mixed parens: Qx + Principle)
//   - "FR-013" bare in text → ""        (bare FR-/SC-/SEC- code with no parens)
//   - "§SectionName"  → ""              (section cross-references)
//   - "data-model §RenderResult"  → ""
//   - "#[napi]" Rust attribute tokens  → ""
//   - "`[prompting_press_core::Foo]`" rustdoc link syntax  → "Foo"
//
// These are applied AFTER stripJargon (which handles the parenthetical cases).
// ---------------------------------------------------------------------------

/**
 * Additional stripping layer for inline jargon patterns that escape the
 * parenthetical-only rules in strip-jargon.mjs.
 *
 * @param {string} s
 * @returns {string}
 */
function stripInlineJargon(s) {
	// Remove "SEC-NNN-" prefix before a word: "SEC-004-scrubbed" -> "scrubbed"
	s = s.replace(/\bSEC-\d+-/gi, "");
	// Remove mixed parentheticals containing Q-codes or slash combos with jargon:
	// "(Q3 / Principle I)", "(Q1)", "(Q3 / Principle I / R2)", "(owned; Principle V: ...)"
	s = s.replace(
		/\s*\([^)]*\b(?:Q\d+|R\d+|Principle\s+[IVXLC]+)\b[^)]*\)/gi,
		"",
	);
	// Remove bare inline "Principle X:" or "Principle X." or "Principle X," references
	// that appear inside list items or prose (not already caught by parenthetical rules).
	// Match: optional leading space/semicolon, "Principle <roman>", optional trailing punct.
	s = s.replace(/[;,]?\s*\bPrinciple\s+[IVXLC]+\s*[;:,.]?\s*/gi, " ");
	// Remove bare FR-/SC-/SEC- codes remaining after parenthetical strip:
	// "FR-013", "FR-020", "SC-006" etc. — match as whole tokens.
	s = s.replace(/\b(?:FR|SC|SEC)-[0-9A-Za-z]+\b/gi, "");
	// Remove rustdoc-style cross-refs: "[`prompting_press_core::Foo`]" -> "Foo"
	// and "[`prompting_press::Bar`]" -> "Bar"
	s = s.replace(/\[`[a-z_:]+::([A-Za-z_]+)`\]/g, "$1");
	// Remove section cross-references: "data-model §RenderResult", "§CheckReport"
	s = s.replace(/\bdata-model\s+§\w+/gi, "");
	s = s.replace(/§\w+/g, "");
	// Remove Rust attribute tokens: "#[napi]", "#[napi(…)]"
	s = s.replace(/#\[[^\]]+\]/g, "");
	// Collapse any doubled spaces left behind.
	return s.replace(/\s{2,}/g, " ").trim();
}

/**
 * Full doc-text pipeline: TypeDoc comment → jargon-free, MDX-escaped string.
 * Applies strip-jargon.mjs rules first, then the inline-jargon pass above.
 *
 * @param {string|null} raw
 * @returns {string|null}
 */
function fullStrip(raw) {
	if (raw === null || raw === undefined) return null;
	const s1 = stripJargon(raw);
	const s2 = stripInlineJargon(s1);
	const s3 = escapeCell(s2);
	return s3.length > 0 ? s3 : null;
}

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, "../../..");

// ---------------------------------------------------------------------------
// CLI args: --version <ver>
// ---------------------------------------------------------------------------

const args = process.argv.slice(2);
const versionFlagIdx = args.indexOf("--version");
const VERSION =
	versionFlagIdx >= 0 ? (args[versionFlagIdx + 1] ?? "latest") : "latest";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const TYPEDOC_BIN = resolve(__dirname, "../node_modules/typedoc/bin/typedoc");
const TS_ENTRY = resolve(REPO_ROOT, "packages/typescript/src/index.ts");
const TS_TSCONFIG = resolve(REPO_ROOT, "packages/typescript/tsconfig.json");

/** The three generated shape types — re-exported, not re-rendered (FR-010). */
const SHAPE_REFS = new Set([
	"PromptDefinition",
	"PromptVariable",
	"PromptVariant",
]);

// ---------------------------------------------------------------------------
// TypeDoc kind constants (TypeDoc 0.28.x ReflectionKind numeric values)
// ---------------------------------------------------------------------------
const KIND_PROJECT = 1;
const KIND_MODULE = 2;
const KIND_NAMESPACE = 4;
const KIND_ENUM = 8;
const KIND_ENUM_MEMBER = 16;
const KIND_VARIABLE = 32;
const KIND_FUNCTION = 64;
const KIND_CLASS = 128;
const KIND_INTERFACE = 256;
const KIND_CONSTRUCTOR = 512;
const KIND_PROPERTY = 1024;
const KIND_METHOD = 2048;
const KIND_TYPE_ALIAS = 2097152;
const KIND_ACCESSOR = 262144; // get/set accessor pair

// ---------------------------------------------------------------------------
// Group assignment: symbol name → canonical group title
// ---------------------------------------------------------------------------

/** Map each public TS symbol to its canonical API-doc group. */
function assignGroup(name) {
	switch (name) {
		case "Prompt":
			return "Prompt";
		case "RenderResult":
			return "RenderResult";
		case "GuardConfig":
		case "RenderOptions":
			return "GuardConfig";
		case "CheckReport":
			return "CheckReport";
		case "Finding":
			return "Finding";
		case "Composition":
		case "CompositionEntry":
			return "Composition";
		case "Message":
			return "Message";
		case "PromptingPressError":
		case "PromptValidationError":
		case "PromptRenderError":
		case "LoadError":
		case "FieldError":
		case "ZodLikeSchema":
		case "ValidatorMap":
		case "coreVersion":
			return "Errors";
		case "PromptDefinition":
		case "PromptVariable":
		case "PromptVariant":
			return "Shape types";
		default:
			// Unknown symbol: assign to Errors group as a fallback (and log to stderr).
			process.stderr.write(
				`[extract-ts-api] WARN: unknown symbol "${name}", assigning to "Errors" group\n`,
			);
			return "Errors";
	}
}

// ---------------------------------------------------------------------------
// TypeDoc kind → IR kind
// ---------------------------------------------------------------------------

function irKind(tdKind, node) {
	switch (tdKind) {
		case KIND_CLASS:
			return "class";
		case KIND_INTERFACE:
			return "interface";
		case KIND_ENUM:
			return "enum";
		case KIND_ENUM_MEMBER:
			return "variant";
		case KIND_FUNCTION:
			return "function";
		case KIND_METHOD:
			return "method";
		case KIND_CONSTRUCTOR:
			return "constructor";
		case KIND_ACCESSOR:
			return "accessor";
		case KIND_PROPERTY:
			return "field";
		case KIND_VARIABLE:
			return "const";
		case KIND_TYPE_ALIAS:
			return "type";
		default:
			return "type";
	}
}

// ---------------------------------------------------------------------------
// Type serialization: TypeDoc type objects → TS-native string
// ---------------------------------------------------------------------------

/** Serialize a TypeDoc type object to a human-readable TS type string. */
function serializeType(type) {
	if (!type) return "unknown";
	switch (type.type) {
		case "intrinsic":
			return type.name;
		case "literal": {
			if (type.value === null) return "null";
			if (typeof type.value === "string") return JSON.stringify(type.value);
			return String(type.value);
		}
		case "reference": {
			const base = type.name ?? "unknown";
			if (type.typeArguments?.length) {
				return `${base}<${type.typeArguments.map(serializeType).join(", ")}>`;
			}
			return base;
		}
		case "array":
			return `${serializeType(type.elementType)}[]`;
		case "union":
			return type.types.map(serializeType).join(" | ");
		case "intersection":
			return type.types.map(serializeType).join(" & ");
		case "typeOperator":
			return `${type.operator} ${serializeType(type.target)}`;
		case "reflection":
			// Anonymous inline type — render as object/function shorthand.
			if (type.declaration?.signatures?.length) {
				const sig = type.declaration.signatures[0];
				const params = (sig.parameters ?? [])
					.map((p) => `${p.name}: ${serializeType(p.type)}`)
					.join(", ");
				return `(${params}) => ${serializeType(sig.type)}`;
			}
			if (type.declaration?.children?.length) {
				const fields = type.declaration.children
					.map((c) => `${c.name}: ${serializeType(c.type)}`)
					.join("; ");
				return `{ ${fields} }`;
			}
			return "object";
		case "tuple":
			return `[${(type.elements ?? []).map(serializeType).join(", ")}]`;
		case "optional":
			return `${serializeType(type.elementType)}?`;
		case "rest":
			return `...${serializeType(type.elementType)}`;
		case "conditional":
			return `${serializeType(type.checkType)} extends ${serializeType(type.extendsType)} ? ${serializeType(type.trueType)} : ${serializeType(type.falseType)}`;
		case "indexedAccess":
			return `${serializeType(type.objectType)}[${serializeType(type.indexType)}]`;
		case "mapped":
			return `{ [${type.parameter} in ${serializeType(type.parameterType)}]: ${serializeType(type.templateType)} }`;
		case "predicate":
			return `${type.asserts ? "asserts " : ""}${type.name}${type.targetType ? ` is ${serializeType(type.targetType)}` : ""}`;
		case "inferred":
			return `infer ${type.name}`;
		case "query":
			return `typeof ${serializeType(type.queryType)}`;
		case "template-literal": {
			const parts = (type.tail ?? []).map(
				([t, s]) => `${serializeType(t)}${s}`,
			);
			return `\`${type.head}${parts.join("")}\``;
		}
		case "named-tuple-member":
			return type.isOptional
				? `${type.name}?: ${serializeType(type.element)}`
				: `${type.name}: ${serializeType(type.element)}`;
		case "unknown":
			return type.name ?? "unknown";
		default:
			return "unknown";
	}
}

// ---------------------------------------------------------------------------
// Signature serialization: TypeDoc signature → TS-native string
// ---------------------------------------------------------------------------

function serializeSignature(sig, ownerName, isStatic = false) {
	const kindNum = sig.kind;
	// Constructor: "new OwnerName(params): OwnerName"
	if (kindNum === 16384 /* constructor signature */) {
		const params = serializeParams(sig.parameters ?? []);
		return `new ${ownerName}(${params}): ${ownerName}`;
	}
	// Function / method call signature
	const typeParams = sig.typeParameter?.length
		? `<${sig.typeParameter
				.map((tp) => {
					let s = tp.name;
					if (tp.type) s += ` extends ${serializeType(tp.type)}`;
					if (tp.default) s += ` = ${serializeType(tp.default)}`;
					return s;
				})
				.join(", ")}>`
		: "";
	const params = serializeParams(sig.parameters ?? []);
	const ret = sig.type ? serializeType(sig.type) : "void";
	const staticPrefix = isStatic ? "static " : "";
	return `${staticPrefix}${sig.name}${typeParams}(${params}): ${ret}`;
}

function serializeParams(params) {
	return params
		.map((p) => {
			const opt = p.flags?.isOptional ? "?" : "";
			const rest = p.flags?.isRest ? "..." : "";
			const typeStr = p.type ? serializeType(p.type) : "unknown";
			return `${rest}${p.name}${opt}: ${typeStr}`;
		})
		.join(", ");
}

// ---------------------------------------------------------------------------
// Comment text extraction: TypeDoc comment object → plain text string
// ---------------------------------------------------------------------------

/**
 * Extract the plain text of a TypeDoc comment `summary` array (inline-tag
 * `@link` → the link text; `code` → backtick-wrapped; `text` → verbatim).
 */
function extractCommentText(comment) {
	if (!comment) return null;
	const parts = comment.summary ?? [];
	if (parts.length === 0) return null;
	const raw = parts
		.map((part) => {
			if (part.kind === "text") return part.text;
			if (part.kind === "code") return part.text; // already backtick-wrapped
			if (part.kind === "inline-tag" && part.tag === "@link") {
				return part.text ?? "";
			}
			return part.text ?? "";
		})
		.join("");
	// Collapse newlines to spaces for single-line prose (MDX table-safe).
	const flat = raw.replace(/\n/g, " ").trim();
	return flat.length > 0 ? flat : null;
}

/** Extract doc text, strip jargon, escape for MDX. Returns null when absent. */
function docText(comment) {
	const raw = extractCommentText(comment);
	return fullStrip(raw);
}

// ---------------------------------------------------------------------------
// Deprecation: extract @deprecated tag text from a comment
// ---------------------------------------------------------------------------

function extractDeprecated(comment) {
	if (!comment?.blockTags) return null;
	const tag = comment.blockTags.find((t) => t.tag === "@deprecated");
	if (!tag) return null;
	const raw = extractCommentText({ summary: tag.content ?? [] }) ?? "";
	return fullStrip(raw) || "deprecated";
}

// ---------------------------------------------------------------------------
// Member extraction: TypeDoc child node → IR Symbol
// ---------------------------------------------------------------------------

/**
 * Build an IR Symbol for a member node. Returns null for private/internal
 * members (e.g. Symbol-keyed members like [PROMPT_HANDLE_KEY]).
 */
function memberToSymbol(child, ownerName) {
	// Skip Symbol-keyed members (e.g. [PROMPT_HANDLE_KEY]) — internal.
	if (
		child.name.startsWith("[") ||
		child.flags?.isPrivate ||
		child.flags?.isProtected
	) {
		return null;
	}

	// Skip members inherited from parent classes (e.g. Error.cause, Error.stack,
	// Error.message, Error.name). These are not part of our public API surface.
	if (child.inheritedFrom !== undefined && child.inheritedFrom !== null) {
		return null;
	}

	const kind = irKind(child.kind, child);

	let sig;
	let comment;
	let deprecated;

	if (child.kind === KIND_ACCESSOR) {
		// get accessor: signature is on getSignature
		const get = child.getSignature;
		const retType = get?.type ? serializeType(get.type) : "unknown";
		const flagPrefix = child.flags?.isReadonly ? "readonly " : "";
		sig = `get ${flagPrefix}${child.name}(): ${retType}`;
		comment = get?.comment ?? null;
		deprecated = extractDeprecated(get?.comment ?? null);
	} else if (child.kind === KIND_CONSTRUCTOR) {
		const cs = child.signatures?.[0];
		// Skip implicit (undocumented) constructors — TypeDoc synthesizes a
		// default constructor for every napi class even when the TS source has
		// no explicit doc comment. These are framework plumbing, not public API.
		if (!cs?.comment) return null;
		sig = cs ? serializeSignature(cs, ownerName) : `new ${ownerName}()`;
		comment = cs?.comment ?? null;
		deprecated = extractDeprecated(comment);
	} else if (child.kind === KIND_METHOD || child.kind === KIND_FUNCTION) {
		// Methods may have multiple overload signatures; use the last (implementation
		// signature is last, or the last overload is the most general public one).
		// For overloaded methods, the first overload signature carries the doc comment.
		const signatures = child.signatures ?? [];
		const docSig = signatures[0]; // first signature has the doc comment
		const implSig = signatures[signatures.length - 1]; // last for parameter shape
		const isStatic = child.flags?.isStatic ?? false;
		if (implSig) {
			sig = serializeSignature(implSig, ownerName, isStatic);
		} else {
			sig = `${isStatic ? "static " : ""}${child.name}()`;
		}
		comment = docSig?.comment ?? null;
		deprecated = extractDeprecated(docSig?.comment ?? null);
	} else if (child.kind === KIND_PROPERTY) {
		// Interface / class property / field
		const readonly = child.flags?.isReadonly ? "readonly " : "";
		const opt = child.flags?.isOptional ? "?" : "";
		const typeStr = child.type ? serializeType(child.type) : "unknown";
		sig = `${readonly}${child.name}${opt}: ${typeStr}`;
		comment = child.comment ?? null;
		deprecated = extractDeprecated(comment);
	} else {
		// Fallback
		const typeStr = child.type ? serializeType(child.type) : "unknown";
		sig = `${child.name}: ${typeStr}`;
		comment = child.comment ?? null;
		deprecated = extractDeprecated(comment);
	}

	return {
		name: child.name,
		kind,
		signature: sig,
		doc: docText(comment),
		members: [],
		shapeRef: null,
		deprecated: deprecated ?? null,
	};
}

// ---------------------------------------------------------------------------
// Top-level symbol → IR Symbol
// ---------------------------------------------------------------------------

function symbolToIR(node) {
	const name = node.name;
	const isShapeRef = SHAPE_REFS.has(name);

	// --- doc comment ---
	// For classes/interfaces the comment is on the node itself.
	// For functions it's on the first signature.
	let topLevelComment = node.comment ?? null;
	if (!topLevelComment && node.signatures?.length) {
		topLevelComment = node.signatures[0].comment ?? null;
	}

	const doc = isShapeRef ? null : docText(topLevelComment);
	const deprecated = extractDeprecated(topLevelComment);

	// --- kind ---
	const kind = irKind(node.kind, node);

	// --- signature ---
	let signature;
	if (node.kind === KIND_CLASS || node.kind === KIND_INTERFACE) {
		// "class Foo" / "interface Foo" — the class/interface declaration line.
		const kw = node.kind === KIND_CLASS ? "class" : "interface";
		signature = `${kw} ${name}`;
	} else if (node.kind === KIND_FUNCTION) {
		const sig = node.signatures?.[0];
		signature = sig ? serializeSignature(sig, name) : `${name}()`;
	} else if (node.kind === KIND_TYPE_ALIAS) {
		// type ValidatorMap = ZodLikeSchema
		const typeStr = node.type ? serializeType(node.type) : "unknown";
		signature = `type ${name} = ${typeStr}`;
	} else {
		signature = name;
	}

	// --- members ---
	let members = [];
	if (!isShapeRef && node.children?.length) {
		members = node.children
			.map((child) => memberToSymbol(child, name))
			.filter(Boolean)
			.sort((a, b) => {
				if (a.kind < b.kind) return -1;
				if (a.kind > b.kind) return 1;
				return a.name.localeCompare(b.name);
			});
	}

	return {
		name,
		kind,
		signature,
		doc: isShapeRef ? null : doc,
		members,
		shapeRef: isShapeRef ? name : null,
		deprecated: deprecated ?? null,
	};
}

// ---------------------------------------------------------------------------
// Kind ordering for deterministic sort within a group (IR R5)
// ---------------------------------------------------------------------------

const KIND_SORT_ORDER = {
	class: 0,
	struct: 1,
	enum: 2,
	interface: 3,
	function: 4,
	method: 5,
	constructor: 6,
	accessor: 7,
	field: 8,
	variant: 9,
	const: 10,
	type: 11,
};

function symbolSortKey(sym) {
	return (KIND_SORT_ORDER[sym.kind] ?? 99) * 1000 + sym.name.charCodeAt(0);
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

function run() {
	// Step 1: run TypeDoc --json to a temp file (TypeDoc writes to a file, not stdout).
	process.stderr.write("[extract-ts-api] Running TypeDoc --json…\n");
	const tmpDir = mkdtempSync(join(tmpdir(), "extract-ts-api-"));
	const tdOutPath = join(tmpDir, "typedoc.json");
	let tdJson;
	try {
		execFileSync(
			process.execPath, // node
			[
				TYPEDOC_BIN,
				"--json",
				tdOutPath,
				"--tsconfig",
				TS_TSCONFIG,
				"--entryPoints",
				TS_ENTRY,
				"--entryPointStrategy",
				"expand",
				"--skipErrorChecking",
				"--logLevel",
				"Warn",
			],
			{
				cwd: REPO_ROOT,
				encoding: "utf8",
				stdio: ["inherit", "inherit", "inherit"],
			},
		);
		const raw = readFileSync(tdOutPath, "utf8");
		tdJson = JSON.parse(raw);
	} catch (err) {
		process.stderr.write(
			`[extract-ts-api] FATAL: TypeDoc failed: ${err.message}\n`,
		);
		process.exit(1);
	} finally {
		try {
			unlinkSync(tdOutPath);
		} catch {
			/* ignore */
		}
	}

	// Step 2: collect the public surface (symbols in top-level groups).
	const exportedIds = new Set(
		(tdJson.groups ?? []).flatMap((g) => g.children ?? []),
	);
	const byId = new Map((tdJson.children ?? []).map((c) => [c.id, c]));

	const publicNodes = [...exportedIds]
		.map((id) => byId.get(id))
		.filter(Boolean);

	if (publicNodes.length === 0) {
		process.stderr.write(
			"[extract-ts-api] FATAL: No public symbols found in TypeDoc output.\n",
		);
		process.exit(1);
	}

	process.stderr.write(
		`[extract-ts-api] Found ${publicNodes.length} public symbols.\n`,
	);

	// Step 3: convert to IR symbols.
	const irSymbols = publicNodes.map(symbolToIR);

	// Step 4: bucket into API_GROUPS and sort within each group.
	const groupBuckets = new Map(API_GROUPS.map((g) => [g.title, []]));
	for (const sym of irSymbols) {
		const groupTitle = assignGroup(sym.name);
		if (!groupBuckets.has(groupTitle)) {
			process.stderr.write(
				`[extract-ts-api] WARN: unknown group title "${groupTitle}" for symbol "${sym.name}"\n`,
			);
			continue;
		}
		groupBuckets.get(groupTitle).push(sym);
	}

	// Sort within each group: kind asc, then name asc (R5).
	for (const syms of groupBuckets.values()) {
		syms.sort((a, b) => {
			const ka = KIND_SORT_ORDER[a.kind] ?? 99;
			const kb = KIND_SORT_ORDER[b.kind] ?? 99;
			if (ka !== kb) return ka - kb;
			return a.name.localeCompare(b.name);
		});
	}

	// Step 5: build the IR groups array (emit all groups, even empty — FR-009).
	const groups = API_GROUPS.map((gDef) => ({
		title: gDef.title,
		anchor: gDef.anchor,
		blurb: null, // group-level blurb: null (no per-group prose in TS source)
		symbols: groupBuckets.get(gDef.title) ?? [],
	}));

	// Step 6: assemble the top-level ApiDoc IR.
	const ir = {
		language: "typescript",
		package: "prompting-press",
		version: VERSION,
		generatedFrom: "typedoc 0.28.19",
		groups,
	};

	// Step 7: emit to stdout.
	process.stdout.write(JSON.stringify(ir, null, 2));
	process.stdout.write("\n");

	// Step 8: summary to stderr.
	const totalSymbols = groups.reduce((n, g) => n + g.symbols.length, 0);
	process.stderr.write(
		`[extract-ts-api] Emitted ${totalSymbols} symbols across ${groups.filter((g) => g.symbols.length > 0).length} non-empty groups.\n`,
	);
}

run();
