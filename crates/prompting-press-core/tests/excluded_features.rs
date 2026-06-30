//! Excluded-feature regression suite (spec 002, T032; FR-002, FR-028, SC-008).
//!
//! Proves that **every** v1-excluded template construct is LOUDLY REJECTED: it never
//! renders, and it never passes the agreement analysis as a benign "requires no
//! variables" `Agreement`. This is the SC-008 guarantee and the V4.1/V4.2/V4.3 coverage.
//!
//! The six excluded constructs (constitution Principle IV / C-04; FR-002):
//!
//! - `{% include "x" %}`
//! - `{% extends "base" %}`
//! - `{% import "m" as m %}`
//! - `{% from "m" import f %}`
//! - `{% macro f() %}{% endmacro %}`
//! - `{% block b %}{% endblock %}`
//!
//! ## How the rejection works (research D1/D4)
//!
//! These are NOT enforced by hand-written kernel checks. The `minijinja` dependency is
//! built with `macros` and `multi_template` disabled, so those tags are *unrecognised*
//! and fail at **parse time** (`add_template_owned`). Both [`render`] and
//! [`required_roots`] parse first, so each returns `Err` before it could render or
//! observe `undeclared_variables`' empty-set-on-parse-error branch.
//!
//! ## Error-kind nuance (research D4, asserted below)
//!
//! [`KernelError`] distinguishes [`KernelError::ExcludedFeature`] from
//! [`KernelError::Parse`] via a best-effort heuristic over MiniJinja's message
//! (`engine::looks_like_excluded_feature`). Research D4 documents that the engine's
//! error kind may not always let the kernel label precisely; SC-008 requires only that
//! the construct is rejected as **one of those two** variants (never a successful render,
//! never an empty `Agreement`). The two hard-gate tests below therefore accept either
//! variant. Empirically (MiniJinja 2.21.0) all six emit `SyntaxError` with detail
//! `unknown statement <kw>`, which the refined heuristic maps to the precise
//! `ExcludedFeature` variant — locked by `excluded_features_classify_precisely_as_…`
//! below, and surfaced for the record by the `classification_table_diagnostic` test
//! (run with `--nocapture`), which prints the resulting `KernelError` variant + detail
//! per construct.
//!
//! ## Self-referential-grep mitigation (spec-001 lesson)
//!
//! Every excluded-feature template body lives in a JSON data fixture under
//! `tests/fixtures/defs/excluded-*.json` — NEVER inlined as a Rust string literal — so a
//! CI forbidden-pattern grep over `**/*.rs` (which defends the v1 boundary) is never
//! tripped by this suite's own NEGATIVE fixtures. This `.rs` file carries only fixture
//! *stems* and assertion logic; it contains no `{% … %}` template literals. See
//! `tests/fixtures/README.md`.

mod common;

use common::load_def_fixture;
use prompting_press_core::{render, required_roots, GuardConfig, KernelError};

/// The six excluded constructs, by fixture stem. Each fixture's `body` is a template that
/// uses exactly one excluded tag; the tag text lives in the JSON data file, not here.
const EXCLUDED_FIXTURES: [&str; 6] = [
    "excluded-include",
    "excluded-extends",
    "excluded-import",
    "excluded-from-import",
    "excluded-macro",
    "excluded-block",
];

fn no_guard() -> GuardConfig {
    GuardConfig { enabled: false }
}

/// `true` iff `err` is one of the two acceptable excluded-feature rejection variants
/// (SC-008): a precise [`KernelError::ExcludedFeature`] or the generic-but-loud
/// [`KernelError::Parse`] fallback. Anything else (e.g. `Render`, `UndefinedVariable`)
/// means the construct slipped through to a later stage — a soundness failure.
fn is_rejection(err: &KernelError) -> bool {
    matches!(
        err,
        KernelError::ExcludedFeature { .. } | KernelError::Parse { .. }
    )
}

/// V4.1/V4.2 — `render` rejects every excluded construct at parse time. The construct
/// must NEVER produce a `RenderResult` (no silent render), only an `ExcludedFeature` /
/// `Parse` error. [FR-002, SC-008]
#[test]
fn render_rejects_every_excluded_feature() {
    let empty = minijinja::Value::from_serialize(serde_json::json!({}));

    for stem in EXCLUDED_FIXTURES {
        let def = load_def_fixture(stem);
        match render(&def, None, empty.clone(), &no_guard()) {
            Ok(result) => panic!(
                "`{stem}`: an excluded feature must be rejected by render, \
                 not rendered to {:?}",
                result.text
            ),
            Err(err) => assert!(
                is_rejection(&err),
                "`{stem}`: render must reject with ExcludedFeature|Parse, got {err:?}",
            ),
        }
    }
}

/// V4.3 — agreement analysis rejects every excluded construct. It must NEVER return an
/// empty/successful `Agreement` (the FR-016a short-circuit): `undeclared_variables`
/// yields an empty set on parse failure, so a non-parsing excluded template would
/// otherwise masquerade as "requires no variables" and silently pass the headline
/// guarantee. [FR-002, FR-016a, SC-008]
#[test]
fn agreement_rejects_every_excluded_feature() {
    for stem in EXCLUDED_FIXTURES {
        let def = load_def_fixture(stem);
        match required_roots(&def, None) {
            Ok(agreement) => panic!(
                "`{stem}`: an excluded feature must error in required_roots, never yield a \
                 (benign) Agreement (got required_roots={:?})",
                agreement.required_roots
            ),
            Err(err) => assert!(
                is_rejection(&err),
                "`{stem}`: required_roots must reject with ExcludedFeature|Parse, got {err:?}",
            ),
        }
    }
}

/// Precision early-warning (NOT the SC-008 gate). Asserts that under MiniJinja 2.21.0
/// each of the six excluded constructs currently classifies *precisely* as
/// [`KernelError::ExcludedFeature`] (the `engine::looks_like_excluded_feature` heuristic
/// matches the `unknown statement <kw>` detail). SC-008 itself only requires
/// `ExcludedFeature | Parse` (covered by the two tests above) — this test exists so a
/// future MiniJinja bump that changes the parse-error wording, and thereby silently
/// downgrades these to the `Parse` fallback, fails LOUDLY here, prompting a re-audit of
/// the heuristic — rather than degrading classification precision unnoticed. [research D4]
#[test]
fn excluded_features_classify_precisely_as_excluded_feature() {
    let empty = minijinja::Value::from_serialize(serde_json::json!({}));

    for stem in EXCLUDED_FIXTURES {
        let def = load_def_fixture(stem);
        match render(&def, None, empty.clone(), &no_guard()) {
            Err(KernelError::ExcludedFeature { .. }) => {}
            other => panic!(
                "`{stem}`: expected precise ExcludedFeature under MiniJinja 2.21.0 \
                 (heuristic drift if this changed); got {other:?}",
            ),
        }
    }
}

/// Tightness guard for the refined heuristic: an ORDINARY syntax error (here an unclosed
/// `{{ … }}` interpolation) must classify as [`KernelError::Parse`], NEVER as
/// [`KernelError::ExcludedFeature`]. This proves the `unknown statement <kw>` matcher does
/// not over-fire — refining the heuristic to label real excluded features precisely must
/// not start mislabelling unrelated syntax errors as excluded features. [research D4]
#[test]
fn ordinary_syntax_error_is_parse_not_excluded_feature() {
    let def = load_def_fixture("malformed-syntax");
    let empty = minijinja::Value::from_serialize(serde_json::json!({}));

    match render(&def, None, empty, &no_guard()) {
        Err(KernelError::Parse { .. }) => {}
        other => {
            panic!("an ordinary syntax error must be Parse, never ExcludedFeature; got {other:?}",)
        }
    }
}

/// Diagnostic (run `cargo test -- --nocapture`): records, per construct, the actual
/// MiniJinja `ErrorKind` and the resulting `KernelError` variant — the research-D4
/// classification table. NOT an assertion of the precise variant (SC-008 only requires
/// one of the two); it documents reality so the heuristic can be audited on a MiniJinja
/// bump. The two tests above are the hard SC-008 gate.
#[test]
fn classification_table_diagnostic() {
    let empty = minijinja::Value::from_serialize(serde_json::json!({}));

    println!("\n--- excluded-feature classification (render path) ---");
    for stem in EXCLUDED_FIXTURES {
        let def = load_def_fixture(stem);
        match render(&def, None, empty.clone(), &no_guard()) {
            Ok(_) => println!("{stem}: UNEXPECTED Ok(_) (soundness failure)"),
            Err(e) => {
                let variant = match &e {
                    KernelError::ExcludedFeature { .. } => "ExcludedFeature",
                    KernelError::Parse { .. } => "Parse",
                    KernelError::UnknownVariant { .. } => "UnknownVariant",
                    KernelError::UndefinedVariable { .. } => "UndefinedVariable",
                    KernelError::Render { .. } => "Render",
                };
                println!("{stem}: KernelError::{variant} -- {e}");
            }
        }
    }
    println!("--- end ---\n");
}
