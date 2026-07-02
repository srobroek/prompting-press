//! Derive guide — the starting pair: an `assistant` system prompt (a `company` +
//! `max_words` body) and a matching `AssistantVars`. Every later example on the page
//! derives from this. Standalone — `cargo run --example guides_derive_setup`.

use garde::Validate;
use prompting_press::Prompt;
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

    // The pair parses and validates: the body's {{ company }}/{{ max_words }} agree
    // with AssistantVars.
    let assistant = Prompt::from_yaml(assistant_yaml)?;
    assert_eq!(assistant.name(), "assistant");

    // AssistantVars is a plain garde-validated struct — construct one to prove the shape.
    let vars = AssistantVars {
        company: "Acme Robotics".into(),
        max_words: 50,
    };
    assert_eq!(vars.company, "Acme Robotics");
    assert_eq!(vars.max_words, 50);
    Ok(())
}
