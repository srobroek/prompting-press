//! Declare the typed Vars struct, validated by `garde`. Its field names must match the
//! prompt's `variables` (`company`, `max_words`); the `#[garde(..)]` rules run at render
//! time, before the kernel is touched. Standalone:
//! `cargo run --example getting-started_rust_declare_vars`.

use garde::Validate;
use serde::Serialize;

#[derive(Serialize, Validate)]
struct AssistantVars {
    #[garde(length(min = 1))]
    company: String,
    #[garde(range(min = 1))]
    max_words: i64,
}

fn main() {
    // A valid instance passes garde validation; an out-of-range one fails.
    let ok = AssistantVars {
        company: "Acme Robotics".into(),
        max_words: 50,
    };
    assert!(ok.validate().is_ok());

    let bad = AssistantVars {
        company: String::new(),
        max_words: 0,
    };
    assert!(bad.validate().is_err());
}
