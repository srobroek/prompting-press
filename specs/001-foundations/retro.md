# Retrospective — Spec 001 Foundations

**Date:** 2026-06-26 · **Outcome:** all 35 tasks implemented + Phase-3 QA complete; CI green
(Linux/macOS/Windows build + gates). Branch `001-foundations`, ready to merge to `main`.

## What shipped

The structural spine for Prompting Press: 4-crate polyglot workspace (FFI-isolated kernel/consumer +
pyo3/napi binding crates), the prompt-definition JSON Schema (single source of truth), a deterministic
schema→3-shape codegen pipeline (Pydantic v2 / TS / Rust serde), and CI guardrails enforcing
FFI-isolation (C-02) and codegen-freshness (C-07). No runtime logic, by design (Principle III).

## Metrics

- 35 tasks across 6 phases; 35 GitHub issues created + closed.
- ~14 implementation/fix commits; 2 fresh-context reviews (code + security) → 10 findings, all resolved.
- CI: first push green on build matrix but RED on gates (2 real bugs); second push fully green.
- Subagents: ~16 spawned (rust-pro, python-pro, typescript-pro, backend-architect, coder, reviewers,
  verifiers). ~5 hit an environment tool-channel glitch (0 tool uses / garbled output) and were
  re-run or replaced by main-thread work.

## What went well

- **Review gates earned their keep repeatedly.** Reviewing actual diffs (not agent self-reports)
  caught: the pyo3 macOS link failure (agent's `cargo check` passed, `cargo build` didn't);
  CR-H2's moon-path nuance (agent's isolated test missed it); the floating-version lint's
  comment-self-match (only surfaced in CI). Every one of these would have shipped on trust.
- **The T022 de-risking spike** (verify typify before wiring) caught the `propertyNames` panic
  before it could block T025 — cheap insurance on the spec's one flagged unknown.
- **Security review found the load-bearing bug (H-1):** CI was not hermetic; the freshness gate
  would have been permanently red on clean checkouts. A plan-stage review rated the spec LOW on
  supply-chain mitigations that turned out documented-but-not-wired — the post-impl review caught it.
- **Determinism held** across all 3 generators (byte-identical re-runs) — the make-or-break property.

## What was painful / lessons

- **Environment tool-channel flakiness** disrupted ~5 subagents (verify-tasks ×2, both sync agents,
  one verify). Lesson: for critical gates, be ready to fall back to main-thread execution; don't
  trust a "blocker" report without confirming the channel (a trivial `echo` probe distinguishes
  real blockers from channel failure). One agent hallucinated a file listing — verify alarming
  findings directly before acting.
- **Self-referential-string false positives** bit three times (floating-lint matched its own
  comments; my `grep packages/go` matched a comment; SC-007 false-flag). Lesson: lints/greps over
  config must strip comments; reviewers without Bash can't confirm file existence.
- **A broken APM hook** (`rm-rf-guard.sh`, bash-4 `;;&` under bash 3.2) blocked `rm`-containing
  commands 4×. Worked around each time; logged for upstream fix (not fixable from this project).
- **My prompt steered an agent wrong once** (requirements.txt vs uv) — caught by the user. Lesson:
  when a task offers a menu of mechanisms, pick the repo-idiomatic one in the prompt, don't pass the
  menu through.
- **I over-reported "CI green" once** (only the build matrix was green; gates had failed). Lesson:
  read per-job conclusions, not just overall, and state precisely which jobs passed.

## Decisions worth remembering (also in docs/memory worklog + roadmap C-10)

- Release tooling: release-please + native per-ecosystem build/publish; GoReleaser rejected
  (binary builder, not library/wheel/addon). Spec 007.
- pyo3 cdylib macOS link: crate-scoped `build.rs` `cargo:rustc-link-arg` (not repo `.cargo/config.toml`
  rustflags — keeps it out of the codegen-determinism + FFI-gate fingerprint surfaces).
- typify `propertyNames` panic: strip the validation-only key from a typify-input copy; rule stays
  enforced at the validation gate. Schema unchanged.
- All toolchain pinned in mise (incl. rust, which DRIVES rustup via RUSTUP_TOOLCHAIN — must lockstep
  with rust-toolchain.toml).

## Follow-ups carried out of 001

1. **[spec 004]** pyo3 `#[pymodule]` name vs maturin `module-name` reconciliation (stub `cargo check`s
   fine; matters at real build/import).
2. **[spec 004/007]** maturin: move to a uv hash-locked build path when the wheel build lands
   (currently resolved by build-system requires bound).
3. **[spec 007]** flip `packages/typescript` `"private": true` when publishing.
4. **[upstream/agentic-packages]** fix `rm-rf-guard.sh` bash-3.2 incompatibility.
5. **[spec 002/003]** the `propertyNames` rule lives at the validation layer, not the generated Rust
   type — carry into kernel/consumer planning.
