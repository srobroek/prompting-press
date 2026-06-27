# Contract — Public Python API (`prompting_press`)

The stable surface a Python application imports. Mirrors the spec-003 Rust consumer in Python idiom.
Signatures are the contract; types are from [data-model.md](../data-model.md). Behavior is delegated to
the shared Rust core (Principle I) — render byte-parity with the Rust/TS bindings is structural.

```python
from prompting_press import (
    Registry, RenderResult, Message, Composition,
    CheckReport, Finding,
    render, get_source, check,
    PromptingPressError, PromptValidationError, PromptRenderError,
    UnknownPromptError, LoadError,
)
from prompting_press.generated import PromptDefinition
from pydantic import BaseModel
```

## Registry — dual-input loader (FR-005..008a)

```python
reg = Registry()
reg.load_yaml(text: str) -> None     # text marshaled to the Rust consumer's YAML loader
reg.load_json(text: str) -> None     # text marshaled to the Rust consumer's JSON loader
reg.insert(definition: PromptDefinition) -> None   # constructed object -> JSON -> consumer load_json
```
- Malformed input → **`LoadError`** (nothing partially loaded). All three paths normalize to one
  representation; YAML and JSON of the same prompt behave identically (parity structural).

## render / get_source (FR-002, FR-009..011)

```python
render(
    reg: Registry,
    name: str,
    vars_model: type[BaseModel], data: dict | <or> vars_instance: BaseModel,
    variant: str | None = None,
    guard: GuardConfig | None = None,
) -> RenderResult

get_source(reg: Registry, name: str, variant: str | None = None) -> str
```
- The binding **owns validation** (clarified Q1): validates `data` against `vars_model` (or re-validates
  `vars_instance`) via `model_validate` **before any templating**.
- Validation failure → **`PromptValidationError`** (every offending field named); **no** render performed.
- Unknown name → **`UnknownPromptError`**. Unknown variant / undefined variable / parse / render →
  **`PromptRenderError`** (`parse`/`render`/`excluded_feature` messages scrubbed — SEC-004).
- Success → **`RenderResult`** (text + name + variant + `template_hash` + `render_hash` + optional
  guard). The hashes are byte-identical to the other bindings' for the same logical input.
- `get_source` returns the unrendered template source for the resolved variant.

## check — agreement + provenance lint (FR-016..020)

```python
report: CheckReport = check(reg: Registry)
report.passed() -> bool
report.findings -> list[Finding]    # Finding(prompt, variant, kind, detail)
```
- Pure analysis: mutates nothing, renders nothing. `kind` ∈ `{undeclared_variable,
  untrusted_without_guard, reserved_variant_name, analysis_error}`. Deterministic order.
- A non-empty `findings` ⇒ a CI gate should fail (exit non-zero).

## Composition — multi-message (FR-012/013)

```python
comp = Composition.from_messages([(name, vars, variant?), ...])   # or Composition() + comp.append(...)
comp.append(name: str, vars, variant: str | None = None) -> None  # eager-validate; raises on invalid
messages: list[Message] = comp.resolve(reg)                       # ordered [Message(role, text)]
```
- Vars validated **eagerly at append/from_messages** (option (a)); one invalid entry → exception, and
  `resolve` never returns a partial list as success. Empty composition → `[]`. No `.chain()`.

## Exceptions (FR-014/015)

```python
class PromptingPressError(Exception):
    errors: list[FieldError]    # each: {field: str, code: str, message: str}
class PromptValidationError(PromptingPressError): ...   # code: "validation"
class PromptRenderError(PromptingPressError): ...       # code: unknown_variant|undefined_variable|parse|render|excluded_feature
class UnknownPromptError(PromptingPressError): ...      # code: "unknown_prompt"
class LoadError(PromptingPressError): ...               # code: "load"
```
- One base, `except`-by-class or branch on `row.code`. `code` vocabulary identical to the Rust
  consumer's. Native `pydantic.ValidationError` and Rust error types **never** appear on this surface.

## Boundary guarantees (C-02/C-03)

- `pyo3` appears only in `prompting-press-py`; `ci:check-ffi` keeps the kernel + Rust consumer FFI-free.
- No I/O, no model calls, no request-body assembly, no output parsing, **no token counting**.
  `output_model` is metadata only.
- Packaged as a maturin abi3 wheel (CPython 3.10 floor), importable as `prompting_press`.
