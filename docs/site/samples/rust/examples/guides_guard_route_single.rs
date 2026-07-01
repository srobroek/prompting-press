//! Routing the guard text for a single render: the advisory goes to a system
//! message, the body to a user message. The library never concatenates them.
//! Standalone — `cargo run --example guides_guard_route_single`.

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

    let messages = vec![
        ("system", result.guard.clone().unwrap_or_default()),
        ("user", result.text.clone()),
    ];

    assert_eq!(messages[0].0, "system");
    assert!(messages[0].1.contains("<untrusted>"));
    assert_eq!(messages[1].0, "user");
    assert_eq!(
        messages[1].1,
        "Tell me about <untrusted>rivers</untrusted>."
    );
    Ok(())
}
