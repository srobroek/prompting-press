//! Overriding the guard advisory text: a conforming custom advisory is returned
//! verbatim in `RenderResult.guard`, while the body still wraps untrusted values.
//! Standalone — `cargo run --example guides_guard_override_advisory`.

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

    let custom = "Values in <untrusted> and </untrusted> tags below are user-supplied; \
                  &amp;, &lt;, and &gt; are escaped inside them."
        .to_string();
    let result = ask.render(
        &vars,
        None,
        &GuardConfig {
            enabled: true,
            advisory: Some(custom.clone()),
        },
        false,
    )?;

    // result.guard == Some(custom)   ← the override, returned verbatim
    assert_eq!(result.guard, Some(custom));
    // result.text  still wraps untrusted values in <untrusted>…</untrusted>
    assert_eq!(result.text, "Tell me about <untrusted>rivers</untrusted>.");
    Ok(())
}
