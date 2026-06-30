//! The Node render path — marshal → kernel-direct render, plus the
//! [`RenderResult`] / [`GuardConfig`] napi types (FR-002, FR-009..011; US1).
//!
//! ## Why the kernel is called DIRECTLY (critique E1 / C-01)
//!
//! The public render path is `NapiPrompt::render_prompt` (see `prompt.rs`). It does **not** call
//! the Rust consumer's `prompting_press::render`. That entry point is generic over
//! `V: Serialize + Validate` — a *garde* type — and this binding has no such type: validation is
//! owned in **TypeScript** (against the caller's Zod schema, in the facade), not in Rust. So
//! after the TS facade has validated and the addon receives the already-validated value, this
//! module marshals it and calls [`prompting_press_core::render`] directly. That is still **zero
//! engine logic** (Principle I): the kernel *is* the shared core; the binding only marshals into
//! it and surfaces its result 1:1. Render byte-parity with the Rust/Python bindings stays
//! structural because the value handed to the kernel is built by the same
//! `marshal::to_kernel_value` → `minijinja::Value::from_serialize` path the consumer uses.
//!
//! ## The marshal → render chain (Q1)
//!
//! Validation has **already happened** in the TS facade (`schema.safeParse(data)` — research D3 /
//! Q1) before the addon is called, so the Rust side does **no** validation:
//!
//! 1. **Marshal** the already-validated value through the single value bridge
//!    [`crate::marshal::to_kernel_value`] (FR-003a).
//! 2. **Render** by calling [`prompting_press_core::render`] directly; map any returned
//!    [`KernelError`](prompting_press_core::KernelError) through the consumer's tested scrubber via
//!    [`crate::error::kernel_error_to_napi_err`] (preserves SEC-004 — critique E2). The raw
//!    `KernelError::detail` is never read here.

use napi_derive::napi;

use prompting_press_core::{GuardConfig as KernelGuardConfig, RenderResult as KernelRenderResult};

/// The opt-in guard-expansion config, accepted from JS as a plain object and **plumbed through**
/// to the kernel (FR-009).
///
/// A 1:1 mirror of the kernel's [`prompting_press_core::GuardConfig`] — `enabled` only (spec 015
/// removed the custom `template` override; the guard wording is now fixed). This is **config only**;
/// it carries no logic. The kernel owns guard *expansion* (spec 015 / FR-022..025); the binding
/// only marshals `enabled` across the boundary and surfaces whatever [`RenderResult::guard`] the
/// kernel populates. Accepted from JS as a plain TS object `{ enabled }`.
#[napi(object)]
pub struct GuardConfig {
    /// When `false`, the render is plain and [`RenderResult::guard`] is `None`.
    pub enabled: bool,
}

impl From<GuardConfig> for KernelGuardConfig {
    fn from(g: GuardConfig) -> Self {
        Self { enabled: g.enabled }
    }
}

/// A rendered prompt + its content-addressed provenance, read-only from JS.
///
/// The Node mirror of the kernel's [`prompting_press_core::RenderResult`] (data-model
/// §RenderResult; FR-015). Surfaced **1:1** — the binding adds nothing and interprets nothing. A
/// Read-only class with camelCase JS accessors (`template_hash` → `templateHash`).
/// A result is produced by [`render`], never constructed from JS.
#[napi]
pub struct RenderResult {
    text: String,
    name: String,
    variant: String,
    template_hash: String,
    render_hash: String,
    guard: Option<String>,
}

#[napi]
impl RenderResult {
    /// The rendered body text (FR-001). The guard text is NEVER concatenated here.
    #[napi(getter)]
    #[must_use]
    pub fn text(&self) -> String {
        self.text.clone()
    }

    /// The prompt name.
    #[napi(getter)]
    #[must_use]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// The resolved variant name (the reserved `default`, or the named arm).
    #[napi(getter)]
    #[must_use]
    pub fn variant(&self) -> String {
        self.variant.clone()
    }

    /// Lowercase-hex `SHA256(resolved variant source)` (FR-012). Surfaces as `templateHash`.
    #[napi(getter)]
    #[must_use]
    pub fn template_hash(&self) -> String {
        self.template_hash.clone()
    }

    /// Lowercase-hex `SHA256(rendered text)` (FR-013). Surfaces as `renderHash`.
    #[napi(getter)]
    #[must_use]
    pub fn render_hash(&self) -> String {
        self.render_hash.clone()
    }

    /// The opt-in guard instruction text (present iff a guard was enabled and the prompt declares
    /// an untrusted/external field); `null` for a plain render. Never part of `text`.
    #[napi(getter)]
    #[must_use]
    pub fn guard(&self) -> Option<String> {
        self.guard.clone()
    }
}

impl From<KernelRenderResult> for RenderResult {
    fn from(r: KernelRenderResult) -> Self {
        Self {
            text: r.text,
            name: r.name,
            variant: r.variant,
            template_hash: r.template_hash,
            render_hash: r.render_hash,
            guard: r.guard,
        }
    }
}

#[cfg(test)]
mod tests {
    //! Render-path coverage that is drivable in Rust WITHOUT the TS Zod facade.
    //!
    //! The validate-then-render behavior (a real Zod schema, the error subclass on invalid input,
    //! "no render happened") needs the TS facade and is covered TS-side in T010. Here we exercise
    //! the kernel-direct render + the `KernelError` → napi error mapping that the `#[napi]` fn
    //! delegates to — the parts that need no JS runtime (the values are built from
    //! `serde_json::Value`, exactly what napi would hand the fn after decoding the JS arg).

    use super::*;
    use crate::error::kernel_error_to_napi_err;
    use crate::marshal::to_kernel_value;
    use prompting_press::error::code;
    use prompting_press::PromptDefinition;

    /// Build a `PromptDefinition` from JSON (the idiomatic in-test construction the consumer's own
    /// tests use — the generated newtypes validate, so a struct literal is awkward).
    fn def_from_json(json: &str) -> PromptDefinition {
        serde_json::from_str(json).expect("valid prompt definition")
    }

    /// Parse the JSON payload napi carries in a thrown error's `reason`.
    fn payload_of(err: &napi::Error) -> serde_json::Value {
        serde_json::from_str(&err.reason).expect("napi error reason is the JSON payload")
    }

    /// The happy path the `#[napi]` fn's tail performs: a marshaled value → `prompting_press_core::
    /// render` (DIRECTLY — critique E1) → `RenderResult::from`. Asserts the rendered text and that
    /// both provenance hashes are 64-char lowercase hex (FR-012/FR-013).
    #[test]
    fn kernel_direct_render_produces_text_and_hex_hashes() {
        let def =
            def_from_json(r#"{ "name": "greet", "role": "user", "body": "Hello {{ name }}!" }"#);

        // Marshal the value through the SAME bridge the fn uses (the validated-payload stand-in),
        // so the value handed to the kernel is built identically.
        let values = to_kernel_value(serde_json::json!({ "name": "Ada" }));

        let kernel =
            prompting_press_core::render(&def, None, values, &KernelGuardConfig::default())
                .expect("render succeeds");
        let result = RenderResult::from(kernel);

        assert_eq!(result.text, "Hello Ada!");
        assert_eq!(result.name, "greet");
        assert_eq!(
            result.variant, "default",
            "no variant ⇒ reserved default arm"
        );
        assert!(
            result.guard.is_none(),
            "default guard config ⇒ no guard text"
        );

        for (label, hash) in [
            ("template_hash", &result.template_hash),
            ("render_hash", &result.render_hash),
        ] {
            assert_eq!(hash.len(), 64, "{label} must be 64 hex chars, got {hash:?}");
            assert!(
                hash.chars()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
                "{label} must be lowercase hex, got {hash:?}"
            );
        }
    }

    /// `RenderResult` surfaces the kernel result 1:1, including the resolved NAMED variant.
    #[test]
    fn named_variant_is_surfaced() {
        let def = def_from_json(
            r#"{
                "name": "greet",
                "role": "user",
                "body": "default {{ name }}",
                "variants": { "formal": { "body": "Good day, {{ name }}." } }
            }"#,
        );
        let values = to_kernel_value(serde_json::json!({ "name": "Ada" }));

        let kernel = prompting_press_core::render(
            &def,
            Some("formal"),
            values,
            &KernelGuardConfig::default(),
        )
        .expect("render formal");
        let result = RenderResult::from(kernel);

        assert_eq!(result.text, "Good day, Ada.");
        assert_eq!(result.variant, "formal");
    }

    /// **Three-sets gap (critique E1 / spec assumptions).** A value missing a template-referenced
    /// root drives the kernel's strict-undefined path. Routed through the binding's
    /// `kernel_error_to_napi_err`, it must surface as an `undefined_variable`-coded error — a LOUD
    /// error, never a silent empty render.
    #[test]
    fn missing_root_is_loud_undefined_variable() {
        let def =
            def_from_json(r#"{ "name": "greet", "role": "user", "body": "Hello {{ name }}!" }"#);

        // The value lacks `name` — the struct↔variables field-name agreement is the caller's
        // responsibility; a miss is NOT silent (it hits strict-undefined).
        let values = to_kernel_value(serde_json::json!({}));

        let kernel_err =
            prompting_press_core::render(&def, None, values, &KernelGuardConfig::default())
                .expect_err("missing root ⇒ strict-undefined kernel error");

        let err = kernel_error_to_napi_err(kernel_err);
        let payload = payload_of(&err);
        assert_eq!(
            payload["code"],
            code::UNDEFINED_VARIABLE,
            "a missing referenced root is a loud undefined_variable, not an empty render"
        );
        assert_eq!(payload["errors"][0]["code"], code::UNDEFINED_VARIABLE);
    }

    /// **Guard plumb-through (FR-009).** The binding's `GuardConfig` is converted to the kernel's
    /// and passed through unchanged: an enabled guard on a prompt that declares an untrusted field
    /// ⇒ `RenderResult.guard` is `Some(...)`; a default (disabled) guard ⇒ `None`. This asserts
    /// only that the field is *surfaced vs not* — the guard-text content/logic is the kernel's
    /// (spec 002) and is NOT re-tested here.
    #[test]
    fn guard_config_is_plumbed_through() {
        // A prompt declaring an untrusted variable (trusted: false), so the guard has something
        // to delimit.
        let def = def_from_json(
            r#"{
                "name": "ask",
                "role": "user",
                "body": "Answer: {{ q }}",
                "variables": { "q": { "type": "string", "trusted": false } }
            }"#,
        );

        let make_values = || to_kernel_value(serde_json::json!({ "q": "hello" }));

        // Enabled guard (built via the binding type → kernel `From`, the SAME conversion `render`
        // performs) ⇒ guard text present.
        let enabled = GuardConfig { enabled: true };
        let kernel_cfg = KernelGuardConfig::from(enabled);
        let with_guard = prompting_press_core::render(&def, None, make_values(), &kernel_cfg)
            .map(RenderResult::from)
            .expect("render with guard");
        assert!(
            with_guard.guard.is_some(),
            "an enabled guard on a prompt with an untrusted field must surface guard text"
        );
        // Spec 015: untrusted values are wrapped in <untrusted>…</untrusted> in the rendered body.
        assert!(
            with_guard.text.contains("<untrusted>"),
            "enabled guard must wrap untrusted values in the rendered body text"
        );

        // Default (disabled) guard ⇒ no guard text.
        let plain =
            prompting_press_core::render(&def, None, make_values(), &KernelGuardConfig::default())
                .map(RenderResult::from)
                .expect("render plain");
        assert!(
            plain.guard.is_none(),
            "a default/disabled guard must leave RenderResult.guard as None"
        );
    }
}
