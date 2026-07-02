//! Reading prompt-level and per-variant metadata back out. The library stores the
//! opaque `metadata` maps and echoes them through accessors; it never interprets
//! them. Standalone — `cargo run --example guides_metadata_reading_it_back`.

use prompting_press::Prompt;
use std::fs;

/// Stand-in for the caller's routing logic — the library never calls this.
fn route_to_model(_hint: &str) {}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/examples");
    let p = Prompt::from_yaml(&fs::read_to_string(format!("{dir}/summary_metadata.yaml"))?)?;

    // metadata() returns &serde_json::Map<String, serde_json::Value>.
    if let Some(hint) = p.metadata().get("model_hint") {
        route_to_model(hint.as_str().unwrap_or("default"));
    }

    // Per-variant metadata is on each Variant in p.variants().
    // The caller reads it in selection logic — the library never does.
    if let Some(terse) = p.variants().get("terse") {
        let _weight = terse.metadata.get("weight");
    }

    // The accessors return the maps as-is; nothing is interpreted or mutated.
    assert_eq!(
        p.metadata().get("model_hint").and_then(|v| v.as_str()),
        Some("claude-sonnet-4-6")
    );
    assert_eq!(
        p.metadata().get("max_tokens").and_then(|v| v.as_i64()),
        Some(512)
    );
    let terse = p.variants().get("terse").expect("terse variant exists");
    assert_eq!(
        terse.metadata.get("weight").and_then(|v| v.as_f64()),
        Some(0.2)
    );
    assert_eq!(
        terse.metadata.get("group").and_then(|v| v.as_str()),
        Some("experiment-q4")
    );

    println!("model_hint = {:?}", p.metadata().get("model_hint"));
    Ok(())
}
