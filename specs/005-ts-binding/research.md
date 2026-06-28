# Research — TypeScript binding (`prompting-press-node`)

Phase 0. Resolves the plan's open D-items. All version checks done by querying crates.io / npm
**directly** this cycle (the project's fabricated-subagent-version guard) — not via a research subagent.

## Verified versions (2026-06-27)

| Dependency | Pin | Source | Note |
|---|---|---|---|
| `napi` / `napi-derive` | **3.9.4** | crates.io (updated 2026-06-24) | crate declares floating `"3"` → pin exact 3.9.4 |
| `zod` | **4.4.3** | npm latest | v4 (clarified Q7); the mapper reads v4's issue API |
| `@napi-rs/cli` | **3.7.2** | npm latest | matches scaffold |
| `json-schema-to-typescript` | **15.0.4** | npm latest | matches scaffold |
| `typescript` | **5.9.2** | scaffold pin | |

## D1 — napi 3.x class/function surface

- **Decision**: Expose the binding via `#[napi]` on Rust structs (→ JS classes) and free functions
  (→ JS functions), mirroring the 004 pyclass layout: `Registry`, `RenderResult`, `CheckReport`,
  `Finding`, `Composition`, `Message` classes + `render`/`getSource`/`check` functions. Each class wraps
  the consumer/kernel type 1:1 with `#[napi(getter)]` read-only accessors for result/finding/message
  fields.
- **Rationale**: Same delegation shape as the merged 004 binding; napi-derive 3.x supports
  `#[napi]` structs with getters and constructors. Field naming is camelCase on the JS side
  (`templateHash`/`renderHash`) — napi-derive renames by default; keep the kernel's snake_case in Rust.
- **Alternatives**: returning plain object literals instead of classes — rejected (loses `instanceof`
  ergonomics for `RenderResult`/`Finding` and the read-only contract).

## D2 — JS value ↔ kernel value marshaling bridge + null/undefined rule

- **Decision**: Marshal the (Zod-validated, plain) JS object across napi into a `serde_json::Value` via
  napi's serde support, then into the kernel value type the same way the consumer builds it
  (`minijinja::Value::from_serialize`) — so render byte-parity stays structural (the same primitive the
  Rust consumer + 004 binding use). Concentrate all translation in `marshal.rs`.
- **null/undefined/absent (clarified Q6)**: `undefined` and an absent object field → **field not
  present** (no kernel value for that root; strict-undefined fires if referenced). Explicit `null` →
  JSON `null`. This matches the Python binding's `None`/absent handling and the kernel's serde model, so
  the two bindings agree for the spec-006 corpus. `number` stays f64/i64 per JS; `bigint` → confirm
  napi's bigint→serde path preserves it losslessly (test pins it).
- **Rationale**: identical to 004's `pythonize → from_serialize` two-hop; keeps the binding a pure value
  bridge, no engine logic.
- **Alternatives**: hand-walking the JS value into `minijinja::Value` — rejected (re-implements what
  serde already does; risks divergence from the consumer's path).

## D3 — Zod v4 → `{field, code, message}` mapper

- **Decision**: Validate at the render boundary with `schema.safeParse(data)`; on `!success`, map
  `result.error.issues[]` → rows: `field` = `issue.path.join(".")`, `code` = the shared `"validation"`,
  `message` = `issue.message` **only**. Never read the rejected input value (`issue.input`/received). This
  is the SEC-004-PY-equivalent scrub on the Zod side.
- **Rationale**: Zod v4's `ZodError.issues` is the stable shape; `path` + `message` give the
  field+message rows without leaking the value. v4-only (not a v3/v4 dual range) per Q7 → one issue shape.
- **Alternatives**: `.parse()` + try/catch — equivalent, but `safeParse` avoids throw-as-control-flow.

## D4 — JS `Error`-subclass hierarchy + how napi surfaces errors

- **Decision**: Define the hierarchy in **TS** (`src/index.ts`): `class PromptingPressError extends Error`
  with `readonly errors: {field,code,message}[]`, and subclasses `PromptValidationError`,
  `PromptRenderError`, `UnknownPromptError`, `LoadError`. The napi addon returns/throws a structured
  payload (a `napi::Error` carrying the scrubbed rows as JSON, OR a normal return the TS wrapper inspects);
  the **TS wrapper** is what constructs and throws the right subclass. The kernel `KernelError` is routed
  through the consumer's `From<KernelError> for ConsumerError` scrubber in Rust **first** (fixed messages
  for parse/render/excluded_feature — SEC-004), so the rows that cross napi are already scrubbed.
- **Rationale**: JS `Error` subclasses can't be minted from Rust cleanly; the TS facade is the natural
  home (it's also where Zod lives). Mirrors the 004 split where the exception hierarchy is binding-side.
  The closed `code` vocabulary (`validation`/`unknown_prompt`/`unknown_variant`/`undefined_variable`/
  `parse`/`render`/`excluded_feature`/`load`) is shared with Rust + Python.
- **Alternatives**: minting JS error classes from napi via `#[napi]` — rejected (awkward, and the rows +
  `instanceof` semantics are cleaner TS-side). One concrete sub-item for research at impl time: how napi
  best returns a structured error payload (a custom `Status`/`reason` string vs a JSON `napi::Error::new`
  message the TS side parses) — pick the one that keeps rows structured, not string-encoded if avoidable.

## D5 — ESM-only packaging + per-platform native binaries

- **Decision**: Ship **ESM-only** (clarified Q8; scaffold already `"type": "module"` + `--esm` build).
  Per-platform `.node` binaries published as `optionalDependencies` via `napi prepublish -t npm` (already
  scaffolded). Platform-triple matrix finalized at impl time from the existing 3-OS CI matrix
  (linux-x64-gnu, darwin-arm64/x64, win32-x64-msvc at minimum).
- **Rationale**: matches the scaffold + the napi-rs standard; no CJS entry point to maintain.
- **Alternatives**: dual ESM+CJS — rejected (Q8: more packaging + test surface, no consumer need).

## D6 — TS test runner

- **Decision**: Use **Vitest** (or node:test if a zero-dep runner is preferred — decided at impl time;
  lean Vitest for ergonomics + watch). Tests import the **built** addon (the `napi build` output in
  `packages/typescript/`) — i.e. tests run after a build step, like 004's pytest-against-the-wheel.
- **Rationale**: the binding's TS-observable behavior (Zod validation, error subclasses, marshaling) is
  only reachable through the built addon; a runner that can import the native `.node` is required.
- **Alternatives**: node:test (no dep) — viable; Jest — heavier, ESM friction. Final pick in tasks.

## D7 — CI gates

- **Decision**: Three CI additions:
  1. **Verify `ci:check-ffi`** (`scripts/ci/check-ffi-isolation.sh`) asserts `napi` absent from
     `prompting-press-core` + `prompting-press`. CORRECTION (analyze F1): the gate ALREADY does — it ships
     `FFI_CRATES=("pyo3" "napi")` from spec 001. So this is a verification, not an extension. FR-022.
  2. **`ci:check-advisories-node`** — an npm advisory gate (e.g. `pnpm audit --audit-level=…` or
     `osv-scanner` over the pnpm lockfile), mirroring `ci:check-advisories` (Rust) + `ci:check-advisories-py`
     (Python). FR-025.
  3. **`ci:test-node`** — build the addon + run the TS tests + `cargo test -p prompting-press-node`,
     mirroring the spec-004 `ci:test-python` gate (the I1 lesson: the OS-matrix `cargo build` doesn't run
     binding tests). Verify the napi build + addon-load works on the **Linux** runner (the 004
     maturin/libpython link lesson — a binding test that builds fine can still fail to load at runtime).
- **Rationale**: parity with the Rust + Python gate set; closes the same coverage gaps 004 closed.
- **Alternatives**: relying on the build matrix alone — rejected (it compiles, never runs the tests).

## Open at impl time (not blocking the plan)

- Exact napi error-return mechanism (D4 sub-item) and bigint serde fidelity (D2) — pin with tests.
- Vitest vs node:test (D6) — pick in tasks.
- Platform-triple matrix (D5) — enumerate in tasks from the CI matrix.
