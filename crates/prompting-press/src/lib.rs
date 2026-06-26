//! # prompting-press
//!
//! The public Rust consumer surface for Prompting Press: a typed, variant-aware
//! prompt-template library. Rust applications depend on **this** crate (not the kernel
//! directly) for a stable, idiomatic API; it re-exports and wraps the engine kernel,
//! [`prompting_press_core`].
//!
//! Prompting Press turns *typed inputs + a prompt template* into *rendered text +
//! provenance* — nothing else. It does **no** I/O, makes **no** model calls, and counts
//! **no** tokens. The headline guarantee is the **sound agreement check**: a template that
//! references a variable the prompt never declared is a caught error, not a silent empty
//! render.
//!
//! ## The four capabilities
//!
//! 1. **Registry + dual-input loader** ([`Registry`]) — a library-owned name →
//!    [`PromptDefinition`] map. Populate it three equal-footing ways:
//!    [`Registry::load_yaml`] / [`Registry::load_json`] (deserialize already-read text) or
//!    [`Registry::insert`] (a constructed object). All three normalize into the **same**
//!    `PromptDefinition` — there is no parallel shape (FR-005/006/008).
//! 2. **Validate-then-render** ([`render`](render()) / [`get_source`]) — pass a prompt **name** plus a
//!    typed Vars value; the vars are validated **once** (before any templating), serialized,
//!    and handed to the kernel, which returns a [`RenderResult`] (text + provenance).
//!    [`get_source`] returns a variant's unrendered template source.
//! 3. **The agreement + provenance lint** ([`check`](check())) — a pure CI pass over a [`Registry`]
//!    that reports undeclared-variable references and untrusted-input-without-guard prompts.
//!    See the [CI-usage example](#checkregistry-as-a-ci-gate) below.
//! 4. **Composition** ([`Composition`] / [`Message`]) — an explicit, ordered `Vec` of
//!    `(prompt, vars, variant)` entries (`append`, never `.chain()`) that resolves to an
//!    ordered list of `{role, text}` [`Message`]s (FR-012/013).
//!
//! ## Error normalization boundary (roadmap decision C-06)
//!
//! Every fallible call surfaces exactly one error type: [`ConsumerError`], carrying the
//! common structured shape `[{field, code, message}]` ([`FieldError`]). The two **native**
//! error sources — garde's `Report` and the kernel's `KernelError` — are normalized at this
//! boundary and **never appear** on a public signature. The `code` field is drawn from a
//! small, **closed vocabulary** (see [`error::code`]) — `validation`, `unknown_prompt`,
//! `unknown_variant`, `undefined_variable`, `parse`, `render`, `excluded_feature`, `load` —
//! so a consumer can `match` on `code` stably. Error messages are scrubbed: raw bound-value
//! content (a value that triggered a kernel `Parse`/`Render` error) never reaches a message
//! or a log derived from it (FR-014/015).
//!
//! ## This crate wraps the kernel — no logic is duplicated (roadmap decision C-01)
//!
//! Rendering, the agreement analysis, variant resolution, and SHA-256 hashing live **once**,
//! in [`prompting_press_core`]; this crate adds **none** of them. [`render`](render()) delegates
//! to the kernel's `render`; [`check`](check()) is set-arithmetic over the kernel's `required_roots` /
//! `provenance_view`; [`get_source`] delegates to the kernel's `get_source`. What this crate
//! adds is exactly what the kernel omits: the typed-Vars (garde) facade, the dual-input
//! loader, the lint-as-CI-pass, idiomatic render/compose ergonomics, and error
//! normalization. Cross-language byte-identity is therefore a structural property of the
//! shared core, not something re-tested here (constitution Principle I).
//!
//! ## Boundary: no I/O, no token counting, output-model is metadata only (roadmap decision C-03)
//!
//! - **No I/O.** The crate reads no files and opens no sockets. The caller hands in
//!   already-read YAML/JSON **text** ([`Registry::load_yaml`] / [`load_json`](Registry::load_json))
//!   or a constructed [`PromptDefinition`] ([`insert`](Registry::insert)) (FR-024).
//! - **`output_model` is carried as metadata only.** If a definition names an output model,
//!   it is echoed through and **never parsed or resolved** by this crate.
//! - **No token counting.** The token-count hook considered for this layer was **dropped**
//!   (spec 003, F4) and deferred to a later spec; the crate ships no token counter and
//!   exposes no token-count seam.
//!
//! ## The three-sets invariant (spec Assumptions / critique E1)
//!
//! Three field-name sets are in play for any one prompt, and the caller is responsible for
//! keeping the **third** aligned with the first two:
//!
//! 1. the prompt's declared `variables` block (the lint's authority);
//! 2. the template's referenced roots (computed by the kernel);
//! 3. the caller's garde Vars struct field names.
//!
//! [`check`](check()) lints **(2) ⊆ (1)** — template-roots are a subset of declared `variables`.
//! garde validates the *values* the struct **(3)** carries. But the **struct ↔ `variables`**
//! field-name agreement — does your `Vars` struct actually name the fields the prompt
//! declares? — is **the caller's responsibility**, not a check. It is **not silent**,
//! though: a misnamed struct field (e.g. a field `usrname` where the template references
//! `username`) serializes to a value map missing the referenced root, so the kernel's
//! strict-undefined environment fires and the failure surfaces here as a loud
//! [`ConsumerError::Kernel`] carrying an [`undefined_variable`](error::code::UNDEFINED_VARIABLE)
//! row — never an empty render. Closing this gap in-library would require per-prompt type
//! registration, which clarify Q3 deliberately rejected for v1; it is documented here and
//! pinned by a test, not enforced by an extra check.
//!
//! ## The `check()` provenance convention (roadmap decision C-09)
//!
//! A prompt that declares one or more `untrusted` / `external` variables is expected to
//! carry a top-level `"guard"` key in its `meta` (or `metadata`) map. If such a prompt
//! declares an untrusted/external field and **no** `"guard"` key is present, [`check`](check())
//! emits an [`UntrustedWithoutGuard`](check::FindingKind::UntrustedWithoutGuard) finding naming
//! the uncovered field. The lint reads `meta`/`metadata` read-only and checks only for the
//! *presence* of the key (the contents are opaque to the library). See the [`check`](mod@crate::check)
//! module docs for the full rationale.
//!
//! ## `check(registry)` as a CI gate
//!
//! [`check`](check()) is pure analysis: it mutates nothing, renders nothing, and returns a
//! [`CheckReport`]. A **non-empty** [`CheckReport::findings`] means the gate should fail
//! (exit non-zero). A CI entry point reads its prompts (as YAML/JSON it already has on disk),
//! loads them into a [`Registry`], runs `check`, and exits on any finding:
//!
//! ```
//! use prompting_press::{check, Registry};
//!
//! // In CI, `prompt_doc` would be the text of a repo YAML file the caller already read
//! // (this crate does no I/O — roadmap decision C-03). Here it is inline.
//! let prompt_doc = r#"
//! name: greet
//! role: user
//! body: "Hi {{ name }}, you have {{ count }} messages"
//! variables:
//!   name:  { type: string,  provenance: trusted }
//!   count: { type: integer, provenance: trusted }
//! "#;
//!
//! let mut registry = Registry::new();
//! registry.load_yaml(prompt_doc).expect("well-formed prompt definition");
//!
//! let report = check(&registry);
//!
//! // The CI gate: a non-empty findings list means fail (a real `main` would
//! // `std::process::exit(1)` here instead of asserting).
//! if !report.passed() {
//!     for finding in &report.findings {
//!         eprintln!("[{}] {}", finding.prompt, finding.detail);
//!     }
//!     // std::process::exit(1);
//! }
//! assert!(report.passed(), "this registry references only declared variables");
//! ```
//!
//! ## Render: validate typed Vars, then render
//!
//! ```
//! use garde::Validate;
//! use prompting_press::{render, Registry};
//! use prompting_press::core::GuardConfig;
//! use serde::Serialize;
//!
//! // Typed Vars: derives BOTH `serde::Serialize` (for the kernel-value bridge) and
//! // `garde::Validate` (for field validation). Its field names match the prompt's
//! // declared `variables` (the three-sets invariant — the caller's responsibility).
//! #[derive(Serialize, Validate)]
//! struct Greeting {
//!     #[garde(length(min = 1, max = 20))]
//!     name: String,
//!     #[garde(range(max = 100))]
//!     count: u32,
//! }
//!
//! let prompt_doc = r#"
//! name: greet
//! role: user
//! body: "Hi {{ name }}, you have {{ count }} messages"
//! variables:
//!   name:  { type: string,  provenance: trusted }
//!   count: { type: integer, provenance: trusted }
//! "#;
//!
//! let mut registry = Registry::new();
//! registry.load_yaml(prompt_doc).expect("well-formed prompt definition");
//!
//! let vars = Greeting { name: "Ada".to_string(), count: 3 };
//! // No guard expansion here, so a default (disabled) GuardConfig.
//! let result = render(&registry, "greet", &vars, None, &GuardConfig::default())
//!     .expect("valid vars render");
//!
//! assert_eq!(result.text, "Hi Ada, you have 3 messages");
//! assert_eq!(result.variant, "default");
//! assert_eq!(result.template_hash.len(), 64); // lowercase SHA-256 hex
//! ```

/// Re-export of the kernel, so consumers can reach core types through one entry point.
pub use prompting_press_core as core;

/// Re-export the generated `PromptDefinition` shape and its supporting types from the
/// kernel, so consumers reach them through this crate's public surface rather than
/// depending on the kernel directly. This crate re-exports but NEVER hand-edits the
/// generated module (which lives in `prompting-press-core`).
pub use prompting_press_core::generated::prompt_definition;
pub use prompting_press_core::generated::prompt_definition::PromptDefinition;

/// Re-export the kernel's `RenderResult` (library-owned render output; FR-009). The
/// consumer surfaces it 1:1 rather than redefining a parallel shape (C-01).
pub use prompting_press_core::RenderResult;

/// The normalized error surface: [`ConsumerError`] + [`FieldError`], the ONLY error type on
/// this crate's public API. garde `Report` / kernel `KernelError` are normalized here and
/// never leak (Principle VI / C-06; FR-014/FR-015).
pub mod error;

/// The prompt [`Registry`]: name → `PromptDefinition`. Backed by a `BTreeMap` for
/// deterministic `check()` ordering (FR-008a).
pub mod registry;

/// Validate-then-render + `get_source` wrappers over the kernel (FR-001..003a, FR-009/010).
pub mod render;

/// The agreement + provenance lint: a pure CI pass over the [`Registry`] that catches
/// undeclared-variable references and untrusted-input-without-guard prompts (FR-016..020).
pub mod check;

/// Multi-message composition: an explicit ordered `Vec` of `(prompt, vars, variant)` entries
/// (`append_*`, never `.chain()`) resolving to `[{role, text}]` messages (FR-012/013).
pub mod compose;

pub use error::{ConsumerError, FieldError};
pub use registry::Registry;

/// Re-export the validate-then-render entry points at the crate root so applications reach
/// them as `prompting_press::render` / `prompting_press::get_source`.
pub use render::{get_source, render};

/// Re-export the lint entry point + its report types at the crate root so applications reach
/// them as `prompting_press::check` / `prompting_press::{CheckReport, Finding, FindingKind}`.
pub use check::{check, CheckReport, Finding, FindingKind};

/// Re-export the composition types at the crate root so applications reach them as
/// `prompting_press::{Composition, Message}`.
pub use compose::{Composition, Message};

/// Returns the underlying kernel version.
///
/// Trivial placeholder that calls into the kernel, making the dependency edge
/// load-bearing rather than declarative-only.
#[must_use]
pub fn core_version() -> &'static str {
    prompting_press_core::version()
}
