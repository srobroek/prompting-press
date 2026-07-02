//! Render the `Prompt` with the typed `AssistantVars` and read the result fields.
//! Standalone — `cargo run --example getting-started_rust_render_and_read`.

use garde::Validate;
use prompting_press::{ConsumerError, GuardConfig, Prompt};
use serde::Serialize;
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

    let vars = AssistantVars {
        company: "Acme Robotics".into(),
        max_words: 50,
    };

    let result = assistant.render(&vars, None, &GuardConfig::default(), false)?;

    assert_eq!(
        result.text,
        "You are a support assistant for Acme Robotics. Keep your replies under 50 words."
    ); // the rendered body
    assert_eq!(result.variant, "default"); // no variant selected → the default arm

    // template_hash / render_hash are 64-char lowercase-hex SHA-256 strings.
    assert_eq!(result.template_hash.len(), 64);
    assert!(result
        .template_hash
        .chars()
        .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    assert_eq!(result.render_hash.len(), 64);
    assert!(result
        .render_hash
        .chars()
        .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));

    // Typed Vars are validated at render, not just declared: `max_words: 0` violates
    // `#[garde(range(min = 1))]`, so the kernel is never reached — render rejects it.
    let bad_vars = AssistantVars {
        company: "Acme Robotics".into(),
        max_words: 0,
    };
    match assistant.render(&bad_vars, None, &GuardConfig::default(), false) {
        Err(ConsumerError::Validation(rows)) => {
            assert!(rows.iter().any(|r| r.field == "max_words"));
        }
        other => panic!("expected a validation error for max_words = 0, got {other:?}"),
    }

    Ok(())
}
