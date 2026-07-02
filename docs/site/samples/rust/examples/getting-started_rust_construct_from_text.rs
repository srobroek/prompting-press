//! Construct a `Prompt` from YAML text with a `from_*` factory — validation runs
//! immediately. Standalone — `cargo run --example getting-started_rust_construct_from_text`.

use prompting_press::Prompt;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // A real caller reads this from a file (`std::fs::read_to_string("assistant.yaml")`);
    // inlined here so the sample is a complete, standalone program.
    let yaml = r#"
name: assistant
role: system
body: "You are a support assistant for {{ company }}. Keep your replies under {{ max_words }} words."
variables:
  company:
    type: string
    trusted: true
  max_words:
    type: integer
    trusted: true
"#;

    let assistant = Prompt::from_yaml(yaml)?; // validates here, or returns Err
                                               // from_json / from_toml parse the JSON / TOML forms into the same Prompt.

    assert_eq!(assistant.name(), "assistant");
    assert_eq!(
        assistant.body(),
        "You are a support assistant for {{ company }}. Keep your replies under {{ max_words }} words."
    );
    Ok(())
}
