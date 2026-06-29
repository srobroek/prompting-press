# Quickstart: Opt-in unsafe render-error detail

Validation that the opt-in surfaces render detail, the default still scrubs, and it can't be enabled
implicitly. References [plan.md](./plan.md), [research.md](./research.md), and
[contracts/unsafe-detail-option.md](./contracts/unsafe-detail-option.md).

## Default still scrubs (SC-002) — the load-bearing safety check

```bash
# The existing scrub corpus exercises the DEFAULT (no flag) and MUST stay green:
cargo test -p prompting-press --test fuzz_scrub
cargo test -p prompting-press --lib error          # render_detail_secret_is_scrubbed
mise exec -- moon run ci:test-python               # python fuzz_scrub suite
mise exec -- moon run ci:test-node                 # node fuzz scrub suite
# All pass UNCHANGED — the feature added no behavior to the default path.
```

## Opt-in surfaces render detail (SC-001)

```rust
// Rust — render with the unsafe flag; a Render error now carries its real detail.
let res = greet.render(/* vars */, /* options with reveal_render_detail = true */);
// On a render failure: err is ConsumerError::Kernel([{ field:"template", code:"render", message:<REAL DETAIL> }])
```
```python
# Python — keyword-only, off by default:
greet.render(Vars, data=data, unsafe_reveal_render_detail=True)   # message carries the real render detail
```
```ts
// TypeScript — a RenderOptions flag:
greet.render(Vars, data, { unsafeRevealRenderDetail: true });     // err.errors[0].message = real detail
```

## No implicit enable (SC-003)

```bash
# There is NO global/env/default-true path. Verify by inspection + a test:
rg -n 'reveal|unsafe|unredacted' crates/ packages/ --glob '!**/target/**' | rg -iv 'render option|per-call'
# A unit test asserts a default render() (no flag) scrubs the Render detail.
```

## Render-detail-only (FR-010)

```bash
# Parse / ExcludedFeature / UnknownVariant / UndefinedVariable / Validation are byte-identical
# whether the flag is true or false. A test toggles the flag against each error kind and asserts
# only the Render case differs.
```

## Cross-binding parity (SC-004) + success path (SC-005)

```bash
# Same render-error scenario in all three bindings → equivalent detail via {field,code,message}.
# A successful render with the flag on vs off → byte-identical text/template_hash/render_hash.
```

## Governance artifact (R6)

```bash
# D3 decision record + SEC-004 carve-out note present:
ls docs/memory/decisions/*unsafe-render-detail*
rg -n 'carve-out|opt-in|D3' docs/memory/decisions/*unsafe-render-detail*
```

## Done when

- Default scrub corpus green (unchanged); opt-in surfaces real Render detail; no implicit-enable path;
  Parse/ExcludedFeature/etc. unaffected; cross-binding parity; success path byte-identical; D3 + SEC-004
  carve-out note authored; the option name + doc-comment read as risky (FR-012).
```
