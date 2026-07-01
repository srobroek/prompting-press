//! The one-variable `ask` prompt used throughout the guard guide: `topic` is
//! declared untrusted (`trusted: false`). Standalone — `cargo run --example
//! guides_guard_construct`.

use garde::Validate;
use prompting_press::Prompt;
use serde::Serialize;

// The prompt: `topic` is declared untrusted (trusted: false).
fn ask() -> Result<Prompt, Box<dyn std::error::Error>> {
    Ok(Prompt::from_yaml(
        r#"
name: ask
role: user
body: "Tell me about {{ topic }}."
variables:
  topic:
    type: string
    trusted: false
"#,
    )?)
}

// The typed vars handed to render().
#[derive(Serialize, Validate)]
struct Ask {
    #[garde(length(min = 1))]
    topic: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ask = ask()?;
    let vars = Ask {
        topic: "rivers".into(),
    };

    // Without the guard the body renders plainly.
    let result = ask.render(&vars, None, &Default::default(), false)?;
    assert_eq!(result.text, "Tell me about rivers.");
    assert!(result.guard.is_none());
    println!("{}", result.text);
    Ok(())
}
