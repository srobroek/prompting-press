# prompting-press

A typed, variant-aware **prompt-template library**. It turns typed inputs and a template into
rendered text plus content-addressed provenance — nothing else (no I/O, no LLM calls, no request
assembly). Rust, Python, and TypeScript all bind one compiled Rust engine, so rendered output is
byte-identical across every language.

This is the TypeScript distribution: a [Zod](https://zod.dev) facade over a
[napi-rs](https://napi.rs) binding to that engine.

## Install

```bash
npm i prompting-press zod   # or: pnpm add prompting-press zod
```

`zod` is a peer dependency (bring your own).

## Quick start

```ts
import { z } from "zod";
import { Prompt } from "prompting-press";

const Vars = z.object({ name: z.string() });

const greet = Prompt.fromYaml(`
name: greet
role: user
body: "Hi {{ name }}"
variables:
  name: { type: string, trusted: true }
`);

const r = greet.render(Vars, { name: "Ada" });
r.text;          // "Hi Ada"
r.templateHash;  // 64-char SHA-256 of the template source
r.renderHash;    // 64-char SHA-256 of the rendered output
```

## Documentation

Full docs — getting started, API reference, template features, guides, and the CI agreement
lint — are at **<https://prompting-press.github.io/>**.

## License

[Apache-2.0](https://github.com/prompting-press/prompting-press/blob/main/LICENSE).
