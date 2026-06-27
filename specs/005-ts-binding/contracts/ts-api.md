# Public TypeScript API Contract — `prompting-press`

Phase 1. The public surface a TS consumer imports from `prompting-press` (ESM-only). Mirrors the spec-004
Python API contract in TS idiom. Names are illustrative; finalized at impl time.

```ts
import { z } from "zod";
import {
  Registry, render, getSource, check, Composition, Message,
  RenderResult, CheckReport, Finding,
  PromptingPressError, PromptValidationError, PromptRenderError,
  UnknownPromptError, LoadError,
  type FieldError, type PromptDefinition,
} from "prompting-press";
```

## Registry / loader (US2)

```ts
const reg = new Registry();
reg.loadYaml(yamlText);                 // throws LoadError on malformed/shape-invalid; nothing partially loaded
reg.loadJson(jsonText);
reg.insert(def);                        // def: PromptDefinition (constructed-object path → JSON → consumer loader)
```
- All three paths normalize through the **one** Rust consumer loader (Q3). YAML↔JSON parity is structural.

## Render (US1)

```ts
const Vars = z.object({ name: z.string(), count: z.number().int().refine(n => n >= 0) });

const r: RenderResult = render(reg, "greet", Vars, { name: "Ada", count: 3 });
r.text;                                 // "Hi Ada, you have 3 messages"
r.name; r.variant;                      // "greet", "default"
r.templateHash; r.renderHash;           // 64-hex SHA-256 (byte-identical to Python/Rust)
r.guard;                                // string | null (separate from text; never concatenated)

// invalid input → PromptValidationError BEFORE any render, naming every offending field:
try { render(reg, "greet", Vars, { name: "Ada", count: -1 }); }
catch (e) {
  e instanceof PromptValidationError;   // true; NOT a ZodError
  e.errors;                             // [{ field: "count", code: "validation", message: "…" }]
}

getSource(reg, "greet");                // unrendered template source (no vars, no validation)
```
- Validation owned at the render boundary (Q1): `safeParse` runs once before any templating.
- Static-only data (no Zod schema) is also accepted (Q4) and marshaled directly.

## Check — the headline lint (US3)

```ts
const report: CheckReport = check(reg);
report.passed();                        // boolean (false if any findings)
for (const f of report.findings) {
  f.kind;                               // "undeclared_variable" | "untrusted_without_guard" | "reserved_variant_name" | "analysis_error"
  f.prompt; f.variant; f.detail;
}
```
- Pure analysis: mutates nothing, renders nothing (FR-019). Deterministic order.

## Compose (US4)

```ts
const msgs: Message[] = Composition.fromMessages([
  ["systemPreamble", SysVars, sysData],
  ["greet", Vars, { name: "Ada", count: 3 }],
]).resolve(reg);
msgs.map(m => m.role);                  // ["system", "user"]  (N entries → N ordered messages)
// one invalid entry → throws (PromptValidationError / UnknownPromptError); NO partial array returned.
// empty composition → resolve(reg) === []
// no .chain() method exists (FR-013).
```

## Error contract (C-06)

- Every failure is a `PromptingPressError` subclass with `readonly errors: FieldError[]`.
- Native types (`ZodError`, Rust errors) never appear on the public API.
- `code` ∈ the shared closed vocabulary. SEC-004: a value triggering a kernel parse/render error never
  appears in the thrown error's `message`/`.stack`/rows; the Zod mapper copies issue `message`+`path` only.

## Boundary

- No I/O, no LLM calls, no request-body assembly, no token counter (`outputModel` is metadata only).
- ESM-only; native binary loaded from the per-platform `optionalDependencies`.
