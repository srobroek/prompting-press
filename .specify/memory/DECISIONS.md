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

**Per-language threshold (decided 2026-06-28, with the user):**
- **TS/JS + Python** — strict: ANY optional param, or >~2 params, moves into an options object /
  keyword-only args. Their positional optionals are the order-fragile `null`-soup the rule targets.
- **Rust** — `Option<T>` is a self-documenting optional at the call site (`Some("formal")`, not a bare
  `null`), so a **single** optional/`Option` param is idiomatic and NOT a violation. The options-struct
  / builder form is required only at **2+** optional params (a genuine long tail). Consequence: the
  `prompting-press` Rust consumer (`render<V>(.., variant: Option<&str>, guard: &GuardConfig)` — one
  optional + one required config; `get_source(.., variant)`; `Composition::append(.., variant)` — one
  optional each) **stays positional, conformant, no refactor.** The kernel likewise.

**Propagation / migration**:
- Roadmap decision **C-11** records the same rule + the Rust threshold in the spec ledger.
- **Applied** in spec 005 (TS binding): `render`/`getSource`/`Composition` → options objects (`329cd20`).
- **Applied** in the Python binding (spec 004): `render`/`get_source`/`Composition.append`/`GuardConfig`
  made keyword-only via PyO3 `signature` `*,` (this change).
- **Rust** (kernel + consumer): no change — below the Rust threshold (see above).
- Dependent templates (plan/spec/tasks) need no structural change; this is a coding-idiom rule a
  reviewer applies, not a new workflow gate.

**Note**: Authored directly (not via `/speckit.constitution`) because the session was running unattended
under explicit user direction. A later `/speckit.constitution` pass may re-derive the sync-impact report;
the change itself is faithful to the amendment policy.
