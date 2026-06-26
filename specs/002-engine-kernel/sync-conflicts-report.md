# Inter-Spec Conflict Report — Spec 002 (Engine kernel)

**Date**: 2026-06-26 · Step 16 (cross-spec contradictions). Run on the main thread (the
`speckit-sync-conflicts` subagent hit the tool-channel glitch).

## Verdict: ✅ No inter-spec conflicts. Two judgment calls assessed; both clean. One cosmetic note for spec 001.

## Assessed

### J1 — 002 relocated the generated shape (consumer → kernel) vs spec 001 — NOT a conflict
Spec-001 **FR-016**: "Generated artifacts MUST be committed … marked as generated, and live at
**predictable per-language locations segregated from hand-written code**." It does NOT pin the Rust
shape to the *consumer* crate — only to a predictable, segregated location. 002 moved it to
`crates/prompting-press-core/src/generated/` (still predictable, segregated, per-language). Spec-001's
load-bearing invariants are **preserved**: the C-01/C-02 dependency direction (kernel never depends on
consumer/binding — 001 FR-002/FR-018) still holds (the kernel now *owns* the shape; the consumer
re-exports it), and the codegen-freshness gate (001 FR-019) was repointed and is green. So this is
compatible forward-evolution, not a contradiction.
- **Cosmetic note (no action required):** spec-001's prose *examples* describe the Rust shape as living
  in the "consumer crate." 001 is merged; its normative FRs aren't violated, so this is at most a dated
  example, not a live conflict. If 001 is ever revisited, a one-line note could record the 002 move.

### J2 — 002's T036 cargo-deny advisory gate vs spec 007's release-tooling scope (C-10) — NOT an over-reach
Roadmap **C-10** scopes **release tooling** to spec 007: release-please, the linked version axes, and
wheel/npm/crate *publishing*. A `cargo-deny` **advisory/CI gate** is a *security CI guardrail* (it scans
the dep tree for known vulnerabilities), in the same class as spec-001's `check-ffi` and
`check-floating-versions` CI gates — not release/publish tooling. It originated from security finding
SEC-001 (a missing recurring advisory scan), not a versioning/release concern. So T036 sits in the
general-CI lane that 001 established, not in 007's release-tooling lane. **Not a scope violation.**
- **Forward note for spec 007:** the advisory gate (`ci:check-advisories`, `deny.toml`,
  `scripts/ci/check-advisories.sh`) already exists; 007 should be aware it's present and that its owner
  also carries the roadmap-Q3 "re-confirm MiniJinja stable-API on each bump" obligation (bound to the
  gate per T036).

### J3 — 002's future-spec deferrals vs roadmap intent — consistent
002 correctly defers to the roadmap's future-spec assignments: error normalization → 003 (the kernel
returns native `KernelError`, C-06), dual-input loader + garde validation → 003, bindings → 004/005,
conformance corpus → 007. The kernel's API bakes in no assumption that contradicts those: it is
validation-blind (so 003's garde layer sits cleanly on top), takes `minijinja::Value` (which the
bindings/corpus will marshal), and consumes the C-07 schema shape exactly as 001 defined it. No
forward contradiction.

## Conclusion

No conflicts requiring resolution. Two cosmetic forward-notes recorded (001 example wording; 007
advisory-gate awareness) — neither blocks anything.
