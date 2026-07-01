//! Derive guide — re-validation on overlay: overlaying a body that references an
//! undeclared variable is rejected over the merged whole (agreement failure).
//! Standalone — `cargo run --example guides_derive_revalidation_error`.

use prompting_press::{ConsumerError, Prompt, PromptOverlay};

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

    let bad = greet.derive(PromptOverlay {
        body: Some("Hi {{ ghost }}".to_string()),
        ..Default::default()
    });
    match bad {
        Err(ConsumerError::Kernel(rows)) => {
            assert_eq!(rows[0].code, "undefined_variable");
            assert_eq!(rows[0].field, "ghost");
        }
        _ => unreachable!("the merged definition is agreement-unsound"),
    }
    Ok(())
}
