# Contract — the `Prompt` public API (per binding)

The library's external interface is its public API in each of the three packages. This is the contract the
reshape must deliver; capability is uniform, idiom is native (Principle VI). Errors are normalized to
`[{field, code, message}]`; no native error type crosses FFI (C-06).

## Common shape (capability contract)

```
Prompt (immutable)
  constructor(shape, validators?)            -> Prompt | <error>     # primary; validating
  static fromYaml(text, validators?)         -> Prompt | <error>
  static fromJson(text, validators?)         -> Prompt | <error>
  static fromToml(text, validators?)         -> Prompt | <error>
  // read-only accessors: name, role, body, variables, variants, outputModel?, metadata?, meta?
  render(<validator+data per idiom>, { variant?, guard? }) -> RenderResult
  getSource({ variant? })                    -> string
  check()                                    -> CheckReport            # advisory (origin/guard)
  with(overlay, validators?)                 -> Prompt | <error>       # sole mutator
Composition
  (ordered aggregation of Prompt objects + vars/variant)
  resolve()                                  -> [{ role, text }]
```

Removed from every binding: `Registry`, `render(reg, name, …)`, `getSource(reg, …)`, `check(reg)`.

## Rust (`prompting-press`)

```rust
pub struct Prompt { /* wraps generated PromptDefinition; immutable */ }

impl Prompt {
    pub fn new(shape: PromptDefinition) -> Result<Prompt, ConsumerError>;
    pub fn from_yaml(text: &str) -> Result<Prompt, ConsumerError>;
    pub fn from_json(text: &str) -> Result<Prompt, ConsumerError>;
    pub fn from_toml(text: &str) -> Result<Prompt, ConsumerError>;   // toml@1.1.2

    pub fn name(&self) -> &str;
    pub fn variables(&self) -> &BTreeMap<String, VariableDecl>;
    pub fn variants(&self) -> /* read-only view */;
    // … other read-only accessors

    pub fn render<V: Serialize + Validate>(
        &self, vars: V, variant: Option<&str>, guard: &GuardConfig,
    ) -> Result<RenderResult, ConsumerError>;                        // validator = the generic V (garde)
    pub fn get_source(&self, variant: Option<&str>) -> Result<String, ConsumerError>;
    pub fn check(&self) -> CheckReport;
    pub fn with(&self, overlay: PromptOverlay) -> Result<Prompt, ConsumerError>;  // shallow-replace, re-validate
}
```
- **`validation_required` in Rust is declarative**; coverage is the generic `V` (a field a `validation_required`
  variable needs is a field on `V`, garde-validated — compile-time). No runtime coverage throw (Principle VI
  v1.2.0).
- Call shape: single `Option<&str>` stays positional (C-11 Rust threshold); an overlay/options struct is used
  where a 2+-optional tail would otherwise appear.

## Python (`prompting_press`)

```python
class Prompt:
    def __init__(self, shape: PromptDefinition | dict, *, validators: ValidatorMap | None = None) -> None: ...
    @classmethod
    def from_yaml(cls, text: str, *, validators=None) -> "Prompt": ...
    @classmethod
    def from_json(cls, text: str, *, validators=None) -> "Prompt": ...
    @classmethod
    def from_toml(cls, text: str, *, validators=None) -> "Prompt": ...   # stdlib tomllib (3.12 floor)
    # read-only @property accessors: name, role, body, variables, variants, ...
    def render(self, model, *, data=None, variant=None, guard=None) -> RenderResult: ...
    def get_source(self, *, variant=None) -> str: ...
    def check(self) -> CheckReport: ...
    def with_(self, overlay, *, validators=None) -> "Prompt": ...   # `with` is a keyword → `with_` / a method name TBD at impl
```
- Construction **raises** (`PromptValidationError`) on invalid shape / parse / agreement / uncovered
  `validation_required` variable. Coverage introspects Pydantic `model_fields`.
- Keyword-only optional tail (C-11). SEC-004 scrub on every error path (copy `msg`/`loc` only).
- **Naming open item:** `with` is a Python keyword — the method needs a non-reserved name (`with_`, `derive`,
  `replace`). Pick at implementation; flagged in the decision log.

## TypeScript (`prompting-press`)

```ts
export class Prompt {
  constructor(shape: PromptShape, validators?: ValidatorMap);   // THROWS PromptValidationError on invalid
  static fromYaml(text: string, validators?: ValidatorMap): Prompt;
  static fromJson(text: string, validators?: ValidatorMap): Prompt;
  static fromToml(text: string, validators?: ValidatorMap): Prompt;   // smol-toml@1.7.0
  readonly name: string; readonly role: string; /* …read-only accessors… */
  render(schema: ZodLikeSchema, data: unknown, opts?: RenderOptions): RenderResult;
  getSource(opts?: { variant?: string }): string;
  check(): CheckReport;
  with(overlay: Partial<PromptShape>, validators?: ValidatorMap): Prompt;   // throws on invalid merged
}
```
- TS shape is a **generated Zod schema** (`json-schema-to-zod@2.8.1 --zodVersion 4 --type`), giving the runtime
  enforcer + `z.infer` static type.
- `new Prompt()` **throws** on invalid (a TS `new` can't return a result — Q6); the thrown
  `PromptValidationError` carries `[{field, code, message}]`.
- Coverage check uses `ZodObject.shape` (`field in schema.shape`); a non-introspectable validator object → a
  documented "cannot assert coverage" limitation.

## Contract tests (map to SC)

- Construction valid/invalid per path (SC-001/005/009/010); shape/parse/agreement/coverage failures each
  return the normalized error.
- `with` immutability + merged-validation (SC-004).
- Render hash parity vs pre-reshape + cross-binding (SC-003) — conformance corpus.
- `origin` accepted, `provenance` rejected, guard unchanged (SC-002, US6).
- No `Registry` symbol in any public surface (SC-001).
- Fixture move + gates (SC-006/007) — schema/conformance tasks.
