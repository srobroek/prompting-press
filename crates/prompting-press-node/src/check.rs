//! The Node agreement + provenance lint — `check(registry)` plus the [`CheckReport`] and
//! [`Finding`] napi types (spec 003 US3 / FR-016..020; constitution Principle IV / C-04/C-09; US3).
//!
//! ## Zero engine logic (C-01 / Principle I)
//!
//! [`check`] performs **no** analysis of its own. It marshals the binding [`Registry`] to the Rust
//! consumer's [`prompting_press::check`] — which owns the registry walk, the agreement
//! set-arithmetic, the provenance view, the reserved-`default` handling, and the analysis-error
//! recording — and then surfaces the resulting [`CheckReport`] 1:1 to JS. The headline
//! differentiator is therefore the *same* lint across Rust/Python/TS by construction: the binding
//! re-derives nothing, so there is nothing to drift.
//!
//! ## Purity (FR-019)
//!
//! [`check`] takes `&Registry` (a shared borrow — mutation is impossible through the type system),
//! never renders, and has no side effects. Its only output is a [`CheckReport`] of [`Finding`]s.
//!
//! ## Determinism preserved across the boundary
//!
//! The consumer emits findings in a stable order (registry iterated by name; variants sorted;
//! roots/fields sorted — BTreeMap/BTreeSet backed). This binding copies that `Vec<Finding>` **in
//! order**, so the JS-visible order is identical — reproducible for a CI gate.
//!
//! ## The `kind` discriminant string (FR-020)
//!
//! JS matches on a [`Finding`]'s **`kind`**, exposed as a stable snake_case **discriminant string**
//! (not an opaque enum): `"undeclared_variable"`, `"untrusted_without_guard"`, `"analysis_error"`,
//! `"reserved_variant_name"`. The kind's inner datum (the undeclared `name`, the uncovered `field`,
//! the scrubbed `reason`) is already echoed in [`Finding::detail`], so `kind` stays a single stable
//! matchable value. The mapping is an **exhaustive** match over the consumer's [`FindingKind`] (no
//! wildcard arm): a new consumer variant becomes a compile error here, forcing the JS surface to be
//! updated deliberately rather than silently mapping to a stale string.

use napi_derive::napi;

use prompting_press::FindingKind;

use crate::registry::Registry;

/// The output of [`check`]: an ordered, read-only list of [`Finding`]s. Empty ⇒ the lint passed.
///
/// The Node mirror of the consumer's [`prompting_press::CheckReport`] (data-model §CheckReport;
/// FR-020). Surfaced **1:1** — the binding adds nothing and interprets nothing. A `#[napi]` class
/// with read-only accessors; a report is produced by [`check`], never constructed from JS.
#[napi]
pub struct CheckReport {
    findings: Vec<Finding>,
}

#[napi]
impl CheckReport {
    /// Every lint failure found, in the consumer's deterministic order. Empty ⇒ pass.
    #[napi(getter)]
    #[must_use]
    pub fn findings(&self) -> Vec<Finding> {
        self.findings.clone()
    }

    /// `report.passed()` — `true` iff there are no findings (the lint passed). Reads clearly at a
    /// CI gate (`if (!check(reg).passed()) process.exit(1)`).
    #[napi]
    #[must_use]
    pub fn passed(&self) -> bool {
        self.findings.is_empty()
    }

    /// `report.isEmpty()` — alias for [`passed`](Self::passed): `true` iff there are no findings.
    #[napi]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.findings.is_empty()
    }
}

impl From<prompting_press::CheckReport> for CheckReport {
    fn from(r: prompting_press::CheckReport) -> Self {
        Self {
            findings: r.findings.into_iter().map(Finding::from).collect(),
        }
    }
}

/// One actionable lint finding, read-only from JS (FR-020).
///
/// The Node mirror of the consumer's [`prompting_press::Finding`]. It names the `prompt`, the
/// `variant` where applicable (`null` for a prompt-level provenance finding), the failure `kind`
/// (a stable snake_case discriminant string — see the module docs), and a human-readable `detail`.
/// The `detail` carries no bound-value content (SEC-004 — it is built by the consumer from names
/// only). A `#[napi(object)]` so a `Finding` crosses as a plain JS object with all four fields
/// (the natural shape for the `findings` array the TS facade iterates / matches on).
#[derive(Clone)]
#[napi(object)]
pub struct Finding {
    /// The prompt's registry name.
    pub prompt: String,
    /// The variant the finding pertains to (`"default"` / `"<name>"` for an agreement, analysis, or
    /// reserved-name finding); `None` (`undefined` in JS) for a prompt-level provenance finding.
    pub variant: Option<String>,
    /// The failure kind as a stable snake_case **discriminant string** — the value JS matches on.
    /// One of `"undeclared_variable"`, `"untrusted_without_guard"`, `"analysis_error"`,
    /// `"reserved_variant_name"`. The kind's inner datum is echoed in [`detail`](Self::detail).
    pub kind: String,
    /// A human-readable, actionable description (FR-020). Carries no bound-value content.
    pub detail: String,
}

impl From<prompting_press::Finding> for Finding {
    fn from(f: prompting_press::Finding) -> Self {
        Self {
            prompt: f.prompt,
            variant: f.variant,
            kind: kind_discriminant(&f.kind).to_string(),
            detail: f.detail,
        }
    }
}

/// Map a consumer [`FindingKind`] to its stable snake_case **discriminant string**.
///
/// **Exhaustive on purpose** (no `_` wildcard): if the consumer adds a `FindingKind` variant, this
/// match fails to compile, forcing the JS-visible string set to be updated deliberately rather than
/// silently mapping a new variant to a stale value. The bound inner data (`name` / `field` /
/// `reason`) is intentionally *not* read here — it is already carried in [`Finding::detail`].
fn kind_discriminant(kind: &FindingKind) -> &'static str {
    match kind {
        FindingKind::UndeclaredVariable { .. } => "undeclared_variable",
        FindingKind::UntrustedWithoutGuard { .. } => "untrusted_without_guard",
        FindingKind::AnalysisError { .. } => "analysis_error",
        FindingKind::ReservedVariantName { .. } => "reserved_variant_name",
    }
}

/// Run the agreement + provenance lint over `reg` (FR-016..020) and surface the report to JS.
///
/// **Pure** (FR-019): never mutates the registry, never renders, no side effects. Marshals to the
/// Rust consumer's [`prompting_press::check`] (C-01 — the binding re-derives nothing) and returns
/// its [`CheckReport`] converted to the napi class, preserving the consumer's deterministic finding
/// order.
///
/// An empty registry yields an empty report (`report.passed() === true`).
#[napi]
#[must_use]
pub fn check(reg: &Registry) -> CheckReport {
    prompting_press::check(reg.inner()).into()
}

#[cfg(test)]
mod tests {
    //! Lint coverage drivable in Rust WITHOUT the TS facade.
    //!
    //! The binding only marshals to the consumer's `check`, whose per-class lint behavior (every
    //! `FindingKind`, the reserved-`default` handling, the provenance convention) is exhaustively
    //! tested in the consumer crate. Here we prove the *binding wiring*: a registry with a known
    //! defect surfaces a `Finding` with the expected discriminant string + prompt name, and a clean
    //! registry passes. The JS-driven proof (matching on `finding.kind` from TS) lives in the T018
    //! suite.

    use super::*;
    use prompting_press::PromptDefinition;

    /// Build a `PromptDefinition` from JSON (the idiomatic in-test construction — the generated
    /// newtypes validate, so a struct literal is awkward; mirrors the render-test helper).
    fn def_from_json(json: &str) -> PromptDefinition {
        serde_json::from_str(json).expect("valid prompt definition")
    }

    /// A prompt whose template references a variable it never declares surfaces one
    /// `undeclared_variable` finding naming that prompt — the headline agreement defect, marshaled
    /// through the binding and exposed with the stable discriminant string JS matches on.
    #[test]
    fn undeclared_variable_surfaces_with_discriminant_string() {
        // `body` references `{{ name }}` but declares no `variables` ⇒ undeclared.
        let def = def_from_json(r#"{ "name": "greet", "role": "user", "body": "Hi {{ name }}" }"#);
        let reg = Registry::from_defs_for_test([def]);

        let report = check(&reg);
        assert!(
            !report.passed(),
            "an undeclared template variable must fail the lint"
        );

        let undeclared: Vec<&Finding> = report
            .findings
            .iter()
            .filter(|f| f.kind == "undeclared_variable")
            .collect();
        assert_eq!(
            undeclared.len(),
            1,
            "exactly one undeclared-variable finding"
        );
        let f = undeclared[0];
        assert_eq!(f.prompt, "greet", "the finding names the offending prompt");
        assert_eq!(
            f.variant.as_deref(),
            Some("default"),
            "the agreement finding pins the default (root-body) arm"
        );
        assert!(
            f.detail.contains("name"),
            "the detail echoes the undeclared root, got {:?}",
            f.detail
        );
    }

    /// A clean registry — a prompt whose template references only declared variables, with no
    /// untrusted/external inputs — passes the lint: an empty report, `passed()` true, `isEmpty()`
    /// true.
    #[test]
    fn clean_registry_passes() {
        let def = def_from_json(
            r#"{
                "name": "greet",
                "role": "user",
                "body": "Hi {{ name }}",
                "variables": { "name": { "type": "string", "provenance": "trusted" } }
            }"#,
        );
        let reg = Registry::from_defs_for_test([def]);

        let report = check(&reg);
        assert!(report.passed(), "a fully-declared prompt passes the lint");
        assert!(report.is_empty(), "no findings ⇒ isEmpty()");
        assert!(report.findings.is_empty());
    }

    /// An empty registry passes (no prompts ⇒ no findings) — the degenerate CI-gate case.
    #[test]
    fn empty_registry_passes() {
        let reg = Registry::from_defs_for_test(std::iter::empty::<PromptDefinition>());
        let report = check(&reg);
        assert!(report.passed(), "an empty registry has nothing to lint");
        assert!(report.findings.is_empty());
    }
}
