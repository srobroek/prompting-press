//! Render the `Prompt` with the typed `GreetVars` and read the result fields.
//! Standalone — `cargo run --example getting-started_rust_render_and_read`.

use garde::Validate;
use prompting_press::{GuardConfig, Prompt};
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

    let vars = GreetVars {
        name: "Ada".into(),
        count: 3,
    };

    let result = greet.render(&vars, None, &GuardConfig::default(), false)?;

    assert_eq!(result.text, "Hi Ada, you have 3 messages."); // the rendered body
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
    Ok(())
}
