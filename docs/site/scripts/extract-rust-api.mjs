/**
 * extract-rust-api.mjs
 *
 * Extracts the public API surface of `prompting-press` from rustdoc JSON and
 * emits the API-doc IR defined in
 * specs/011-autogen-api-refs/contracts/api-doc-ir.md.
 *
 * Usage (standalone):
 *   node docs/site/scripts/extract-rust-api.mjs
 *   node docs/site/scripts/extract-rust-api.mjs --version 0.3.0
 *
 * Steps:
 *   1. Run `cargo +nightly-2026-05-15 rustdoc` for prompting-press and
 *      prompting-press-core (needed for RenderResult + GuardConfig docs/fields).
 *   2. Assert format_version == 57 on the consumer crate output.
 *   3. Filter to the public re-export set from lib.rs (FR-005 / IO-2).
 *   4. Set shapeRef on PromptDefinition/PromptVariable/PromptVariant (FR-010).
 *   5. Emit IR JSON to stdout.
 *
 * A public symbol with no doc comment → doc: null (FR-008).
 * Doc text is jargon-stripped + MDX-escaped via strip-jargon.mjs.
 */

import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { API_GROUPS } from "./lib/api-groups.mjs";
import { escapeCell, stripJargon } from "./lib/strip-jargon.mjs";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, "../../..");

const NIGHTLY_CHANNEL = "nightly-2026-05-15";
const EXPECTED_FORMAT_VERSION = 57;

/** Shape re-exports: set shapeRef, do NOT expand members (FR-010). */
const SHAPE_REF_TYPES = new Set([
	"PromptDefinition",
	"PromptVariable",
	"PromptVariant",
]);

// ---------------------------------------------------------------------------
// CLI args
// ---------------------------------------------------------------------------

const args = process.argv.slice(2);
const versionIdx = args.indexOf("--version");
const version = versionIdx !== -1 ? args[versionIdx + 1] : "latest";

// ---------------------------------------------------------------------------
// Step 1: Run rustdoc for the consumer crate + core crate
// ---------------------------------------------------------------------------

function runRustdoc(pkg) {
	process.stderr.write(
		`[extract-rust-api] cargo +${NIGHTLY_CHANNEL} rustdoc -p ${pkg} -- -Z unstable-options --output-format json\n`,
	);
	try {
		execFileSync(
			"cargo",
			[
				`+${NIGHTLY_CHANNEL}`,
				"rustdoc",
				"-p",
				pkg,
				"--",
				"-Z",
				"unstable-options",
				"--output-format",
				"json",
			],
			{ cwd: REPO_ROOT, stdio: ["inherit", "inherit", "inherit"] },
		);
	} catch (e) {
		process.stderr.write(
			`[extract-rust-api] FATAL: cargo rustdoc -p ${pkg} failed: ${e.message}\n`,
		);
		process.exit(1);
	}
}

runRustdoc("prompting-press");
runRustdoc("prompting-press-core");

// ---------------------------------------------------------------------------
// Step 2: Load + validate the consumer crate JSON
// ---------------------------------------------------------------------------

const CONSUMER_JSON = resolve(
	REPO_ROOT,
	"target/doc/prompting_press.json",
);
const CORE_JSON = resolve(
	REPO_ROOT,
	"target/doc/prompting_press_core.json",
);

process.stderr.write(`[extract-rust-api] Loading ${CONSUMER_JSON}\n`);
const rdoc = JSON.parse(readFileSync(CONSUMER_JSON, "utf-8"));

if (rdoc.format_version !== EXPECTED_FORMAT_VERSION) {
	process.stderr.write(
		`[extract-rust-api] FATAL: rustdoc JSON format_version is ${rdoc.format_version}, ` +
			`expected ${EXPECTED_FORMAT_VERSION}. ` +
			`Update EXPECTED_FORMAT_VERSION in this script and rust-toolchain-nightly.toml ` +
			`after verifying the new nightly.\n`,
	);
	process.exit(1);
}

process.stderr.write(`[extract-rust-api] Loading ${CORE_JSON}\n`);
const rdocCore = JSON.parse(readFileSync(CORE_JSON, "utf-8"));

if (rdocCore.format_version !== EXPECTED_FORMAT_VERSION) {
	process.stderr.write(
		`[extract-rust-api] FATAL: core crate rustdoc JSON format_version is ${rdocCore.format_version}, ` +
			`expected ${EXPECTED_FORMAT_VERSION}.\n`,
	);
	process.exit(1);
}

const idx = rdoc.index; // consumer crate index
const idxCore = rdocCore.index; // core crate index

// ---------------------------------------------------------------------------
// Step 3: Type-rendering helper
// ---------------------------------------------------------------------------

/**
 * Resolve the short name for a path ID using the rdoc.paths dict.
 * Falls back to the inline path string when paths has no entry or ID is null.
 *
 * @param {number|null} id
 * @param {string} inlinePath  - the path field already on the node
 * @param {object} rdocRef     - the rdoc object whose .paths dict to use
 * @returns {string}
 */
function resolvePathName(id, inlinePath, rdocRef) {
	if (id !== null && id !== undefined) {
		const pe = rdocRef.paths[id];
		if (pe?.path?.length) {
			// Use only the last segment for brevity (e.g. "Validate" not "garde::validate::Validate")
			return pe.path[pe.path.length - 1];
		}
	}
	// Fall back to the inline path string (may be empty for external trait refs)
	return inlinePath || "?";
}

/**
 * Render a rustdoc JSON "type" node to a human-readable Rust type string.
 * Best-effort: accurate for all types that appear on the prompting-press
 * public surface.
 *
 * @param {any} typeNode
 * @param {object} sourceIdx  - the index to use for ID resolution (usually idx)
 * @param {object} rdocRef    - the rdoc object (for .paths lookup); defaults to rdoc
 * @returns {string}
 */
function renderType(typeNode, sourceIdx = idx, rdocRef = rdoc) {
	if (!typeNode) return "_";

	if (typeNode.primitive !== undefined) {
		return typeNode.primitive; // "str", "bool", "usize", …
	}

	if (typeNode.generic !== undefined) {
		return typeNode.generic; // "Self", "V", …
	}

	if (typeNode.resolved_path !== undefined) {
		const rp = typeNode.resolved_path;
		// Resolve the name: prefer paths dict for external types, fall back to inline path.
		const name = resolvePathName(rp.id, rp.path, rdocRef);
		if (!rp.args) return name;
		const aa = rp.args.angle_bracketed;
		if (!aa || aa.args.length === 0) return name;
		const rendered = aa.args
			.map((a) => {
				if (a.type) return renderType(a.type, sourceIdx, rdocRef);
				// Lifetime strings in rustdoc JSON already include the leading '
				// (e.g. "'static", "'a") — do not prepend another one.
				if (a.lifetime) return a.lifetime;
				if (a.const) return a.const.expr ?? "_";
				return "_";
			})
			.join(", ");
		return `${name}<${rendered}>`;
	}

	if (typeNode.borrowed_ref !== undefined) {
		const br = typeNode.borrowed_ref;
		// Lifetime strings already include the leading ' (e.g. "'static", "'a").
		const lt = br.lifetime ? `${br.lifetime} ` : "";
		const mut_ = br.is_mutable ? "mut " : "";
		const inner = renderType(br.type, sourceIdx, rdocRef);
		if (lt === "'static " && inner === "str") return "&'static str";
		return `&${lt}${mut_}${inner}`;
	}

	if (typeNode.qualified_path !== undefined) {
		const qp = typeNode.qualified_path;
		const selfTy = renderType(qp.self_type, sourceIdx, rdocRef);
		// NOTE: the JSON key is literally "trait" (a JS reserved word) — use bracket access.
		// Resolve trait name from paths when the inline path is empty (external trait refs).
		const traitRef = qp["trait"];
		const traitName = resolvePathName(traitRef?.id, traitRef?.path ?? "", rdocRef);
		return `<${selfTy} as ${traitName}>::${qp.name}`;
	}

	if (typeNode.tuple !== undefined) {
		if (typeNode.tuple.length === 0) return "()";
		return `(${typeNode.tuple.map((t) => renderType(t, sourceIdx, rdocRef)).join(", ")})`;
	}

	if (typeNode.slice !== undefined) {
		return `[${renderType(typeNode.slice, sourceIdx, rdocRef)}]`;
	}

	if (typeNode.array !== undefined) {
		return `[${renderType(typeNode.array.type, sourceIdx, rdocRef)}; ${typeNode.array.len}]`;
	}

	if (typeNode.raw_pointer !== undefined) {
		const rp_ = typeNode.raw_pointer;
		return `*${rp_.is_mutable ? "mut" : "const"} ${renderType(rp_.type, sourceIdx, rdocRef)}`;
	}

	if (typeNode.dyn_trait !== undefined) {
		const traits = typeNode.dyn_trait.traits
			.map((tb) => resolvePathName(tb.trait?.id, tb.trait?.path ?? "", rdocRef))
			.join(" + ");
		return `dyn ${traits}`;
	}

	return "_";
}

// ---------------------------------------------------------------------------
// Step 4: Signature renderers
// ---------------------------------------------------------------------------

/**
 * Render a self receiver from its type node into the idiomatic Rust form.
 *
 * In rustdoc JSON, `self`/`&self`/`&mut self` are encoded as a named param
 * `["self", <typeNode>]` where the typeNode is a borrowed_ref wrapping
 * `{generic: "Self"}`. We map these back to Rust receiver syntax.
 *
 * @param {any} typeNode
 * @returns {string}
 */
function renderSelfReceiver(typeNode) {
	if (!typeNode) return "self";
	// &self  → {borrowed_ref: {is_mutable: false, type: {generic: "Self"}}}
	// &mut self → {borrowed_ref: {is_mutable: true,  type: {generic: "Self"}}}
	if (typeNode.borrowed_ref !== undefined) {
		const br = typeNode.borrowed_ref;
		if (br.type?.generic === "Self") {
			return br.is_mutable ? "&mut self" : "&self";
		}
	}
	// bare `self` (consuming) → {generic: "Self"}
	if (typeNode.generic === "Self") return "self";
	return "self";
}

/**
 * Render a full function/method signature string.
 *
 * @param {string} name
 * @param {object} fnInner   - item.inner.function
 * @param {boolean} isMethod - when true it's an impl method (self receiver possible)
 * @param {object} sourceIdx - index to use for type resolution
 * @param {object} rdocRef   - rdoc object for .paths lookup
 * @returns {string}
 */
function renderFnSig(name, fnInner, isMethod = false, sourceIdx = idx, rdocRef = rdoc) {
	const sig = fnInner.sig;
	const generics = fnInner.generics ?? { params: [], where_predicates: [] };

	// Generic type params: <V, T, …> — skip lifetime-only params for brevity
	const typeParams = generics.params.filter(
		(p) => p.kind?.type !== undefined,
	);
	const genericStr =
		typeParams.length > 0
			? `<${typeParams.map((p) => p.name).join(", ")}>`
			: "";

	// Parameters — handle self receiver specially
	const params = sig.inputs.map(([pName, pType]) => {
		if (pName === "self") return renderSelfReceiver(pType);
		return `${pName}: ${renderType(pType, sourceIdx, rdocRef)}`;
	});

	// Return type
	const retStr = sig.output
		? ` -> ${renderType(sig.output, sourceIdx, rdocRef)}`
		: "";

	// Where clause
	const wherePreds = generics.where_predicates ?? [];
	let whereStr = "";
	if (wherePreds.length > 0) {
		const clauses = wherePreds.map((wp) => {
			const bp = wp.bound_predicate;
			const ty = renderType(bp.type, sourceIdx, rdocRef);
			const bounds = bp.bounds
				.map((b) => {
					if (b.trait_bound) {
						// NOTE: JSON key is "trait" (reserved word) — use bracket access.
						const tr = b.trait_bound["trait"];
						return resolvePathName(tr?.id, tr?.path ?? "", rdocRef);
					}
					// Lifetime strings already include the leading ' (e.g. "'a").
					if (b.lifetime) return b.lifetime;
					return "?";
				})
				.join(" + ");
			return `${ty}: ${bounds}`;
		});
		whereStr = `\nwhere\n    ${clauses.join(",\n    ")}`;
	}

	return `pub fn ${name}${genericStr}(${params.join(", ")})${retStr}${whereStr}`;
}

/**
 * Render a constant's signature.
 * e.g. `pub const VALIDATION: &str = "validation";`
 */
function renderConstSig(name, constInner, rdocRef = rdoc) {
	const typeStr = renderType(constInner.type, idx, rdocRef);
	const expr = constInner.const?.expr ?? "_";
	return `pub const ${name}: ${typeStr} = ${expr};`;
}

/**
 * Render a struct field's signature.
 * e.g. `pub field: String`
 */
function renderFieldSig(name, fieldTypeNode, sourceIdx = idx, rdocRef = rdoc) {
	const typeStr = renderType(fieldTypeNode, sourceIdx, rdocRef);
	return `pub ${name}: ${typeStr}`;
}

// ---------------------------------------------------------------------------
// Step 5: Doc processing
// ---------------------------------------------------------------------------

/**
 * Strip jargon, MDX-escape, return null when empty/absent.
 * FR-008: a public symbol with NO doc comment → null (never invent text).
 */
function processDoc(docs) {
	if (docs === null || docs === undefined) return null;
	const stripped = stripJargon(escapeCell(docs));
	return stripped.length > 0 ? stripped : null;
}

function deprecationNote(item) {
	return item.deprecation ? item.deprecation.note ?? "deprecated" : null;
}

// ---------------------------------------------------------------------------
// Step 6: Symbol builders
// ---------------------------------------------------------------------------

const KIND_ORDER = [
	"struct",
	"enum",
	"type",
	"const",
	"function",
	"method",
	"field",
	"variant",
	"interface",
	"class",
	"constructor",
	"accessor",
];

function kindRank(k) {
	const i = KIND_ORDER.indexOf(k);
	return i === -1 ? 99 : i;
}

/** Sort comparator: kind asc, then name asc (deterministic per contract R5). */
function sortSymbols(a, b) {
	const ko = kindRank(a.kind) - kindRank(b.kind);
	return ko !== 0 ? ko : a.name.localeCompare(b.name);
}

/**
 * Collect public methods from the first non-trait (own) impl block of a struct.
 *
 * @param {object} structInner - the inner.struct object
 * @param {object} sourceIdx   - index containing the struct's items
 * @param {object} rdocRef     - rdoc object for .paths lookup
 * @returns {object[]}         - array of IR Symbol (method kind)
 */
function collectStructMethods(structInner, sourceIdx = idx, rdocRef = rdoc) {
	const implIds = structInner?.impls ?? [];
	const methods = [];

	for (const implId of implIds) {
		const impl = sourceIdx[implId];
		if (!impl?.inner?.impl) continue;
		const implInner = impl.inner.impl;
		// Only own (non-trait) impls.
		// NOTE: JSON key is "trait" (a JS reserved word) — use bracket access.
		const implTrait = implInner["trait"];
		if (implTrait !== null && implTrait !== undefined) continue;

		for (const methodId of implInner.items ?? []) {
			const m = sourceIdx[methodId];
			if (!m || m.visibility !== "public") continue;
			if (Object.keys(m.inner ?? {})[0] !== "function") continue;

			const fnInner = m.inner.function;
			methods.push({
				name: m.name,
				kind: "method",
				signature: renderFnSig(m.name, fnInner, true, sourceIdx, rdocRef),
				doc: processDoc(m.docs),
				members: [],
				shapeRef: null,
				deprecated: deprecationNote(m),
			});
		}
		// Use the first own impl block only
		break;
	}

	return methods;
}

/**
 * Build a struct Symbol, optionally from a different index (e.g. core).
 *
 * @param {string} exportedName
 * @param {object} item
 * @param {object} sourceIdx
 * @param {object} rdocRef     - rdoc object for .paths lookup
 * @returns {object}
 */
function buildStructSymbol(exportedName, item, sourceIdx = idx, rdocRef = rdoc) {
	// Shape re-exports: shapeRef, no member expansion (FR-010)
	if (SHAPE_REF_TYPES.has(exportedName)) {
		return {
			name: exportedName,
			kind: "struct",
			signature: `pub struct ${exportedName}`,
			doc: processDoc(item.docs),
			members: [],
			shapeRef: exportedName,
			deprecated: deprecationNote(item),
		};
	}

	const structInner = item.inner.struct;

	// Public fields
	const fieldIds = structInner?.kind?.plain?.fields ?? [];
	const fieldMembers = fieldIds
		.map((fid) => {
			const f = sourceIdx[fid];
			if (!f || f.visibility !== "public") return null;
			const fType = f.inner?.struct_field;
			if (!fType) return null;
			return {
				name: f.name,
				kind: "field",
				signature: renderFieldSig(f.name, fType, sourceIdx, rdocRef),
				doc: processDoc(f.docs),
				members: [],
				shapeRef: null,
				deprecated: deprecationNote(f),
			};
		})
		.filter(Boolean);

	const methodMembers = collectStructMethods(structInner, sourceIdx, rdocRef);
	const members = [...fieldMembers, ...methodMembers].sort(sortSymbols);

	return {
		name: exportedName,
		kind: "struct",
		signature: `pub struct ${exportedName}`,
		doc: processDoc(item.docs),
		members,
		shapeRef: null,
		deprecated: deprecationNote(item),
	};
}

/**
 * Build an enum Symbol.
 *
 * @param {string} exportedName
 * @param {object} item
 * @param {object} sourceIdx
 * @param {object} rdocRef
 * @returns {object}
 */
function buildEnumSymbol(exportedName, item, sourceIdx = idx, rdocRef = rdoc) {
	const enumInner = item.inner.enum;
	const variantIds = enumInner?.variants ?? [];

	const variants = variantIds
		.map((vid) => {
			const v = sourceIdx[vid];
			if (!v) return null;
			const vKind = v.inner?.variant?.kind;

			// Render variant shape
			let sig = v.name;
			if (vKind?.struct?.fields?.length > 0) {
				const flds = vKind.struct.fields
					.map((fid) => {
						const f = sourceIdx[fid];
						if (!f) return null;
						return `${f.name}: ${renderType(f.inner?.struct_field, sourceIdx, rdocRef)}`;
					})
					.filter(Boolean)
					.join(", ");
				sig += ` { ${flds} }`;
			} else if (vKind?.tuple?.length > 0) {
				const types = vKind.tuple
					.map((t) => {
						if (t === null) return "_";
						if (typeof t === "number") {
							const tf = sourceIdx[t];
							return tf ? renderType(tf.inner?.struct_field, sourceIdx, rdocRef) : "_";
						}
						return "_";
					})
					.join(", ");
				sig += `(${types})`;
			}

			return {
				name: v.name,
				kind: "variant",
				signature: sig,
				doc: processDoc(v.docs),
				members: [],
				shapeRef: null,
				deprecated: deprecationNote(v),
			};
		})
		.filter(Boolean)
		.sort(sortSymbols);

	return {
		name: exportedName,
		kind: "enum",
		signature: `pub enum ${exportedName}`,
		doc: processDoc(item.docs),
		members: variants,
		shapeRef: null,
		deprecated: deprecationNote(item),
	};
}

// ---------------------------------------------------------------------------
// Step 7: Discover the public surface from lib.rs
// ---------------------------------------------------------------------------
// Walk the root module's items and collect:
//   - `use` items (re-exports): the name they are re-exported as + resolved id
//   - `function` items (pub free functions)
//   - `module` items listed in DOCUMENTED_MODULES (pub mod error, etc.)
//
// We do NOT descend into private modules for their pub items (IO-2 / FR-005).

const rootItem = idx[rdoc.root];
const rootModuleItems = rootItem?.inner?.module?.items ?? [];

/**
 * @type {Map<string, {id: number|null, source: string}>}
 * Maps exported name → {id (resolved item), source (use source path or kind)}
 */
const publicSurface = new Map();

for (const itemId of rootModuleItems) {
	const item = idx[itemId];
	if (!item || item.visibility !== "public") continue;
	const kind = Object.keys(item.inner ?? {})[0];

	if (kind === "use") {
		const use_ = item.inner.use;
		publicSurface.set(use_.name, { id: use_.id, source: use_.source });
	} else if (kind === "function") {
		publicSurface.set(item.name, { id: itemId, source: "function" });
	} else if (kind === "module") {
		// Include pub mod items we want to document (error, check, etc.)
		publicSurface.set(item.name, { id: itemId, source: "module" });
	}
}

// ---------------------------------------------------------------------------
// Step 8: Assign symbols to groups
// ---------------------------------------------------------------------------

/**
 * Canonical map from exported symbol name → group title.
 * Covers the prompting-press public surface + error::code constants.
 */
const SYMBOL_TO_GROUP = new Map([
	["Prompt", "Prompt"],
	["PromptOverlay", "Prompt"],
	["core_version", "Prompt"],

	["RenderResult", "RenderResult"],

	["GuardConfig", "GuardConfig"],

	["CheckReport", "CheckReport"],

	["Finding", "Finding"],
	["FindingKind", "Finding"],

	["Composition", "Composition"],
	["Message", "Message"],

	["ConsumerError", "Errors"],
	["FieldError", "Errors"],
	// error::code constants
	["VALIDATION", "Errors"],
	["UNKNOWN_VARIANT", "Errors"],
	["UNDEFINED_VARIABLE", "Errors"],
	["PARSE", "Errors"],
	["RENDER", "Errors"],
	["EXCLUDED_FEATURE", "Errors"],
	["LOAD", "Errors"],

	["PromptDefinition", "Shape types"],
	["PromptVariable", "Shape types"],
	["PromptVariant", "Shape types"],
]);

/** @type {Map<string, object[]>} group title → symbols array */
const groupSymbols = new Map(API_GROUPS.map((g) => [g.title, []]));

function addToGroup(name, sym) {
	const groupTitle = SYMBOL_TO_GROUP.get(name);
	if (!groupTitle) {
		process.stderr.write(
			`[extract-rust-api] WARN: no group mapping for "${name}" — skipping\n`,
		);
		return;
	}
	const bucket = groupSymbols.get(groupTitle);
	if (!bucket) {
		process.stderr.write(
			`[extract-rust-api] WARN: group "${groupTitle}" not in API_GROUPS\n`,
		);
		return;
	}
	bucket.push(sym);
}

// ---------------------------------------------------------------------------
// Step 9: Process each public surface entry
// ---------------------------------------------------------------------------

for (const [exportedName, info] of publicSurface) {
	// Skip module-level re-exports that are just namespaces (not individual symbols)
	// The `core` re-export is a whole-module alias; we pull GuardConfig from it manually.
	// The `prompt_definition` re-export is also a module alias.
	// The sub-modules error/prompt/check/compose are documented by expanding their
	// re-exported items (already in publicSurface via the `use` items at the root).
	if (
		["core", "prompt_definition", "prompt", "check", "compose"].includes(
			exportedName,
		)
	) {
		continue;
	}

	// ── error module: expand error::code constants ───────────────────────────
	if (exportedName === "error") {
		const errMod = idx[info.id];
		if (!errMod) continue;
		// Find the code sub-module
		const codeModId = (errMod.inner?.module?.items ?? []).find((mid) => {
			const m = idx[mid];
			return m?.name === "code" && Object.keys(m.inner ?? {})[0] === "module";
		});
		if (codeModId !== undefined) {
			const codeMod = idx[codeModId];
			for (const cid of codeMod?.inner?.module?.items ?? []) {
				const c = idx[cid];
				if (!c || c.visibility !== "public") continue;
				if (Object.keys(c.inner ?? {})[0] !== "constant") continue;
				const constInner = c.inner.constant;
				addToGroup(c.name, {
					name: c.name,
					kind: "const",
					signature: renderConstSig(c.name, constInner),
					doc: processDoc(c.docs),
					members: [],
					shapeRef: null,
					deprecated: deprecationNote(c),
				});
			}
		}
		continue;
	}

	const itemId = info.id;
	const item = idx[itemId];

	// ── External crate items (not in consumer index) ─────────────────────────
	if (!item) {
		// PromptDefinition / PromptVariable / PromptVariant — shape re-exports
		// Their items are in the core crate index; find them by name.
		if (SHAPE_REF_TYPES.has(exportedName)) {
			// Find the item in the core index by name + crate_id 0
			const coreItem = Object.values(idxCore).find(
				(it) => it.name === exportedName && it.crate_id === 0,
			);
			const sym = {
				name: exportedName,
				kind: "struct",
				signature: `pub struct ${exportedName}`,
				doc: coreItem ? processDoc(coreItem.docs) : null,
				members: [],
				shapeRef: exportedName,
				deprecated: coreItem ? deprecationNote(coreItem) : null,
			};
			addToGroup(exportedName, sym);
			continue;
		}

		// RenderResult — from core crate
		if (exportedName === "RenderResult") {
			const coreItem = Object.values(idxCore).find(
				(it) => it.name === "RenderResult" && it.crate_id === 0,
			);
			if (coreItem) {
				addToGroup(exportedName, buildStructSymbol(exportedName, coreItem, idxCore, rdocCore));
			} else {
				process.stderr.write(
					`[extract-rust-api] WARN: RenderResult not found in core index\n`,
				);
			}
			continue;
		}

		process.stderr.write(
			`[extract-rust-api] WARN: "${exportedName}" (id ${itemId}) not in consumer index and no fallback\n`,
		);
		continue;
	}

	// ── Items in the consumer index ──────────────────────────────────────────
	const rdocKind = Object.keys(item.inner ?? {})[0];

	if (rdocKind === "struct") {
		addToGroup(exportedName, buildStructSymbol(exportedName, item, idx));
		continue;
	}

	if (rdocKind === "enum") {
		addToGroup(exportedName, buildEnumSymbol(exportedName, item, idx));
		continue;
	}

	if (rdocKind === "function") {
		addToGroup(exportedName, {
			name: exportedName,
			kind: "function",
			signature: renderFnSig(exportedName, item.inner.function, false, idx),
			doc: processDoc(item.docs),
			members: [],
			shapeRef: null,
			deprecated: deprecationNote(item),
		});
		continue;
	}

	process.stderr.write(
		`[extract-rust-api] WARN: "${exportedName}" has unhandled kind "${rdocKind}" — skipping\n`,
	);
}

// ── GuardConfig from core (accessible via prompting_press::core::GuardConfig) ──
// GuardConfig is not directly re-exported at the root as a named item, but it is
// a first-class type on the public surface (Prompt::render's third parameter) and
// the API_GROUPS defines a group for it. Find it in the core index.
if (!groupSymbols.get("GuardConfig") || groupSymbols.get("GuardConfig").length === 0) {
	const gcItem = Object.values(idxCore).find(
		(it) => it.name === "GuardConfig" && it.crate_id === 0,
	);
	if (gcItem && Object.keys(gcItem.inner ?? {})[0] === "struct") {
		const sym = buildStructSymbol("GuardConfig", gcItem, idxCore, rdocCore);
		addToGroup("GuardConfig", sym);
	} else {
		process.stderr.write(
			`[extract-rust-api] WARN: GuardConfig not found in core index\n`,
		);
	}
}

// ---------------------------------------------------------------------------
// Step 10: Sort symbols within each group
// ---------------------------------------------------------------------------

for (const [, symbols] of groupSymbols) {
	symbols.sort(sortSymbols);
}

// ---------------------------------------------------------------------------
// Step 11: Assemble the IR
// ---------------------------------------------------------------------------

const groups = API_GROUPS.map((g) => ({
	title: g.title,
	anchor: g.anchor,
	blurb: null, // group blurbs are maintainer-facing only; extractor emits null
	symbols: groupSymbols.get(g.title) ?? [],
}));

const ir = {
	language: "rust",
	package: "prompting-press",
	version,
	generatedFrom: `rustdoc-json format_version ${rdoc.format_version} (nightly ${NIGHTLY_CHANNEL})`,
	groups,
};

// ---------------------------------------------------------------------------
// Step 12: Emit
// ---------------------------------------------------------------------------

process.stdout.write(JSON.stringify(ir, null, 2) + "\n");

const totalSymbols = groups.reduce((n, g) => n + g.symbols.length, 0);
process.stderr.write(
	`[extract-rust-api] Done. Groups: ${groups.length}, total top-level symbols: ${totalSymbols}\n`,
);
