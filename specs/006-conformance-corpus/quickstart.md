# Quickstart — Running & extending the conformance corpus

A validation/run guide for the spec-006 conformance corpus. Implementation bodies live in `tasks.md` /
the implementation phase; this shows how to prove the feature works and how to add a case.

## Prerequisites

- The repo toolchain via `mise` (Rust `1.95.0`, Python 3.10+, Node 20+, `moon`, `uv`, `pnpm`) — same as
  every other gate. No new tooling.
- Both bindings build (specs 004/005 are merged): the Python extension and the Node addon.

## Run the whole corpus locally (the gate)

```bash
mise exec -- moon run ci:conformance
```

Expected: all three runners pass — the Rust consumer, the Python binding, and the TypeScript binding each
render every marshaling fixture to the committed golden text + hashes, and each agrees with the expected
accept/reject verdict on every schema fixture. A divergence fails with a message naming the binding, the
fixture `case`, and the divergence kind (text / template_hash / render_hash / verdict).

## Run one binding's runner

```bash
# Rust (the reference binding — also the one CI didn't previously test)
mise exec -- cargo test -p prompting-press --test conformance

# Python (also covered by ci:test-python)
mise exec -- moon run ci:test-python      # or: pytest packages/python/tests/test_conformance.py

# TypeScript (also covered by ci:test-node)
mise exec -- moon run ci:test-node        # or: node --test packages/typescript/test/conformance.test.mjs
```

## Validation scenarios (map to the spec)

| Scenario | How to see it | Spec ref |
|---|---|---|
| Marshaling parity (dates) | `conformance/marshaling/date.json` passes in all three runners | US1 AS-1, SC-001 |
| Marshaling parity (Decimal) | `decimal.json` passes in all three | US1 AS-2 |
| Nested models | `nested-model.json` passes in all three | US1 AS-3 |
| null / undefined / None | `null-undefined-none.json` passes in all three | US1 AS-4, FR-008 |
| int vs float | `int-vs-float.json` passes in all three | US1 AS-5 |
| Schema accept/reject parity | `conformance/schema/manifest.json` verdicts hold in all three loaders | US2, SC-003 |
| YAML path exercised | the YAML twins under `conformance/schema/yaml/` load via each `load_yaml` | FR-011 |
| Gate detects drift (seeded) | temporarily corrupt one runner's constructed value → its runner fails, naming binding+fixture | US1 AS-6, SC-004 |
| Render set untouched | `git status` shows `crates/prompting-press-core/tests/fixtures/` unchanged | FR-016, SC-006 |

## Seed a divergence (prove the gate isn't vacuous — SC-004)

Temporarily change one runner to construct a wrong value for one fixture (e.g. the TS runner passes a
`Date` one day off), run that runner, and confirm it fails citing the `case` and that `render_hash`
diverged from the golden. Revert.

## Regenerate the goldens (after an intentional kernel/template change)

```bash
# Runs the committed generator: renders each marshaling fixture through the Rust reference
# binding and writes expected.{text,template_hash,render_hash} back into the fixtures.
mise exec -- <generator command — defined in tasks.md>
```

Review the resulting fixture diff in the PR — a golden change is a deliberate, reviewable event (the
tripwire working as intended). Goldens MUST NOT be regenerated as a reflex to make a red runner green;
investigate the divergence first (it may be the real marshaling bug the corpus is built to catch).

## Add a new fixture

1. **Marshaling case**: add `conformance/marshaling/<case>.json` with `definition`, `input` (each field a
   `{type,value}` descriptor — see [data-model.md](./data-model.md)), and a placeholder `expected`.
   Run the generator to fill the golden. Confirm all three runners pass.
2. **Schema case**: add the document (reuse `schemas/jsonschema/fixtures/` where possible) and an entry in
   `conformance/schema/manifest.json` with `path`, `form`, `verdict`. Confirm all three runners agree.
3. If you introduce a new logical `type`, add its constructor row to each runner's type switch (the D2
   table) — all three or none, so parity holds.

## Definition of done (validation)

- `mise exec -- moon run ci:conformance` green on a clean checkout.
- All five marshaling hard-cases present and passing in all three bindings (SC-002).
- All schema fixtures' verdicts identical across the three loaders (SC-003).
- A seeded divergence fails the gate naming binding+fixture+kind (SC-004).
- `ci:check-ffi` still green; the spec-002 render fixtures byte-unchanged (SC-006).
- The gate runs on PRs and blocks merge on divergence (SC-007).
