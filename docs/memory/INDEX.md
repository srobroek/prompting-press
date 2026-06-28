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

_The load-bearing design decisions C-01…C-09 still live as constitution principles in
`.specify/memory/roadmap.md` and `docs/research/feature-scope.md`; migrate technical implementation
decisions here as they are made._

## Workflow

- Memory-first workflow: `.specify/memory/workflow.md`
