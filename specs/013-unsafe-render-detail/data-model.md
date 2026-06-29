# Phase 1 Data Model: Opt-in unsafe render-error detail

No persistent/runtime data. The "model" is one per-call option + the two error-normalization paths it
selects between. No kernel state change.

## Entities

### Render option: the unsafe-detail flag

- A per-call boolean carried on the render options surface (per binding — see
  [contracts/unsafe-detail-option.md](./contracts/unsafe-detail-option.md)). Off-by-default. Carries no
  data beyond on/off. Not stored on the `Prompt`; not global. Not a pluggable interface.

### Error normalization paths (two, selected by the flag)

- **Scrubbing path** (default): the existing `impl From<KernelError> for ConsumerError` — `Render` detail
  discarded, fixed message. Used by every non-render call site and by render when the flag is false.
- **Revealing path** (opt-in): a new explicit consumer constructor that behaves identically to the
  scrubbing path EXCEPT that a `KernelError::Render` surfaces its real `detail` in `message`. Used only by
  `Prompt::render` when the per-call flag is true.

### Normalized error (`{field, code, message}`) — unchanged shape

- The existing cross-binding error contract. The flag changes only the `message` content for a `Render`
  error; `field` (`"template"`), `code` (`"render"`), and the overall shape are invariant. Native error
  types never cross FFI.

## Relationships

```
caller sets per-call flag (render option, off by default)
        │
        ▼
Prompt::render(... , reveal)
        │  on KernelError:
        ├── reveal=false ─→ ConsumerError::from(kernel)            [scrubbing default — unchanged]
        └── reveal=true  ─→ ConsumerError::from_kernel_revealing(kernel, true)
                                   │  Render → surface detail;  all other kinds → identical to scrubbing
                                   ▼
                          {field, code, message}  (shape unchanged; bindings normalize from here)
```

## State transitions

None. The flag is read per call; it mutates nothing and persists nothing. Same inputs + same flag ⇒
same error; same inputs across flag values differ ONLY in a `Render` error's `message`.
