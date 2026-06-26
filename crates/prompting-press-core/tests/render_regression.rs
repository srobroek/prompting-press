//! Engine-regression render-fixture suite (spec 002, FR-029).
//!
//! FR-029 calls for "a small engine-regression render-fixture set that pins
//! representative template -> output results as a regression guard." This file is
//! that guard: it loads each `tests/fixtures/render/*.json` case (template, values,
//! and expected output), drives it through the kernel `render()`, and asserts the
//! rendered text equals the pinned `expected` byte-for-byte.
//!
//! Template bodies live exclusively in the JSON data fixtures (loaded at runtime by
//! `common::load_regression_case`), never inlined here, per the self-referential-grep
//! mitigation (see `tests/fixtures/README.md`).

mod common;

use common::{load_regression_case, RegressionCase};
use prompting_press_core::{render, GuardConfig, PromptDefinition};

/// The render fixtures this guard pins. Each names a `tests/fixtures/render/<stem>.json`
/// data file. New fixtures are covered by adding their stem here.
const RENDER_FIXTURES: &[&str] = &["interpolation", "conditional-loop"];

/// A disabled guard config — these regression cases render plain template features
/// (interpolation, conditionals, loops) and never opt into guard expansion (US3 owns that).
fn no_guard() -> GuardConfig {
    GuardConfig {
        enabled: false,
        template: None,
    }
}

/// Build a minimal default-arm `PromptDefinition` whose root `body` is the fixture's
/// template, mirroring how `tests/fixtures/defs/*.json` deserialize. Constructed via
/// serde_json so the kernel shape is exercised exactly as a real definition would be.
fn def_from_case(case: &RegressionCase) -> PromptDefinition {
    serde_json::from_value(serde_json::json!({
        "name": "render-regression",
        "role": "user",
        "body": case.template,
    }))
    .expect("regression-case template forms a valid default-arm PromptDefinition")
}

/// FR-029 — every pinned render fixture renders to its recorded `expected` output.
///
/// This is the actual regression guard: a behavioral change in the kernel's render
/// path that alters any pinned output breaks this test.
#[test]
fn render_fixtures_match_pinned_output() {
    for &name in RENDER_FIXTURES {
        let case = load_regression_case(name);
        let def = def_from_case(&case);
        let values = minijinja::Value::from_serialize(&case.values);

        let result = render(&def, None, values, &no_guard())
            .unwrap_or_else(|e| panic!("render fixture `{name}` must succeed: {e:?}"));

        assert_eq!(
            result.text, case.expected,
            "render fixture `{name}` output drifted from pinned expected"
        );
        assert_eq!(
            result.variant, "default",
            "regression fixture `{name}` renders the default arm"
        );
    }
}
