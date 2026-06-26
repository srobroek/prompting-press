//! Scaffold smoke test for the spec-002 fixture harness (T012).
//!
//! Proves both loader paths work end to end:
//! - a render regression case deserializes from a `tests/fixtures/render/*.json` data file;
//! - a spec-001 valid schema fixture deserializes into the kernel's `PromptDefinition`.
//!
//! The full per-user-story suites (render/agreement/provenance) land in later tasks and
//! reuse `common`. This file is the minimum that exercises the harness.

mod common;

use common::{load_prompt_definition, load_regression_case};

#[test]
fn regression_case_loads_from_data_file() {
    let case = load_regression_case("interpolation");
    // Template + expected come from the JSON data file, not Rust source.
    assert!(!case.template.is_empty(), "fixture must carry a template");
    assert!(
        !case.expected.is_empty(),
        "fixture must carry an expected output"
    );
    assert!(
        case.values.is_object(),
        "fixture values must be a JSON object"
    );
}

#[test]
fn schema_fixture_deserializes_into_prompt_definition() {
    let def = load_prompt_definition("single-body");
    // The kernel reads the spec-001 shape it consumes (FR-027).
    assert_eq!(&*def.name, "greeting");
    assert!(!def.body.is_empty(), "default arm body must be present");
}

#[test]
fn multi_variant_schema_fixture_deserializes() {
    let def = load_prompt_definition("multi-variant");
    assert_eq!(&*def.name, "content-summariser");
    assert!(
        def.variants.contains_key("concise"),
        "multi-variant fixture must expose named variants"
    );
}
