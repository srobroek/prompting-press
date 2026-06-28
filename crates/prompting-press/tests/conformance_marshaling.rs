//! Conformance corpus — Rust marshaling runner (spec 006, T007; US1).
//!
//! Drives every `conformance/marshaling/*.json` fixture through the REAL consumer render path
//! (`prompting_press::render`) and asserts the rendered text + `template_hash` + `render_hash` equal the
//! committed golden. Because all three bindings assert against the SAME committed golden, cross-binding
//! parity is transitive (FR-005). NOTE (critique E2): the goldens are generated FROM this Rust reference
//! binding, so this Rust marshaling assertion is effectively a DETERMINISM guard (render twice → same
//! golden); the independent marshaling-parity votes are the Python and TS runners. The Rust leg's
//! genuinely-independent parity check is the schema round-trip (conformance_schema.rs).

mod common;

use common::{build_vars, load_marshaling_fixtures, RawVars};
use prompting_press::{render, Registry};
use prompting_press_core::{GuardConfig, PromptDefinition};

#[test]
fn marshaling_fixtures_match_golden() {
    let fixtures = load_marshaling_fixtures();
    assert!(!fixtures.is_empty(), "no marshaling fixtures found");

    let mut failures = Vec::new();

    for (path, fx) in &fixtures {
        let def: PromptDefinition = serde_json::from_value(fx.definition.clone())
            .unwrap_or_else(|e| panic!("{}: invalid prompt definition: {e}", fx.case));
        let name = def.name.clone();
        let mut reg = Registry::new();
        reg.insert(def);

        let vars = RawVars(build_vars(&fx.input));
        let result = render(
            &reg,
            &name,
            &vars,
            fx.variant.as_deref(),
            &GuardConfig::default(),
        )
        .unwrap_or_else(|e| panic!("{} ({}): render failed: {e:?}", fx.case, path.display()));

        // Guard against an un-regenerated fixture: an empty golden means T006 was never run.
        assert!(
            !fx.expected.text.is_empty()
                && !fx.expected.template_hash.is_empty()
                && !fx.expected.render_hash.is_empty(),
            "{}: golden is empty — run `moon run conformance:regen` (T006)",
            fx.case
        );

        if result.text != fx.expected.text {
            failures.push(format!(
                "[rust] case={} divergence=text: got {:?}, golden {:?}",
                fx.case, result.text, fx.expected.text
            ));
        }
        if result.template_hash != fx.expected.template_hash {
            failures.push(format!(
                "[rust] case={} divergence=template_hash: got {}, golden {}",
                fx.case, result.template_hash, fx.expected.template_hash
            ));
        }
        if result.render_hash != fx.expected.render_hash {
            failures.push(format!(
                "[rust] case={} divergence=render_hash: got {}, golden {}",
                fx.case, result.render_hash, fx.expected.render_hash
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "marshaling parity divergences (binding+case+kind):\n{}",
        failures.join("\n")
    );
}
