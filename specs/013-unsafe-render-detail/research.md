# Phase 0 Research: Opt-in unsafe render-error detail

All Technical-Context unknowns resolved. No new dependencies.

## R1 — The consumer seam (how to thread a per-call flag through `From<KernelError>`)

- **Decision**: Keep the existing `impl From<KernelError> for ConsumerError` exactly as-is (it scrubs
  Render detail) — it remains the default used by every non-render call site (e.g. `get_source`, the
  bindings' generic error mapping). Add an explicit constructor
  `ConsumerError::from_kernel_revealing(err, reveal_render_detail: bool)` (final name TBD in impl) that
  the `Prompt::render` path calls with the caller's per-render flag. When `reveal_render_detail` is true
  AND the error is `KernelError::Render`, surface the real detail in `message`; in every other case
  (false, or any non-Render kind), behave identically to the scrubbing `From`.
- **Rationale**: a `From` impl can't take a parameter; a parallel explicit constructor is the minimal,
  non-global way to make the choice per-call. The default `From` stays the safe path, so any code that
  forgets the new constructor keeps scrubbing (fail-safe).
- **Alternatives rejected**: (a) a field on `ConsumerError` toggling display — leaks the choice into the
  type; (b) a thread-local/global — violates FR-003 (no ambient enable); (c) scrub in the kernel and
  un-scrub later — impossible, kernel already hands the detail to the consumer.

## R2 — Per-binding option shape (per-call, off-by-default)

- **Rust** (`crates/prompting-press/src/prompt.rs`): `render` currently is
  `render<V>(&self, vars: &V, variant: Option<&str>, guard: &GuardConfig)`. Add the flag. Two options:
  (a) one more positional `bool`; (b) fold render options into a small `RenderOptions`-style struct for
  C-11 consistency with TS. **Decision: a dedicated options struct OR a single additional typed argument**
  — resolve the exact ergonomics in implementation, but it MUST default to "scrub" when unset and read as
  risky (R3). (Leaning: a `RenderOptions { variant, guard, reveal_render_detail }`-style struct to match
  TS and avoid a bare positional `bool`, but keeping the current positional form + an options overload is
  acceptable; not a spec-level decision.)
- **Python** (`-py`): `render(... , *, reveal_render_detail: bool = False)` — a keyword-only arg in the
  existing keyword-only tail (alongside `variant=`, `guard=`), defaulting False.
- **TypeScript** (`-node` + facade): add `revealRenderDetail?: boolean` to the existing `RenderOptions`
  object (alongside `variant`, `guard`); absent/false ⇒ scrub.
- All three thread the flag down to the consumer's `from_kernel_revealing` on the render path.

## R3 — Naming + risk signaling (FR-012)

- **Decision**: the option name MUST signal danger. Candidate names: `reveal_render_detail` /
  `unsafe_render_detail` / `unredacted_errors`. **Lean: `unsafe_`-prefixed** (Rust idiom for "you accept
  the risk", e.g. `unsafe_reveal_render_detail`) or at minimum a name containing "unsafe"/"unredacted",
  plus a doc-comment at the call site warning that enabling it may place bound-value content (untrusted
  input / PII / secrets) into the returned error message, stack, and any log derived from it. Final token
  chosen in implementation; the requirement (FR-012) is that it cannot read as innocuous.

## R4 — Scope is render-detail-only (FR-010)

- The new path changes behavior ONLY for `KernelError::Render`. `Parse` (already preserved, D2),
  `ExcludedFeature` (templated, low-risk), `UnknownVariant`, `UndefinedVariable`, and `Validation` are
  byte-identical whether the flag is on or off. Verified against the current `From<KernelError>` arms in
  error.rs.

## R5 — Default-safety verification (FR-002/003 / SC-002/003)

- **Decision**: the existing scrub test corpus (`fuzz_scrub.rs`, `render_detail_secret_is_scrubbed`,
  the py/node fuzz-scrub suites) MUST pass unchanged — they exercise the default (no flag). Add an
  inspection-level assertion/test that there is no global/env/default-true path (grep + a unit test that a
  default-constructed render scrubs). This is the load-bearing safety check.

## R6 — Governance: D3 + SEC-004 carve-out note

- **Decision**: author a decision record `docs/memory/decisions/<date>-unsafe-render-detail-optin.md` (D3)
  + a one-line carve-out note on SEC-004 wherever it is stated, exactly as D2 did for the Parse refinement.
  NOT a constitution amendment (default unchanged; not a pluggable interface). Recorded as the standard.

## R7 — Hashes / success path unaffected (FR-004 / SC-005)

- The option only affects the error path. On a successful render, `text`, `template_hash`, `render_hash`,
  `variant`, and `guard` are byte-identical with the flag on or off. No assertion needed beyond a test
  that toggling the flag changes nothing on a render that succeeds.
