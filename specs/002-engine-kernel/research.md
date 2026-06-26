# Phase 0 Research — Spec 002 (Engine kernel)

All Technical-Context unknowns resolved. Facts verified against **MiniJinja 2.21.0** (the pinned
version) directly from primary sources (crates.io sparse index + the `2.21.0` git tag source), not
from a model snapshot — see D1's note on a corrected version claim.

---

## D1 — Template engine: MiniJinja, pinned `2.21`, minimal feature set

**Decision**: Depend on `minijinja` pinned to `2.21` with **`default-features = false`** and the
explicit feature set **`["builtins", "deserialization", "serde", "std_collections",
"adjacent_loop_items"]`** — deliberately **omitting `macros` and `multi_template`** (and leaving
`loader` off, as it is non-default).

> **Feature-subset audit (critique E1, 2026-06-26):** relative to 2.21.0's default set
> (`builtins, debug, deserialization, macros, multi_template, adjacent_loop_items, std_collections,
> serde`), this drops `macros`+`multi_template` **intentionally** (the FR-002 exclusion mechanism) and
> keeps `adjacent_loop_items` (verified in `loop_object.rs` to gate ONLY `loop.previtem`/`nextitem`/
> `changed()`; core `loop.index`/`first`/`last`/`length` are ungated). `adjacent_loop_items` is kept so
> "loops" (FR-001) means full Jinja loops with no surprising gaps. `debug` is left OFF deliberately: it
> only enriches error context and would register a `debug()` global; FR-020's env-derived allowlist
> covers globals regardless, so omitting it keeps the global surface minimal.

**Rationale**:
- Latest stable on crates.io is **2.21.0** (verified via the sparse index
  `https://index.crates.io/mi/ni/minijinja`: published series … 2.20.0, **2.21.0**). This confirms the
  roadmap's "verified against MiniJinja 2.21" and satisfies roadmap Q3's re-confirm-at-spec-time
  requirement. ⚠️ *Correction*: an automated research pass initially claimed latest was `2.10.2` and
  that 2.21 "did not exist"; that was a stale snapshot and is **wrong** — the version was
  re-verified directly against crates.io and the 2.21.0 source tag.
- In 2.21.0, `macros` and `multi_template` are **separate, default-on engine features**
  (`default = ["builtins", "debug", "deserialization", "macros", "multi_template",
  "adjacent_loop_items", "std_collections", "serde"]`). Turning them off makes `{% macro %}`,
  `{% include %}`, `{% extends %}`, `{% import %}`, `{% from … import %}`, and `{% block %}`
  **unrecognised tags → parse errors at `add_template` time**. This is *compile-time structural*
  enforcement of FR-002 — strictly stronger than relying on a render-time `TemplateNotFound`, and it
  needs no loader trick.
- `builtins` stays on: it provides the filters/tests and the constructs needed for v1's
  interpolation/conditionals/loops. `deserialization`/`serde`/`std_collections` support the `Value`
  input path (D5).
- MiniJinja is **pure Rust, zero FFI** — keeps the spec-001 `check-ffi` gate green (D7).

**Alternatives considered**:
- *Keep default features + register a no-op error loader* — only yields a render-time error for
  includes and does nothing for inline macros; weaker and later-failing than disabling the features.
- *Walk the AST via `unstable_machinery` to reject excluded nodes* — pulls in an explicitly unstable
  API the constitution forbids for v1 (C-04). Rejected.
- *A different engine (Tera, Askama, handlebars)* — MiniJinja is the resolved design choice
  (feature-scope §3, the engine BAML embeds) and the only one exposing the sound stable
  `undeclared_variables`. Not reopened.

---

## D2 — The sound agreement analysis: `undeclared_variables(nested=false)` + env-derived allowlist

**Decision**: Compute required roots with `Template::undeclared_variables(false)` and subtract an
allowlist built **dynamically from the kernel's own `Environment` globals** (plus Jinja literals
`true`/`false`/`none`, which are not reported anyway). Expose the result **per resolved variant**
(FR-016) as a `BTreeSet<String>` (sorted → deterministic output).

**Rationale**:
- Confirmed in 2.21.0 stable source (`minijinja/src/template.rs:425`): signature
  `pub fn undeclared_variables(&self, nested: bool) -> HashSet<String>`, **not** behind
  `unstable_machinery`. With `nested=false` it returns root names only.
- The doc's own example proves the soundness the constitution claims: for
  `"{% set x = foo %}{{ x }}{{ bar.baz }}"`, `undeclared_variables(false)` returns `["foo", "bar"]` —
  the `{% set %}` target `x` is excluded and only the root `bar` (not `bar.baz`) is reported. Loop
  variables and `with`/block locals are excluded by the same mechanism.
- The doc explicitly warns: "*this does not special case global variables … a template that uses
  `namespace()` will return `namespace`*." So globals **must** be subtracted. Because the kernel
  constructs its own `Environment` and registers exactly the globals it wants, the soundest,
  drift-proof allowlist is **the set of names the kernel itself registered as globals** (queryable on
  the env) rather than a hardcoded list. With `builtins` on, that is `range`, `dict`, `namespace`
  (and `debug` when the `debug` feature is on) — but deriving it from the env means the list can never
  drift from the actual engine config.
- Built-in **filters and tests are never reported** by `undeclared_variables` (they are syntactically
  distinct from variable lookups), so they need no allowlist entry.

**Soundness boundary note (implementation-critical)**: `undeclared_variables` returns an **empty set on
parse error** (`Err(_) => HashSet::new()` in source). The kernel MUST therefore treat parse success as
a precondition of analysis — a template that fails to parse must surface a parse error, never be
silently reported as "requires no variables." Practically: parse/add the template first (which, with
`macros`/`multi_template` off, also rejects excluded features), then analyse.

**Residual limits** (documented, benign, per feature-scope §4.2): dynamic subscripts (`obj[key]`) are
reported conservatively; flow-insensitivity can miss a use-before-`set` — both are false-negatives, not
false-positives, acceptable for v1.

---

## D3 — Strict undefined handling (FR-001a)

**Decision**: Configure the kernel's `Environment` with `UndefinedBehavior::Strict`.

**Rationale**: 2.21.0 exposes `Environment::set_undefined_behavior(UndefinedBehavior::Strict)`. Under
Strict, using/printing an undefined variable raises an `UndefinedError`, satisfying FR-001a's
"loud error, never silent empty". The `is defined` test (and `{% if x is defined %}`) continues to work
under Strict — it is a presence check, not a value access — so intentionally-optional references remain
expressible (the documented pattern behind FR-001a). A fixture will lock both behaviors (quickstart).

**Alternatives**: `Lenient` (stock Jinja empty-string — rejected by the clarify session) and
`Chainable` (partial backstop) — both weaker than the clarified decision.

---

## D4 — Excluded-feature rejection mechanism (FR-002, FR-028)

**Decision**: Rely on the **feature-flag exclusion from D1** as the primary mechanism: with `macros`
and `multi_template` disabled, `{% include/import/extends/macro/block %}` are unrecognised tags and
`Environment::add_template` returns a **parse `Error`**. The kernel maps that to a structured
`excluded-feature`/`parse` error variant. A regression fixture asserts each excluded construct fails at
add/parse time (quickstart).

**Rationale**: Compile-time structural rejection is the strongest, earliest guarantee and keeps the
agreement analysis sound (an excluded feature can never reach it). No loader, no AST walking, no
unstable API. *Caveat to verify during implementation*: confirm the parse error for a disabled-feature
tag is reliably distinguishable (by `ErrorKind`) so the kernel can label it precisely; if MiniJinja
reports a generic syntax error, the kernel surfaces it as a parse/`SyntaxError` variant (still loud and
correct, just less specific) — acceptable for FR-028.

---

## D5 — Kernel input "values" wire type

**Decision**: The kernel accepts values as a **`minijinja::Value`** (constructed by callers/bindings,
typically via `Value::from_serialize` over a serde-compatible map). The render API takes
`&PromptDefinition`, an optional `&str` variant name, the values `Value`, and a guard-config option.

**Rationale**: `Value` is MiniJinja's native context type; taking it directly avoids a second
conversion layer in the kernel and keeps the kernel validation-blind (it never inspects/validates the
values, just binds them). Bindings (specs 004/005) and the Rust consumer (003) already hold serde data
and can build a `Value` cheaply; the FFI conformance corpus (spec 007) will pin that marshaling. Using
`serde_json::Value` instead would force a json→minijinja conversion inside the kernel for no benefit and
would lose non-JSON-native distinctions the corpus cares about.

**Alternatives**: `serde_json::Value` (extra conversion, lossy for the corpus); a bespoke value enum
(reinvents `minijinja::Value`). Both rejected.

---

## D6 — Where the generated `PromptDefinition` shape lives (FR-027)

**Decision**: **Relocate** the generated Rust shape from the consumer crate into the **kernel crate**:
- Move `crates/prompting-press/src/generated/` → `crates/prompting-press-core/src/generated/`.
- Move the Rust codegen script → `crates/prompting-press-core/scripts/codegen.sh` and repoint its
  `OUT` + header.
- Move the moon `codegen` task to the `prompting-press-core` project (it owns its own generated source
  and freshness); the kernel `build` depends on its own `codegen`.
- Update the `schemas:codegen-check` gate: dep `prompting-press:codegen` → `prompting-press-core:codegen`
  and the input path to the kernel location.
- The consumer keeps a thin re-export: `pub use prompting_press_core::generated::prompt_definition::…`.

**Rationale**: FR-027 requires the *kernel* to consume the shape; the kernel must not depend on the
consumer (C-01/C-02 direction: kernel ← consumer ← bindings). Hosting the generated module in the
kernel is the only placement that satisfies both. Moving the task (not just the output) keeps moon
`outputs` **project-local** (a task writing into another project's dir is fragile in moon) and keeps the
kernel self-contained — it builds from the committed generated file with no cross-project task
dependency. Python/TS codegen are **independent** (verified — their scripts reference no Rust path), so
they are untouched.

**Alternatives**: *Keep the codegen task in the consumer, write output into the kernel dir* (the
blast-radius scan's "Option A") — lower file-churn but introduces a cross-project moon output write and
leaves the kernel's source dependent on a foreign project's task; rejected as more fragile. *A separate
`prompting-press-schema` crate* — premature; no second consumer needs it (C-08 scope discipline).

**Blast radius** (from the relocation scan, all verified): `crates/prompting-press/src/lib.rs`
(re-export edge), `crates/prompting-press/src/generated.rs` (delete), the moved script + its header,
`crates/prompting-press-core/src/lib.rs` (+`pub mod generated;`), `crates/prompting-press-core/moon.yml`
(+`codegen` task), `schemas/moon.yml` (`codegen-check` dep + input path), and READMEs/spec-doc path
mentions. `.github/workflows/ci.yml` needs no change (it drives moon tasks, not paths).

---

## D7 — FFI isolation holds (C-02 / SC-007)

**Decision**: No action beyond keeping the existing `check-ffi` gate; add `minijinja` + `sha2` and
confirm neither pulls `pyo3`/`napi`.

**Rationale**: `minijinja` 2.21 is pure Rust (its deps: `serde`, optional `memo-map`,
`percent-encoding`/`aho-corasick` only under non-default features we don't enable — no native linkage).
`sha2` (RustCrypto) is pure Rust. The `cargo tree -i pyo3` / `-i napi` gate stays green. The kernel
remains binding-agnostic and validation-blind.

---

## D8 — Hashing: `sha2`

**Decision**: Compute `template_hash` and `render_hash` as lowercase-hex `SHA256` over the UTF-8 bytes
of, respectively, the resolved variant source string and the rendered output string, using the `sha2`
crate (`Sha256`). No `vars_hash` (C-05).

**Rationale**: `sha2` is the standard pure-Rust SHA-256 (keeps D7 green) and is deterministic, so the
hashes are byte-identical across languages for free (Principle I). Hashing over the string (not
structured vars) is exactly C-05; it sidesteps the JCS/RFC-8785 canonicalization problem the design
already eliminated. Hex encoding is the conventional, stable provenance representation for traces.

---

## RESOLVED CONTRADICTION — variant default (FR-010 vs FR-011 + 001 schema)

**Not a research item — a spec internal inconsistency found during planning. Surfaced, confirmed by the
user, and ratified via `/speckit.refine.update` on 2026-06-26 (spec FR-010 + US1 scenario 4 + SC-004
amended). Recorded here for provenance.**

- **FR-011 + the 001 schema**: `body` is a **required** field and **is** the default arm (reserved name
  `default`, `is_default=true`). So *every* prompt — including a multi-variant one — always has a
  default: its root `body`.
- **FR-010** (carried from feature-scope §4.3): "a multi-variant prompt MUST declare an explicit
  default, else a no-variant render is a loud error." Given the schema, a multi-variant prompt's root
  `body` *is* that default — so the "no explicit default exists" condition is **structurally
  unreachable**. feature-scope §4.3 predates the resolved 001 schema (which made `body` required).

**Ratified resolution** (user-confirmed 2026-06-26): the root `body` is always the default arm for any
prompt, variant or not. `render(variant=None)` → root body; `render(variant="x")` → that arm or an
unknown-variant error. The only variant error is **unknown variant** (FR-009). FR-010's separate
"missing-default" loud-error path was vestigial and has been amended out of the spec (refine.update).
The plan's data model and contracts already reflect this.

---

## Resolved unknowns checklist

| Unknown (from Technical Context / spec Assumptions) | Resolved by |
|---|---|
| MiniJinja version pin + stable-API soundness (roadmap Q3) | D1, D2 |
| Globals/filters allowlist contents | D2 (env-derived) |
| Strict-undefined mechanism | D3 |
| Excluded-feature rejection mechanism | D1 + D4 |
| Kernel "values" wire type | D5 |
| Where the generated shape lives (FR-027 vs C-01/C-02) | D6 |
| New deps vs FFI gate (SC-007) | D7 |
| Hashing crate / encoding | D8 |
| Variant-default contradiction (FR-010 vs FR-011) | Surfaced; ratified 2026-06-26 via spec refine (root body = always default) |
