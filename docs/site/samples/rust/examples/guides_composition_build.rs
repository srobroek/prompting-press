//! Build a Composition by appending (prompt, vars, variant?) entries, then
//! resolve it to an ordered Vec<Message>. Standalone:
//! `cargo run --example guides_composition_build`.

use garde::Validate;
use prompting_press::{Composition, Prompt, PromptDefinition};
use serde::Serialize;

#[derive(Serialize, Validate)]
struct SysVars {
    #[garde(length(min = 1))]
    instructions: String,
}

#[derive(Serialize, Validate)]
struct UserVars {
    #[garde(length(min = 1))]
    query: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build the two prompts inline from their shape, so the content is explicit.
    let sys_def: PromptDefinition = serde_json::from_value(serde_json::json!({
        "name": "system-preamble",
        "role": "system",
        "body": "{{ instructions }}",
        "variables": { "instructions": { "type": "string", "trusted": true } }
    }))?;
    let user_def: PromptDefinition = serde_json::from_value(serde_json::json!({
        "name": "user-turn",
        "role": "user",
        "body": "{{ query }}",
        "variables": { "query": { "type": "string", "trusted": false } }
    }))?;

    let sys = Prompt::new(sys_def)?;
    let user = Prompt::new(user_def)?;

    let mut comp = Composition::new();
    comp.append(
        &sys,
        &SysVars {
            instructions: "Be concise.".into(),
        },
        None, // variant = None → default arm
    )?;
    comp.append(
        &user,
        &UserVars {
            query: "What is Rust?".into(),
        },
        None,
    )?;

    let messages = comp.resolve()?;
    for m in &messages {
        println!("{}: {}", m.role, m.text);
        // "system: Be concise."
        // "user: What is Rust?"
    }

    assert_eq!(
        messages
            .iter()
            .map(|m| (m.role.as_str(), m.text.as_str()))
            .collect::<Vec<_>>(),
        vec![("system", "Be concise."), ("user", "What is Rust?")],
    );
    Ok(())
}
