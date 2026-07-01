//! Construct a `Prompt` from YAML text with a `from_*` factory — validation runs
//! immediately. Standalone — `cargo run --example getting-started_rust_construct_from_text`.

use prompting_press::Prompt;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // A real caller reads this from a file (`std::fs::read_to_string("greet.yaml")`);
    // inlined here so the sample is a complete, standalone program.
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

    let greet = Prompt::from_yaml(yaml)?; // validates here, or returns Err
                                          // from_json / from_toml parse the JSON / TOML forms into the same Prompt.

    assert_eq!(greet.name(), "greet");
    assert_eq!(
        greet.body(),
        "Hi {{ name }}, you have {{ count }} messages."
    );
    Ok(())
}
