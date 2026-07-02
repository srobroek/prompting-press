//! `ConsumerError` is a closed three-variant enum — the render match is exhaustive.
//! Standalone — `cargo run --example getting-started_rust_error_types`.

use garde::Validate;
use prompting_press::{error::code, ConsumerError, GuardConfig, Prompt};
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

    // An empty company violates `#[garde(length(min = 1))]` → ConsumerError::Validation.
    let vars = AssistantVars {
        company: String::new(),
        max_words: 50,
    };

    // ConsumerError is a closed three-variant enum — the match is exhaustive.
    match assistant.render(&vars, None, &GuardConfig::default(), false) {
        Ok(_result) => { /* ... */ }
        Err(ConsumerError::Validation(rows)) => {
            for row in &rows {
                eprintln!("{}: {} [{}]", row.field, row.message, row.code);
            }
            // Every validation row carries the stable `"validation"` code.
            assert!(rows.iter().all(|r| r.code == code::VALIDATION));
            assert!(rows.iter().any(|r| r.field == "company"));
            return Ok(());
        }
        Err(ConsumerError::Kernel(_rows)) => { /* parse/render/agreement failure */ }
        Err(ConsumerError::Load(_msg)) => { /* malformed YAML/JSON/TOML */ }
    }

    Err("expected a validation error for the empty name".into())
}
