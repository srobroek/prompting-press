# Phase 1 Data Model — Spec 003 (Rust consumer `prompting-press`)

The consumer adds the **language-native layer** the kernel deliberately omits: typed-Vars validation,
the dual-input loader, the registry, the `check()` lint, error normalization, composition, and the
and error normalization. It introduces a few small consumer-owned types and **wraps** the kernel — it
defines no rendering/agreement/variant/hashing logic (C-01).

## Consumed from spec 002 (the kernel — wrapped, not redefined)

- `prompting_press_core::PromptDefinition` (re-exported generated shape) — `{ name, role, body,
  variables: HashMap<String, VariableDecl>, variants, meta, metadata, output_model }`. **`variables`
  is the authoritative declared set** for the agreement check (clarify Q1).
- `render(&def, variant: Option<&str>, values: minijinja::Value, &GuardConfig) -> Result<RenderResult, KernelError>`
- `get_source(&def, variant) -> Result<&str, KernelError>`
- `required_roots(&def, variant) -> Result<Agreement, KernelError>` where `Agreement { variant,
  required_roots: BTreeSet<String> }` — the per-variant referenced roots (the `⊆` input).
- `provenance_view(&def) -> ProvenanceView { untrusted: BTreeSet<String>, external: BTreeSet<String> }`.
- `RenderResult { text, name, variant, template_hash, render_hash, guard: Option<String> }`,
  `GuardConfig { enabled, template }` (has `Default`), closed `KernelError` (5 variants).

## Consumer-defined types

### `Registry` (FR-008a, clarify Q2)
A library-owned map of prompt name → loaded `PromptDefinition`.

| Aspect | Decision |
|---|---|
| Shape | `Registry { prompts: BTreeMap<String, PromptDefinition> }` (BTreeMap → deterministic iteration for `check()` ordering) |
| Population | `insert`/`load_yaml`/`load_json`/`load_def` (via the dual-input loader, FR-005) |
| Resolution | `render(name, …)` / `get_source(name, …)` / `check()` look up by name; absent name → `ConsumerError::UnknownPrompt` (FR-008a) |

### Typed Vars (application-defined, FR-001/003a)
Not a library type — the application authors a struct deriving **both** `serde::Serialize` and the
native validator (garde `Validate`). The library accepts it generically (`V: Serialize + Validate`).
Flow: `validate()` → on success, serialize to the kernel value type (`minijinja::Value::from_serialize`,
pending research confirm) → kernel `render`. The library never inspects the struct's fields.

### `RenderResult` (re-exported / thinly wrapped)
The kernel's `RenderResult` surfaced as the consumer's render output (library-owned data; FR-009). No
new fields; the consumer may re-export it directly or wrap it 1:1.

### `Message` + composition (FR-012, clarify — `Vec` + `append_*`, never `.chain()`)
| Type | Shape |
|---|---|
| `Message` | `{ role: String, text: String }` — one resolved message (role from the prompt's `role` metadata) |
| Composition | an ordered `Vec<(prompt-ref, vars)>` built via `append_*`; `resolve() -> Result<Vec<Message>, ConsumerError>` renders each entry with its own validated vars, in order |

### `NormalizedError` / `ConsumerError` (FR-014, FR-015)
The common structured shape; the ONLY error type on the public API. Native types (garde `Report`,
kernel `KernelError`) are mapped here and never leak.

| Field | Type | Notes |
|---|---|---|
| (entry) | `FieldError { field: String, code: String, message: String }` | one per failing field / error condition |
| error | `ConsumerError(Vec<FieldError>)` (or an enum wrapping it + `UnknownPrompt`) | exact shape settled in plan vs research |

Mapping rules:
- **garde `Report`** → one `FieldError` per reported path: `field` = the garde path, `code` = the
  garde error kind, `message` = the garde message. (Shape pending research on `Report`'s API.)
- **`KernelError`** → one `FieldError` per variant: e.g. `UnknownVariant{requested}` → `{field:
  "variant", code: "unknown_variant", message: …}`; `UndefinedVariable{name}` → `{field: name, code:
  "undefined_variable", …}`; `Parse`/`Render`/`ExcludedFeature` → `{field: "template", code: …, …}`.
- **SEC-004 scrub (FR-015):** `Parse`/`Render` `detail` strings may carry bound-value content — the
  normalizer MUST NOT copy raw detail verbatim into `message`/logs; use a sanitized/templated message.

### `CheckReport` (FR-016..020)
Output of the agreement + provenance lint over a `Registry`. Pure analysis (no mutation, no render).

| Field | Type | Notes |
|---|---|---|
| `findings` | `Vec<Finding>` | empty = pass |
| `Finding` | `{ prompt: String, variant: Option<String>, kind: FindingKind, detail: String }` | actionable (FR-020) |
| `FindingKind` | `UndeclaredVariable { name }` \| `UntrustedWithoutGuard { field }` | the two lint classes (FR-016/018) |

The agreement finding = `required_roots(def, variant)` (kernel) **minus** `def.variables.keys()`
(declared) → any leftover is `UndeclaredVariable`. The provenance finding = `provenance_view(def)`
reports untrusted/external fields; if the prompt declares any such field but carries no guard
configuration covering it, each uncovered field is an `UntrustedWithoutGuard` finding (reframed F1 —
the kernel has no in-template "guard position"; the lint is "declared untrusted input + no guard set up").

### ~~`TokenCountHook`~~ — DROPPED (F4)
The token-count hook is removed from spec 003 (deferred to a later spec). No token-counting type ships.

## State / lifecycle

Operations are pure over their inputs except the registry's load/insert mutators (which build the
in-memory map — no I/O beyond the caller handing in already-read YAML/JSON text or an object; the
crate does no file reads itself, C-03). `render`/`get_source`/`check`/`resolve` do not mutate the
registry. Validation + serialization happen in the consumer; the kernel receives already-valid values.
