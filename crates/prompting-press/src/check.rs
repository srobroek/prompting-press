//! The agreement + provenance lint (spec 003, US3; T017–T019; FR-016..020).
//!
//! [`check`] is the library's **headline differentiator** (constitution Principle IV /
//! C-04/C-09): a single, pure pass over a [`Registry`] that catches — *before render, in CI*
//! — two classes of prompt bug:
//!
//! 1. **Agreement (FR-016/017).** A template that references a variable the prompt never
//!    declared. Under a lenient engine that renders as a silent empty string; here it is a
//!    reported [`FindingKind::UndeclaredVariable`].
//! 2. **Provenance (FR-018, reframed — see below).** A prompt that declares an
//!    `untrusted`/`external` input but configures **no guard** for it →
//!    [`FindingKind::UntrustedWithoutGuard`].
//!
//! The lint owns **only the comparison and the registry walk**. The hard parts — the sound
//! referenced-roots set and the provenance view — are computed *once* by the kernel
//! ([`prompting_press_core::required_roots`] / [`prompting_press_core::provenance_view`]); this
//! module never re-derives them (constitution Principle I / FR-017 / C-01).
//!
//! ## Purity (FR-019)
//!
//! [`check`] takes `&Registry` (a shared borrow — mutation is impossible through the type
//! system), never renders, and has no side effects. Its only output is the [`CheckReport`].
//!
//! ## Determinism
//!
//! Findings are emitted in a stable order: the registry iterates by name ([`BTreeMap`]); each
//! prompt's variants are visited in sorted order (default arm first, then named variants
//! sorted via a [`BTreeSet`]); within a variant, undeclared roots are already sorted (the
//! kernel returns a [`BTreeSet`]); provenance findings follow, fields sorted (the kernel's
//! [`ProvenanceView`](prompting_press_core::ProvenanceView) sets are sorted). So the report is
//! reproducible for a CI gate.
//!
//! ## The `meta.guard` convention (FR-018, reframed — analyze F1)
//!
//! The spec-002 kernel has **no in-template "guard position"** concept, so the literal
//! "untrusted field used outside a guard position" lint is not implementable against the
//! kernel surface. This crate therefore adopts a concrete, implementable interpretation of
//! "a guard is configured for this prompt":
//!
//! > **A prompt has a guard configured iff a `"guard"` key is present in its `meta` map OR
//! > its `metadata` map.**
//!
//! Both `meta` and `metadata` are library-**opaque** `serde_json::Map`s on the prompt
//! definition (the library never interprets their *contents*); this lint reads them
//! **read-only** and only checks for the *presence* of a top-level `"guard"` key — it does not
//! validate the guard's shape (that is the caller's concern, and a render-time
//! [`GuardConfig`](prompting_press_core::GuardConfig) is what actually drives guard expansion).
//! The rule:
//!
//! - `declared_untrusted = provenance_view(def).untrusted ∪ .external`.
//! - If `declared_untrusted` is non-empty **and** neither `meta` nor `metadata` carries a
//!   `"guard"` key → emit one [`FindingKind::UntrustedWithoutGuard`] per uncovered field.
//! - If a `"guard"` key **is** present → the obligation is satisfied; no provenance finding.
//!
//! This is the consumer's chosen, documented interpretation of C-09's "you declared untrusted
//! inputs and set up no guard for them," given the kernel surface available in v1.
//!
//! ## Handling a `required_roots` error (T018)
//!
//! [`prompting_press_core::required_roots`] can return `Err` — a malformed template, or one
//! using an excluded feature (`{% include %}` / macros / inheritance — these never parse under
//! the kernel's feature set). Rather than make `check()` fallible (it must stay a `-> CheckReport`
//! CI pass — F7), such an error is recorded as a finding with the distinct
//! [`FindingKind::AnalysisError`] kind, so a broken template surfaces loudly in the report
//! instead of being swallowed. This keeps `check()` infallible while still failing the gate on
//! an un-analyzable template.

use std::collections::BTreeSet;

use prompting_press_core::{provenance_view, required_roots, PromptDefinition};

use crate::Registry;

/// The reserved name of the default arm (the root `body`), mirroring the kernel's
/// variant-resolution convention (`None` ⇒ `"default"`).
const DEFAULT_VARIANT: &str = "default";

/// The opaque-metadata key whose *presence* (in `meta` or `metadata`) marks a prompt as
/// having a guard configured (the documented `meta.guard` convention — module docs).
const GUARD_KEY: &str = "guard";

/// One actionable lint finding (FR-020): it names the prompt, the variant where applicable,
/// the failure `kind`, and a human-readable `detail`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The prompt's registry name.
    pub prompt: String,
    /// The variant the finding pertains to (`Some("default")` / `Some("<name>")` for an
    /// agreement or analysis finding); `None` for a prompt-level provenance finding.
    pub variant: Option<String>,
    /// The kind of failure (the discriminant a consumer matches on).
    pub kind: FindingKind,
    /// A human-readable, actionable description (FR-020). Carries no bound-value content.
    pub detail: String,
}

/// The closed set of lint-failure classes.
///
/// `UndeclaredVariable` and `UntrustedWithoutGuard` are the two C-04/C-09 lint classes
/// (FR-016/018). `AnalysisError` is the third, distinct kind used when the kernel cannot
/// analyze a variant's template (see module docs, "Handling a `required_roots` error") — it
/// keeps [`check`] infallible while still failing the gate on an un-analyzable template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FindingKind {
    /// A template references `name`, but `name` is not in the prompt's declared `variables`
    /// (the agreement half — FR-016/017).
    UndeclaredVariable {
        /// The undeclared root variable name the template referenced.
        name: String,
    },
    /// The prompt declares `field` as `untrusted`/`external` but configures no guard for it
    /// (the reframed provenance half — FR-018; the `meta.guard` convention, module docs).
    UntrustedWithoutGuard {
        /// The uncovered untrusted/external field name.
        field: String,
    },
    /// The kernel could not analyze a variant's template (a parse failure or an excluded
    /// feature). Recorded as a finding so [`check`] stays infallible (F7) while still failing
    /// the gate. The `detail` carries a scrubbed description (no bound-value content).
    AnalysisError {
        /// A stable, scrubbed reason code (e.g. `"parse"`, `"excluded_feature"`,
        /// `"unknown_variant"`).
        reason: String,
    },
}

/// The output of [`check`]: an ordered list of [`Finding`]s. Empty ⇒ the lint passes.
///
/// Carries **only** findings — no rendered text, no mutated state (FR-019). The findings are
/// in a deterministic order (see module docs, "Determinism").
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CheckReport {
    /// Every lint failure found, in deterministic order. Empty ⇒ pass.
    pub findings: Vec<Finding>,
}

impl CheckReport {
    /// `true` iff there are no findings (the lint passed). Equivalent to
    /// `self.findings.is_empty()`; reads more clearly at a CI gate call site.
    #[must_use]
    pub fn passed(&self) -> bool {
        self.findings.is_empty()
    }

    /// Alias for [`passed`](Self::passed): `true` iff there are no findings.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.findings.is_empty()
    }
}

/// Run the agreement + provenance lint over `reg` (FR-016..020). **Pure**: never mutates,
/// never renders, no side effects (FR-019). Returns a [`CheckReport`]; empty ⇒ pass.
///
/// For each prompt (iterated by name — deterministic), in order:
///
/// 1. **Agreement** — for the default arm and each named variant (sorted), ask the kernel for
///    the variant's referenced roots ([`required_roots`]) and subtract the prompt's declared
///    `variables` keys; each leftover root is an [`FindingKind::UndeclaredVariable`]. A kernel
///    analysis `Err` becomes an [`FindingKind::AnalysisError`] finding (keeping `check`
///    infallible — F7).
/// 2. **Provenance** — ask the kernel for the prompt's untrusted/external fields
///    ([`provenance_view`]); if any are declared and no `"guard"` key is present in `meta` or
///    `metadata` (the `meta.guard` convention — module docs), each uncovered field is an
///    [`FindingKind::UntrustedWithoutGuard`].
///
/// An empty registry yields an empty report (pass — F7).
#[must_use]
pub fn check(reg: &Registry) -> CheckReport {
    let mut findings = Vec::new();

    for (name, def) in reg.iter() {
        check_agreement(name, def, &mut findings);
        check_provenance(name, def, &mut findings);
    }

    CheckReport { findings }
}

/// The agreement half for one prompt (FR-016/017): for each variant, subtract the declared
/// `variables` from the kernel's referenced roots and emit a finding per leftover.
///
/// The consumer owns ONLY the subtraction; the roots come from the kernel (FR-017 / C-01).
fn check_agreement(name: &str, def: &PromptDefinition, findings: &mut Vec<Finding>) {
    // The authoritative declared set (clarify Q1): the definition's own `variables` keys.
    let declared: BTreeSet<&str> = def.variables.keys().map(String::as_str).collect();

    for variant in variants_to_check(def) {
        // `None` ⇒ the default arm (root body), matching the kernel's resolution rule.
        let variant_arg = if variant == DEFAULT_VARIANT {
            None
        } else {
            Some(variant.as_str())
        };

        match required_roots(def, variant_arg) {
            Ok(agreement) => {
                // Subtract declared from referenced; each leftover root is undeclared.
                // `required_roots` is a sorted `BTreeSet`, so leftovers are emitted sorted.
                for root in &agreement.required_roots {
                    if !declared.contains(root.as_str()) {
                        findings.push(Finding {
                            prompt: name.to_string(),
                            variant: Some(variant.clone()),
                            kind: FindingKind::UndeclaredVariable { name: root.clone() },
                            detail: format!(
                                "template references undeclared variable `{root}` \
                                 (variant `{variant}`); add it to the prompt's `variables`",
                            ),
                        });
                    }
                }
            }
            // A malformed / excluded-feature template can't be analyzed. Record it (keeping
            // `check` infallible — F7) rather than silently passing it. The reason is a
            // scrubbed code, never the kernel's raw detail (which may carry bound-value text).
            Err(err) => {
                findings.push(Finding {
                    prompt: name.to_string(),
                    variant: Some(variant.clone()),
                    kind: FindingKind::AnalysisError {
                        reason: analysis_error_reason(&err).to_string(),
                    },
                    detail: format!(
                        "template for variant `{variant}` could not be analyzed \
                         ({})",
                        analysis_error_reason(&err),
                    ),
                });
            }
        }
    }
}

/// The provenance half for one prompt (FR-018, reframed): if it declares any untrusted /
/// external field and carries no `"guard"` key in `meta`/`metadata`, flag each uncovered
/// field. Prompt-level (no variant) — the obligation is on the prompt, not a single arm.
fn check_provenance(name: &str, def: &PromptDefinition, findings: &mut Vec<Finding>) {
    let view = provenance_view(def);

    // The full obligation set: untrusted ∪ external (both sorted `BTreeSet`s). Chaining into
    // a fresh sorted set keeps the emit order deterministic and de-duplicated.
    let declared_untrusted: BTreeSet<&str> = view
        .untrusted
        .iter()
        .chain(view.external.iter())
        .map(String::as_str)
        .collect();

    if declared_untrusted.is_empty() {
        return; // No untrusted/external inputs → no guard obligation.
    }

    // A guard is "configured" iff a top-level `"guard"` key is present in either opaque map
    // (the documented `meta.guard` convention). Presence only — the contents are opaque.
    if has_guard_configured(def) {
        return;
    }

    for field in declared_untrusted {
        findings.push(Finding {
            prompt: name.to_string(),
            variant: None,
            kind: FindingKind::UntrustedWithoutGuard {
                field: field.to_string(),
            },
            detail: format!(
                "field `{field}` is declared untrusted/external but the prompt configures \
                 no guard (add a `guard` key under the prompt's `meta` or `metadata`)",
            ),
        });
    }
}

/// The set of variant identifiers to analyze for a prompt, in deterministic order: the
/// reserved [`DEFAULT_VARIANT`] (root body) first, then each named variant **sorted**
/// (`def.variants` is a `HashMap`, whose key order is non-deterministic — sorting via a
/// `BTreeSet` makes the report reproducible).
fn variants_to_check(def: &PromptDefinition) -> Vec<String> {
    let named: BTreeSet<&str> = def.variants.keys().map(String::as_str).collect();
    let mut out = Vec::with_capacity(named.len() + 1);
    out.push(DEFAULT_VARIANT.to_string());
    out.extend(named.into_iter().map(str::to_string));
    out
}

/// `true` iff a top-level `"guard"` key is present in the prompt's `meta` OR `metadata` map
/// (the `meta.guard` convention — module docs). Read-only; presence only (contents opaque).
fn has_guard_configured(def: &PromptDefinition) -> bool {
    def.meta.contains_key(GUARD_KEY) || def.metadata.contains_key(GUARD_KEY)
}

/// Map a kernel analysis error to a stable, **scrubbed** reason code for an
/// [`FindingKind::AnalysisError`]. Never copies the kernel's raw `detail` (which may carry
/// bound-value content — SEC-004 / FR-015); only the variant class is surfaced.
fn analysis_error_reason(err: &prompting_press_core::KernelError) -> &'static str {
    use prompting_press_core::KernelError;
    match err {
        KernelError::UnknownVariant { .. } => "unknown_variant",
        KernelError::UndefinedVariable { .. } => "undefined_variable",
        KernelError::Parse { .. } => "parse",
        KernelError::Render { .. } => "render",
        KernelError::ExcludedFeature { .. } => "excluded_feature",
    }
}
