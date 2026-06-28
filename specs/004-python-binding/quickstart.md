# Quickstart / Validation Guide — Python binding (`prompting-press-py`)

Runnable scenarios that prove the binding works end-to-end. Implementation lives in `tasks.md` + the
binding crate; this is the **validation** guide. Scenarios map to the spec's user stories + SCs.

## Prerequisites

- Toolchain via `mise` (Rust `1.95.0`, Python `3.14.x` for dev/test; abi3 floor 3.10).
- Build the extension into the dev venv: `mise exec -- maturin develop -m crates/prompting-press-py/Cargo.toml`
  (or `moon run python:build` once wired). Pydantic v2 installed as a package dep.
- Codegen current: `bash packages/python/scripts/codegen.sh` (no diff expected — freshness-gated).

## Build & import (SC-009)

```bash
mise exec -- maturin build -m crates/prompting-press-py/Cargo.toml   # produces an abi3 wheel
python -c "import prompting_press; print(prompting_press.__version__)"  # import succeeds
```
**Expected**: a `*-abi3-*.whl` is produced; `import prompting_press` succeeds in a fresh 3.10+ env.

## US1 — validate typed inputs and render (P1; SC-001/002)

```python
from pydantic import BaseModel, field_validator
from prompting_press import Registry, render, PromptValidationError

class Greeting(BaseModel):
    name: str
    count: int
    @field_validator("count")
    @classmethod
    def non_negative(cls, v):  # custom validator
        assert v >= 0, "count must be >= 0"; return v

reg = Registry()
reg.load_yaml('''
name: greet
role: user
body: "Hi {{ name }}, you have {{ count }} messages"
variables:
  name:  { type: string,  provenance: trusted }
  count: { type: integer, provenance: trusted }
''')

r = render(reg, "greet", Greeting, data={"name": "Ada", "count": 3})  # data/variant/guard keyword-only (C-11)
assert r.text == "Hi Ada, you have 3 messages"
assert r.variant == "default" and len(r.template_hash) == 64
```
- **Invalid input** → `PromptValidationError`, **no** render:
```python
try:
    render(reg, "greet", Greeting, data={"name": "Ada", "count": -1})
    assert False
except PromptValidationError as e:
    # e.errors is a list of FieldError (attribute access, not dict subscription).
    assert any(row.field == "count" and row.code == "validation" for row in e.errors)
```
**Expected**: valid → text + provenance; invalid → structured exception naming `count`, kernel never reached.

## US2 — YAML / JSON / object parity (P2; SC-003)

```python
# Same logical prompt three ways → identical render for identical inputs.
reg_y = Registry(); reg_y.load_yaml(yaml_text)
reg_j = Registry(); reg_j.load_json(json_text)
reg_o = Registry(); reg_o.insert(PromptDefinition.model_validate(obj))
out = lambda reg: render(reg, "greet", Greeting, data={"name":"Ada","count":3}).text
assert out(reg_y) == out(reg_j) == out(reg_o)
```
- **Malformed** input → `LoadError`, nothing partially loaded:
```python
import pytest; from prompting_press import LoadError
with pytest.raises(LoadError):
    Registry().load_yaml("name: [unterminated")
```
**Expected**: 100% parity across the three input forms; malformed input raises `LoadError`.

## US3 — agreement + provenance lint as a CI check (P2; SC-004/005)

```python
from prompting_press import Registry, check
reg = Registry()
reg.load_yaml(undeclared_ref_prompt)      # body references {{ ghost }} not in variables
reg.load_yaml(untrusted_without_guard)    # declares an untrusted var, no meta.guard
report = check(reg)
assert not report.passed()
kinds = {(f.prompt, f.kind) for f in report.findings}
assert any(k == "undeclared_variable" for _, k in kinds)
assert any(k == "untrusted_without_guard" for _, k in kinds)
# clean prompt → empty report
assert check(clean_registry).passed()
```
**Expected**: undeclared-var + untrusted-without-guard flagged with prompt/variant/field; clean passes;
nothing rendered or mutated. (Reserved-`default` and un-analyzable templates surface as
`reserved_variant_name` / `analysis_error` findings.)

## US4 — multi-message composition (P3; SC-008)

```python
from prompting_press import Composition
comp = Composition.from_messages([
    ("system_preamble", Sys(...), None),
    ("greet", Greeting(name="Ada", count=3), None),
])
msgs = comp.resolve(reg)
assert [m.role for m in msgs] == ["system", "user"]
assert len(msgs) == 2
```
- One invalid entry → exception, **no** partial result. Empty composition → `[]`.
**Expected**: N entries → exactly N ordered `{role, text}` messages, each rendered with its own
validated vars.

## Boundary & isolation (SC-006/007/010)

```bash
mise exec -- moon run ci:check-ffi --force        # pyo3 only in prompting-press-py; kernel+consumer FFI-free
mise exec -- moon run schemas:codegen-check --force  # generated Pydantic shape is fresh (byte-identical)
mise exec -- cargo test -p prompting-press-py     # Rust-side marshaling + scrub tests
mise exec -- pytest packages/python/tests         # Python-side US1-US4 + exception scenarios
rg -n "count_tokens|token_count|TokenCount|count-tokens" packages/python/python crates/prompting-press-py/src || echo "no token surface (F4)"
```
**Expected**: FFI gate green; codegen fresh; no native error type on the public API (every error is a
`PromptingPressError` subtype); SEC-004 — a seeded secret in a render-error value never appears in the
raised exception's `str()`; no token-counting surface anywhere.
