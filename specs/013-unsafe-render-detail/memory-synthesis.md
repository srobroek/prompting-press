# Memory Synthesis

_(Direct-read fallback: `speckit_memory_*` MCP tools + `.spec-kit-memory/` SQLite cache absent this session.
Sources read: `.specify/memory/constitution.md`, `docs/memory/decisions/2026-06-29-parse-detail-preserved-render-scrubbed.md` (D2),
`crates/prompting-press/src/{error.rs, prompt.rs}`, the binding render signatures, the spec + its Clarifications. Token banner N/A — no cache.)_

## Current Scope

Spec 013 — an explicit, **off-by-default, per-render-call** opt-in that surfaces the otherwise-scrubbed
`Render`-error detail in the normalized error message. Inverse tradeoff from the SEC-004 default
(debuggability over automatic scrubbing). Affected: `crates/prompting-press/src/error.rs` (the
`From<KernelError>` Render arm), `crates/prompting-press/src/prompt.rs` (`render` gains the option),
and the three binding render surfaces (`-py`, `-node`, TS facade). Kernel UNCHANGED (it already produces
the detail; this is purely the consumer/binding scrub choice). No I/O, no logging.

## Relevant Decisions

- **D2 — Parse preserved / Render scrubbed** (Status: active; Source: docs/memory/decisions/2026-06-29-…).
  013 is the explicit follow-up D2 flagged: Parse detail is already preserved (pre-binding, no values);
  013 adds the opt-in for the Render half. 013 must NOT change Parse behavior.
- **Clarify D3 (this spec, Session 2026-06-29)**: per-render-call opt-in (render-options flag, NOT
  per-Prompt, NOT global); render-detail-only (ExcludedFeature/Parse untouched); governance = a recorded
  decision **D3** + a SEC-004 carve-out note, NOT a full constitution amendment; the boolean flag is not a
  pluggable interface. The off-by-default guarantee is the fixed safeguard.
- **C-11 — options-object call shape** (Source: roadmap/DECISIONS). The opt-in rides the existing
  per-render options tail (TS `RenderOptions {variant?, guard?}`; Python keyword-only `variant=/guard=`;
  Rust currently positional `render<V>(&vars, variant, guard)`). 013 adds one more option in that same slot.

## Active Architecture Constraints

- **SEC-004 scrub (the load-bearing constraint)** (Source: spec 001/004 + error.rs). Default MUST stay
  scrubbed everywhere; the existing `fuzz_scrub` corpus + `render_detail_secret_is_scrubbed` tests MUST
  stay green with no opt-in set. 013 only changes behavior when the flag is explicitly true.
- **Principle I — kernel unchanged** (Source: constitution). The kernel `KernelError::Render { detail }`
  already carries the full detail; the scrub happens in the consumer `From<KernelError> for ConsumerError`
  (error.rs ~line 190, the `Render` arm emits a fixed "render error" message, discarding detail). 013
  conditionally surfaces that detail at the consumer layer — no core change.
- **Principle VI — normalized error shape** (Source: constitution + error.rs). The surfaced detail must
  come through `{field, code, message}`; native error types MUST NOT leak across FFI. Only `message`
  content changes (full detail vs fixed) — the shape/code (`render`) are unchanged.
- **Principle III — no I/O / no logging** (Source: constitution). "Surfacing" = placing detail on the
  returned error value; the library still never logs. What the caller does with it is the caller's job.
- **The bindings delegate to the consumer scrubber** (Source: -py/-node error.rs route through
  `ConsumerError::from(kernel)` first). So the opt-in must be threaded as a parameter INTO that conversion
  (or the render call must choose the message before normalization) — the bindings should inherit the
  behavior, not re-implement the scrub decision. Design the consumer seam so all three bindings get it for free.

## Accepted Deviations

- **D3 sanctioned SEC-004 carve-out**: an off-by-default, explicit, per-call opt-out of the Render scrub.
  Recorded as the standard (a decision + carve-out note), not a deviation from the principle — the default
  guarantee is intact. This is the one place the otherwise-absolute scrub is intentionally relaxable.

## Relevant Security Constraints

- **Off-by-default is non-negotiable (FR-002/003/SC-002/003)**: no default-on value, no env var, no global.
  Verifiable by inspection + a test that the existing scrub corpus passes with zero opt-in.
- **The opt-in must read as risky at the call site (FR-012)**: an `unsafe`/`unredacted`-style name +
  doc-comment warning, so a reader cannot enable it without meeting the bound-value-exposure caveat.

## Related Historical Lessons

- **Subagent verification glitches** (this session, repeatedly — incl. the 011 Phase-2 workflow returning
  placeholder verdicts): verify load-bearing claims main-thread; do not trust a subagent's pass/fail without
  re-checking against the code. Applies to 013's "default still scrubs" assertion — prove it with the corpus.
- **Tests flip in lockstep with the behavior** (D2 lesson): when 013 lands, add tests for BOTH states
  (opt-in surfaces detail; default still scrubs) and keep the existing scrub corpus green.
- **Commit via `git -c commit.gpgsign=false`/local toggle** (precommit-gate false-flags signing as --no-verify).

## Conflict Warnings

- **No HARD conflicts.** 013 is consistent with D2 (Parse vs Render split) and the constitution (off-by-default
  preserves SEC-004's guarantee; recorded-decision governance, not amendment).
- **Soft watch — the consumer seam**: `From<KernelError> for ConsumerError` is a plain `From` impl with no
  parameter. Threading a per-call boolean into it cleanly is the key design question for the plan (e.g. a
  separate `ConsumerError::from_kernel(err, reveal_render_detail: bool)` constructor used by `render`, with
  the `From` impl keeping the scrubbed default for all other call sites). Resolve in plan/contracts; don't
  let it leak into a global.
- **Soft watch — Rust render signature**: currently positional `render<V>(&vars, variant, guard)`. Adding a
  per-call flag may argue for a render-options struct in Rust too (consistency with TS RenderOptions), or a
  new positional/builder arg — decide in plan (C-11 favors an options object, but Rust has used positional).

## Retrieval Notes

- Index entries considered: docs/memory/INDEX.md (A1 loader-layers, D1 marshaling, D2 scrub). D2 is the
  direct parent — included. A1/D1 not relevant. Governance read directly (Principle I/III/VI + SEC-004 +
  Scope Discipline). Budget: under limits (3 decisions, 5 constraints, 1 deviation, 2 security, 3 lessons).
  Full-memory-read not required.
