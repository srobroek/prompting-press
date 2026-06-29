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
//! (not an opaque enum): `"untrusted_without_guard"`. The kind's inner datum (the uncovered
//! `field`) is already echoed in [`Finding::detail`], so `kind` stays a single stable matchable
//! value. The mapping is an **exhaustive** match over the consumer's [`FindingKind`] (no wildcard
//! arm): a new consumer variant becomes a compile error here, forcing the JS surface to be updated
//! deliberately rather than silently mapping to a stale string.

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

impl CheckReport {
    /// Crate-internal constructor used by sibling modules (e.g. `prompt::NapiPrompt::check_prompt`)
    /// that build a `CheckReport` from an already-converted `Vec<Finding>`. The `findings` field
    /// is private to prevent JS construction; this is the one safe crate-internal path.
    pub(crate) fn from_findings(findings: Vec<Finding>) -> Self {
        Self { findings }
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
    /// The variant the finding pertains to; `None` (`undefined` in JS) for a prompt-level
    /// provenance finding.
    pub variant: Option<String>,
    /// The failure kind as a stable snake_case **discriminant string** — the value JS matches on.
    /// Currently always `"untrusted_without_guard"`. The kind's inner datum is echoed in
    /// [`detail`](Self::detail).
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
/// silently mapping a new variant to a stale value. The bound inner data (`field`) is intentionally
/// *not* read here — it is already carried in [`Finding::detail`].
fn kind_discriminant(kind: &FindingKind) -> &'static str {
    match kind {
        FindingKind::UntrustedWithoutGuard { .. } => "untrusted_without_guard",
    }
}

/// Run the advisory origin/guard lint over each definition in `reg`.
///
/// **Not a `#[napi]` function** (SC-001 / T046): the registry-keyed `check(reg)` is gone from
/// the public JS surface. Kept as a plain Rust function for `#[cfg(test)]` coverage only.
///
/// The public check path is `NapiPrompt::check_prompt` (see `prompt.rs`).
#[must_use]
pub fn check(reg: &Registry) -> CheckReport {
    // Iterate in deterministic order; for each definition build a temporary Prompt handle
    // and run the advisory check. The only live finding for a constructed Prompt is
    // UntrustedWithoutGuard (construction enforces agreement/parse/reserved-name).
    let mut findings: Vec<Finding> = Vec::new();
    for (_, def) in reg.definitions() {
        // Safety: the definition was already validated when it was loaded into the registry
        // (via Prompt::from_yaml/from_json/from_json-via-insert), so this from_json call
        // on a re-serialized def is infallible in practice. On the impossible error, skip.
        let json = match serde_json::to_string(def) {
            Ok(j) => j,
            Err(_) => continue,
        };
        if let Ok(prompt) = prompting_press::Prompt::from_json(&json) {
            let report = prompt.check();
            findings.extend(report.findings.into_iter().map(Finding::from));
        }
    }
    CheckReport { findings }
}

#[cfg(test)]
mod tests {
    //! Lint coverage drivable in Rust WITHOUT the TS facade.
    //!
    //! Post-reshape, agreement / parse / reserved-name violations are enforced at construction
    //! (Prompt::new), so `check(reg)` surfaces only the advisory `UntrustedWithoutGuard` finding
    //! for prompts that declare untrusted/external variables without a configured guard. The
    //! registry-level `check` iterates entries and calls `Prompt::check()` per entry.

    use super::*;
    use prompting_press::PromptDefinition;

    fn def_from_json(json: &str) -> PromptDefinition {
        serde_json::from_str(json).expect("valid prompt definition")
    }

    /// A prompt declaring an `untrusted` variable without a guard configured surfaces one
    /// `untrusted_without_guard` finding — the only LIVE finding class for a constructed Prompt.
    #[test]
    fn untrusted_without_guard_surfaces_with_discriminant_string() {
        let def = def_from_json(
            r#"{
                "name": "ask",
                "role": "user",
                "body": "{{ topic }}",
                "variables": { "topic": { "type": "string", "origin": "untrusted" } }
            }"#,
        );
        let reg = Registry::from_defs_for_test([def]);

        let report = check(&reg);
        assert!(
            !report.passed(),
            "unguarded untrusted var must fail the lint"
        );

        let advisory: Vec<&Finding> = report
            .findings
            .iter()
            .filter(|f| f.kind == "untrusted_without_guard")
            .collect();
        assert_eq!(advisory.len(), 1, "exactly one advisory finding");
        let f = advisory[0];
        assert_eq!(f.prompt, "ask", "the finding names the offending prompt");
        assert!(
            f.detail.contains("topic"),
            "the detail echoes the untrusted field, got {:?}",
            f.detail
        );
    }

    /// A clean registry — a prompt with trusted variables and no advisory issues — passes.
    #[test]
    fn clean_registry_passes() {
        let def = def_from_json(
            r#"{
                "name": "greet",
                "role": "user",
                "body": "Hi {{ name }}",
                "variables": { "name": { "type": "string", "origin": "trusted" } }
            }"#,
        );
        let reg = Registry::from_defs_for_test([def]);

        let report = check(&reg);
        assert!(report.passed(), "a trusted-only prompt passes the lint");
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
