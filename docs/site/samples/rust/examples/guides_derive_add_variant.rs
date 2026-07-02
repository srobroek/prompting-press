//! Derive guide — add a variant at runtime: READ the current variants with the
//! `.variants()` accessor, add to a clone, then WRITE the merged map back via the sole
//! mutator `derive`. The original is untouched.
//! Standalone — `cargo run --example guides_derive_add_variant`.

use prompting_press::{Prompt, PromptOverlay};
use serde_json::json;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/examples");
    let assistant = Prompt::from_yaml(&fs::read_to_string(format!("{dir}/assistant.yaml"))?)?;

    // READ the current variants, then add to a clone — so existing arms survive.
    let mut variants = assistant.variants().clone();
    variants.insert(
        "formal".to_string(),
        serde_json::from_value(json!({
            "body": "You are the official support assistant for {{ company }}. Please keep every reply under {{ max_words }} words."
        }))?,
    );

    // WRITE the merged map back via the sole mutator.
    let formal_assistant = assistant.derive(PromptOverlay {
        variants: Some(variants),
        ..Default::default()
    })?;
    // assistant is unchanged; formal_assistant is a new, fully-validated Prompt.

    assert!(assistant.variants().is_empty(), "original is untouched");
    assert!(formal_assistant.variants().contains_key("formal"));
    Ok(())
}
