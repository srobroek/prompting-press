//! Sound agreement analysis (spec 002, US2; T023–T025) — the library's headline
//! differentiator (constitution Principle IV / C-04).
//!
//! [`required_roots`] reports, **per resolved variant**, the set of **root** variable names
//! a template references — the "required roots". A consumer (spec 003) compares this set
//! against its declared, typed Vars-model fields to catch, *before* render, the class of bug
//! where a template references a variable the caller never declared (which would otherwise
//! render to a silent empty string under a lenient engine — here it is a reported
//! requirement). The kernel only RETURNS the set; it does **not** perform the
//! `referenced ⊆ declared` comparison (FR-019 — that ⊆ check is the consumer's lint).
//!
//! ## Why this is *sound* (research D2)
//!
//! The set is computed with MiniJinja's **stable**
//! [`minijinja::Template::undeclared_variables`] in its **non-nested** mode
//! (`undeclared_variables(false)`), verified against MiniJinja 2.21.0 source
//! (`template.rs:425`; **not** behind `unstable_machinery`). In non-nested mode it returns
//! ROOT names only (`user`, never `user.name`). The engine analysis itself EXCLUDES names
//! locally bound inside the template — loop variables, `{% set %}` targets, and
//! `with`/block locals — which is the soundness property (strictly better than a naive
//! undeclared-variable scan, e.g. `jinja2.meta`). The kernel does **not** re-filter those
//! itself; the engine guarantees it.
//!
//! ## The two things the kernel MUST add on top
//!
//! 1. **Subtract the engine globals (FR-017, FR-020).** `undeclared_variables` does NOT
//!    special-case globals: its own doc warns that a template using `namespace()` reports
//!    `namespace`. So engine-provided globals (`range`, `dict`, `namespace`, …) must be
//!    subtracted. The allowlist is derived **dynamically from the kernel's own
//!    [`minijinja::Environment`] globals** via the stable [`minijinja::Environment::globals`]
//!    accessor — NOT a hardcoded list — so it can never drift from the actual engine config
//!    (FR-020). Built-in **filters and tests are never reported** by `undeclared_variables`
//!    (they are syntactically distinct from variable lookups), so they need no allowlist
//!    entry.
//!
//! 2. **Short-circuit the parse-error footgun (FR-016a).** `undeclared_variables` returns an
//!    **empty set on parse failure** (`Err(_) => HashSet::new()` in `template.rs:434`). A
//!    broken or excluded-feature template would therefore masquerade as "requires no
//!    variables" and silently pass the headline guarantee. The kernel forecloses this by
//!    PARSING THE TEMPLATE FIRST (via `add_template_owned`, which — with `macros` /
//!    `multi_template` disabled — also rejects excluded features), returning a parse /
//!    excluded-feature `Err` *before* it could ever observe an empty set. A non-parseable
//!    source never yields a `Template` handle, so the empty-set branch is structurally
//!    unreachable from this code path.

use std::collections::BTreeSet;

use crate::engine::{build_environment, resolve_variant};
use crate::error::KernelError;
use crate::generated::prompt_definition::PromptDefinition;

/// The per-variant agreement-analysis output (data-model §Agreement; FR-016..FR-019).
///
/// `required_roots` is a [`BTreeSet`] so the output is **sorted and deterministic**
/// (roadmap decision C-01 / structural parity): the same template always yields the same
/// ordered set, byte-identically across languages once the binding crates surface it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Agreement {
    /// Which arm was analysed — the reserved `"default"` (root body) or the named variant.
    pub variant: String,
    /// The root variable names the template references, minus names locally bound inside
    /// the template (loop vars, `{% set %}` targets, block/with locals — excluded by the
    /// engine analysis) and minus the env-derived globals allowlist (subtracted here).
    pub required_roots: BTreeSet<String>,
}

/// Report the **per-variant** set of required root variable names for one variant's source
/// (spec 002, T023–T025; FR-016, FR-016a, FR-017, FR-018, FR-019, FR-020).
///
/// Resolves the variant (reusing `resolve_variant`; same `None`/`"default"` → root body
/// and unknown-variant rule as [`crate::render`]), parses the resolved source against the
/// kernel's canonical [`minijinja::Environment`], and returns the engine's
/// `undeclared_variables(false)` result minus the env-derived globals allowlist.
///
/// **Pure (FR-018):** takes `&PromptDefinition` (a shared borrow — it cannot mutate the def
/// through the type system) and never takes `values`; it does **not** render and has no side
/// effects. **Does not** compare against declared `variables` (FR-019 — the consumer's job).
///
/// # Errors
/// - [`KernelError::UnknownVariant`] — `variant` names a non-existent arm (FR-009).
/// - [`KernelError::ExcludedFeature`] / [`KernelError::Parse`] — the resolved source uses an
///   excluded feature (`{% include %}` / `{% import %}` / `{% extends %}` / `{% macro %}` /
///   `{% block %}` — unrecognised under the disabled `macros`/`multi_template` features) or
///   otherwise fails to parse. This is the FR-016a short-circuit: the analysis errors here,
///   **never** returning an empty/successful [`Agreement`] for a non-parseable template
///   (research D2). The error mapping is shared with the render path (`engine::map_*`),
///   so excluded-feature labelling is consistent across operations.
pub fn required_roots(
    def: &PromptDefinition,
    variant: Option<&str>,
) -> Result<Agreement, KernelError> {
    let resolved = resolve_variant(def, variant)?;

    // T025 (FR-016a): parse FIRST. With `macros`/`multi_template` disabled, an
    // excluded-feature tag fails right here at parse time (research D1/D4), and an ordinary
    // syntax error fails too. Either way we return `Err` BEFORE we could ever observe
    // `undeclared_variables`' empty-set-on-parse-error branch — a broken/excluded template
    // can never masquerade as "requires no variables".
    let mut env = build_environment();
    env.add_template_owned("kernel".to_string(), resolved.source.to_string())
        .map_err(crate::engine::map_minijinja_error)?;
    let template = env
        .get_template("kernel")
        .map_err(crate::engine::map_minijinja_error)?;

    // Stable analysis, non-nested → ROOT names only. Loop locals, `{% set %}` targets, and
    // block/with locals are already excluded by the engine (the soundness property); we do
    // NOT re-filter them.
    let undeclared = template.undeclared_variables(false);

    // T024 (FR-017, FR-020): subtract the env-derived globals allowlist. Built-in
    // filters/tests are never reported by `undeclared_variables`, so they need no entry.
    let globals = env_globals(&env);
    let required_roots: BTreeSet<String> = undeclared
        .into_iter()
        .filter(|name| !globals.contains(name.as_str()))
        .collect();

    Ok(Agreement {
        variant: resolved.name,
        required_roots,
    })
}

/// Build the globals allowlist **dynamically from the environment's own registered globals**
/// (FR-020), via the stable [`minijinja::Environment::globals`] accessor (verified in
/// MiniJinja 2.21.0 `environment.rs:786`: `pub fn globals(&self) -> impl Iterator<Item =
/// (&str, Value)>`). Deriving it from the live env — rather than hardcoding `["range",
/// "dict", "namespace", …]` — means the allowlist can **never drift** from the actual engine
/// configuration: re-enabling a feature that registers a new global, or registering one via
/// `add_global`, is reflected automatically.
///
/// Under the kernel's feature set (`builtins` on) this yields exactly the names MiniJinja's
/// `builtins` registers — `range`, `dict`, `debug`, `namespace` (verified in
/// `defaults.rs::build_globals`, all four inside the `#[cfg(feature = "builtins")]` block).
fn env_globals(env: &minijinja::Environment<'_>) -> BTreeSet<String> {
    env.globals()
        .map(|(name, _value)| name.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::env_globals;
    use crate::engine::build_environment;

    /// FR-020: the allowlist is derived from the env's own globals — it includes the
    /// `builtins` globals so they are subtractable, and is non-empty (proving the accessor
    /// returns real entries rather than silently yielding nothing, which would let globals
    /// leak into the required-roots set).
    #[test]
    fn env_globals_include_builtin_globals() {
        let env = build_environment();
        let globals = env_globals(&env);

        assert!(
            !globals.is_empty(),
            "the env-derived allowlist must be non-empty (else globals leak into roots)",
        );
        // The `builtins`-registered globals must be present so they get subtracted.
        for expected in ["range", "dict", "namespace"] {
            assert!(
                globals.contains(expected),
                "env globals must include `{expected}` (a builtins global), got {globals:?}",
            );
        }
    }
}
