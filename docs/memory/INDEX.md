# Memory Index

This is a compact routing map for durable project memory (`docs/memory/`). Keep it short.

> [!NOTE]
> High-level project governance, constitution, and standards are stored in the **Governance Layer**
> at `.specify/memory/` (constitution v1.0.0, roadmap ledger) and should be reviewed before
> technical planning.

## Architecture

- [A1 — Loader does serde shape-validation, not full JSON-Schema](architecture/2026-06-28-loader-vs-schema-validation-layers.md) — three validity layers (schema-validator / loader / `check()`) are not equivalent; `variant-named-default` is schema-invalid but loader-accepted. (spec 006)

## Bugs

_(none yet)_

## Decisions

- [D1 — Cross-binding type parity via canonical serialized form](decisions/2026-06-28-canonical-serialized-form-marshaling.md) — date/decimal pinned by serialized string, not native objects (which recanonicalize: Pydantic `Z`/`1E-17`, JS `Date` `.000Z`). (spec 006)
- [D2 — Parse detail preserved, Render detail scrubbed](decisions/2026-06-29-parse-detail-preserved-render-scrubbed.md) — SEC-004 refinement: parse errors are pre-binding template syntax (safe to surface); only render errors can carry bound values (scrubbed). (spec 008/010 follow-up)
- [D3 — Opt-in unsafe render-error detail per render call](decisions/2026-06-29-unsafe-render-detail-opt-in.md) — SEC-004 carve-out: explicit, off-by-default, per-call boolean surfaces Render-error detail; default unchanged; not a constitution amendment (no pluggable seam, no I/O). (spec 013)

_The load-bearing design decisions C-01…C-09 still live as constitution principles in
`.specify/memory/roadmap.md` and `docs/research/feature-scope.md`; migrate technical implementation
decisions here as they are made._

## Workflow

- Memory-first workflow: `.specify/memory/workflow.md`
