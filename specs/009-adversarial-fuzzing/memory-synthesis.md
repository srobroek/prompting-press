# Memory Synthesis — 009 Adversarial hardening & fuzzing

_(Direct-read fallback: no SQLite/MCP this session. Branched on top of 008 — targets the NEW Prompt surface.)_

## Current Scope

A library-hardening TEST pass (no behavior change): fuzz + hostile-input + scrub-verification coverage in the
library's OWN suites (kernel + all 3 bindings), proving the boundary holds under abuse. Four parts: (a)
robustness fuzzing (malformed/huge/deeply-nested/Unicode/control-char → no panic, structured error, no leak);
(b) property-based fuzzing (Rust `proptest`, Python `hypothesis`, TS `fast-check` — invariants: never-panic,
validate-before-render, hash-determinism); (c) injection/guard demo (untrusted input → `check()` flags
unguarded field + guard text names it, with EXPLICIT "advisory text NOT enforcement" framing); (d) secret-scrub
verification (secret-looking values triggering parse/render errors never appear in message/stack — SEC-004 end
to end).

## Active Constraints (governance)

- **C-03 / Principle III** — fuzzing PROVES the minimal boundary, never expands it. No I/O, no LLM, no
  request-body assembly may be added. The "break the model" framing is HONEST: library robustness, NOT LLM
  jailbreak (the library has no model).
- **C-09 / Principle IV** — origin tag is metadata + the guard is ADVISORY, never silent mutation. The
  injection demo must state this honestly (the guard names fields; it does not sanitize).
- **No floating versions** — new dev-deps (proptest/hypothesis/fast-check) pinned EXACT (verify latest
  main-thread; do NOT trust a research subagent — see the 008 fabrication incident).
- Kernel unchanged (Principle I) — this is tests only.

## Reused lessons (008 + project memory)

- **The surface under test is the NEW Prompt object** (008): `Prompt::new`/`from_yaml`/`from_json`/`from_toml`,
  `render`, `check` (advisory), `with`. NOT the dropped Registry. Construction now enforces parse + agreement,
  so fuzzing construction is a key target (does a hostile body panic the constructor? → must be structured Err).
- **SEC-004 scrub** (spec 001/004/005): the existing scrub routes KernelError through the consumer From
  scrubber; Pydantic mapper copies msg/loc only; Zod mapper copies message+path only. 009 (d) ADVERSARIALLY
  verifies this end-to-end with secret-looking inputs.
- **D1 (canonical serialized form)** — if any fuzz fixture uses date/decimal, pin by serialized string.
- **Test-channel**: run gates main-thread or via coder agents with the integrity-check preamble; the read-only
  research-agent fabrication risk is real (008 [A8-6]).

## Conflict Warnings

- No hard conflicts. 009 is additive test coverage over an unchanged boundary. The ONLY watch: a fuzz test that
  tries to assert the guard SANITIZES would be wrong (C-09 — it's advisory); the injection demo must assert the
  untrusted value passes through UNCHANGED + the guard merely names it.

## Retrieval Notes

Governance read direct (constitution v1.2.0, roadmap 009 entry, DECISIONS). Project memory: SEC-004 +
fabrication-risk + D1. Budget well under limits. Branched stacked on 008 (009 needs the Prompt surface).
