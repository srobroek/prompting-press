//! Derive guide — render a named variant: after adding a variant with `derive`, select
//! it by name at render time. Variant selection is caller-owned.
//! Standalone — `cargo run --example guides_derive_render_variant`.

use garde::Validate;
use prompting_press::{GuardConfig, Prompt, PromptOverlay};
use serde::Serialize;
use serde_json::json;

#[derive(Serialize, Validate)]
struct GreetVars {
    #[garde(length(min = 1))]
    name: String,
    #[garde(range(min = 0))]
    count: i64,
}

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
    let mut variants = greet.variants().clone();
    variants.insert(
        "formal".to_string(),
        serde_json::from_value(json!({
            "body": "Good day, {{ name }}. You have {{ count }} messages."
        }))?,
    );
    let formal_greet = greet.derive(PromptOverlay {
        variants: Some(variants),
        ..Default::default()
    })?;

    let vars = GreetVars {
        name: "Ada".into(),
        count: 3,
    };
    let result = formal_greet.render(&vars, Some("formal"), &GuardConfig::default(), false)?;
    assert_eq!(result.text, "Good day, Ada. You have 3 messages.");
    assert_eq!(result.variant, "formal");
    Ok(())
}
