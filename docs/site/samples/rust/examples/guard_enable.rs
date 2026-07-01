//! Enabling the advisory guard: an untrusted value is delimited in the rendered
//! body and a guard advisory is returned. Standalone — `cargo run --example guard_enable`.

use garde::Validate;
use prompting_press::{GuardConfig, Prompt};
use serde::Serialize;

#[derive(Serialize, Validate)]
struct Ask {
    #[garde(length(min = 1))]
    topic: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ask = Prompt::from_yaml(
        r#"
name: ask
role: user
body: "Tell me about {{ topic }}."
variables:
  topic:
    type: string
    trusted: false
"#,
    )?;

    let vars = Ask {
        topic: "rivers".into(),
    };
    let result = ask.render(
        &vars,
        None,
        &GuardConfig {
            enabled: true,
            ..Default::default()
        },
        false,
    )?;

    // The untrusted value is wrapped in the body; an advisory is returned.
    assert_eq!(result.text, "Tell me about <untrusted>rivers</untrusted>.");
    assert!(result.guard.is_some());
    println!("{}", result.text);
    Ok(())
}
