# Memory Synthesis

## Current Scope

Spec 006 — **Conformance corpus + cross-language hardening**. Implements constitution **Principle VII** as
a CI gate proving two per-binding properties the shared core cannot self-verify: (1) **marshaling parity**
— same logical input → identical rendered text + identical `template_hash`/`render_hash` across the Rust
consumer, Python binding, and TypeScript binding, over dates / Decimal / nested models /
null-undefined-None / int-vs-float; (2) **schema round-trip parity** — same accept/reject of prompt
documents across all three. Affected modules: a new shared `conformance/` fixture set + one thin runner
per binding (`prompting-press`, `prompting-press-py`/`packages/python`, `prompting-press-node`/
`packages/typescript`), wired as a moon-task CI gate. Adds **tests + a gate, no library capability**.

Clarified (Session 2026-06-28): shared corpus + 3 runners; cross-binding cross-check + small committed
golden tripwire; canonical-serialized-form for types lacking a universal native equivalent.

## Relevant Decisions

- **C-01 / Principle I — render parity is STRUCTURAL, not tested** (Reason: defines the hard scope
  boundary — corpus tests marshaling + schema acceptance, NEVER re-tests the renderer; Status: active;
  Source: constitution + roadmap).
- **C-07 / Principle VII — JSON Schema is the single source of truth** (Reason: schema round-trip parity
  is the corpus's 2nd guarantee; reuse `schemas/jsonschema/fixtures/`; Status: active; Source:
  constitution + roadmap §278).
- **C-02 / Principle II — zero engine logic in bindings** (Reason: corpus must drive each binding's real
  marshaling path, add no engine logic; `ci:check-ffi` stays green; Status: active).
- **C-05 / Principle V — provenance hashes** (`template_hash`=SHA256(variant source),
  `render_hash`=SHA256(output), per variant, no `vars_hash`) (Reason: the values the corpus pins; parity
  already proven empirically TS==Python; Status: active).
- **null/undefined/None contract is FIXED** (specs 004/005 FR-003a): explicit `null`/`None` → JSON
  `null`; `undefined`/absent → field-not-present → kernel strict-undefined (Reason: corpus pins it as a
  cross-binding equality, does NOT redesign it; Status: active, as-built).

## Active Architecture Constraints

- **Crate/package layout is load-bearing** (Reason: corpus runners attach to existing surfaces —
  `prompting-press` consumer, `prompting-press-py`, `prompting-press-node`; Source: constitution Dev
  Workflow).
- **CI gate pattern**: gate logic in moon tasks (`ci/moon.yml`) + `scripts/ci/*.sh`, locally runnable via
  `mise exec -- moon run <task>`, called from `.github/workflows/ci.yml` (jobs: gates, test-python,
  test-node, build matrix) (Reason: the conformance gate MUST follow this; Source: spec-001/004/005 CI).
- **Existing fixtures to reuse, not fork**: `schemas/jsonschema/fixtures/{valid,invalid}/` (3+7);
  `crates/prompting-press-core/tests/fixtures/render/` is the spec-002 engine-regression set — leave
  byte-unchanged (Reason: FR-003 builds on schema fixtures; FR-016 forbids touching render set).
- **No floating versions** (`ci:check-floating-versions` scans whole manifests) (Reason: any test-harness
  dep must be pinned exact; prefer adding none — no 2nd YAML parser, no JS decimal lib in shipped lib).

## Accepted Deviations

- None applicable. (No `accepted-deviations` memory exists; `docs/memory/{decisions,architecture,bugs}/`
  are empty — load-bearing decisions live in the governance layer per INDEX.md.)

## Relevant Security Constraints

- **SEC-004 scrub posture** (Reason: corpus failure output MUST NOT leak raw bound-value content beyond
  what the fixture file already contains; fixtures hold only non-secret test data by construction;
  Source: specs 004/005 as-built).
- **Boundary defense (Principle III)** (Reason: corpus adds no I/O to the library, no LLM/token surface;
  runners may read fixture files because they are test harnesses, not the library).

## Related Historical Lessons

- **Subagent fabrication risk** (Reason: speckit-workflow-gotchas — audit/research subagents have returned
  `tool_uses:0` fabricated results; bake an integrity-check preamble and re-verify load-bearing findings
  main-thread).
- **A package's tests need explicit CI wiring** (Reason: spec-004 review I1 — `cargo build --workspace`
  does NOT run pytest/node:test; the conformance gate must be wired as its own task or the runners rot
  green).
- **Verify versions at plan time against crates.io/npm directly** (Reason: spec-005 — the napi-derive
  version assumption was wrong, caught at build).

## Conflict Warnings

- **None.** Roadmap §278, constitution (VII + C-01/C-07), and the as-built bindings all agree on the
  marshaling-+-schema-round-trip scope and the render-parity-is-structural exclusion. No hard or soft
  conflict; planning may proceed.

## Retrieval Notes

- `speckit_memory_*` MCP tools + the SQLite cache (`.spec-kit-memory/`) are NOT available in this session;
  followed the documented retrieval-order fallback (read governance layer + durable memory directly).
- Read: constitution.md, DECISIONS.md, roadmap.md §278, docs/memory/{INDEX,PROJECT_CONTEXT}.md, and the
  as-built spec memories 002/004/005. `docs/memory/{decisions,architecture,bugs}/` are empty (confirmed).
- Budget: within limits (5 decisions, 4 arch constraints, 0 deviations, 2 security, 3 lessons). ~600 words.
