//! Derive guide — add a variant at runtime: READ the current variants with the
//! `.variants()` accessor, add to a clone, then WRITE the merged map back via the sole
//! mutator `derive`. The original is untouched.
//! Standalone — `cargo run --example guides_derive_add_variant`.

use prompting_press::{Prompt, PromptOverlay};
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let greet_yaml = r#"
name: greet
role: user
body: "Hi {{ name }}, you have {{ count }} messages."
variables:
  name: { type: string, trusted: true }
  count: { type: integer, trusted: true }
"#;

    let greet = Prompt::from_yaml(greet_yaml)?;

    // READ the current variants, then add to a clone — so existing arms survive.
    let mut variants = greet.variants().clone();
    variants.insert(
        "formal".to_string(),
        serde_json::from_value(json!({
            "body": "Good day, {{ name }}. You have {{ count }} messages."
        }))?,
    );

    // WRITE the merged map back via the sole mutator.
    let formal_greet = greet.derive(PromptOverlay {
        variants: Some(variants),
        ..Default::default()
    })?;
    // greet is unchanged; formal_greet is a new, fully-validated Prompt.

    assert!(greet.variants().is_empty(), "original is untouched");
    assert!(formal_greet.variants().contains_key("formal"));
    Ok(())
}
