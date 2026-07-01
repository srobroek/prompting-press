//! Declare the typed Vars struct, validated by `garde`. Its field names must match the
//! prompt's `variables` (`name`, `count`); the `#[garde(..)]` rules run at render time,
//! before the kernel is touched. Standalone:
//! `cargo run --example getting-started_rust_declare_vars`.

use garde::Validate;
use serde::Serialize;

#[derive(Serialize, Validate)]
struct GreetVars {
    #[garde(length(min = 1))]
    name: String,
    #[garde(range(min = 0))]
    count: i64,
}

fn main() {
    // A valid instance passes garde validation; an out-of-range one fails.
    let ok = GreetVars {
        name: "Ada".into(),
        count: 3,
    };
    assert!(ok.validate().is_ok());

    let bad = GreetVars {
        name: String::new(),
        count: -1,
    };
    assert!(bad.validate().is_err());
}
