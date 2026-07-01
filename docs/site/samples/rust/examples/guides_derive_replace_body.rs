//! Derive guide — replace only the root body (the default arm) with `derive`.
//! Standalone — `cargo run --example guides_derive_replace_body`.

use garde::Validate;
use prompting_press::{GuardConfig, Prompt, PromptOverlay};
use serde::Serialize;

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

    let brief_greet = greet.derive(PromptOverlay {
        body: Some("Hi {{ name }}!".to_string()),
        ..Default::default()
    })?;

    let vars = GreetVars {
        name: "Ada".into(),
        count: 3,
    };
    let result = brief_greet.render(&vars, None, &GuardConfig::default(), false)?;
    assert_eq!(result.text, "Hi Ada!");
    Ok(())
}
