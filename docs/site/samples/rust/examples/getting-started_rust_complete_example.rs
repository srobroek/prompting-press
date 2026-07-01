//! The complete construct → declare → render walk in one program.
//! Standalone — `cargo run --example getting-started_rust_complete_example`.

use garde::Validate;
use prompting_press::GuardConfig;
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
    // 1. Construct (validates here). A real caller reads this from a file
    //    (`std::fs::read_to_string("greet.yaml")`); inlined here to stay standalone.
    let yaml = r#"
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
"#;
    let greet = Prompt::from_yaml(yaml)?;

    // 2 + 3. Render with typed, garde-validated vars.
    let vars = GreetVars {
        name: "Ada".into(),
        count: 3,
    };
    let result = greet.render(&vars, None, &GuardConfig::default(), false)?;

    println!("{}", result.text); // Hi Ada, you have 3 messages.
    println!("{}", result.template_hash); // 64-char hex

    assert_eq!(result.text, "Hi Ada, you have 3 messages.");
    assert_eq!(result.template_hash.len(), 64);
    Ok(())
}
