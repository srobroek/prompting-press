# Quickstart / Validation Guide — TypeScript binding (`prompting-press-node`)

Runnable scenarios that prove the binding works end-to-end. Implementation lives in `tasks.md` + the
binding crate; this is the **validation** guide. Scenarios map to the spec's user stories + SCs.

## Prerequisites

- Toolchain via `mise` (Rust `1.95.0`, Node + pnpm). napi/napi-derive pinned 3.9.4; Zod 4.4.3 added as a
  runtime dep; @napi-rs/cli 3.7.2; json-schema-to-typescript 15.0.4.
- Build the native addon: `pnpm -C packages/typescript build` (`napi build --release --platform --esm`).
- Codegen current: `pnpm -C packages/typescript codegen` (no diff expected — freshness-gated).

## Build & import (SC-009)

```bash
pnpm -C packages/typescript build         # produces the platform .node + index.{js,d.ts} (ESM)
node --input-type=module -e "import('prompting-press').then(m => console.log(typeof m.render))"  # "function"
```
**Expected**: a per-platform `.node` binary + ESM entry; `import` succeeds on Node 16+; render/check/
compose execute against the compiled core.

## US1 — validate typed inputs and render (P1; SC-001/002)

```ts
import { z } from "zod";
import { Registry, render, PromptValidationError } from "prompting-press";

const Vars = z.object({
  name: z.string(),
  count: z.number().int().refine(n => n >= 0, "count must be >= 0"),
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

const r = render(reg, "greet", Vars, { name: "Ada", count: 3 });
// r.text === "Hi Ada, you have 3 messages"; r.variant === "default"; r.templateHash.length === 64
```
- **Invalid input** → `PromptValidationError`, **no** render:
```ts
try { render(reg, "greet", Vars, { name: "Ada", count: -1 }); }
catch (e) {
  // e instanceof PromptValidationError; e.errors.some(row => row.field === "count" && row.code === "validation")
}
```
**Expected**: valid → text + provenance; invalid → structured error naming `count`, kernel never reached;
the error is NOT a `ZodError` (SC-006).

## US2 — YAML / JSON / object parity (P2; SC-003)

```ts
// Same logical prompt three ways → identical render + provenance for identical inputs.
const out = (reg) => render(reg, "greet", Vars, { name: "Ada", count: 3 });
// out(loadedFromYaml).text === out(loadedFromJson).text === out(insertedObject).text
// …and identical templateHash + renderHash across all three.
```
- **Malformed** input → `LoadError`, nothing partially loaded:
```ts
import { LoadError } from "prompting-press";
// expect(() => new Registry().loadYaml("name: [unterminated")).toThrow(LoadError)
```
**Expected**: 100% parity across the three input forms; malformed input throws `LoadError`.

## US3 — agreement + provenance lint as a CI check (P2; SC-004/005)

```ts
import { Registry, check } from "prompting-press";
const reg = new Registry();
reg.loadYaml(undeclaredRefPrompt);    // body references {{ ghost }} not in variables
reg.loadYaml(untrustedWithoutGuard);  // declares an untrusted var, no meta.guard
const report = check(reg);
// !report.passed()
// report.findings has kind "undeclared_variable" and kind "untrusted_without_guard"
// check(cleanRegistry).passed() === true; nothing rendered or mutated
```
**Expected**: undeclared-var + untrusted-without-guard flagged with prompt/variant/field; clean passes;
pure. Reserved-`default` and un-analyzable templates surface as `reserved_variant_name` / `analysis_error`.

## US4 — multi-message composition (P3; SC-008)

```ts
import { Composition } from "prompting-press";
const msgs = Composition.fromMessages([
  ["systemPreamble", SysVars, sysData],
  ["greet", Vars, { name: "Ada", count: 3 }],
]).resolve(reg);
// msgs.map(m => m.role) === ["system", "user"]; msgs.length === 2
```
- One invalid entry → throws, **no** partial result. Empty composition → `[]`. No `.chain()` method.
**Expected**: N entries → exactly N ordered `{role, text}` messages, each rendered with its own validated
vars.

## Boundary & isolation (SC-006/007/010/011)

```bash
mise exec -- moon run ci:check-ffi --force          # napi only in prompting-press-node; kernel+consumer FFI-free
mise exec -- moon run schemas:codegen-check --force  # generated TS shape fresh (byte-identical)
mise exec -- moon run ci:test-node                   # build addon + cargo test -p prompting-press-node + TS tests
mise exec -- moon run ci:check-advisories-node       # npm dep CVE scan
rg -n "count_tokens|tokenCount|countTokens|count-tokens" packages/typescript/src crates/prompting-press-node/src || echo "no token surface (F4)"
```
**Expected**: FFI gate green (now asserts `napi`); codegen fresh; no native error type on the public API
(every error is a `PromptingPressError` subclass); SEC-004 — a seeded secret in a render-error value never
appears in the thrown error's message/stack; no token-counting surface anywhere.
