//! Derive guide — re-validation on overlay: overlaying a body that references an
//! undeclared variable is rejected over the merged whole (agreement failure).
//! Standalone — `cargo run --example guides_derive_revalidation_error`.

use prompting_press::{ConsumerError, Prompt, PromptOverlay};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/examples");
    let assistant = Prompt::from_yaml(&fs::read_to_string(format!("{dir}/assistant.yaml"))?)?;

    let bad = assistant.derive(PromptOverlay {
        body: Some("You help {{ ghost }}.".to_string()),
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
