//! `ConsumerError` is a closed three-variant enum — the render match is exhaustive.
//! Standalone — `cargo run --example getting-started_rust_error_types`.

use garde::Validate;
use prompting_press::{error::code, ConsumerError, GuardConfig, Prompt};
use serde::Serialize;

#[derive(Serialize, Validate)]
struct GreetVars {
    #[garde(length(min = 1))]
    name: String,
    #[garde(range(min = 0))]
    count: i64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let greet = Prompt::from_yaml(
        r#"
name: greet
role: user
body: "Hi {{ name }}, you have {{ count }} messages."
variables:
  name:
    type: string
    trusted: true
  count:
    type: integer
    trusted: true
"#,
    )?;

    // An empty name violates `#[garde(length(min = 1))]` → ConsumerError::Validation.
    let vars = GreetVars {
        name: String::new(),
        count: 3,
    };

    // ConsumerError is a closed three-variant enum — the match is exhaustive.
    match greet.render(&vars, None, &GuardConfig::default(), false) {
        Ok(_result) => { /* ... */ }
        Err(ConsumerError::Validation(rows)) => {
            for row in &rows {
                eprintln!("{}: {} [{}]", row.field, row.message, row.code);
            }
            // Every validation row carries the stable `"validation"` code.
            assert!(rows.iter().all(|r| r.code == code::VALIDATION));
            assert!(rows.iter().any(|r| r.field == "name"));
            return Ok(());
        }
        Err(ConsumerError::Kernel(_rows)) => { /* parse/render/agreement failure */ }
        Err(ConsumerError::Load(_msg)) => { /* malformed YAML/JSON/TOML */ }
    }

    Err("expected a validation error for the empty name".into())
}
