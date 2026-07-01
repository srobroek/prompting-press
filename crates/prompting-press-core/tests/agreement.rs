//! US2 agreement-analysis suite (spec 002, T021): the sound required-roots report.
//!
//! Covers quickstart scenarios V2.1, V2.2, V2.3, V2.4, V2.6 and V4.3. Template bodies
//! live in JSON data fixtures (`tests/fixtures/defs/*.json`), never inlined here, per the
//! self-referential-grep mitigation (see `tests/fixtures/README.md`). Each fixture's
//! `body` is the template under analysis; `required_roots(&def, None)` analyses the root
//! `body` arm (the reserved `default`).
//!
//! Soundness property (constitution Principle IV / C-04, SC-002): the reported set is the
//! externally-supplied ROOT names only — it excludes loop locals, `{% set %}` targets, and
//! engine-provided globals, and reports a root, never a nested field. The KERNEL (via
//! `MiniJinja`'s stable `undeclared_variables(false)` + the env-derived globals allowlist)
//! guarantees these exclusions; these tests verify that guarantee, they do not re-implement
//! the filtering.

mod common;

use std::collections::BTreeSet;

use common::{load_def_fixture, load_prompt_definition};
use prompting_press_core::{required_roots, KernelError};

/// Convenience: build the expected `BTreeSet<String>` from string literals.
fn roots<const N: usize>(names: [&str; N]) -> BTreeSet<String> {
    names.iter().map(|s| (*s).to_string()).collect()
}

/// V2.1 — `"{{ greeting }}, {{ user.name }}"` → roots `{greeting, user}`.
///
/// The nested field `name` is NOT a root — deep field shape is the type system's job, so
/// `undeclared_variables(false)` reports only the root `user`. [FR-016, SC-002]
#[test]
fn v2_1_interpolation_reports_roots_not_nested_fields() {
    let def = load_def_fixture("agreement-nested");

    let agreement = required_roots(&def, None).expect("analysis must succeed");

    assert_eq!(agreement.variant, "default");
    assert_eq!(
        agreement.required_roots,
        roots(["greeting", "user"]),
        "nested field `name` must not appear; only the roots greeting + user",
    );
    assert!(
        !agreement.required_roots.contains("name"),
        "the nested field `name` must not be reported as a root",
    );
}

/// V2.2 — `"{% for item in items %}{{ item }}{% endfor %}"` → roots `{items}`.
///
/// The loop local `item` is EXCLUDED by the engine analysis (soundness, strictly better
/// than a naive scan). [FR-017, SC-002]
#[test]
fn v2_2_loop_local_is_excluded() {
    let def = load_def_fixture("agreement-loop");

    let agreement = required_roots(&def, None).expect("analysis must succeed");

    assert_eq!(
        agreement.required_roots,
        roots(["items"]),
        "the loop local `item` must be excluded; only `items` is a root",
    );
    assert!(
        !agreement.required_roots.contains("item"),
        "the loop variable `item` must not be reported",
    );
}

/// V2.3 — `"{% set x = 1 %}{{ x }}{{ y }}"` → roots `{y}`.
///
/// The `{% set %}` target `x` is EXCLUDED; only the externally-supplied root `y` remains.
/// [FR-017, SC-002]
#[test]
fn v2_3_set_target_is_excluded() {
    let def = load_def_fixture("agreement-set");

    let agreement = required_roots(&def, None).expect("analysis must succeed");

    assert_eq!(
        agreement.required_roots,
        roots(["y"]),
        "the `{{% set %}}` target `x` must be excluded; only `y` is a root",
    );
    assert!(
        !agreement.required_roots.contains("x"),
        "the set target `x` must not be reported",
    );
}

/// V2.4 (range) — a template using the `range` engine global → the global name is absent;
/// the real root `n` is present. The allowlist is derived from the env's own globals
/// (FR-020), so `range` is subtracted and `i` (a loop local) is excluded by the engine.
#[test]
fn v2_4_range_global_absent_real_root_present() {
    let def = load_def_fixture("agreement-global");

    let agreement = required_roots(&def, None).expect("analysis must succeed");

    assert!(
        agreement.required_roots.contains("n"),
        "the external root `n` must be reported, got {:?}",
        agreement.required_roots,
    );
    assert!(
        !agreement.required_roots.contains("range"),
        "the engine global `range` must be allowlisted out, got {:?}",
        agreement.required_roots,
    );
    assert!(
        !agreement.required_roots.contains("i"),
        "the loop local `i` must be excluded",
    );
    assert_eq!(
        agreement.required_roots,
        roots(["n"]),
        "only the external root `n` remains after subtracting the env globals + loop local",
    );
}

/// V2.4 (namespace) — a template using the `namespace` engine global → the global name is
/// absent; the real root `n` is present. Proves the env-derived allowlist covers the full
/// global surface, not just `range`. [FR-017, FR-020]
#[test]
fn v2_4_namespace_global_absent_real_root_present() {
    let def = load_def_fixture("agreement-namespace");

    let agreement = required_roots(&def, None).expect("analysis must succeed");

    assert!(
        agreement.required_roots.contains("n"),
        "the external root `n` must be reported, got {:?}",
        agreement.required_roots,
    );
    assert!(
        !agreement.required_roots.contains("namespace"),
        "the engine global `namespace` must be allowlisted out, got {:?}",
        agreement.required_roots,
    );
    assert_eq!(
        agreement.required_roots,
        roots(["n"]),
        "only the external root `n` remains after subtracting `namespace` + the set target",
    );
}

/// V2.6 — `"{{ foo }}"` where `foo` is undeclared → `foo` appears in the required-roots
/// set, making it DETECTABLE rather than rendering to a silent empty value. [FR-016, SC-003]
#[test]
fn v2_6_undeclared_reference_is_detectable() {
    let def = load_def_fixture("agreement-undeclared");

    let agreement = required_roots(&def, None).expect("analysis must succeed");

    assert!(
        agreement.required_roots.contains("foo"),
        "`foo` must surface as a required root (detectable, not silent), got {:?}",
        agreement.required_roots,
    );
    assert_eq!(agreement.required_roots, roots(["foo"]));
}

/// V4.3 — analysing an excluded-feature template (an `{% include %}` body) MUST return an
/// `Err`, NEVER an empty/successful `Agreement`. This is the FR-016a short-circuit: the
/// underlying `undeclared_variables` yields an empty set on parse failure, so a broken or
/// excluded-feature template would otherwise masquerade as "requires no variables" and
/// silently pass the headline guarantee. [FR-016a, FR-028, research D2]
#[test]
fn v4_3_excluded_feature_template_errors_never_empty_agreement() {
    let def = load_def_fixture("agreement-excluded-include");

    let err = required_roots(&def, None)
        .expect_err("an excluded-feature template must error, never yield an empty Agreement");

    assert!(
        matches!(
            err,
            KernelError::ExcludedFeature { .. } | KernelError::Parse { .. }
        ),
        "expected ExcludedFeature or Parse, got {err:?}",
    );
}

/// TS-I5 — per-variant analysis works for a NAMED arm, not just the reserved `default`.
///
/// `required_roots(&def, Some("concise"))` analyses the `concise` arm
/// (`"In one sentence, summarise: {{article}}"`) → roots `{article}`, with the reported
/// `variant == "concise"`. Proves the analysis resolves and reports per-variant. [FR-016]
#[test]
fn ts_i5_named_variant_agreement_analyses_that_arm() {
    let def = load_prompt_definition("multi-variant");

    let agreement =
        required_roots(&def, Some("concise")).expect("named-variant analysis must succeed");

    assert_eq!(agreement.variant, "concise");
    assert_eq!(
        agreement.required_roots,
        roots(["article"]),
        "the concise arm references exactly the root `article`",
    );
}

/// TS-I1 (agreement half) — a prompt with an empty `body` has an empty required-roots set
/// (nothing referenced ⇒ nothing required), reported under the reserved `default` arm.
/// [spec Edge Cases]
#[test]
fn ts_i1_empty_body_has_empty_required_roots() {
    let def = load_def_fixture("empty-body");

    let agreement = required_roots(&def, None).expect("analysis of an empty body must succeed");

    assert_eq!(agreement.variant, "default");
    assert!(
        agreement.required_roots.is_empty(),
        "an empty body references no roots, got {:?}",
        agreement.required_roots,
    );
}
