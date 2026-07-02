//! Derive guide — render a named variant: after adding a variant with `derive`, select
//! it by name at render time. Variant selection is caller-owned.
//! Standalone — `cargo run --example guides_derive_render_variant`.

use garde::Validate;
use prompting_press::{GuardConfig, Prompt, PromptOverlay};
use serde::Serialize;
use serde_json::json;
use std::fs;

#[derive(Serialize, Validate)]
struct AssistantVars {
    #[garde(length(min = 1))]
    company: String,
    #[garde(range(min = 1))]
    max_words: i64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/examples");
    let assistant = Prompt::from_yaml(&fs::read_to_string(format!("{dir}/assistant.yaml"))?)?;
    let mut variants = assistant.variants().clone();
    variants.insert(
        "formal".to_string(),
        serde_json::from_value(json!({
            "body": "You are the official support assistant for {{ company }}. Please keep every reply under {{ max_words }} words."
        }))?,
    );
    let formal_assistant = assistant.derive(PromptOverlay {
        variants: Some(variants),
        ..Default::default()
    })?;

    let vars = AssistantVars {
        company: "Acme Robotics".into(),
        max_words: 50,
    };
    let result = formal_assistant.render(&vars, Some("formal"), &GuardConfig::default(), false)?;
    assert_eq!(
        result.text,
        "You are the official support assistant for Acme Robotics. Please keep every reply under 50 words."
    );
    assert_eq!(result.variant, "formal");
    Ok(())
}
