//! The Node render path — marshal → kernel-direct render, plus `getSource` and the
//! [`RenderResult`] / [`GuardConfig`] napi types (FR-002, FR-009..011; US1).
//!
//! ## Why the kernel is called DIRECTLY (critique E1 / C-01)
//!
//! [`render`] does **not** call the Rust consumer's `prompting_press::render`. That entry point
//! is generic over `V: Serialize + Validate` — a *garde* type — and this binding has no such
//! type: validation is owned in **TypeScript** (against the caller's Zod schema, in the facade),
//! not in Rust. So after the TS facade has validated and the addon receives the already-validated
//! value, this module marshals it and calls [`prompting_press_core::render`] directly. That is
//! still **zero engine logic** (Principle I): the kernel *is* the shared core; the binding only
//! marshals into it and surfaces its result 1:1. Render byte-parity with the Rust/Python bindings
//! stays structural because the value handed to the kernel is built by the same
//! `marshal::to_kernel_value` → `minijinja::Value::from_serialize` path the consumer uses.
//!
//! ## The marshal → render chain (Q1)
//!
//! Validation has **already happened** in the TS facade (`schema.safeParse(data)` — research D3 /
//! Q1) before the addon is called, so the Rust side does **no** validation:
//!
//! 1. **Resolve** the prompt by name against the registry's inner consumer registry; absent ⇒
//!    an `unknown_prompt` error, never a panic (FR-008a).
//! 2. **Marshal** the already-validated value through the single value bridge
//!    [`crate::marshal::to_kernel_value`] (FR-003a).
//! 3. **Render** by calling [`prompting_press_core::render`] directly; map any returned
//!    [`KernelError`](prompting_press_core::KernelError) through the consumer's tested scrubber via
//!    [`crate::error::kernel_error_to_napi_err`] (preserves SEC-004 — critique E2). The raw
//!    `KernelError::detail` is never read here.

use napi_derive::napi;

use prompting_press::ConsumerError;
use prompting_press_core::{GuardConfig as KernelGuardConfig, RenderResult as KernelRenderResult};

use crate::error::{consumer_error_to_napi_err, kernel_error_to_napi_err};
use crate::marshal::to_kernel_value;
use crate::registry::Registry;

/// The opt-in guard-expansion config, accepted from JS as a plain object and **plumbed through**
/// to the kernel (FR-009).
///
/// A 1:1 mirror of the kernel's [`prompting_press_core::GuardConfig`] — `enabled` plus an optional
/// override `template`. This is **config only**; it carries no logic. The kernel owns guard
/// *expansion* (spec 002 / FR-022..025 — naming the declared untrusted/external fields, the
/// `{fields}` substitution, the never-touches-`text` invariant); the binding only marshals these
/// two fields across the boundary and surfaces whatever [`RenderResult::guard`] the kernel
/// populates. As a `#[napi(object)]` it crosses as a plain TS object `{ enabled, template? }`.
#[napi(object)]
pub struct GuardConfig {
    /// When `false`, the render is plain and [`RenderResult::guard`] is `None`.
    pub enabled: bool,
    /// Optional caller override of the guard instruction text; absent ⇒ the kernel default.
    pub template: Option<String>,
}

impl From<GuardConfig> for KernelGuardConfig {
    fn from(g: GuardConfig) -> Self {
        Self {
            enabled: g.enabled,
            template: g.template,
        }
    }
}

/// A rendered prompt + its content-addressed provenance, read-only from JS.
///
/// The Node mirror of the kernel's [`prompting_press_core::RenderResult`] (data-model
/// §RenderResult; FR-015). Surfaced **1:1** — the binding adds nothing and interprets nothing. A
/// `#[napi]` class with read-only getters; the snake_case Rust getters surface as camelCase JS
/// accessors (`template_hash` → `templateHash`). A result is produced by [`render`], never
/// constructed from JS.
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

/// Render `name`'s resolved variant through the kernel with the already-validated `value`.
///
/// **Not a `#[napi]` function** (SC-001 / T046): the registry-keyed render path is gone from
/// the public JS surface. This plain Rust function is kept only for the `#[cfg(test)]` suites
/// that exercise the kernel-direct render + napi error mapping without a Node runtime.
///
/// The public render path is `NapiPrompt::render_prompt` (see `prompt.rs`).
///
/// # Errors
/// - `unknown_prompt` — `name` absent from `reg`.
/// - kernel codes — the kernel rejected the render (SEC-004 scrubbed).
pub fn render(
    reg: &Registry,
    name: String,
    value: serde_json::Value,
    variant: Option<String>,
    guard: Option<GuardConfig>,
) -> napi::Result<RenderResult> {
    // 1. Resolve the prompt by name once (absent ⇒ structured error, never a panic — FR-008a).
    //    Done first and entirely in Rust against the inner consumer registry.
    let Some(def) = reg.get(&name) else {
        return Err(consumer_error_to_napi_err(ConsumerError::UnknownPrompt(
            name,
        )));
    };

    // 2. Marshal the already-validated value through the single value bridge (FR-003a).
    let values = to_kernel_value(value);

    // 3. Plumb the guard config through (FR-009) and render by calling the KERNEL DIRECTLY
    //    (critique E1 / C-01). Absent guard ⇒ the kernel default (disabled). The binding does NO
    //    guard logic — it only marshals the two config fields; the kernel decides the `guard` field.
    let guard_cfg = guard.map_or_else(KernelGuardConfig::default, KernelGuardConfig::from);

    prompting_press_core::render(def, variant.as_deref(), values, &guard_cfg)
        .map(RenderResult::from)
        .map_err(kernel_error_to_napi_err)
}

/// Return a prompt variant's **unrendered** template source.
///
/// **Not a `#[napi]` function** (SC-001 / T046): registry-keyed getSource is gone from the
/// public JS surface. Kept as a plain Rust function for `#[cfg(test)]` coverage only.
///
/// The public getSource path is `NapiPrompt::get_source_prompt` (see `prompt.rs`).
///
/// # Errors
/// - `unknown_prompt` — `name` absent from `reg`.
/// - a kernel code (e.g. `unknown_variant`) — the kernel rejected the lookup.
pub fn get_source(reg: &Registry, name: String, variant: Option<String>) -> napi::Result<String> {
    // 1. Resolve the definition by name (absent ⇒ structured error, never a panic).
    let Some(def) = reg.get(&name) else {
        return Err(consumer_error_to_napi_err(ConsumerError::UnknownPrompt(
            name,
        )));
    };

    // 2. Delegate to the kernel directly (the consumer's free-fn get_source is gone
    //    post-reshape; calling the kernel is the same zero-engine-logic pattern as render).
    prompting_press_core::get_source(def, variant.as_deref())
        .map(str::to_owned)
        .map_err(crate::error::kernel_error_to_napi_err)
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
        // A prompt declaring an `untrusted` variable, so the guard text has something to name.
        let def = def_from_json(
            r#"{
                "name": "ask",
                "role": "user",
                "body": "Answer: {{ q }}",
                "variables": { "q": { "type": "string", "origin": "untrusted" } }
            }"#,
        );

        let make_values = || to_kernel_value(serde_json::json!({ "q": "hello" }));

        // Enabled guard (built via the binding type → kernel `From`, the SAME conversion `render`
        // performs) ⇒ guard text present.
        let enabled = GuardConfig {
            enabled: true,
            template: None,
        };
        let kernel_cfg = KernelGuardConfig::from(enabled);
        let with_guard = prompting_press_core::render(&def, None, make_values(), &kernel_cfg)
            .map(RenderResult::from)
            .expect("render with guard");
        assert!(
            with_guard.guard.is_some(),
            "an enabled guard on a prompt with an untrusted field must surface guard text"
        );
        // Plumb-through, not concatenation: the body text is unchanged by the guard (FR-023).
        assert_eq!(with_guard.text, "Answer: hello");

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

    /// `get_source` returns the unrendered source for the resolved variant (FR-010), and an unknown
    /// name maps to an `unknown_prompt`-coded error (via the inner-registry resolution).
    #[test]
    fn get_source_returns_unrendered_source() {
        let def =
            def_from_json(r#"{ "name": "greet", "role": "user", "body": "Hello {{ name }}!" }"#);
        let reg = Registry::from_defs_for_test([def]);

        let src = get_source(&reg, "greet".to_string(), None).expect("source");
        assert_eq!(
            src, "Hello {{ name }}!",
            "source is UNrendered (no interpolation)"
        );

        // Unknown name ⇒ unknown_prompt-coded error.
        let err = get_source(&reg, "absent".to_string(), None).expect_err("unknown name");
        let payload = payload_of(&err);
        assert_eq!(payload["code"], code::UNKNOWN_PROMPT);
    }
}
