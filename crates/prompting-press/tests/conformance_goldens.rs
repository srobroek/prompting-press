//! Conformance corpus — GOLDEN GENERATOR (spec 006, T005; research D3).
//!
//! This is the REGENERATION tool, not a gated test. It is `#[ignore]`d so the CI conformance gate never
//! runs it (the gate would otherwise rewrite the committed goldens). Run it deliberately via
//! `moon run conformance:regen` (or `cargo test -p prompting-press --test conformance_goldens -- --ignored`).
//!
//! It renders each `conformance/marshaling/*.json` fixture through the **Rust reference binding**
//! (`prompting_press::render`) and writes `expected.{text, template_hash, render_hash}` back into the
//! fixture file. The runners (Rust/Python/TS) then assert against these committed goldens, so cross-binding
//! parity is transitive (all three == the one golden) and the golden also trips a lockstep kernel
//! regression. A golden change is a deliberate, PR-reviewed event — do NOT regenerate to silence a red
//! runner; investigate the divergence first (it may be the real marshaling bug the corpus exists to catch).

mod common;

use common::{build_vars, load_marshaling_fixtures, RawVars};
use prompting_press::Prompt;
use prompting_press_core::GuardConfig;

/// Regenerate the goldens in every marshaling fixture. `#[ignore]`d: never runs in the conformance gate.
#[test]
#[ignore = "regeneration tool, not a gated test; run via `moon run conformance:regen`"]
fn regenerate_marshaling_goldens() {
    let fixtures = load_marshaling_fixtures();
    assert!(
        !fixtures.is_empty(),
        "no marshaling fixtures found to regenerate"
    );

    for (path, fx) in &fixtures {
        // 1. Load the prompt definition into a Prompt (the constructed-object path: definition
        //    JSON -> PromptDefinition via serde, then Prompt::new).
        let def: prompting_press_core::PromptDefinition =
            serde_json::from_value(fx.definition.clone())
                .unwrap_or_else(|e| panic!("{}: invalid prompt definition: {e}", fx.case));
        let prompt =
            Prompt::new(def).unwrap_or_else(|e| panic!("{}: Prompt::new failed: {e:?}", fx.case));

        // 2. Build the Vars value from the typed `input` (Rust arm of the D2 mapping; absent => dropped).
        let vars = RawVars(build_vars(&fx.input));

        // 3. Render via the REAL consumer render path (no engine logic here — C-01/C-02).
        let result = prompt
            .render(&vars, fx.variant.as_deref(), &GuardConfig::default())
            .unwrap_or_else(|e| panic!("{}: render failed: {e:?}", fx.case));

        // 4. Write the goldens back into the fixture JSON, preserving everything else.
        let mut doc: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).expect("re-read fixture"))
                .expect("re-parse fixture");
        let expected = doc
            .get_mut("expected")
            .and_then(|v| v.as_object_mut())
            .expect("fixture has an `expected` object");
        expected.insert(
            "text".into(),
            serde_json::Value::String(result.text.clone()),
        );
        expected.insert(
            "template_hash".into(),
            serde_json::Value::String(result.template_hash.clone()),
        );
        expected.insert(
            "render_hash".into(),
            serde_json::Value::String(result.render_hash.clone()),
        );

        let mut serialized = serde_json::to_string_pretty(&doc).expect("serialize fixture");
        serialized.push('\n'); // trailing newline (matches the authored files)
        std::fs::write(path, serialized).expect("write fixture goldens");

        eprintln!(
            "regenerated {}: template_hash={} render_hash={}",
            fx.case, result.template_hash, result.render_hash
        );
    }
}
