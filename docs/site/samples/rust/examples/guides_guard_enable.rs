//! Enabling the advisory guard: the untrusted `topic` value is delimited in the
//! rendered body and a guard advisory is returned. Standalone —
//! `cargo run --example guides_guard_enable`.

use garde::Validate;
use prompting_press::{GuardConfig, Prompt};
use serde::Serialize;
use std::fs;

#[derive(Serialize, Validate)]
struct Ask {
    #[garde(length(min = 1))]
    topic: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/examples");
    let ask = Prompt::from_yaml(&fs::read_to_string(format!("{dir}/ask.yaml"))?)?;
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

    // topic wrapped in the body; an advisory is returned.
    assert_eq!(result.text, "Tell me about <untrusted>rivers</untrusted>.");
    assert!(result.guard.is_some());
    assert!(result.guard.as_deref().unwrap().contains("<untrusted>"));

    // GuardConfig::default() (or enabled: false) wraps nothing and guard is None.
    let plain = ask.render(&vars, None, &GuardConfig::default(), false)?;
    assert_eq!(plain.text, "Tell me about rivers.");
    assert!(plain.guard.is_none());

    println!("{}", result.text);
    Ok(())
}
