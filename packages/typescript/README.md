# prompting-press (Node / TypeScript)

TypeScript distribution of [Prompting Press](https://github.com/srobroek/prompting-press) — a typed,
variant-aware prompt-template library. It validates typed inputs, renders prompt text, stamps
content-addressed provenance, and lints your prompts in CI — all backed by the **same shared Rust
engine** the Rust and Python bindings use. This package is a thin [napi-rs](https://napi.rs) binding +
a [Zod](https://zod.dev) facade over that engine; it contains **no rendering logic of its own**.

> ESM-only · Node 20+ · ships as a native addon (per-platform binary).

## Install

```bash
npm i prompting-press   # or: pnpm add prompting-press
```

(Not yet published — version `0.0.0`. Build from source: `pnpm -C packages/typescript build`.)

## Quick start

```ts
import { z } from "zod";
import { Registry, render, check, Composition, PromptValidationError } from "prompting-press";

// Define your prompt's variables as a Zod schema (with custom .refine() validators).
const Greeting = z.object({
  name: z.string(),
  count: z.number().int().refine((n) => n >= 0, "count must be >= 0"),
});

const reg = new Registry();
reg.loadYaml(`
name: greet
role: user
body: "Hi {{ name }}, you have {{ count }} messages"
variables:
  name:  { type: string,  provenance: trusted }
  count: { type: integer, provenance: trusted }
`);

const r = render(reg, "greet", Greeting, { name: "Ada", count: 3 });
r.text;          // "Hi Ada, you have 3 messages"
r.templateHash;  // 64-hex SHA-256 of the template source
r.renderHash;    // 64-hex SHA-256 of the rendered output
```

- **Validate-then-render**: the Zod schema is `safeParse`d **before** any templating. Invalid input
  throws a `PromptValidationError` naming every offending field, and nothing renders. You can also pass
  already-typed plain data (no schema) — it's marshaled directly.
- **Dual-input loader**: `reg.loadYaml(text)`, `reg.loadJson(text)`, or `reg.insert(obj)` — all three
  normalize through the **one** Rust loader, so YAML/JSON/object render identically (parity is
  structural, not re-tested here).
- **`PromptDefinition`** (the prompt-definition shape) is re-exported, code-generated from the published
  JSON Schema — never hand-written.

## The agreement + provenance lint (the headline differentiator)

`check(reg)` is a **pure** analysis pass (never mutates, never renders) — the static guarantee no
file-based prompt library provides. Wire it as a CI gate: it flags a template referencing an
**undeclared variable**, and an `untrusted`/`external` field used **without a declared guard**, *before*
anything renders.

```ts
import { check } from "prompting-press";

const report = check(reg);
if (!report.passed()) {
  for (const f of report.findings) {
    // f.kind ∈ undeclared_variable | untrusted_without_guard | reserved_variant_name | analysis_error
    console.error(f.kind, f.prompt, f.variant, f.detail);
  }
  process.exit(1);
}
```

## Composition (multi-message prompts)

```ts
import { Composition } from "prompting-press";

const messages = Composition.fromMessages([
  ["systemPreamble", SysVars, sysData],
  ["greet", Greeting, { name: "Ada", count: 3 }],
]).resolve(reg);
// messages: [{ role: "system", text: ... }, { role: "user", text: "Hi Ada, ..." }]
```

N entries → exactly N ordered `{ role, text }` messages, each rendered with its own validated vars. One
invalid entry throws — no partial result. There is **no** fluent `.chain()` API (it can't cross the napi
boundary and collides with JS idiom).

## Errors

Every failure is a `PromptingPressError` subclass carrying `errors: { field, code, message }[]`:

| Class | When |
|-------|------|
| `PromptValidationError` | Zod validation failed (before render) — `code: "validation"` |
| `PromptRenderError` | the kernel rejected the render — `code`: `unknown_variant` / `undefined_variable` / `parse` / `render` / `excluded_feature` |
| `UnknownPromptError` | the prompt name isn't in the registry |
| `LoadError` | malformed YAML/JSON or shape-violating definition |

Native error types (`ZodError`, Rust errors) **never** appear on the public API — every error is
normalized to the shared `{ field, code, message }` shape (identical across the Rust, Python, and TS
bindings). A value that triggers a kernel parse/render error is **scrubbed** from the thrown error's
message, `.stack`, and rows (SEC-004) — the `ZodError` mapper copies only the issue message + path,
never the rejected value.

> **Three-sets invariant**: your Zod field names must match the prompt's declared `variables`. A
> mismatch is **not** silent — it surfaces as a loud `PromptRenderError` with `code: "undefined_variable"`,
> never an empty render. `check()` catches the same class at lint time.

## Guard usage (the system-prompt addendum doctrine)

When a prompt declares `untrusted`/`external` inputs and you enable a guard, the rendered `guard` text is
returned as a **separate** field on `RenderResult` — it is **never** concatenated into `text`. Route it
as a **system-prompt addendum**:

- **Single render** → put `RenderResult.guard` into your system prompt and send `text` as the user message.
- **Multi-message** → place the guard as its own `system` message.

The library never assembles the provider request body (constitution Principle III); `guard` and `text`
stay separate. (Roadmap decision C-09.)

## Boundary

This library does **no I/O** (you push prompt text/objects in), makes **no** LLM calls, assembles **no**
provider request body, parses **no** model output, and ships **no token counter** (the `outputModel`
field is carried as metadata only). It stays a drop-in alongside any call layer.

## Layout

```
packages/typescript/
├── package.json                 # ESM-only; zod runtime dep; napi build scripts
├── src/
│   ├── index.ts                 # the public Zod facade (compiled to dist/)
│   └── generated/               # codegen'd PromptDefinition shape — DO NOT EDIT (freshness-gated)
├── test/                        # node:test suites (run against the built addon)
└── index.{js,d.ts} + *.node     # napi-generated addon loader + native binary (not committed)
```

The napi binding crate lives at `crates/prompting-press-node/` (a workspace member). `napi`/`napi-derive`
appear **only** there — the kernel and Rust consumer stay FFI-free (CI-enforced; roadmap decision C-02).
