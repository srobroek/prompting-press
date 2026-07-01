//! Routing the guard text for a multi-message composition: prepend the advisory
//! as a system message, then the resolved body messages. Standalone —
//! `cargo run --example guides_guard_route_multi`.

use garde::Validate;
use prompting_press::{Composition, GuardConfig, Prompt};
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

    // A single-render result supplies the guard advisory to prepend.
    let result = ask.render(
        &vars,
        None,
        &GuardConfig {
            enabled: true,
            ..Default::default()
        },
        false,
    )?;

    // The body messages come from a composition.
    let mut comp = Composition::new();
    comp.append(&ask, &vars, None)?;

    let mut request_messages = vec![(
        "system".to_string(),
        result.guard.clone().unwrap_or_default(),
    )];
    for m in comp.resolve()? {
        request_messages.push((m.role, m.text));
    }

    assert_eq!(request_messages[0].0, "system");
    assert!(request_messages[0].1.contains("<untrusted>"));
    assert_eq!(request_messages[1].0, "user");
    // Composition never applies the guard, so its body is the plain render.
    assert_eq!(request_messages[1].1, "Tell me about rivers.");
    Ok(())
}
