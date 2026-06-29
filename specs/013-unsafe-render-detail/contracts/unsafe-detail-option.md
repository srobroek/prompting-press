# Contract: the unsafe-render-detail opt-in

A single per-render-call boolean, off-by-default, that selects render-error-detail surfacing over
scrubbing. Same semantics across all three bindings; delivered through the shared error contract.

## The option (per binding, per-call)

| Binding | Where | Default |
|---------|-------|---------|
| Rust | a per-render argument/option on `Prompt::render` (a `RenderOptions`-style field or explicit arg) | scrub (off) |
| Python | keyword-only `render(..., *, <unsafe_flag>=False)` | scrub (off) |
| TypeScript | `RenderOptions { ..., <unsafeFlag>?: boolean }` | scrub (off) |

- The option name MUST signal risk (contains "unsafe"/"unredacted"); the call site MUST carry a
  doc-comment warning that enabling it may place bound-value content (untrusted input / PII / secrets)
  into the returned error message + any log/stack derived from it (FR-012).
- It is set ONLY per render call. There is NO per-Prompt, construction-time, environment, or global form
  (FR-003/FR-009).

## Behavior

Let `reveal` be the per-call flag. On a render that produces a `KernelError`:

| Kernel error | `reveal = false` (default) | `reveal = true` |
|--------------|----------------------------|-----------------|
| `Render { detail }` | message = fixed scrubbed string (unchanged) | message = the real `detail`, surfaced verbatim |
| `Parse { detail }` | detail preserved (D2) | **identical** — `reveal` does not affect Parse |
| `ExcludedFeature` | templated (unchanged) | **identical** — unaffected |
| `UnknownVariant` / `UndefinedVariable` | name surfaced (unchanged) | **identical** — unaffected |
| `Validation` | unchanged | **identical** — unaffected |

- The error **shape is unchanged** in all cases: `{ field: "template", code: "render", message: <…> }`.
  Only the `message` content differs for the `Render` case. Native error types never leak (FR-005/006).
- On a **successful** render, the option has no effect: `text`/`template_hash`/`render_hash`/`variant`/
  `guard` are byte-identical with `reveal` true or false (FR-004/SC-005).

## Consumer seam (implementation contract)

- The existing `impl From<KernelError> for ConsumerError` is UNCHANGED and remains the scrubbing default
  used by all non-render paths.
- A new explicit consumer constructor (e.g. `ConsumerError::from_kernel_revealing(err, reveal: bool)`)
  is what `Prompt::render` calls with the per-call flag. `reveal = false` ⇒ byte-for-byte the same result
  as the `From` impl. `reveal = true` ⇒ only the `Render` arm changes (surfaces `detail`).
- Both binding crates route kernel errors on the render path through this same consumer constructor, so
  they inherit the behavior; they do NOT re-implement the scrub decision.

## Invariants the tests must hold

1. **Default scrubs** — with no option set anywhere, the existing scrub corpus passes unchanged (SC-002).
2. **No implicit enable** — no global/env/default-true path enables it (SC-003; inspection + a test).
3. **Opt-in surfaces** — with the flag true, a `Render` error's real detail appears in `message` (SC-001).
4. **Render-only** — Parse/ExcludedFeature/etc. are byte-identical regardless of the flag (FR-010).
5. **Cross-binding parity** — the same scenario yields equivalent detail exposure via `{field,code,message}`
   in Rust/Python/TS (SC-004).
6. **Success path unchanged** — toggling the flag changes nothing on a successful render (SC-005).
