//! Engine construction (spec 002, T011).
//!
//! This module owns the single, canonical [`minijinja::Environment`] the kernel
//! renders and analyses against. Centralizing construction here guarantees every
//! kernel operation (render, agreement analysis, source resolution) sees the same
//! engine configuration, which is what makes cross-language output a structural
//! property (constitution Principle I).
//!
//! Two configuration choices are load-bearing:
//!
//! 1. **Strict undefined behavior** (research D3, FR-001a). The environment is set to
//!    [`minijinja::UndefinedBehavior::Strict`], so using or printing an undefined
//!    variable raises a loud error instead of rendering a silent empty string. The
//!    `is defined` test still works under Strict (it is a presence check, not a value
//!    access), so intentionally-optional references remain expressible.
//!
//! 2. **Excluded template features are PARSE ERRORS, by crate feature, not by code**
//!    (research D1/D4, FR-002). The `minijinja` dependency is built with
//!    `default-features = false` and **without** the `macros` and `multi_template`
//!    features (see the crate / workspace `Cargo.toml`). With those engine features
//!    off, the tags `{% include %}`, `{% import %}`, `{% from … import %}`,
//!    `{% extends %}`, `{% macro %}`, and `{% block %}` are **unrecognised** — adding
//!    such a template fails at parse time. That is why the kernel needs **no loader,
//!    no AST walk, and no unstable API** to enforce the v1 excluded-feature boundary:
//!    the feature subset enforces it structurally at compile/parse time. Do NOT
//!    re-enable `macros` / `multi_template`; doing so would silently reopen the
//!    excluded features and break FR-002.

/// Build the kernel's canonical MiniJinja environment.
///
/// Configured with [`minijinja::UndefinedBehavior::Strict`] (FR-001a). The excluded
/// template features are enforced by the disabled `macros` / `multi_template` crate
/// features (FR-002, see the module docs), so this builder adds no feature-rejection
/// logic of its own.
///
/// Returns an environment with the `'static` lifetime: it owns no borrowed template
/// source, so templates are added per-operation against borrowed definition bytes by
/// later tasks.
#[allow(dead_code)] // wired into render / analysis in later spec-002 tasks (T013+).
pub(crate) fn build_environment() -> minijinja::Environment<'static> {
    let mut env = minijinja::Environment::new();
    env.set_undefined_behavior(minijinja::UndefinedBehavior::Strict);
    env
}

#[cfg(test)]
mod tests {
    use super::build_environment;

    #[test]
    fn builds_with_strict_undefined() {
        let env = build_environment();
        assert_eq!(
            env.undefined_behavior(),
            minijinja::UndefinedBehavior::Strict,
            "kernel environment must be strict-undefined (FR-001a)",
        );
    }
}
