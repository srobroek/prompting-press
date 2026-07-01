//! Constructing a three-arm prompt inline from a typed shape object — the
//! `variants` map mirrors the document form one-to-one and runs the same
//! validation as `Prompt::from_yaml`. Standalone:
//! `cargo run --example guides_variants_construct_inline`.

use prompting_press::{Prompt, PromptDefinition};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let def: PromptDefinition = serde_json::from_value(serde_json::json!({
        "name": "summary",
        "role": "user",
        "body": "Summarise the following article in {{ max_words }} words:\n\n{{ article }}",
        "variables": {
            "article":   { "type": "string",  "trusted": false },
            "max_words": { "type": "integer", "trusted": true }
        },
        "variants": {
            "concise":    { "body": "In one sentence, summarise: {{ article }}" },
            "structured": {
                "body": "Summarise {{ article }} as a title, three bullets, and a one-line conclusion.",
                "metadata": { "group": "experiment-q4" }
            }
        }
    }))?;

    let summary = Prompt::new(def)?; // same validation as Prompt::from_yaml
    assert!(summary.variants().contains_key("concise")); // true
    Ok(())
}
