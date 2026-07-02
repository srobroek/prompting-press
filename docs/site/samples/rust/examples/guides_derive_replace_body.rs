//! Derive guide — replace only the root body (the default arm) with `derive`.
//! Standalone — `cargo run --example guides_derive_replace_body`.

use garde::Validate;
use prompting_press::{GuardConfig, Prompt, PromptOverlay};
use serde::Serialize;

#[derive(Serialize, Validate)]
struct AssistantVars {
    #[garde(length(min = 1))]
    company: String,
    #[garde(range(min = 1))]
    max_words: i64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assistant_yaml = r#"
name: assistant
role: system
body: "You are a support assistant for {{ company }}. Keep your replies under {{ max_words }} words."
variables:
  company: { type: string, trusted: true }
  max_words: { type: integer, trusted: true }
"#;
    let assistant = Prompt::from_yaml(assistant_yaml)?;

    let brief_assistant = assistant.derive(PromptOverlay {
        body: Some("You are a support assistant for {{ company }}.".to_string()),
        ..Default::default()
    })?;

    let vars = AssistantVars {
        company: "Acme Robotics".into(),
        max_words: 50,
    };
    let result = brief_assistant.render(&vars, None, &GuardConfig::default(), false)?;
    assert_eq!(result.text, "You are a support assistant for Acme Robotics.");
    Ok(())
}
