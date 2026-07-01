//! Discovering the selectable variants: `variants()` returns the declared
//! variant map; read its keys (the default arm is not listed — it is the root
//! body, name `"default"`). Standalone:
//! `cargo run --example guides_variants_discover`.

use prompting_press::Prompt;

const SUMMARY_YAML: &str = r#"
name: summary
role: user
body: "Summarise the following article in {{ max_words }} words:\n\n{{ article }}"
variables:
  article:
    type: string
    trusted: false
  max_words:
    type: integer
    trusted: true
variants:
  concise:
    body: "In one sentence, summarise: {{ article }}"
  structured:
    body: "Summarise {{ article }} as a title, three bullets, and a one-line conclusion."
"#;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let summary = Prompt::from_yaml(SUMMARY_YAML)?;

    let mut keys = summary.variants().keys().collect::<Vec<_>>(); // ["concise", "structured"]
    keys.sort();
    assert_eq!(keys, ["concise", "structured"]);
    assert!(summary.variants().contains_key("concise")); // true
    Ok(())
}
