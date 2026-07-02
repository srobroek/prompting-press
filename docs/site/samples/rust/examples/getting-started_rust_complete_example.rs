//! The complete construct → declare → render walk in one program.
//! Standalone — `cargo run --example getting-started_rust_complete_example`.

use garde::Validate;
use prompting_press::GuardConfig;
use prompting_press::Prompt;
use serde::Serialize;
use std::fs;

#[derive(Serialize, Validate)]
struct AssistantVars {
    #[garde(length(min = 1))]
    company: String,
    #[garde(range(min = 1))]
    max_words: i64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Construct (validates here). The caller reads the definition; the library does
    //    no file I/O itself. Resolve the file next to this program (a real app uses its
    //    own path).
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/examples");
    let assistant = Prompt::from_yaml(&fs::read_to_string(format!("{dir}/assistant.yaml"))?)?;

    // 2 + 3. Render with typed, garde-validated vars.
    let vars = AssistantVars {
        company: "Acme Robotics".into(),
        max_words: 50,
    };
    let result = assistant.render(&vars, None, &GuardConfig::default(), false)?;

    println!("{}", result.text); // You are a support assistant for Acme Robotics. Keep your replies under 50 words.
    println!("{}", result.template_hash); // 64-char hex

    assert_eq!(
        result.text,
        "You are a support assistant for Acme Robotics. Keep your replies under 50 words."
    );
    assert_eq!(result.template_hash.len(), 64);
    Ok(())
}
