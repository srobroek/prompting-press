//! Construct a `Prompt` from an already-built `PromptDefinition` value with `Prompt::new`
//! — same validation as the `from_*` text factories. Standalone:
//! `cargo run --example getting-started_rust_construct_from_definition`.

use prompting_press::{Prompt, PromptDefinition};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // PromptDefinition is the codegen'd shape. Building it inline is verbose (its
    // fields are newtypes), so it's usually deserialized — here, from a JSON literal:
    let def: PromptDefinition = serde_json::from_value(serde_json::json!({
        "name": "greet",
        "role": "user",
        "body": "Hi {{ name }}, you have {{ count }} messages.",
        "variables": {
            "name":  { "type": "string",  "trusted": true },
            "count": { "type": "integer", "trusted": true }
        }
    }))?;

    let greet = Prompt::new(def)?; // same validation as the from_* factories

    assert_eq!(greet.name(), "greet");
    Ok(())
}
