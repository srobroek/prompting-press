---
description: "Task list for spec 013 — opt-in unsafe render-error detail"
---

# Tasks: Opt-in unsafe render-error detail

**Input**: Design documents from `specs/013-unsafe-render-detail/`

**Prerequisites**: spec.md, plan.md, research.md, data-model.md, contracts/unsafe-detail-option.md, quickstart.md, memory-synthesis.md

**Organization**: Consumer seam first (the load-bearing change), then the three bindings thread the per-call flag to it, then tests (default-scrub must stay green + opt-in surfaces), then docs + the D3 decision record.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: parallelizable (different files, no incomplete-dep)
- **[Story]**: US1 (operator gets full detail on opt-in), US2 (default stays safe / never implicit), US3 (cross-language consistency)

## Implementation-open items (settle during execution)

- **IO-1 — option name (FR-012)**: pick the risk-signaling name once, use it across all bindings. Lean `unsafe`-prefixed (e.g. Rust `reveal_render_detail` on an options struct named to read as unsafe, Python `unsafe_reveal_render_detail`, TS `unsafeRevealRenderDetail`). Final tokens chosen in T002; MUST read as deliberate/risky, never innocuous.
- **IO-2 — Rust render signature**: render is currently positional `render<V>(&vars, variant, guard)`. Decide: add a 4th explicit arg vs. introduce a small `RenderOptions`-style struct (C-11 consistency with TS). Either is fine if it defaults to scrub and reads as risky. Resolve in T003.

---

## Phase 1: Foundational — the consumer seam (Blocking)

**Purpose**: the single decision point. Everything else threads a flag into this.

- [ ] T001 In `crates/prompting-press/src/error.rs`: KEEP `impl From<KernelError> for ConsumerError` unchanged (the scrubbing default, used by every non-render path). ADD an explicit constructor `ConsumerError::from_kernel_revealing(err: KernelError, reveal_render_detail: bool) -> Self` (final name your call): when `reveal_render_detail == true` AND the error is `KernelError::Render { detail }`, surface the real `detail` in `message`; in EVERY other case (false, or any non-Render kind — Parse/ExcludedFeature/UnknownVariant/UndefinedVariable/Validation) behave byte-for-byte identically to the `From` impl. Document it with a risk warning. **GATE**: `cargo test -p prompting-press --lib error` — add unit tests proving reveal=false ≡ the `From` impl for all arms, and reveal=true changes ONLY the Render message.

---

## Phase 2 — User Story 1 (P1): operator opts in to full render detail

**Goal**: enabling the per-call flag surfaces the real render detail; default scrubs.

**Independent test**: trigger a render error with the flag on (full detail in message) and off (scrubbed) — they differ only in render-detail exposure.

- [ ] T002 [US1] (resolve IO-1) Rust `Prompt::render` in `crates/prompting-press/src/prompt.rs`: add the per-call opt-in (IO-2 shape) defaulting to scrub; on a render error, call `from_kernel_revealing(err, <flag>)` instead of the plain `From`. The flag name reads as risky (FR-012) with a doc-comment warning that enabling it may place bound-value content in the error. **GATE**: `cargo test -p prompting-press` — a Rust test: flag on ⇒ Render error message contains the seeded detail; flag off ⇒ scrubbed.
- [ ] T003 [US1] Confirm scope: the flag affects ONLY `KernelError::Render` (FR-010). Parse stays preserved (D2), ExcludedFeature stays templated, success path byte-identical (text/template_hash/render_hash unchanged with flag on vs off). **GATE**: a test toggling the flag across each error kind asserts only Render differs, and a successful render is byte-identical either way (SC-005).

---

## Phase 3 — User Story 3 (P2): cross-binding consistency

**Goal**: the same per-call opt-in, same semantics, via the normalized `{field,code,message}` shape in all three bindings.

**Independent test**: enable the flag in each binding, trigger a render error, confirm the full detail arrives via the binding's normalized error (no native error leak).

- [ ] T004 [P] [US3] Python binding `crates/prompting-press-py/src/prompt.rs` (+ render.rs): add keyword-only `render(..., *, <unsafe_flag>=False)`; thread it to the consumer `from_kernel_revealing`. Native Pydantic/Rust errors MUST NOT leak. **GATE**: `cargo test -p prompting-press-py` + a pytest: flag on ⇒ `PromptRenderError.errors[0].message` carries detail; off ⇒ scrubbed; default (omitted) ⇒ scrubbed.
- [ ] T005 [P] [US3] TS binding `crates/prompting-press-node/src/{prompt.rs,render.rs}` + facade `packages/typescript/src/index.ts`: add `<unsafeFlag>?: boolean` to `RenderOptions`; thread it down to the consumer seam. **GATE**: `cargo test -p prompting-press-node` + node:test: flag on ⇒ `PromptRenderError.errors[0].message` carries detail; off/absent ⇒ scrubbed.
- [ ] T006 [US3] Cross-binding parity check: the same render-error scenario yields equivalent detail exposure via `{field,code,message}` in Rust/Python/TS (SC-004). **GATE**: the three binding tests assert the same shape + behavior.

---

## Phase 4 — User Story 2 (P1): default stays safe / never implicit

**Goal**: doing nothing always scrubs; the opt-in cannot be turned on ambiently.

**Independent test**: with no flag anywhere, the existing scrub corpus passes; no global/env/default-true path exists.

- [ ] T007 [US2] Regression: the EXISTING scrub corpus passes UNCHANGED — `fuzz_scrub.rs`, `render_detail_secret_is_scrubbed` (consumer), and the py/node fuzz-scrub suites. These exercise the default (no flag). **GATE**: `cargo test --workspace` + `ci:test-python` + `ci:test-node` all green; zero edits to the existing scrub assertions.
- [ ] T008 [US2] No-implicit-enable proof (FR-003/SC-003): inspection + a test that a default render() (no flag) scrubs, and a grep confirming there is no env var / global / default-true toggle. **GATE**: grep clean; default-scrub test passes.

---

## Phase 5: Docs + Governance

- [ ] T009 [P] Docs: update the error-reference pages (rust/python/typescript) + the relevant guide to document the opt-in + a risk warning that enabling it may surface bound-value content (FR-008). Since the reference pages are now GENERATED (spec 011), the doc text lives in the SOURCE doc comments (the option's doc-comment from T002/T004/T005) — regenerate the reference pages; add prose to a guide if needed. **GATE**: the option appears in the generated reference with its risk warning.
- [ ] T010 Author the **D3 decision record** `docs/memory/decisions/<date>-unsafe-render-detail-optin.md` + add an explicit SEC-004 carve-out note (the same path D2 took): records the sanctioned, off-by-default, per-call render-detail opt-in; states it is NOT a constitution amendment (default unchanged; not a pluggable interface); add the INDEX.md pointer (D3). **GATE**: D3 file + INDEX pointer present.

---

## Phase 6: Verification

- [ ] T011 Run the full quickstart.md locally: default scrubs (corpus green), opt-in surfaces (all 3 bindings), no implicit enable, render-detail-only, success path byte-identical, D3 present. **GATE**: every quickstart check passes.

---

## Dependencies & order

- **T001 (consumer seam) blocks all binding work** (T002/T004/T005 thread into it).
- T002 (Rust render) before T003 (its scope test).
- T004 + T005 are parallel [P] once T001 exists (different binding crates).
- T007/T008 (default-safety) can run anytime but are the gate before merge.
- T009/T010 docs+governance after the behavior lands.

## MVP scope

**T001 + T002 (consumer seam + Rust opt-in)** = the MVP proving the behavior. T004/T005 extend to the other bindings; T007/T008 lock the safety guarantee.

## Note

This touches the SEC-004 Render-scrub half — governed by the **recorded decision D3 + carve-out note** (T010), per the spec's clarification, NOT a constitution amendment. The off-by-default guarantee (T007/T008) is the load-bearing safeguard and is non-negotiable.
