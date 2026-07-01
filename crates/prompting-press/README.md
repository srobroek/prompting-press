# prompting-press

A typed, variant-aware **prompt-template library**. It turns typed inputs and a template into
rendered text plus content-addressed provenance — nothing else (no I/O, no LLM calls, no request
assembly). Rust, Python, and TypeScript all bind one compiled Rust engine, so rendered output is
byte-identical across every language.

This is the public Rust consumer crate: an idiomatic API over the
[`prompting-press-core`](https://crates.io/crates/prompting-press-core) engine kernel, using
[`garde`](https://docs.rs/garde) for typed-input validation.

## Install

```bash
cargo add prompting-press
cargo add garde --features derive
cargo add serde --features derive
```

## Quick start

```rust
use prompting_press::Prompt;
use prompting_press_core::GuardConfig;
use garde::Validate;
use serde::Serialize;

#[derive(Serialize, Validate)]
struct Vars { #[garde(length(min = 1))] name: String }

let greet = Prompt::from_yaml(r#"
name: greet
role: user
body: "Hi {{ name }}"
variables:
  name: { type: string, trusted: true }
"#)?;

let r = greet.render(&Vars { name: "Ada".into() }, None, &GuardConfig::default())?;
r.text;           // "Hi Ada"
r.template_hash;  // 64-char SHA-256 of the template source
```

## Documentation

Full docs — getting started, API reference, template features, guides, and the CI agreement
lint — are at **<https://prompting-press.github.io/>**.

## License

Apache-2.0.
