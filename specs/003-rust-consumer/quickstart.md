# Quickstart / Validation — Spec 003 (Rust consumer `prompting-press`)

How to prove the consumer crate works end to end. This is a **validation guide**, not implementation
— the real tests live in `crates/prompting-press/` (unit + integration) and run under the existing
moon/CI pipeline. Scenario IDs map to the spec's user stories and SCs.

## Prerequisites

- The spec-002 kernel is merged on `main` (it is). Toolchain via `mise` (Rust 1.95).
- New deps added (research D1/D2): `garde = { version = "0.23", features = ["derive","serde"] }`,
  `serde_yaml_ng = "0.10"`.

## Build & test

```bash
mise exec -- cargo build -p prompting-press
mise exec -- cargo test  -p prompting-press
# Gates that must stay green:
mise exec -- moon run ci:check-ffi              # SC-007: consumer FFI-free with garde + serde_yaml_ng added
mise exec -- moon run ci:check-floating-versions # new deps pinned
mise exec -- moon run :build
```

## Validation scenarios

### US1 — Validate typed inputs + render (P1, MVP)

| Scenario | Action | Expected | Proves |
|---|---|---|---|
| V1.1 | Define Vars struct (`#[derive(Serialize, Validate)]`, a `#[garde(custom)]` field), valid values, `render(reg,name,&vars,None,&GuardConfig::default())` | validation passes once → kernel renders → `RenderResult` (text + name/variant/hashes) | FR-001/002/009 |
| V1.2 | Same Vars with a field violating its validator | `Err(ConsumerError::Validation([FieldError{field,..}]))`, **no render** | FR-002, **SC-002** |
| V1.3 | Multiple fields fail at once | error lists all failing fields (one validate pass) | FR-002, SC-002 |
| V1.4 | Inspect a success/failure result's types | only `RenderResult` / `ConsumerError` — no garde `Report`, no `KernelError` | FR-014, **SC-006** |
| V1.5 | Render same prompt+vars twice | byte-identical text + equal hashes (kernel determinism surfaced) | SC-001 |

### US2 — Dual-input loader (P2)

| Scenario | Action | Expected | Proves |
|---|---|---|---|
| V2.1 | `Registry::load_yaml(yaml_doc)` | normalizes to `PromptDefinition` | FR-005 |
| V2.2 | `load_json(equivalent_json)` | representation identical to the YAML-loaded one | FR-006, **SC-003** |
| V2.3 | `insert(constructed_def)` | accepted on equal footing | FR-005 |
| V2.4 | malformed YAML/JSON or shape-violating data | `Err(ConsumerError)`, nothing partially loaded | FR-007 |
| V2.5 | YAML containing `no`/`yes`/`off` as a string value | parsed as a string, not a bool (yaml-rust2 / YAML 1.2 — Norway-safe) | FR-005 (D2) |

### US3 — Agreement + provenance lint (P2)

| Scenario | Action | Expected | Proves |
|---|---|---|---|
| V3.1 | `check(reg)` over a registry of well-formed prompts | pass (empty findings) | FR-016 |
| V3.2 | a prompt whose template references a var not in `def.variables` | `Finding::UndeclaredVariable{prompt,variant,name}` | FR-016/017, **SC-004** |
| V3.3 | an `untrusted`/`external` field used outside a declared guard position | `Finding::UntrustedOutsideGuard{prompt,field}` | FR-018, **SC-005** |
| V3.4 | inspect inputs after `check` | registry/defs/inputs unchanged; nothing rendered | FR-019, SC-004 |
| V3.5 | a multi-variant prompt | each variant's template analyzed against declared vars | FR-016 |

### US4 — Composition (P3)

| Scenario | Action | Expected | Proves |
|---|---|---|---|
| V4.1 | `Composition` with N appended (name,vars) entries → `resolve(reg)` | exactly N `Message{role,text}` in append order, each rendered with its own validated vars | FR-012, **SC-008** |
| V4.2 | one entry's vars fail validation | `Err(ConsumerError)` naming the entry/field; no partial-as-success | US4 sc.3 |
| V4.3 | render a fragment, pass its text into a parent prompt as a declared variable | composition-by-value works, no template include | FR-012 |

### Boundary

| Scenario | Action | Expected | Proves |
|---|---|---|---|
| V5.3 | `cargo tree -p prompting-press -i pyo3 / -i napi` | absent (FFI-free) | **SC-007** |
| _(V5.1/V5.2 token-hook rows removed — F4, hook dropped/deferred)_ | | | |

## Done = green

All scenarios pass, `cargo test -p prompting-press` green, `ci:check-ffi` +
`ci:check-floating-versions` + `:build` green, and `cargo tree` shows no pyo3/napi in the consumer.
The consumer contains no rendering/agreement/variant/hashing logic of its own (delegates to the
kernel) — confirmed by review.
