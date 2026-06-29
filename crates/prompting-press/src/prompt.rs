//! The immutable [`Prompt`] value object and its [`PromptOverlay`] (spec 008, T026–T030).
//!
//! [`Prompt`] is the library's primary public type: a validated, immutable facade over a
//! [`PromptDefinition`]. Construction (`new`, `from_yaml`, `from_json`, `from_toml`) is
//! **validating** — a `Prompt` that constructs is guaranteed to be:
//!
//! 1. **Shape-valid** — the document parsed to a `PromptDefinition` (serde layer).
//! 2. **Template-parseable and analyzable** — every variant body (including the root body)
//!    is parseable by the kernel and free of excluded features; construction FAILS on an
//!    un-analyzable body (R7/Q4).
//! 3. **Agreement-sound** — every variable a variant template references is declared in
//!    `variables`; a referenced-but-undeclared variable is a construction failure (FR-020 /
//!    Principle IV). The agreement check therefore moves ONTO construction; a constructed
//!    `Prompt` carries no undeclared-variable agreements.
//! 4. **Reserved-name clean** — no variant is literally named `"default"` (the kernel's
//!    reserved root-body alias); that is a construction failure (CR-1).
//!
//! After construction every operation is infallible with respect to the above invariants;
//! `check()` is a pure advisory pass that can only surface the origin/guard finding (a prompt
//! with `untrusted`/`external` vars and no guard configured).
//!
//! ## `with` — the sole mutator (R6)
//!
//! [`Prompt::derive`] shallow-replaces top-level fields via a [`PromptOverlay`] and routes the
//! merged definition through `Prompt::new` (full re-validation). The original `Prompt` is
//! untouched. In Rust the validator is generic `V` named at the `render` / `with` call site
//! (compile-time coverage); `PromptOverlay` therefore carries only data fields — no runtime
//! validator object (the Rust asymmetry from constitution Principle VI v1.2.0).
//!
//! ## No I/O (Principle III / C-03)
//!
//! The text-factory methods accept already-read text — the caller hands it in. This crate
//! reads no files.

use std::collections::HashMap;

use garde::Validate;
use prompting_press_core::{
    origin_view, required_roots, GuardConfig, KernelError, OriginView, RenderResult,
};
use serde::Serialize;

use crate::check::{has_guard_configured, CheckReport, Finding, FindingKind};
use crate::error::code;
use crate::{ConsumerError, FieldError};
use prompting_press_core::generated::prompt_definition::{
    PromptDefinition, PromptDefinitionName, PromptDefinitionRole, PromptVariable, PromptVariant,
};

// ─── constants ───────────────────────────────────────────────────────────────

/// The kernel's reserved variant name for the root body (mirrors `check::DEFAULT_VARIANT`).
/// Re-declared here so `prompt.rs` has no public dep on `check`'s internal constants.
const DEFAULT: &str = "default";

// ─── Prompt ──────────────────────────────────────────────────────────────────

/// An immutable, fully-validated prompt. Wraps a [`PromptDefinition`]; all invariants
/// (shape-valid, template-parseable, agreement-sound, reserved-name clean) are enforced at
/// construction time. There are no setters; the sole mutator is [`Prompt::derive`].
#[derive(Debug, Clone)]
pub struct Prompt {
    /// The validated definition. Private; exposed only through read-only accessors.
    def: PromptDefinition,
}

impl Prompt {
    // ── constructors ────────────────────────────────────────────────────────

    /// The primary validating constructor.
    ///
    /// Runs the construction invariants on `def`:
    /// 1. For each variant arm (root body + every named variant), asks the kernel for the
    ///    arm's [`required_roots`]. A kernel `Err` (parse failure or excluded feature) is a
    ///    **construction failure** — the `Prompt` is not built (R7/Q4).
    /// 2. Each analyzable arm's referenced roots must be a subset of the declared `variables`.
    ///    Any root not declared is an agreement violation (FR-020 / Principle IV).
    /// 3. A variant named literally `"default"` is rejected (CR-1 — the kernel reserves that
    ///    name for the root body; the declared arm would be unreachable).
    ///
    /// On success the `Prompt` is returned. On any violation a structured
    /// [`ConsumerError`] is returned — never a panic.
    ///
    /// # Errors
    ///
    /// - [`ConsumerError::Kernel`] — a variant template could not be parsed or uses an
    ///   excluded feature (`{% include %}` / macros / inheritance).
    /// - [`ConsumerError::Kernel`] — a variant template references a variable not declared
    ///   in `variables` (agreement failure; `code::UNDEFINED_VARIABLE`).
    /// - [`ConsumerError::Kernel`] — a variant is literally named `"default"` (reserved;
    ///   `code::UNDEFINED_VARIABLE` with field `"variant"`).
    pub fn new(def: PromptDefinition) -> Result<Self, ConsumerError> {
        validate_prompt_def(&def)?;
        Ok(Self { def })
    }

    /// Deserialize a `Prompt` from already-read **YAML** text, then validate.
    ///
    /// Equivalent to `serde_yaml_ng::from_str(..)` + [`Prompt::new`]. A parse/shape error
    /// returns [`ConsumerError::Load`]; a validation error returns the same errors as `new`.
    ///
    /// The crate reads no files — the caller supplies already-read text (C-03 / FR-024).
    ///
    /// # Errors
    ///
    /// [`ConsumerError::Load`] if `text` is not valid YAML or does not match the
    /// `PromptDefinition` shape, or any error from [`Prompt::new`].
    pub fn from_yaml(text: &str) -> Result<Self, ConsumerError> {
        let def: PromptDefinition =
            serde_yaml_ng::from_str(text).map_err(|e| ConsumerError::Load(e.to_string()))?;
        Self::new(def)
    }

    /// Deserialize a `Prompt` from already-read **JSON** text, then validate.
    ///
    /// Equivalent to `serde_json::from_str(..)` + [`Prompt::new`]. Error semantics mirror
    /// [`from_yaml`](Self::from_yaml).
    ///
    /// # Errors
    ///
    /// [`ConsumerError::Load`] if `text` is not valid JSON or does not match the
    /// `PromptDefinition` shape, or any error from [`Prompt::new`].
    pub fn from_json(text: &str) -> Result<Self, ConsumerError> {
        let def: PromptDefinition =
            serde_json::from_str(text).map_err(|e| ConsumerError::Load(e.to_string()))?;
        Self::new(def)
    }

    /// Deserialize a `Prompt` from already-read **TOML** text, then validate.
    ///
    /// Uses `toml::from_str` (the serde-native TOML crate — research R3 / `toml@1.1.2`).
    /// Error semantics mirror [`from_yaml`](Self::from_yaml).
    ///
    /// # Errors
    ///
    /// [`ConsumerError::Load`] if `text` is not valid TOML or does not match the
    /// `PromptDefinition` shape, or any error from [`Prompt::new`].
    pub fn from_toml(text: &str) -> Result<Self, ConsumerError> {
        let def: PromptDefinition =
            toml::from_str(text).map_err(|e| ConsumerError::Load(e.to_string()))?;
        Self::new(def)
    }

    // ── read-only accessors ──────────────────────────────────────────────────

    /// The prompt's name (the `name` field of the underlying definition).
    #[must_use]
    pub fn name(&self) -> &str {
        &self.def.name
    }

    /// The conversational role (`system` / `user` / `assistant`).
    #[must_use]
    pub fn role(&self) -> &PromptDefinitionRole {
        &self.def.role
    }

    /// The root body template source (the default arm's unrendered template).
    #[must_use]
    pub fn body(&self) -> &str {
        &self.def.body
    }

    /// The declared variables map (`name → PromptVariable`).
    #[must_use]
    pub fn variables(&self) -> &HashMap<String, PromptVariable> {
        &self.def.variables
    }

    /// The named variants map (`name → PromptVariant`). Empty when the prompt has no named
    /// variants (only the implicit default arm).
    #[must_use]
    pub fn variants(&self) -> &HashMap<String, PromptVariant> {
        &self.def.variants
    }

    /// The output model reference, if declared (`output_model` field). Carried as metadata
    /// only — never parsed or resolved by this library (Principle III).
    #[must_use]
    pub fn output_model(&self) -> Option<&str> {
        self.def.output_model.as_deref()
    }

    /// The `metadata` opaque map (library-defined top-level annotations, if any).
    #[must_use]
    pub fn metadata(&self) -> &serde_json::Map<String, serde_json::Value> {
        &self.def.metadata
    }

    // ── operations ───────────────────────────────────────────────────────────

    /// Validate-then-render this prompt.
    ///
    /// 1. Validates `vars` once via garde, BEFORE any templating (FR-002). On failure returns
    ///    [`ConsumerError::Validation`] — the kernel is never reached.
    /// 2. Bridges the validated struct to the kernel's value type via
    ///    [`minijinja::Value::from_serialize`] (FR-003a).
    /// 3. Delegates to [`prompting_press_core::render`], normalizing any
    ///    [`KernelError`] to [`ConsumerError::Kernel`].
    ///
    /// `variant = None` selects the default (root body) arm. `guard` is plumbed straight
    /// through to the kernel; `RenderResult::guard` is surfaced unchanged (guard *expansion*
    /// is the kernel's contract — spec 002 / F5).
    ///
    /// `V::Context: Default` so the whole-struct [`Validate::validate`] convenience applies
    /// (one validation pass over the entire input set, FR-002). Context-carrying validation
    /// is intentionally out of v1 scope (scope discipline — C-11 / one concrete path per
    /// concern).
    ///
    /// ## Byte-identical output (FR-016)
    ///
    /// The kernel path is identical to the pre-reshape `render(reg, name, …)` path. The
    /// `RenderResult` hashes are therefore byte-identical.
    ///
    /// # Errors
    ///
    /// - [`ConsumerError::Validation`] — garde rejected `vars`.
    /// - [`ConsumerError::Kernel`] — the kernel rejected the render (unknown variant,
    ///   strict-undefined reference, parse/render failure). `Parse`/`Render` detail scrubbed
    ///   (FR-015).
    pub fn render<V>(
        &self,
        vars: &V,
        variant: Option<&str>,
        guard: &GuardConfig,
    ) -> Result<RenderResult, ConsumerError>
    where
        V: Serialize + Validate,
        V::Context: Default,
    {
        // 1. Validate once, BEFORE any templating (FR-002).
        vars.validate().map_err(ConsumerError::from)?;

        // 2. Bridge the validated struct to the kernel's value type (FR-003a).
        //    `from_serialize` is infallible (ER-2): a custom-Serialize failure would
        //    surface downstream as a strict-undefined kernel error, never silently here.
        let values = minijinja::Value::from_serialize(vars);

        // 3. Delegate to the kernel; normalize KernelError → ConsumerError::Kernel.
        //    The kernel receives ONLY already-validated values (FR-003); the consumer adds
        //    no render/agreement/variant/hash logic of its own (FR-011).
        prompting_press_core::render(&self.def, variant, values, guard).map_err(ConsumerError::from)
    }

    /// Return a variant's unrendered template source (the exact string the kernel hashes
    /// into `template_hash`). Delegates to the kernel; no vars, no validation.
    ///
    /// `variant = None` returns the root body source.
    ///
    /// # Errors
    ///
    /// [`ConsumerError::Kernel`] — the kernel rejected the lookup (unknown variant name).
    pub fn get_source(&self, variant: Option<&str>) -> Result<&str, ConsumerError> {
        prompting_press_core::get_source(&self.def, variant).map_err(ConsumerError::from)
    }

    /// Pure advisory lint: returns a [`CheckReport`] containing only the origin/guard
    /// finding class.
    ///
    /// Construction already enforces agreement, parse, and reserved-name invariants, so those
    /// arms are structurally unreachable for a constructed `Prompt`. The only LIVE finding
    /// `check()` can surface is [`FindingKind::UntrustedWithoutGuard`] — a prompt declaring
    /// `untrusted`/`external` vars but carrying no `"guard"` key in `metadata`.
    ///
    /// Pure: takes `&self`, never renders, never mutates (FR-019).
    #[must_use]
    pub fn check(&self) -> CheckReport {
        let mut findings = Vec::new();
        check_origin_advisory(self.name(), &self.def, &mut findings);
        CheckReport { findings }
    }

    /// The sole mutator: shallow-replace top-level fields from `overlay` onto a clone of
    /// this prompt's definition, then route the merged definition through [`Prompt::new`]
    /// (full re-validation). The original `Prompt` is untouched.
    ///
    /// Each `Some(field)` in `overlay` replaces the corresponding field; `None` fields are
    /// left as-is. `name` IS overlayable (the overlay can rename a prompt). After the
    /// merge, every construction invariant is re-checked over the whole merged definition —
    /// so an overlay that introduces an agreement violation or a reserved variant name is
    /// rejected.
    ///
    /// In Rust the validator is the generic `V` named at the `render` call site (Principle
    /// VI compile-time coverage); `derive` takes `&self` and carries no runtime validator.
    /// `PromptOverlay` therefore contains only data fields (R6 — the Rust asymmetry).
    ///
    /// # Errors
    ///
    /// Same error classes as [`Prompt::new`]: a merged definition that fails any construction
    /// invariant returns the structured error.
    pub fn derive(&self, overlay: PromptOverlay) -> Result<Self, ConsumerError> {
        // Clone the underlying definition, then shallow-replace each Some field.
        let mut merged = self.def.clone();

        if let Some(name) = overlay.name {
            merged.name = name;
        }
        if let Some(role) = overlay.role {
            merged.role = role;
        }
        if let Some(body) = overlay.body {
            merged.body = body;
        }
        if let Some(variables) = overlay.variables {
            merged.variables = variables;
        }
        if let Some(variants) = overlay.variants {
            merged.variants = variants;
        }
        if let Some(output_model) = overlay.output_model {
            merged.output_model = output_model;
        }
        if let Some(metadata) = overlay.metadata {
            merged.metadata = metadata;
        }

        // Re-validate the merged whole through the same construction path.
        Self::new(merged)
    }

    /// Borrow the underlying [`PromptDefinition`] for use by binding crates
    /// (e.g. `prompting-press-node`, `prompting-press-py`) that need to call the kernel
    /// directly for render/get_source (their validation is owned in the binding layer, not
    /// in Rust garde, so the consumer's generic `render<V>` is not usable there). Bindings
    /// call `prompting_press_core::render(prompt.definition(), ...)` directly after doing
    /// their own validation — the same zero-engine-logic pattern as the registry render
    /// paths (critique E1 / C-01). Also used by `Composition::resolve` within this crate.
    pub fn definition(&self) -> &PromptDefinition {
        &self.def
    }
}

// ─── PromptOverlay ───────────────────────────────────────────────────────────

/// A shallow-replacement overlay for [`Prompt::derive`].
///
/// Each field is `Option<T>`. A `Some(value)` replaces the corresponding field on the
/// cloned definition; a `None` leaves it unchanged. All fields are optional — pass only
/// what should change.
///
/// `name` is overlayable: a prompt can be renamed (useful for template-derived variants).
/// After merging, the full construction invariants (agreement, parse, reserved name) are
/// re-checked over the merged whole.
///
/// In Rust the validator is the generic `V` named at the call site; `PromptOverlay` carries
/// **only data fields** — no runtime validator object (the Rust compile-time asymmetry
/// documented in R6).
#[derive(Debug, Clone, Default)]
pub struct PromptOverlay {
    /// Replace the prompt's `name`.
    pub name: Option<PromptDefinitionName>,
    /// Replace the prompt's `role`.
    pub role: Option<PromptDefinitionRole>,
    /// Replace the root body template source.
    pub body: Option<String>,
    /// Replace the full `variables` map.
    pub variables: Option<HashMap<String, PromptVariable>>,
    /// Replace the full `variants` map.
    pub variants: Option<HashMap<String, PromptVariant>>,
    /// Replace (or clear) the `output_model` reference.
    pub output_model: Option<Option<String>>,
    /// Replace the `metadata` opaque map.
    pub metadata: Option<serde_json::Map<String, serde_json::Value>>,
}

// ─── internal helpers ────────────────────────────────────────────────────────

/// Run all construction invariants over `def`. Returns `Ok(())` on success; the first
/// violated invariant returns the structured `ConsumerError`.
///
/// Invariants (in order):
/// 1. Reserved variant name (`"default"` in `variants`) → rejected (CR-1).
/// 2. For each variant arm: kernel `required_roots` must not `Err` (parse / excluded
///    feature → construction failure, R7/Q4).
/// 3. Referenced roots must be a subset of declared `variables` (agreement, FR-020).
fn validate_prompt_def(def: &PromptDefinition) -> Result<(), ConsumerError> {
    // 1. Reject a variant literally named "default" (CR-1).
    if def.variants.contains_key(DEFAULT) {
        return Err(ConsumerError::Kernel(vec![FieldError {
            field: "variant".to_string(),
            code: code::UNDEFINED_VARIABLE.to_string(),
            message: format!(
                "variant `{DEFAULT}` uses the reserved name for the root body; rename it \
                 or move its body to the root"
            ),
        }]));
    }

    let declared: std::collections::BTreeSet<&str> =
        def.variables.keys().map(String::as_str).collect();

    // Variants to analyze: default arm first (None), then every named arm.
    // The "default" key is already excluded above (construction-failed).
    let mut arms: Vec<Option<&str>> = vec![None];
    arms.extend(def.variants.keys().map(|k| Some(k.as_str())));

    for variant_opt in arms {
        let variant_label = variant_opt.unwrap_or(DEFAULT);

        // 2. Parse + required_roots (construction fails on Err — R7/Q4).
        let agreement = required_roots(def, variant_opt).map_err(|e| {
            let (field, msg, c) = kernel_analysis_error_to_field(&e);
            ConsumerError::Kernel(vec![FieldError {
                field: field.to_string(),
                code: c.to_string(),
                message: msg,
            }])
        })?;

        // 3. Agreement check: referenced roots ⊆ declared variables (FR-020).
        for root in &agreement.required_roots {
            if !declared.contains(root.as_str()) {
                return Err(ConsumerError::Kernel(vec![FieldError {
                    field: root.clone(),
                    code: code::UNDEFINED_VARIABLE.to_string(),
                    message: format!(
                        "template references undeclared variable `{root}` \
                         (variant `{variant_label}`); add it to the prompt's `variables`"
                    ),
                }]));
            }
        }
    }

    Ok(())
}

/// Map a kernel analysis error to `(field, message, code)` for a construction-failure
/// `ConsumerError`. Scrubbed — no bound-value content (SEC-004 / FR-015).
fn kernel_analysis_error_to_field(err: &KernelError) -> (&'static str, String, &'static str) {
    match err {
        KernelError::UnknownVariant { requested } => (
            "variant",
            format!("unknown variant: `{requested}`"),
            code::UNKNOWN_VARIANT,
        ),
        KernelError::UndefinedVariable { name } => (
            "template",
            format!("undefined variable at render: `{name}`"),
            code::UNDEFINED_VARIABLE,
        ),
        // SEC-004: detail may embed bound-value content — DO NOT copy it.
        KernelError::Parse { detail: _ } => {
            ("template", "template parse error".to_string(), code::PARSE)
        }
        KernelError::Render { detail: _ } => ("template", "render error".to_string(), code::RENDER),
        KernelError::ExcludedFeature { detail: _ } => (
            "template",
            "template uses an excluded feature".to_string(),
            code::EXCLUDED_FEATURE,
        ),
    }
}

/// The origin/guard advisory check for a single prompt (the only LIVE finding class for a
/// constructed `Prompt`).
///
/// A prompt declaring `untrusted`/`external` variables that carry no `"guard"` key in
/// `metadata` gets one [`FindingKind::UntrustedWithoutGuard`] per uncovered field.
/// This mirrors `check::check_provenance` but operates on a single `Prompt`, not a registry.
pub(crate) fn check_origin_advisory(
    name: &str,
    def: &PromptDefinition,
    findings: &mut Vec<Finding>,
) {
    let view: OriginView = origin_view(def);

    // Union of untrusted ∪ external — both already sorted BTreeSets.
    let declared_untrusted: std::collections::BTreeSet<&str> = view
        .untrusted
        .iter()
        .chain(view.external.iter())
        .map(String::as_str)
        .collect();

    if declared_untrusted.is_empty() {
        return;
    }

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
                 no guard (add a `guard` key under the prompt's `metadata`)"
            ),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── helpers ──────────────────────────────────────────────────────────────

    fn valid_json() -> &'static str {
        r#"{"name":"greet","role":"user","body":"Hi {{ name }}","variables":{"name":{"type":"string","origin":"trusted"}}}"#
    }

    fn make_prompt() -> Prompt {
        Prompt::from_json(valid_json()).expect("valid prompt must construct")
    }

    // ── T033: construction valid ──────────────────────────────────────────────

    #[test]
    fn construct_valid_prompt_succeeds() {
        let p = make_prompt();
        assert_eq!(p.name(), "greet");
        assert_eq!(p.body(), "Hi {{ name }}");
        assert!(p.variables().contains_key("name"));
        assert!(p.variants().is_empty());
    }

    // ── T033: construction invalid — undeclared variable ─────────────────────

    #[test]
    fn construct_rejects_undeclared_variable() {
        // `body` references `ghost` which is not in `variables`.
        let json = r#"{"name":"bad","role":"user","body":"{{ ghost }}","variables":{"name":{"type":"string","origin":"trusted"}}}"#;
        let err = Prompt::from_json(json).expect_err("undeclared var must fail construction");
        match &err {
            ConsumerError::Kernel(rows) => {
                assert!(
                    rows.iter().any(|r| r.code == code::UNDEFINED_VARIABLE),
                    "expected undefined_variable, got {rows:?}"
                );
                assert!(
                    rows.iter().any(|r| r.message.contains("ghost")),
                    "error must name the offending variable, got {rows:?}"
                );
            }
            other => panic!("expected ConsumerError::Kernel, got {other:?}"),
        }
    }

    // ── T033: construction invalid — excluded feature in body ─────────────────

    #[test]
    fn construct_rejects_excluded_feature_body() {
        let json = r#"{"name":"bad","role":"user","body":"{% include \"x\" %}","variables":{}}"#;
        let err = Prompt::from_json(json).expect_err("excluded feature must fail construction");
        match &err {
            ConsumerError::Kernel(rows) => {
                let codes: Vec<&str> = rows.iter().map(|r| r.code.as_str()).collect();
                assert!(
                    codes.contains(&code::EXCLUDED_FEATURE) || codes.contains(&code::PARSE),
                    "expected excluded_feature or parse code, got {codes:?}"
                );
            }
            other => panic!("expected ConsumerError::Kernel, got {other:?}"),
        }
    }

    // ── T033: construction invalid — syntax error ─────────────────────────────

    #[test]
    fn construct_rejects_syntax_error() {
        let json = r#"{"name":"bad","role":"user","body":"{{ unclosed","variables":{}}"#;
        let err = Prompt::from_json(json).expect_err("syntax error must fail construction");
        match &err {
            ConsumerError::Kernel(rows) => {
                let codes: Vec<&str> = rows.iter().map(|r| r.code.as_str()).collect();
                assert!(
                    codes.contains(&code::PARSE) || codes.contains(&code::EXCLUDED_FEATURE),
                    "expected parse code, got {codes:?}"
                );
            }
            other => panic!("expected ConsumerError::Kernel, got {other:?}"),
        }
    }

    // ── T033: construction invalid — variant named "default" ─────────────────

    #[test]
    fn construct_rejects_reserved_variant_name() {
        let json = r#"{"name":"bad","role":"user","body":"Hi","variables":{},"variants":{"default":{"body":"shadowed"}}}"#;
        let err =
            Prompt::from_json(json).expect_err("reserved variant name must fail construction");
        match &err {
            ConsumerError::Kernel(rows) => {
                assert_eq!(rows[0].field, "variant", "field must be 'variant'");
                assert!(
                    rows[0].message.contains("reserved") || rows[0].message.contains("default"),
                    "message must mention the reserved name, got {:?}",
                    rows[0].message
                );
            }
            other => panic!("expected ConsumerError::Kernel, got {other:?}"),
        }
    }

    // ── T033: `with` adds a variant; original unchanged ───────────────────────

    #[test]
    fn derive_adds_variant_original_unchanged() {
        let original = make_prompt();
        let original_body = original.body().to_string();
        let original_variants_count = original.variants().len();

        // Overlay: add a named variant that references the same declared variable.
        let mut new_variants = original.variants().clone();
        new_variants.insert(
            "brief".to_string(),
            serde_json::from_value(serde_json::json!({"body": "Hey {{ name }}"}))
                .expect("valid variant"),
        );

        let derived = original
            .derive(PromptOverlay {
                variants: Some(new_variants),
                ..Default::default()
            })
            .expect("derive must succeed for a valid overlay");

        // Derived has the new variant.
        assert!(derived.variants().contains_key("brief"));

        // Original is untouched (immutability — SC-004).
        assert_eq!(original.body(), original_body);
        assert_eq!(original.variants().len(), original_variants_count);
    }

    // ── T033: `derive` producing undeclared var → Err ────────────────────────

    #[test]
    fn derive_undeclared_var_body_returns_err() {
        let original = make_prompt();

        // Overlay replaces body with one that references an undeclared variable.
        let err = original
            .derive(PromptOverlay {
                body: Some("{{ name }} {{ ghost }}".to_string()),
                ..Default::default()
            })
            .expect_err("overlay with undeclared var must fail");
        match &err {
            ConsumerError::Kernel(rows) => {
                assert!(
                    rows.iter().any(|r| r.code == code::UNDEFINED_VARIABLE),
                    "expected undefined_variable, got {rows:?}"
                );
            }
            other => panic!("expected ConsumerError::Kernel, got {other:?}"),
        }
    }

    // ── T033: from_toml round-trips ────────────────────────────────────────────

    #[test]
    fn from_toml_round_trips() {
        let toml_text = r#"
name = "greeting"
role = "user"
body = "Hi {{ name }}"

[variables.name]
type = "string"
origin = "trusted"
"#;
        let p = Prompt::from_toml(toml_text).expect("TOML must construct");
        assert_eq!(p.name(), "greeting");
        assert_eq!(p.body(), "Hi {{ name }}");
        assert!(p.variables().contains_key("name"));
    }

    // ── T033: render byte-identical hashes ────────────────────────────────────

    #[test]
    fn render_byte_identical_hashes_across_two_renders() {
        use garde::Validate;
        use serde::Serialize;

        #[derive(Serialize, Validate)]
        struct V {
            #[garde(length(min = 1))]
            name: String,
        }

        let p = make_prompt();
        let vars = V {
            name: "Ada".to_string(),
        };

        let r1 = p
            .render(&vars, None, &GuardConfig::default())
            .expect("render 1");
        let r2 = p
            .render(&vars, None, &GuardConfig::default())
            .expect("render 2");

        assert_eq!(r1.text, r2.text, "text must be byte-identical");
        assert_eq!(
            r1.template_hash, r2.template_hash,
            "template_hash must be byte-identical"
        );
        assert_eq!(
            r1.render_hash, r2.render_hash,
            "render_hash must be byte-identical"
        );
    }

    // ── T033: check() returns only origin/guard advisory ─────────────────────

    #[test]
    fn check_returns_origin_advisory_only() {
        // A prompt with an untrusted variable and no guard → should find UntrustedWithoutGuard.
        let json = r#"{"name":"unguarded","role":"user","body":"{{ payload }}","variables":{"payload":{"type":"string","origin":"untrusted"}}}"#;
        let p = Prompt::from_json(json).expect("valid shape, should construct");
        let report = p.check();
        assert!(
            !report.passed(),
            "unguarded untrusted field should produce a finding"
        );
        assert!(report
            .findings
            .iter()
            .all(|f| matches!(&f.kind, FindingKind::UntrustedWithoutGuard { .. })));
    }

    #[test]
    fn check_passes_for_guarded_untrusted_field() {
        let json = r#"{"name":"guarded","role":"user","body":"{{ payload }}","variables":{"payload":{"type":"string","origin":"untrusted"}},"metadata":{"guard":{"enabled":true}}}"#;
        let p = Prompt::from_json(json).expect("valid shape");
        assert!(p.check().passed(), "guard configured → check must pass");
    }

    // ── T033: get_source delegates to kernel ──────────────────────────────────

    #[test]
    fn get_source_returns_root_body() {
        let p = make_prompt();
        let src = p.get_source(None).expect("root source must resolve");
        assert_eq!(src, "Hi {{ name }}");
    }
}
