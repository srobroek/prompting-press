# Constitution Amendment Decisions

Records constitution amendments per the Governance section's amendment policy (written rationale +
version bump + propagation). Newest first.

## 2026-06-28 — v1.0.0 → v1.1.0 (MINOR): Principle VI gains the options-object call-shape rule

**Change**: Added a bullet to **Principle VI (Per-Language Idiom Over Forced Uniformity)** requiring
public functions with optional or >~2 meaningful parameters to take their optional/config tail as a
single named **options object** (TS/JS) or **keyword-only args** (Python `*, kw=...`) / options struct
(Rust), never a positional list of optionals. Required positional operands stay positional.

**Version bump**: MINOR (1.0.0 → 1.1.0) — a principle was *materially expanded* with a new MUST, not
removed or redefined (which would be MAJOR), and not a mere clarification (PATCH). Per the
Governance versioning policy.

**Rationale**: Surfaced during the spec-005 (TypeScript binding) review. The TS `render` had grown a
positional optional tail and could not select a variant without colliding with the `guard` arg
(`render(reg, name, schema, data, variant?, guard?)` is order-fragile and forces `null` placeholders);
the composition entry was a positional tuple that forced schema-vs-data **duck-typing** (sniffing for a
`.safeParse` method) — a Long Parameter List + Primitive-Obsession smell (refactoring.guru). Moving the
optional tail into a named options object (`render(reg, name, schema, data, { variant, guard })`,
composition entries as `{ name, schema?, data, variant? }`) fixed the variant parity gap, dissolved the
duck-typing, and is the idiomatic call shape in every target ecosystem. Python's parallel is
keyword-only args; Rust's is an options struct / builder. This is "uniform capability, native idiom" —
the existing spirit of Principle VI — made explicit as a call-shape rule.

**Propagation / migration**:
- Roadmap decision **C-11** records the same rule in the spec ledger (`.specify/memory/roadmap.md`).
- **Applied** in spec 005 (TS binding): `render`/`getSource`/`Composition` refactored to options
  objects (commit `329cd20`).
- **Deferred follow-ups** (tracked at roadmap-debrief, not blocking): the Python binding's `render`
  (`#[pyo3(signature = (reg, name, vars, data=None, variant=None, guard=None))]`) should make
  `data`/`variant`/`guard` keyword-only (`*,`) to conform — recorded as a spec-004 follow-up.
- Dependent templates (plan/spec/tasks) need no structural change; this is a coding-idiom rule a
  reviewer applies, not a new workflow gate.

**Note**: Authored directly (not via `/speckit.constitution`) because the session was running unattended
under explicit user direction. A later `/speckit.constitution` pass may re-derive the sync-impact report;
the change itself is faithful to the amendment policy.
