//! Greeter CLI — a realistic Prompting Press consumer sample app (spec 014, WU-C).
//!
//! Walks the FULL public feature surface end-to-end (FR-014): construct → validate →
//! render default + a named variant → compose a 2-message prompt → `check()` → the
//! advisory guard → provenance hashes → an error path. The "hand to an LLM" step is a
//! printed stub — the library never calls a provider (FR-018).
//!
//! Run it: `cargo run -p prompting-press-greeter-cli`.

use garde::Validate;
use prompting_press::GuardConfig;
use prompting_press::{Composition, ConsumerError, FindingKind, Prompt};
use serde::Serialize;

// ── Typed vars (validated by garde before any templating) ────────────────────

#[derive(Serialize, Validate)]
pub struct GreetVars {
    #[garde(length(min = 1))]
    pub name: String,
    #[garde(range(min = 0))]
    pub count: i64,
}

#[derive(Serialize, Validate)]
pub struct AskVars {
    #[garde(length(min = 1))]
    pub topic: String,
}

// ── Prompt documents (a real consumer would read these from files) ───────────

/// A greeting prompt with a `formal` variant, both sharing the same variables.
pub const GREET_YAML: &str = r#"
name: greet
role: user
body: "Hi {{ name }}, you have {{ count }} messages."
variables:
  name:
    type: string
    trusted: true
  count:
    type: integer
    trusted: true
variants:
  formal:
    body: "Good day, {{ name }}. You have {{ count }} messages awaiting your attention."
"#;

/// A prompt with an UNTRUSTED variable, used to demonstrate the guard + `check()`.
pub const ASK_YAML: &str = r#"
name: ask
role: user
body: "Tell me about {{ topic }}."
variables:
  topic:
    type: string
    trusted: false
"#;

/// Run the full feature walk, returning an error only on an *unexpected* failure
/// (the demonstrated error path is caught and reported inline, not propagated).
///
/// # Errors
///
/// Returns an error if any of the prompt construction, render, or composition
/// steps fail unexpectedly. The deliberately-demonstrated error path (unknown
/// variant) is caught inline and does not propagate.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    #[derive(Serialize, Validate)]
    struct SysVars {
        #[garde(length(min = 1))]
        instruction: String,
    }

    println!("=== Prompting Press — Rust consumer sample ===\n");

    // 1. CONSTRUCT (validates the template↔variables agreement immediately).
    let greet = Prompt::from_yaml(GREET_YAML)?;
    println!("[construct] loaded prompt {:?}", greet.name());
    println!(
        "[construct] variants: {:?}",
        greet.variants().keys().collect::<Vec<_>>()
    );

    // 2. VALIDATE + RENDER the default arm.
    let vars = GreetVars {
        name: "Ada".into(),
        count: 3,
    };
    let default = greet.render(&vars, None, &GuardConfig::default(), false)?;
    println!("\n[render:default] {}", default.text);

    // 3. RENDER a named variant — a different body from the same vars.
    let formal = greet.render(&vars, Some("formal"), &GuardConfig::default(), false)?;
    println!("[render:formal]  {}", formal.text);

    // 4. PROVENANCE — content-addressed hashes on the result.
    println!(
        "\n[provenance] variant={} template_hash={}… render_hash={}…",
        default.variant,
        &default.template_hash[..8],
        &default.render_hash[..8],
    );

    // 5. COMPOSE a 2-message prompt (system preamble + the greeting).
    let sys = Prompt::from_yaml(
        "name: sys\nrole: system\nbody: \"{{ instruction }}\"\nvariables:\n  instruction:\n    type: string\n    trusted: true\n",
    )?;
    let mut comp = Composition::new();
    comp.append(
        &sys,
        &SysVars {
            instruction: "Be concise.".into(),
        },
        None,
    )?;
    comp.append(&greet, &vars, None)?;
    let messages = comp.resolve()?;
    println!("\n[compose] {} messages:", messages.len());
    for m in &messages {
        println!("  {}: {}", m.role, m.text);
    }

    // 6. CHECK — the advisory lint. `ask` declares an untrusted var with no guard
    //    metadata, so check() surfaces one finding.
    let ask = Prompt::from_yaml(ASK_YAML)?;
    let report = ask.check();
    println!(
        "\n[check] ask.check() passed={} findings={}",
        report.passed(),
        report.findings.len()
    );
    for f in &report.findings {
        match &f.kind {
            FindingKind::UntrustedWithoutGuard { field } => {
                println!("  untrusted_without_guard: {} — {}", field, f.detail);
            }
        }
    }

    // 7. GUARD — enable it: the untrusted value is delimited in the body and an
    //    advisory is returned. The library never sends this anywhere.
    let guarded = ask.render(
        &AskVars {
            topic: "rivers".into(),
        },
        None,
        &GuardConfig {
            enabled: true,
            ..Default::default()
        },
        false,
    )?;
    println!("\n[guard] text  = {}", guarded.text);
    println!(
        "[guard] guard = {}",
        guarded.guard.as_deref().unwrap_or("<none>")
    );

    // 8. ERROR PATH — an unknown variant fails loudly with a structured error.
    match greet.render(&vars, Some("nonexistent"), &GuardConfig::default(), false) {
        Ok(_) => return Err("expected the unknown-variant render to fail".into()),
        Err(ConsumerError::Kernel(rows)) => {
            println!(
                "\n[error] unknown variant rejected: code={} field={}",
                rows[0].code, rows[0].field
            );
        }
        Err(other) => return Err(format!("unexpected error variant: {other:?}").into()),
    }

    // 9. HAND-OFF STUB — a real app would send `messages` to a provider here.
    //    The library does no I/O and calls no model; this is a printed placeholder.
    println!(
        "\n[handoff] (stub) would POST {} messages to the configured LLM provider.",
        messages.len()
    );

    println!("\n=== done ===");
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("greeter: {e}");
        std::process::exit(1);
    }
}
