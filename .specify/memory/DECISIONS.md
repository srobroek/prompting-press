# Constitution Amendment Decisions

Records constitution amendments per the Governance section's amendment policy (written rationale +
version bump + propagation). Newest first.

## 2026-06-30 — v1.2.0 → v2.0.0 (MAJOR): guard delimits untrusted values in the body

**Change**: The opt-in guard, when enabled, now **delimits untrusted values directly in the rendered
body** — wrapping each untrusted interpolation in fixed `<untrusted>…</untrusted>` tags with `<`/`>`/`&`
inside the value entity-escaped — instead of leaving the body byte-identical to a plain render and only
naming the fields in a separate advisory string. The advisory text now references those markers so the
downstream model can locate every untrusted span. Two coupled breaking changes land with it: the
per-variable `origin` enum (`trusted|untrusted|external`) collapses to a **`trusted` boolean**, and the
caller-supplied **custom guard template (spec 002 FR-024) is removed** (the advisory is fixed and
references the markers).

**Why (rationale)**: a guard that names untrusted fields but leaves the body unchanged is incomplete —
a model told "treat `query` as data" cannot locate where `query`'s value appears once the key name is
gone and only the substituted value remains. Wrapping untrusted spans with explicit delimiters is the
industry-standard injection defense (Anthropic, OpenAI, OWASP LLM guidance); it gives the model both a
policy statement and a textual locator. Fixed tags + entity-escaping (not random nonces) keep the output
deterministic, preserving cross-binding parity (Principle I) and `render_hash` determinism.

**Constitution clauses amended**:
- **Principle III (Minimal Boundary)** — clarified that inserting structural delimiter markers around an
  untrusted value's interpolation is IN-boundary (still rendered-text + provenance; no I/O / LLM / request
  assembly). Non-mutation doctrine narrowed: VALUE CONTENT is never altered (spec 002 FR-025 preserved —
  no sanitize/strip/semantic change; entity-escaping is reversible structure), but the guard-on body is no
  longer byte-identical to a plain render.
- **Principle V (Provenance)** — `render_hash = SHA256(rendered output)` preserved in form; the rendered
  output now depends on the guard mode (guard-on ⇒ delimited body; guard-off ⇒ plain body). Both
  deterministic; a trace records which mode produced a hash. `template_hash` unaffected.

**Spec 002 FR/SC redefined** (downstream of the amendment, applied in spec 015):
- **FR-022** — "MUST NOT concatenate the guard into the body … body identical to a plain render": the
  guard-OFF half is preserved; the guard-ON half is replaced — the body now carries delimiter markup.
- **FR-023** — "producing the guard MUST NOT modify the rendered body content": narrowed — structural
  marker insertion is permitted; value content remains unaltered.
- **FR-024** — caller-overridable guard template: **removed** (fixed advisory referencing the markers).
- **SC-005** — split: guard-off body byte-identity preserved; guard-on body-identity replaced by the new
  SC-D01/SC-D04 (untrusted spans wrapped; guard-off unchanged).
- **FR-025** — PRESERVED: untrusted values are never sanitized/stripped/semantically mutated.

**Roadmap decision amended**: **C-09** ("var provenance is metadata + lint + opt-in guard, never silent
mutation") — the guard now also inserts structural delimiters into the body when opted in, while the
no-silent-value-mutation core is reaffirmed.

**Migration**: callers who enabled the guard and relied on the body being byte-identical to a plain
render MUST update — the guard-on body now contains `<untrusted>` markup and `render_hash` differs from
the guard-off render. The guard-OFF body invariant is fully preserved (no markup, no escaping). Callers
using a custom guard template must drop it (the advisory is now fixed). Schema consumers must migrate
`origin: trusted|untrusted|external` → `trusted: true|false`.

**Propagation**: spec 015 implements the kernel pre-pass, the schema change (+ regenerated Rust/Pydantic/TS
shapes), the binding updates, and the guard-guide rewrite (written as current behavior, landing in the
same PR). CLAUDE.md carries an APM-rendered copy of this constitution that is independently stale (still
v1.0.0) — regenerate via APM; tracked separately from this amendment.

**Note**: Authored directly (not via `/speckit.constitution`, which is not installed as an extension)
under explicit user direction in an unattended session — the same precedent as the v1.1.0/v1.2.0
amendments. The change is faithful to the amendment policy (rationale + MAJOR bump + propagation recorded).

## 2026-06-28 — v1.1.0 → v1.2.0 (MINOR): Principle VI gains construction-time validator binding + per-variable `validation_required`

**Change**: Added three bullets to **Principle VI (Per-Language Idiom Over Forced Uniformity)**: (1) validators
MAY be bound to the prompt object **at construction** (not only supplied per render) — a first-class immutable
prompt holds its validator(s) and reuses them at render; (2) an optional per-variable **`validation_required`**
boolean, **orthogonal to the `origin` trust tag**, lets a prompt mandate that a covering validator was supplied
for that variable; (3) enforcement of that coverage is **intentionally asymmetric** across languages —
TypeScript (Zod) and Python (Pydantic) introspect the supplied validator's per-field coverage and **throw/raise
at construction** when a `validation_required` variable is uncovered, while **Rust keeps garde** with coverage
guaranteed **structurally at compile time** and treats `validation_required` as **declarative metadata** (no
runtime coverage throw — Rust surfaces such errors at compile time, the idiomatic expectation). The kernel stays
validation-blind (Principle III): per-variable validators and `validation_required` enforcement live only in the
binding/consumer layer.

**Version bump**: MINOR (1.1.0 → 1.2.0) — a principle was *materially expanded* with new additive guidance, not
removed or redefined (which would be MAJOR), and more than a clarification (PATCH). Per the Governance policy.

**Rationale**: Surfaced at the spec-008 (Pre-publish API & schema reshape) clarify session. The prompt-as-object
reshape introduces a first-class immutable `Prompt`; binding the validator onto that object (rather than only
passing it at each render) is the natural ergonomic consequence. The user directed (verbatim) that "the
constitution needs to be adjusted to what people would expect with rust; garde is the idiomatic way, and rust
should not run into runtime errors when we can do compile time errors." That is the crux: TS/Python can
runtime-introspect a Zod schema's `.shape` / a Pydantic model's `model_fields` to enforce per-variable coverage
at construction, but garde derives rules on a compile-time struct and exposes **no** runtime rule-introspection.
Forcing Rust to fake a runtime coverage throw would be the alien, non-idiomatic API this principle exists to
prevent. Endorsing the asymmetry (runtime in the dynamic languages, compile-time/structural in Rust) IS
"uniform capability, native idiom."

**Propagation / migration**:
- **Applied** in spec 008: the per-variable `validation_required` schema field, construction-time validator
  binding, and the asymmetric coverage enforcement (FR-022..FR-025 of `specs/008-api-schema-reshape/spec.md`).
- Dependent templates (plan/spec/tasks) need no structural change; like the v1.1.0 C-11 amendment, this is a
  coding-idiom rule a reviewer applies, not a new workflow gate. The plan's Constitution Check verifies it.
- Roadmap decisions C-06 (native validators; errors normalized) and C-11 (call-shape) are the lineage; no
  roadmap-ledger renumber needed (this expands Principle VI, not a new C-NN).

**Note**: Authored via `/speckit.constitution` under explicit user pre-authorization given at the spec-008
clarify session (the user was stepping away and directed the amendment direction). Faithful to the amendment
policy; trivially revertable (three additive bullets + version line + this record).

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
