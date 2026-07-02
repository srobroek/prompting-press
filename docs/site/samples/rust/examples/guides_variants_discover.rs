//! Discovering the selectable variants: `variants()` returns the declared
//! variant map; read its keys (the default arm is not listed — it is the root
//! body, name `"default"`). Standalone:
//! `cargo run --example guides_variants_discover`.

use prompting_press::Prompt;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/examples");
    let summary = Prompt::from_yaml(&fs::read_to_string(format!("{dir}/summary.yaml"))?)?;

    let mut keys = summary.variants().keys().collect::<Vec<_>>(); // ["concise", "structured"]
    keys.sort();
    assert_eq!(keys, ["concise", "structured"]);
    assert!(summary.variants().contains_key("concise")); // true
    Ok(())
}
