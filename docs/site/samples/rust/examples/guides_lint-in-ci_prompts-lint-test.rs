//! Wiring `Prompt::check()` as a CI gate: a test that loads every `*.yaml` under a
//! `prompts/` directory, constructs each prompt, and asserts `check()` returns no
//! findings — failing the build (and naming the offender) otherwise.
//!
//! Standalone — `cargo run --example guides_lint-in-ci_prompts-lint-test`. To keep the
//! program self-contained it first materializes a `prompts/` directory of shipped
//! fixtures in a temp dir and `cd`s into it; a real repo checks its own `prompts/` in.

use prompting_press::Prompt;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Materialize the `prompts/` directory a real repo would keep under version control.
    // A clean, shipped prompt: its untrusted-free variable needs no guard, so check() passes.
    let dir = std::env::temp_dir().join("pp_lint_in_ci_prompts");
    let prompts = dir.join("prompts");
    fs::create_dir_all(&prompts)?;
    fs::write(
        prompts.join("assistant.yaml"),
        r#"
name: assistant
role: system
body: "You are a support assistant for {{ company }}. Keep your replies under {{ max_words }} words."
variables:
  company:
    type: string
    trusted: true
  max_words:
    type: integer
    trusted: true
"#,
    )?;
    std::env::set_current_dir(&dir)?;

    // ── The CI gate itself: a `#[test]` in a real crate; here inlined into `main`. ──
    // tests/prompts_lint.rs  — runs under `cargo test`
    let mut failures = Vec::new();

    for entry in fs::read_dir("prompts").expect("prompts/ dir") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap();
        // Construction itself enforces the hard invariants — surface a load/agreement
        // failure as a test failure too, not a panic.
        let prompt = match Prompt::from_yaml(&text) {
            Ok(p) => p,
            Err(e) => {
                failures.push(format!("{}: construction failed: {e:?}", path.display()));
                continue;
            }
        };
        for f in &prompt.check().findings {
            failures.push(format!("{}: {} — {}", path.display(), f.prompt, f.detail));
        }
    }

    assert!(
        failures.is_empty(),
        "prompt lint findings:\n{}",
        failures.join("\n")
    );

    println!("prompt lint: clean — {} findings", failures.len());
    Ok(())
}
