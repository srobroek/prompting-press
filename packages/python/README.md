# prompting-press (Python)

Python binding of [Prompting Press](https://github.com/prompting-press/prompting-press) — a typed,
variant-aware **prompt-template** library. It turns *typed inputs + a template* into *rendered
text + provenance*, and nothing else.

Parsing, validation, rendering, hashing, and the agreement lint all run **once in the shared
Rust core** (Principle I / roadmap decision C-01). This package is a thin PyO3 binding: it
marshals typed values across the FFI boundary and re-exports the core's surface. It contains
**no rendering, hashing, or analysis logic of its own** (Principle II / C-02).

The import name is `prompting_press`; the PyPI distribution name is `prompting-press`.

## Install / build

The package ships as a single **abi3 wheel** (one wheel for CPython 3.10+):

```bash
pip install prompting-press   # not yet published (version 0.0.0) — build from source below
```

To build from source (a mixed Rust/Python project, built with [maturin](https://www.maturin.rs/)):

```bash
# from packages/python, with the local virtualenv active
maturin develop          # editable build into the active venv
maturin build            # produce a distributable *-abi3-*.whl
```

The PyO3 binding crate lives outside this directory at `crates/prompting-press-py/`
(a workspace member); `pyproject.toml`'s `[tool.maturin].manifest-path` references it.
The `PromptDefinition` shape under `prompting_press.generated` is **code-generated** from the
published JSON Schema (decision C-07) — do not hand-edit it.

## Boundary (what this library does NOT do)

Per Principle III (roadmap decision C-03), the library:

- does **no I/O** — no file reads, no network/DB/Redis/S3; the caller **pushes** prompt data in;
- never **calls an LLM**, assembles a provider request body, or parses model output;
- ships **no token counter** (there is no `count_tokens` surface at all);
- carries `output_model` as a **metadata reference only** — it never parses against it.

## Prompt — the primary object

A `Prompt` is an **immutable, fully-validated** prompt object. There is no registry — you
construct a `Prompt` once and hold or pass it directly. Four construction forms all normalize
through the same Rust loader (decision C-07):

```python
from prompting_press import Prompt
from prompting_press.generated import PromptDefinition

# From a plain dict or a validated PromptDefinition instance:
greet = Prompt({
    "name": "greet",
    "role": "user",
    "body": "Hi {{ name }}, you have {{ count }} messages",
    "variables": {
        "name":  {"type": "string",  "trusted": True},
        "count": {"type": "integer", "trusted": True},
    },
})

# From already-read YAML, JSON, or TOML text — the library does no file I/O:
greet = Prompt.from_yaml("""
name: greet
role: user
body: "Hi {{ name }}, you have {{ count }} messages"
variables:
  name:  { type: string,  trusted: true }
  count: { type: integer, trusted: true }
""")

greet = Prompt.from_json('{"name": "greet", "role": "user", "body": "..."}')
greet = Prompt.from_toml('name = "greet"\nrole = "user"\nbody = "..."')
```

**Construction validates.** An undeclared-variable reference, a template syntax error, an
un-analyzable template (e.g. `{% include %}`), or a reserved variant name raises
`PromptValidationError` (or `LoadError` for a malformed document) **here at construction** —
never silently deferred to render.

All constructors accept an optional `validators` keyword argument (a Pydantic model class). If
any variable in the definition has `validation_required = true`, a covering `validators` class
**must** be supplied at construction or construction raises.

## Validate, then render

Define a typed Vars model in Pydantic (Principle VI / C-06). Validators run in Python; the
validated values are then marshaled to the core, which renders and stamps provenance:

```python
from pydantic import BaseModel, field_validator
from prompting_press import Prompt

class Greeting(BaseModel):
    name: str
    count: int

    @field_validator("count")
    @classmethod
    def _non_negative(cls, value: int) -> int:
        if value < 0:
            raise ValueError("count must be non-negative")
        return value

greet = Prompt.from_yaml("""
name: greet
role: user
body: "Hi {{ name }}, you have {{ count }} messages"
variables:
  name:  { type: string,  trusted: true }
  count: { type: integer, trusted: true }
""")

result = greet.render(Greeting, data={"name": "Ada", "count": 3})

result.text           # "Hi Ada, you have 3 messages"  (the rendered BODY only)
result.name           # "greet"
result.variant        # "default"  (no variant selected → the reserved default arm)
result.template_hash  # SHA-256 of the resolved variant template source (64-char hex)
result.render_hash    # SHA-256 of the rendered output (64-char hex)
result.guard          # None  (no guard requested — see "Guard" below)
```

`prompt.render(model=None, *, data=None, variant=None, guard=None)` accepts either a Vars
**class** plus a `data` dict (validated for you, as above) or a pre-built Vars **instance**
(pass it as `model` with no `data`). `data`, `variant`, and `guard` are **keyword-only** (C-11).
Select a named variant with `greet.render(Greeting, data=..., variant="formal")`.

### The three-sets invariant (loud, never silent)

The Vars field names must match the prompt's declared `variables`, which must cover the
template's references. A mismatch — e.g. a Vars field `nam` against a template `{{ name }}` —
is surfaced as a **loud** `PromptRenderError` (`code == "undefined_variable"`), never a silent
empty render. (And the agreement check below catches the same class of gap before render.)

## The agreement lint (the headline differentiator)

`prompt.check() -> CheckReport` is a **pure** analysis pass (it never mutates the prompt and
never renders) — the static guarantee no file-based prompt library provides (decisions C-04 /
C-09). Wire it as a **CI gate**: the hard invariants (a template referencing an **undeclared
variable**, an un-analyzable template, a reserved variant name) are now enforced at
**construction**; `check()` surfaces the remaining live advisory: an `untrusted`/`external`
field used **without a declared guard**.

```python
from prompting_press import Prompt

ghosty = Prompt({
    "name": "ghosty",
    "role": "user",
    "body": "Hi {{ name }}",
    "variables": {"name": {"type": "string", "trusted": False}},
})

report = ghosty.check()

report.passed()      # False  (a clean prompt → True)
report.is_empty()    # False  (alias for passed(): True iff there are no findings)
bool(report)         # True   (truthy iff there ARE findings — the inverse of passed())
len(report)          # 1      (number of findings)
for f in report.findings:
    print(f.kind, f.prompt, f.variant, f.detail)
    # -> "untrusted_without_guard" "ghosty" "default" "..."
```

Each `Finding` is read-only: `.prompt`, `.variant` (`str | None`), `.kind`, `.detail`. The
stable `kind` vocabulary is:

| `kind`                    | meaning                                                              |
| ------------------------- | -------------------------------------------------------------------- |
| `undeclared_variable`     | the template references a name absent from the declared `variables`  |
| `untrusted_without_guard` | a prompt declares a `trusted: false` field but no guard               |
| `reserved_variant_name`   | a variant key collides with the reserved `default` arm               |
| `analysis_error`          | a template could not be statically analyzed (e.g. an excluded feature) |

Note: `undeclared_variable`, `reserved_variant_name`, and `analysis_error` are enforced at
construction and are therefore unreachable from a successfully-constructed `Prompt`. They
appear in the table for completeness; `untrusted_without_guard` is the only live advisory
`check()` returns.

## Immutability and `derive`

A `Prompt` has read-only properties (`name`, `role`, `body`, `variables`, `variants`,
`output_model`, `metadata`, `meta`) and no setters. The sole mutator is `derive`:

```python
derived = greet.derive({"body": "Hey {{ name }}, you have {{ count }} items"})
# greet is unchanged; derived is a new fully-validated Prompt.
```

`derive(overlay, *, validators=None)` shallow-replaces any subset of top-level fields, routes
the merged definition through the Rust consumer's full re-validation (agreement, parse,
reserved name), and returns a **new `Prompt`**. The original is untouched. Validators carry
forward from the original by default; pass `validators=SomeModel` to override.

## Composition (multi-message)

A `Composition` is an **explicit, ordered** sequence of `(prompt, vars, variant?)` entries
that resolves to a `list[Message]` in append order (Principle VI / C-06). Entries reference
**`Prompt` objects**, not names — there is no registry. There is deliberately **no fluent
`.chain()`** — it cannot cross the FFI boundary and collides with `Iterator::chain`.

```python
from prompting_press import Composition

comp = Composition()
comp.append(sys_preamble, SystemVars())           # variant defaults to "default"
comp.append(greet, Greeting(name="Ada", count=3))
comp.append(salute, Greeting(...), variant="formal")

# or build it all at once (2-tuples default the variant; 3-tuples select one):
comp = Composition.from_messages([
    (sys_preamble, SystemVars()),
    (greet,        Greeting(name="Ada", count=3)),
    (salute,       Greeting(...), "formal"),
])

messages = comp.resolve()   # [Message(role, text), ...] in append order
for m in messages:
    m.role   # "system" / "user" / "assistant" (the prompt definition's role)
    m.text   # that prompt rendered with the entry's own validated vars
```

`append` eager-validates the vars (a Pydantic model **instance**) at append time and stores
**nothing** on failure — no partial state. Rendering happens at `resolve`: a render failure
(unknown variant, strict-undefined reference) propagates as the mapped Python exception and
no partial list is returned. An empty composition resolves to `[]`.

## Guard usage doctrine — the system-prompt addendum

When a field is tagged `trusted: false`, pass `guard=GuardConfig(enabled=True)` to
`render`. The advisory `RenderResult.guard` string that comes back is an **opt-in, additive**
instruction (decision C-09) — and it is **separate from `text` by design**: the library never
concatenates them and there is deliberately **no composed field** (decided 2026-06-27, roadmap
*Deferred*; assembling the request body is the caller's job — Principle III).

Route it as a **system-prompt addendum**:

- **Single render** → put `RenderResult.guard` into your **system** prompt, and send
  `RenderResult.text` as the **user** message.
- **Multi-message** → place the guard as its **own `system` message** ahead of the rendered
  user turns.

```python
from prompting_press import Prompt, GuardConfig
from pydantic import BaseModel

class Ask(BaseModel):
    topic: str

ask = Prompt({
    "name": "ask",
    "role": "user",
    "body": "Tell me about {{ topic }}.",
    "variables": {"topic": {"type": "string", "trusted": False}},
})

result = ask.render(Ask, data={"topic": "rivers"}, guard=GuardConfig(enabled=True))
result.text    # "Tell me about rivers."   (body only — unchanged whether or not a guard is on)
result.guard   # advisory guard instruction → route into YOUR system prompt

# you assemble the request (the library never does):
messages = [
    {"role": "system", "content": result.guard},
    {"role": "user",   "content": result.text},
]
```

A `GuardConfig()` / `GuardConfig(enabled=False)` is equivalent to passing no guard (`guard`
stays `None`).

## Errors

All failures raise the binding's own exception hierarchy. **Native Pydantic and Rust error
types never leak across the FFI boundary** (decision C-06); each error carries an `.errors`
list of normalized rows, each a `(field, code, message)` triple (`FieldError`):

```text
PromptingPressError        # base; carries .errors -> list[FieldError]
├── PromptValidationError  # typed-Vars validation failed   (code = "validation")
├── PromptRenderError      # kernel render/source/analysis failure
│                          #   (code in unknown_variant | undefined_variable | parse | render | excluded_feature)
├── UnknownPromptError      # a prompt name was absent from the registry  (code = "unknown_prompt")
└── LoadError              # malformed YAML/JSON/TOML or a shape violation at construction  (code = "load")
```

```python
from prompting_press import Prompt, PromptValidationError

greet = Prompt.from_yaml("""
name: greet
role: user
body: "Hi {{ name }}, you have {{ count }} messages"
variables:
  name:  { type: string,  trusted: true }
  count: { type: integer, trusted: true }
""")

try:
    greet.render(Greeting, data={"name": "Ada", "count": -1})
except PromptValidationError as exc:
    for row in exc.errors:
        print(row.field, row.code, row.message)   # "count" "validation" "..."
```

Sensitive rejected inputs are never echoed onto the error surface (only the validator's own
value-free message is copied across — SEC-004-PY).

## Layout

```
packages/python/
├── pyproject.toml                  # maturin build backend; points at the binding crate
├── uv.lock                         # hash-locked codegen toolchain (dependency-group)
├── README.md
├── scripts/
│   └── codegen.sh                  # regenerates the Pydantic shape from the JSON Schema
├── tests/                          # Python-observable binding tests
└── python/
    └── prompting_press/
        ├── __init__.py             # the public facade (re-exports the compiled extension)
        └── generated/              # codegen output — DO NOT EDIT (freshness-gated)
            ├── __init__.py
            ├── README.md
            └── prompt_definition.py  # generated Pydantic v2 model
```
