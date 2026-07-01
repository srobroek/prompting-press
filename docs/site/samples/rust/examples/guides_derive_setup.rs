//! Derive guide — the starting pair: a `greet` prompt (a `name` + `count` body) and a
//! matching `GreetVars`. Every later example on the page derives from this.
//! Standalone — `cargo run --example guides_derive_setup`.

use garde::Validate;
use prompting_press::Prompt;
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

    // The pair parses and validates: the body's {{ name }}/{{ count }} agree with GreetVars.
    let greet = Prompt::from_yaml(greet_yaml)?;
    assert_eq!(greet.name(), "greet");

    // GreetVars is a plain garde-validated struct — construct one to prove the shape.
    let vars = GreetVars {
        name: "Ada".into(),
        count: 3,
    };
    assert_eq!(vars.name, "Ada");
    assert_eq!(vars.count, 3);
    Ok(())
}
