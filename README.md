# Prompting Press

A typed, variant-aware **prompt-template library** — the prompt analogue of a typed config
system. It turns *typed inputs + a template* into *rendered text + content-addressed provenance*,
across **Rust, Python, and TypeScript** from one shared compiled Rust engine. Byte-identical
output across all three by construction (constitution Principle I), not by re-implementation.

**The headline feature:** the **sound agreement check** catches a template that references a
variable the prompt never declared — at construction time and as a CI lint — never as a silent
empty render.

> **Status:** Specs 001–009 implemented. Prompts render with typed-input validation, the
> agreement lint, content-addressed provenance, variants, and multi-message composition from
> Rust, Python, and TypeScript today.

## Documentation

**[Full docs site →](https://prompting-press.github.io/)**

The docs site covers:
- [Getting started](https://prompting-press.github.io/getting-started/rust/) (Rust / Python / TypeScript)
- [API reference](https://prompting-press.github.io/reference/rust/) per language
- [Template features](https://prompting-press.github.io/templates/) (what MiniJinja features are supported)
- [How-to guides](https://prompting-press.github.io/guides/lint-in-ci/) (CI lint, variants, composition, guard)
- [FAQ](https://prompting-press.github.io/faq/)

## Quick start

### Rust

```toml
# Cargo.toml
[dependencies]
prompting-press = "0.0.0"
garde = { version = "0.22", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
```

```rust
use prompting_press::Prompt;
use garde::Validate;
use serde::Serialize;
use prompting_press_core::GuardConfig;

#[derive(Serialize, Validate)]
struct Vars { #[garde(length(min = 1))] name: String }

let p = Prompt::from_yaml(r#"
name: greet
role: user
body: "Hi {{ name }}"
variables:
  name: { type: string, trusted: true }
"#)?;

let result = p.render(&Vars { name: "Ada".into() }, None, &GuardConfig::default())?;
println!("{}", result.text);           // "Hi Ada"
println!("{}", result.template_hash);  // 64-char SHA-256
```

### Python

```bash
pip install prompting-press
```

```python
from prompting_press import Prompt
from pydantic import BaseModel

class Vars(BaseModel):
    name: str

p = Prompt.from_yaml("""
name: greet
role: user
body: "Hi {{ name }}"
variables:
  name: { type: string, trusted: true }
""")

result = p.render(Vars, data={"name": "Ada"})
print(result.text)           # "Hi Ada"
print(result.template_hash)  # 64-char SHA-256
```

### TypeScript

```bash
npm install prompting-press zod
```

```ts
import { z } from "zod";
import { Prompt } from "prompting-press";

const Vars = z.object({ name: z.string() });

const p = Prompt.fromYaml(`
name: greet
role: user
body: "Hi {{ name }}"
variables:
  name: { type: string, trusted: true }
`);

const result = p.render(Vars, { name: "Ada" });
console.log(result.text);          // "Hi Ada"
console.log(result.templateHash);  // 64-char SHA-256
```

## What it deliberately does not do

- No I/O — you push prompt data in; the library never reads files, a database, or the network.
- No LLM calls, no provider request-body assembly, no token counting, no output parsing.
- The untrusted-input guard is advisory text, not enforcement.

## Architecture

```
crates/
├── prompting-press-core/   # FFI-free engine kernel (the shared core)
├── prompting-press/        # public Rust consumer API
├── prompting-press-py/     # PyO3 binding
└── prompting-press-node/   # napi-rs binding
packages/
├── python/                 # maturin-built wheel
└── typescript/             # napi-rs npm package
schemas/jsonschema/         # prompt-definition.schema.json — single source of truth
docs/site/                  # Astro + Starlight docs site
```

## Develop

Toolchain pinned via [`mise`](https://mise.jdx.dev) (`mise install`), orchestrated with
[`moon`](https://moonrepo.dev):

```bash
mise exec -- moon run :build        # build all crates
mise exec -- moon run :codegen      # regenerate language shapes from the schema
pnpm -C docs/site build             # build the docs site
```

Licensed under Apache-2.0.
