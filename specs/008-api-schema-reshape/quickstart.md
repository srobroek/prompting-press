# Quickstart / Validation Guide — Pre-publish API & schema reshape

Runnable scenarios that prove the reshape works end-to-end in each binding. These are the acceptance
demonstrations for `/speckit.verify`; full test bodies live in `tasks.md` / the suites, not here.

## Prerequisites

- Rust 1.95 (mise), Python ≥3.12 (raised floor — R5), Node 20+ / pnpm, moon.
- Build: kernel + consumer `cargo build`; Python `maturin develop` **from `packages/python/`** (not repo root —
  see workflow gotchas); TS `pnpm -C packages/typescript build`.

## Scenario A — Construct a Prompt and render (US1, all bindings)

**Rust**
```rust
let p = Prompt::from_yaml(yaml_text, None)?;        // validating; Err on invalid
let out = p.render::<MyVars>(vars, RenderOpts::default())?;
assert!(!out.template_hash.is_empty());
```
**Python**
```python
p = Prompt.from_yaml(yaml_text)                      # raises on invalid
out = p.render(MyModel, data={...})
```
**TypeScript**
```ts
const p = new Prompt(shapeObject, mySchema);         // throws on invalid
const out = p.render(mySchema, { name: "Ada" });
```
**Expected:** a `RenderResult` with `text`, `template_hash`, `render_hash`; **no `Registry` type exists**
(grep the public surface — SC-001).

## Scenario B — Cross-binding hash parity (SC-003)

Render the same prompt + vars in all three; assert `template_hash` and `render_hash` are byte-identical across
bindings. (The existing conformance corpus is the gate; the kernel is unchanged, so parity is structural.)

## Scenario C — `with(overlay)` is the sole mutator, original untouched (US2, SC-004)

```ts
const base = new Prompt(shape, schema);
const terse = base.with({ variants: { ...base.variants, terse: { body: "{{ name }}!" } } });
assert(base.variants.terse === undefined);           // original untouched
assert(terse.variants.terse !== undefined);          // derived has it
```
Also: an overlay that yields an invalid merged definition (e.g. a `body` referencing an undeclared variable)
returns/raises/throws a structured error and produces **no** prompt.

## Scenario D — Construction fails on un-analyzable templates (US3, Q4/R7)

Construct a prompt whose body is `{% include "x" %}` (excluded feature) or `Hello {{ name` (syntax error).
**Expected:** construction **fails** with a structured error in all three bindings — you cannot hold such a
`Prompt`. (`check()` is never reached for it.)

## Scenario E — `validation_required` coverage (US7, SC-009)

Given a prompt with a variable `age` declaring `validation_required: true`:
- **Covered:** supply a validator (Zod schema / Pydantic model / garde Vars) that includes `age` → constructs.
- **Uncovered (TS/Python):** supply a validator missing `age` → construction **throws/raises** naming `age`.
- **Rust:** the generic `V` Vars type must include the field → a missing field is a **compile error**, not a
  runtime throw (Principle VI v1.2.0).

## Scenario F — `origin` field (US6, SC-002)

- A document declaring `origin: untrusted` for a variable loads/constructs; the tag is readable as `origin`.
- A document still using `provenance:` is **rejected** (unknown field — schema is closed).
- The opt-in guard names the `origin: untrusted`/`external` fields exactly as before (behavior unchanged).
- The render-result `template_hash`/`render_hash` are unchanged in name and value.

## Scenario G — Three text factories (SC-010)

The same logical prompt expressed as YAML, JSON, and TOML constructs an equivalent `Prompt` via
`fromYaml`/`fromJson`/`fromToml`; malformed text in any format yields a structured load error (never a panic).

## Scenario H — Fixture move + gates (SC-006/007)

- `schemas/jsonschema/tests/fixtures/{valid,invalid}/` resolves; `validate_fixtures.py`, the
  `schemas:validate-fixtures` task, and `conformance/schema/manifest.json` all reference the new path.
- The `variant-named-default` loader-exclusion note is preserved (architecture memory A1).
- All CI gates green: FFI isolation, codegen freshness (incl. the new Zod codegen twice-run), agreement/origin
  lint, conformance.

## Verification commands

```bash
mise exec -- moon run schemas:check-schema schemas:validate-fixtures schemas:codegen-check
cargo test -p prompting-press-core -p prompting-press -p prompting-press-py -p prompting-press-node
mise exec -- moon run ci:test-python ci:test-node ci:conformance ci:check-ffi
```
