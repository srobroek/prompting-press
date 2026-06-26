# Phase 0 Research — Spec 003 (Rust consumer `prompting-press`)

All Technical-Context unknowns resolved. Versions verified against the **crates.io sparse index** and
the crates' **tagged source**, NOT a model snapshot — see the ⚠️ note in D1 (a research subagent
returned `tool_uses: 0` and fabricated version numbers; every fact below was independently
re-verified).

---

## D1 — Validation facade: `garde` pinned `0.23`

**Decision**: `garde = { version = "0.23", features = ["derive", "serde"] }` (custom validators are in
the core; `derive` gives `#[derive(Validate)]`, `serde` lets the same struct derive serde + interop).

**Rationale** (verified against the `v0.23.0` source tag, `garde/src/`):
- Latest stable on crates.io is **0.23.0** (sparse index `ga/rd/garde`: …0.22.1, **0.23.0**). The
  roadmap's "garde 0.23" is correct and current. MSRV ≥ 1.78 (well under our 1.95 pin).
- **`Validate` trait** (`validate.rs:12`): `type Context;` + `fn validate_with(&self, ctx: &Self::Context) -> Result<(), Report>`, with a `validate()` convenience when `Context: Default`. One `validate()` call validates the whole struct (FR-002).
- **Custom validators**: `#[garde(custom(func))]` where `func: fn(&T, &Context) -> garde::Result`
  (`garde::Result = Result<(), garde::Error>`). Stable since 0.18.
- **Context**: `#[garde(context(Ctx))]` + `validate_with(&ctx)` threads external data to validators.
- **`serde` feature**: a struct can derive `serde::{Serialize, Deserialize}` AND `garde::Validate`
  independently — deserialize then validate are two steps (correct Rust idiom, feature-scope §4.6).
- **Pure-Rust / FFI**: the only non-pure dep is `js-sys`, which is **optional + behind the `js-sys`
  feature** (not default). With `["derive","serde"]` the tree pulls no `pyo3`/`napi`/native linkage →
  the `check-ffi` gate stays green (verified `garde/Cargo.toml` features block).

> ⚠️ **Correction**: an automated research pass claimed garde latest was `0.22.0` and minijinja
> `2.10.2` with a `Report::flatten()` method — ALL fabricated (`tool_uses: 0`). Re-verified: garde is
> **0.23.0**, minijinja is **2.21.0** (the pin spec-002 shipped), and `Report` has **no `flatten()`**
> (see D3).

## D2 — YAML parser: `serde_yaml_ng`

**Decision**: `serde_yaml_ng = "0.10"` for the YAML arm of the dual-input loader. JSON uses the
existing `serde_json`.

**Rationale**:
- `serde_yaml` (dtolnay) is **archived** (sparse index shows the last release literally tagged
  `0.9.34+deprecated`). Must not use.
- `serde_yaml_ng` latest is **0.10.0** (sparse index `se/rd/serde_yaml_ng`), an actively maintained,
  drop-in `serde_yaml`-API fork backed by `yaml-rust2` (a pure-Rust YAML-1.2 parser). YAML 1.2 means
  `no`/`yes`/`on`/`off` are strings, not booleans — the "Norway problem" is resolved, which matters
  for prompt YAML that may contain such tokens. Deserializes into existing `#[derive(Deserialize)]`
  types (the kernel's `PromptDefinition`) with zero type changes.
- **Alternative considered**: `serde_norway` (0.9.42, also maintained, also Norway-aware). Either
  works; `serde_yaml_ng` is chosen for the cleaner drop-in `serde_yaml` API surface and the higher
  (0.10) line. *Confirm the exact current version + pure-Rust dep tree at implementation time* (pin
  exactly, keep the FFI + floating-version gates green).

## D3 — Error normalization: `garde::Report` → `[{field, code, message}]`

**Decision**: Map via **`Report::iter() -> impl Iterator<Item = &(Path, Error)>`** (verified
`error.rs:40`). For each `(path, error)`: `field = path.to_string()` (`Path: Display` → dot-path),
`message = error.message()` (`Error::message() -> &str`, `error.rs:83`), and **`code` is synthesized**
by the consumer (garde exposes no machine code — `Error` carries only a message). Use a stable code
like `"validation"` for garde-sourced rows; map each `KernelError` variant to its own code.

**Rationale**: `Report` in 0.23.0 exposes `iter()` / `into_inner() -> Vec<(Path, Error)>` / `is_empty()`
— **there is NO `flatten()`** (the research agent invented it). `iter()` over `(Path, Error)` is the
real normalization path. The normalized shape is the consumer's own `Vec<FieldError>`; garde `Report`
and kernel `KernelError` never leak past the public boundary (C-06, FR-014). **SEC-004 (FR-015)**: the
kernel's `Parse`/`Render` `KernelError` detail may carry bound-value content — the normalizer maps it
to a sanitized message, never copying raw detail into logs.

## D4 — Vars → kernel value bridge: `minijinja::Value::from_serialize`

**Decision**: After garde validation passes, bridge the validated struct to the kernel's value type
with **`minijinja::Value::from_serialize(&vars)`** (clarify Q4).

**Rationale**: Verified in minijinja `2.21.0` source (`value/mod.rs:856`):
`pub fn from_serialize<T: Serialize>(value: T) -> Value`. The kernel's `render` takes a
`minijinja::Value`; the validated Vars struct (which derives `Serialize`) converts in one call. The
consumer already (transitively) has minijinja via the kernel; it references `minijinja::Value` at the
render boundary only. No manual map-building by the caller (FR-003a).

## D5 — Agreement + provenance lint: pure set ops over kernel output

**Decision**: `check(registry)` iterates each prompt + each variant, calls the kernel's
`required_roots(def, variant)` and `provenance_view(def)`, and does the comparisons in the consumer:
- **Agreement (FR-016/017)**: `required_roots.required_roots` (a `BTreeSet<String>`, kernel) **minus**
  `def.variables.keys()` (the declared set — clarify Q1) → any leftover root is an
  `UndeclaredVariable` finding.
- **Provenance (FR-018)**: `provenance_view(def).untrusted ∪ .external` checked against the declared
  guard positions → a tagged field used outside a guard position is an `UntrustedOutsideGuard` finding.

**Rationale**: The kernel already computes the hard part (the sound referenced-roots set, the
provenance view) — the consumer owns only the set comparison and the registry walk. Pure analysis,
pass/fail, no mutation, no render (FR-019). `BTreeSet` difference is deterministic. The authoritative
declared set is `def.variables` (the spec-001 shape), so `check()` needs no introspection of the
user's garde struct — it runs on pure data and is CI-portable.

## D6 — Registry, render, composition, token hook (API shapes)

**Decision**:
- **Registry** (FR-008a): `Registry { prompts: BTreeMap<String, PromptDefinition> }` with
  `load_yaml(&str)`/`load_json(&str)`/`insert(def)` populators and name lookup; absent name →
  `ConsumerError::UnknownPrompt`. BTreeMap → deterministic `check()` ordering.
- **render** (FR-009): `render<V: Serialize + Validate>(&self, name: &str, vars: &V, variant: Option<&str>, guard: &GuardConfig) -> Result<RenderResult, ConsumerError>` — validate `vars`, `Value::from_serialize`, look up the def, delegate to kernel `render`, normalize errors. Caller passes prompt-name + vars together (clarify Q3); no per-prompt type registration.
- **Composition** (FR-012): a `Vec`-backed builder with `append_*` methods over `(name, vars)` entries,
  `resolve() -> Result<Vec<Message>, ConsumerError>` where `Message { role, text }`. No `.chain()`.
- **Token hook** (FR-021/022): `count_tokens` as a `Fn(&str, &str) -> usize` boxed closure / trait
  object, optional on the registry or render call; absent ⇒ no counting (not an error). No built-in
  counter ships.

**Rationale**: These are the language-native ergonomics the kernel deliberately omits; each is a thin
wrapper + the validation/loader/normalization layer. Exact generic-vs-trait-object choices for the
hook are an implementation detail; the seam shape is fixed by C-03 (hook only) and C-06 (Vec, not
`.chain()`).

## D7 — Crate structure & FFI isolation (C-01/C-02)

**Decision**: Build out `crates/prompting-press/` (the existing stub) — no new crate, no relocation.
Add modules: `registry`, `render` (the wrappers), `check` (the lint), `error` (normalization +
`ConsumerError`), `compose`, `tokens` (the hook seam). Deps added: `garde` (`derive`+`serde`),
`serde_yaml_ng`; `serde`/`serde_json` already present; `minijinja` referenced transitively via the
kernel at the render boundary (or added explicitly for `Value`). Fix the stale `Cargo.toml` comment
(it still says the generated shape lives in `src/generated/` — it's in the kernel since 002).

**Rationale**: The kernel/consumer split is already in place; 003 fills the consumer with logic that
wraps the kernel (C-01 — no rendering/agreement/hashing duplicated). All added deps are pure-Rust →
`check-ffi` stays green (SC-007 / C-02). The consumer does no I/O — it accepts already-read YAML/JSON
*text* or a constructed object; the caller reads files (C-03).

## Resolved unknowns checklist

| Unknown | Resolved by |
|---|---|
| garde current version + validate/custom/Report API (verify-at-spec-time) | D1, D3 |
| YAML parser choice (serde_yaml archived) | D2 |
| `Report` → `[{field,code,message}]` mapping (no `flatten()` in 0.23) | D3 |
| Vars → kernel value bridge | D4 |
| `⊆` comparison mechanics + declared-set authority | D5 |
| Registry / render / composition / token-hook shapes | D6 |
| Crate structure + FFI isolation | D7 |
