//! Selecting a variant at render: omit the name for the default (root body),
//! pass a name for that arm. The resolved name comes back on
//! `RenderResult.variant` and the text is that arm's rendered body. Standalone:
//! `cargo run --example guides_variants_select`.

use garde::Validate;
use prompting_press::{GuardConfig, Prompt};
use serde::Serialize;
use std::fs;

#[derive(Serialize, Validate)]
struct SummaryVars {
    #[garde(length(min = 1))]
    article: String,
    #[garde(range(min = 1))]
    max_words: i64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/examples");
    let summary = Prompt::from_yaml(&fs::read_to_string(format!("{dir}/summary_select.yaml"))?)?;
    let vars = SummaryVars {
        article: "The Nile floods yearly.".into(),
        max_words: 20,
    };

    let def = summary.render(&vars, None, &GuardConfig::default(), false)?; // default (root body)
    let concise = summary.render(&vars, Some("concise"), &GuardConfig::default(), false)?;

    assert_eq!(def.variant, "default");
    assert_eq!(
        def.text,
        "Summarise the following article in 20 words:\n\nThe Nile floods yearly."
    );
    assert_eq!(concise.variant, "concise");
    assert_eq!(
        concise.text,
        "In one sentence, summarise: The Nile floods yearly."
    );
    Ok(())
}
