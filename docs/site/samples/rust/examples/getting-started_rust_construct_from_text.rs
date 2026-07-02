//! Construct a `Prompt` from a definition file with a `from_*` factory — validation runs
//! immediately. Standalone — `cargo run --example getting-started_rust_construct_from_text`.

use prompting_press::Prompt;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // The caller reads the definition; the library does no file I/O itself.
    // Resolve the file next to this program (a real app uses its own path).
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/examples");

    let assistant = Prompt::from_yaml(&fs::read_to_string(format!("{dir}/assistant.yaml"))?)?; // validates here, or Err
                                                                                               // The same definition in JSON or TOML parses into an identical Prompt:
                                                                                               // let assistant = Prompt::from_json(&fs::read_to_string(format!("{dir}/assistant.json"))?)?;
                                                                                               // let assistant = Prompt::from_toml(&fs::read_to_string(format!("{dir}/assistant.toml"))?)?;

    assert_eq!(assistant.name(), "assistant");
    assert_eq!(
        assistant.body(),
        "You are a support assistant for {{ company }}. Keep your replies under {{ max_words }} words."
    );
    Ok(())
}
