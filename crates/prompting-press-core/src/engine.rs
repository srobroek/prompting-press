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

use crate::error::KernelError;
use crate::generated::prompt_definition::PromptDefinition;
use crate::hashing::sha256_hex;
use crate::origin::{build_guard_text, origin_view, GuardConfig};

/// The reserved variant name that always resolves to the prompt's root `body`
/// (FR-007/FR-010/FR-011). The generated shape does not encode this rule, so the kernel
/// enforces it here in its own resolution logic.
const DEFAULT_VARIANT: &str = "default";

/// Build the kernel's canonical MiniJinja environment.
///
/// Configured with [`minijinja::UndefinedBehavior::Strict`] (FR-001a). The excluded
/// template features are enforced by the disabled `macros` / `multi_template` crate
/// features (FR-002, see the module docs), so this builder adds no feature-rejection
/// logic of its own.
///
/// Returns an environment with the `'static` lifetime: it owns no borrowed template
/// source, so templates are added per-operation against borrowed definition bytes.
pub(crate) fn build_environment() -> minijinja::Environment<'static> {
    let mut env = minijinja::Environment::new();
    env.set_undefined_behavior(minijinja::UndefinedBehavior::Strict);
    env
}

/// The variant selected for a render — the reserved `default` (root body) or an
/// explicitly named arm (data-model §ResolvedVariant). It borrows the definition, so
/// `source` is the exact unrendered bytes `template_hash` is computed over (FR-012).
#[derive(Debug)]
pub(crate) struct ResolvedVariant<'a> {
    /// The reserved `"default"`, or the explicit variant key.
    pub name: String,
    /// The template source string of the resolved arm (borrow of the definition).
    pub source: &'a str,
}

/// Resolve the arm a render/analysis should run against (spec 002, T016; FR-007..FR-011).
///
/// Resolution rule (data-model §ResolvedVariant):
/// - `None` or `Some("default")` → name `"default"`, source = `def.body` (the root body
///   is ALWAYS the default; there is no "missing default" error path).
/// - `Some(k)` where `k` is a declared variant → name `k`, source = `variants[k].body`.
/// - `Some(k)` where `k` is neither `"default"` nor a declared variant →
///   [`KernelError::UnknownVariant`] naming `k` (FR-009 — the only variant-resolution error).
pub(crate) fn resolve_variant<'a>(
    def: &'a PromptDefinition,
    variant: Option<&str>,
) -> Result<ResolvedVariant<'a>, KernelError> {
    match variant {
        None | Some(DEFAULT_VARIANT) => Ok(ResolvedVariant {
            name: DEFAULT_VARIANT.to_string(),
            source: &def.body,
        }),
        Some(name) => match def.variants.get(name) {
            Some(arm) => Ok(ResolvedVariant {
                name: name.to_string(),
                source: &arm.body,
            }),
            None => Err(KernelError::UnknownVariant {
                requested: name.to_string(),
            }),
        },
    }
}

/// Return the unrendered source of the resolved variant (spec 002, T017; FR-006).
///
/// Same resolution and [`KernelError::UnknownVariant`] rule as [`render`]. The returned
/// `&str` is the exact byte string `template_hash` is computed over (FR-012), so a caller
/// can hash it independently and cross-check a render's `template_hash`.
///
/// # Errors
/// Returns [`KernelError::UnknownVariant`] if `variant` names an arm that does not exist
/// (and is not the reserved `"default"`).
pub fn get_source<'a>(
    def: &'a PromptDefinition,
    variant: Option<&str>,
) -> Result<&'a str, KernelError> {
    Ok(resolve_variant(def, variant)?.source)
}

/// Render result + content-addressed provenance (data-model §RenderResult; FR-015).
///
/// Plain data returned to the caller — no telemetry sink, no tracing coupling. There is
/// deliberately **no** `vars_hash` field (FR-014).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderResult {
    /// The rendered body text (FR-001). The guard text is NEVER concatenated here.
    pub text: String,
    /// The prompt name (`def.name`). [FR-015]
    pub name: String,
    /// The resolved variant name (the reserved `default`, or the named arm). [FR-015]
    pub variant: String,
    /// Lowercase-hex `SHA256(resolved variant source)`. [FR-012]
    pub template_hash: String,
    /// Lowercase-hex `SHA256(rendered text)`. [FR-013]
    pub render_hash: String,
    /// The guard instruction text, present only when guard expansion was opted in
    /// (US3); `None` for US1's disabled config. Never concatenated into `text`. [FR-022]
    pub guard: Option<String>,
}

/// Render a prompt's resolved variant to text and stamp provenance (spec 002, T019).
///
/// Resolves the variant (FR-007..FR-011), renders the resolved source against `values`
/// using the kernel's strict-undefined environment (`build_environment`), and computes
/// `template_hash`/`render_hash` (FR-012/FR-013). Rendering is deterministic: identical
/// `(def, variant, values)` yields byte-identical `text` and equal hashes (FR-003,
/// SC-001). The kernel is validation-blind and performs no I/O (FR-004/FR-005).
///
/// Guard expansion is opt-in (US3, FR-022..FR-025): when `guard.enabled`, the result's
/// [`RenderResult::guard`] carries an instruction *naming* the prompt's untrusted/external
/// fields (computed by `build_guard_text` over [`origin_view`]). It is a **separate**
/// field — never concatenated into `text` — and is purely additive: enabling it does not
/// change `text`, the template, or the values (FR-023, SC-005), and it never inspects or
/// sanitizes a value (FR-025). When `!guard.enabled`, `guard` is `None`.
///
/// # Errors
/// - [`KernelError::UnknownVariant`] — `variant` names a non-existent arm (FR-009).
/// - [`KernelError::ExcludedFeature`] / [`KernelError::Parse`] — the resolved source uses
///   an excluded feature (unrecognised tag under the disabled `macros`/`multi_template`
///   features) or otherwise fails to parse (FR-002, FR-028).
/// - [`KernelError::UndefinedVariable`] — a strict-undefined reference was hit at render
///   (FR-001a): a referenced variable was absent from `values`.
/// - [`KernelError::Render`] — any other render-time failure (FR-028).
pub fn render(
    def: &PromptDefinition,
    variant: Option<&str>,
    values: minijinja::Value,
    guard: &GuardConfig,
) -> Result<RenderResult, KernelError> {
    let resolved = resolve_variant(def, variant)?;

    // Per-render environment + a single anonymous template against the resolved source.
    // With `macros`/`multi_template` disabled, an excluded-feature tag fails right here
    // at parse time (research D1/D4); `add_template_owned` parses eagerly.
    let mut env = build_environment();
    env.add_template_owned("kernel".to_string(), resolved.source.to_string())
        .map_err(map_minijinja_error)?;
    let template = env.get_template("kernel").map_err(map_minijinja_error)?;

    let text = template.render(values).map_err(map_minijinja_error)?;

    let template_hash = sha256_hex(resolved.source);
    let render_hash = sha256_hex(&text);

    // Opt-in, additive guard text — a SEPARATE field, computed from the declared
    // origin tags. `build_guard_text` returns `None` unless `guard.enabled` (and the
    // untrusted∪external union is non-empty), and never touches `text`/values (FR-022..25).
    let guard = build_guard_text(&origin_view(def), guard);

    Ok(RenderResult {
        text,
        name: def.name.to_string(),
        variant: resolved.name,
        template_hash,
        render_hash,
        guard,
    })
}

/// Map a MiniJinja [`minijinja::Error`] to the kernel's structured [`KernelError`]
/// (FR-028). The discriminator is the error's [`minijinja::ErrorKind`]:
///
/// - `SyntaxError` → [`KernelError::ExcludedFeature`] when the message names an excluded
///   construct (unrecognised tag under disabled `macros`/`multi_template`), else
///   [`KernelError::Parse`] (research D4: the kernel labels precisely when it can, and
///   falls back to a loud, generic parse error otherwise).
/// - `UndefinedError` → [`KernelError::UndefinedVariable`] (strict undefined, FR-001a).
///   MiniJinja raises this with no variable-name payload, so `name` is best-effort: the
///   error `detail` if present, else the error's `Display` (still informative).
/// - anything else → [`KernelError::Render`].
///
/// `pub(crate)` so the agreement analysis ([`crate::agreement::required_roots`]) maps its
/// parse / excluded-feature failures through the **same** logic as render — keeping the
/// excluded-feature labelling consistent across operations (FR-016a, FR-028).
pub(crate) fn map_minijinja_error(err: minijinja::Error) -> KernelError {
    match err.kind() {
        minijinja::ErrorKind::SyntaxError => {
            let detail = err.to_string();
            if looks_like_excluded_feature(&detail) {
                KernelError::ExcludedFeature { detail }
            } else {
                KernelError::Parse { detail }
            }
        }
        minijinja::ErrorKind::UndefinedError => {
            // Strict undefined carries no variable name (verified in 2.21.0 source:
            // `Error::from(ErrorKind::UndefinedError)`), so this is best-effort.
            let name = err
                .detail()
                .map(str::to_string)
                .unwrap_or_else(|| err.to_string());
            KernelError::UndefinedVariable { name }
        }
        _ => KernelError::Render {
            detail: err.to_string(),
        },
    }
}

/// Best-effort heuristic distinguishing an excluded-feature parse failure from an
/// ordinary syntax error (research D4). With `macros`/`multi_template` disabled, each
/// excluded tag surfaces as an **unknown statement** and MiniJinja names the offending
/// keyword. This only refines the error *label* — both branches are loud parse-time
/// errors, so a miss is benign (it falls back to [`KernelError::Parse`]).
///
/// **Matching contract (verified against MiniJinja 2.21.0):** with `macros` /
/// `multi_template` off, `add_template` fails with a `SyntaxError` whose detail is
/// `"unknown statement <keyword>"` — the keyword is **bare**, not quoted/backticked
/// (verified empirically: `unknown statement include`, `… from`, `… macro`, `… block`,
/// etc.). The heuristic therefore matches the exact phrase `unknown statement <kw>` for
/// each of the six disabled-tag keywords. This is deliberately tight in two directions:
///
/// - It will **not** fire on an ordinary syntax error (an unclosed `{{`, a bad filter,
///   etc.), because those do not emit the `unknown statement` phrase — so a real syntax
///   error is correctly labelled [`KernelError::Parse`], never mislabelled as excluded.
/// - It keys off the `unknown statement` phrase, not a loose substring like `"block"`
///   (which could otherwise appear in an unrelated message), so it does not over-match.
///
/// A genuinely unknown tag that is *not* one of the six (e.g. `{% frobnicate %}`) also
/// emits `unknown statement frobnicate`, but `frobnicate` is not in the keyword set, so
/// it correctly falls through to [`KernelError::Parse`].
fn looks_like_excluded_feature(detail: &str) -> bool {
    /// The six v1-excluded tag keywords (FR-002), as MiniJinja names them in an
    /// `unknown statement <kw>` detail when `macros`/`multi_template` are disabled.
    const EXCLUDED_KEYWORDS: [&str; 6] = ["include", "extends", "import", "from", "macro", "block"];
    let lowered = detail.to_ascii_lowercase();
    EXCLUDED_KEYWORDS
        .iter()
        .any(|kw| lowered.contains(&format!("unknown statement {kw}")))
}

#[cfg(test)]
mod tests {
    use super::{build_environment, looks_like_excluded_feature};

    #[test]
    fn builds_with_strict_undefined() {
        let env = build_environment();
        assert_eq!(
            env.undefined_behavior(),
            minijinja::UndefinedBehavior::Strict,
            "kernel environment must be strict-undefined (FR-001a)",
        );
    }

    /// The heuristic matches MiniJinja 2.21.0's actual `unknown statement <kw>` detail for
    /// each of the six disabled-tag keywords (verified empirically against the engine).
    #[test]
    fn excluded_feature_detail_is_recognised() {
        for kw in ["include", "extends", "import", "from", "macro", "block"] {
            let detail = format!("syntax error: unknown statement {kw} (in kernel:1)");
            assert!(
                looks_like_excluded_feature(&detail),
                "must recognise the excluded keyword `{kw}` in `{detail}`",
            );
        }
    }

    /// Tightness: an ordinary syntax error (no `unknown statement <kw>` phrase) must NOT be
    /// mislabelled as an excluded feature — it falls back to `Parse`. Guards against the
    /// refined matcher over-firing.
    #[test]
    fn ordinary_syntax_error_detail_is_not_excluded() {
        for detail in [
            "syntax error: unexpected end of input (in kernel:1)",
            "syntax error: unexpected `}` (in kernel:1)",
            // An unknown tag that is NOT one of the six excluded keywords: still a parse
            // error, but not an excluded feature.
            "syntax error: unknown statement frobnicate (in kernel:1)",
        ] {
            assert!(
                !looks_like_excluded_feature(detail),
                "must NOT mislabel `{detail}` as an excluded feature",
            );
        }
    }
}
