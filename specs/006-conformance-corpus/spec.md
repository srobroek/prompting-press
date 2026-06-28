# Feature Specification: Conformance corpus + cross-language hardening

**Feature Branch**: `006-conformance-corpus`

**Created**: 2026-06-28

**Status**: Draft

**Input**: User description: "Conformance corpus + cross-language hardening. Implements constitution Principle VII as a CI gate across all three packages (Rust, Python, TypeScript). The corpus guards two things ONLY: (1) FFI-boundary marshaling parity — the same logical input pushed through each binding must yield identical rendered text AND identical template_hash/render_hash, exercised across the hard marshaling cases: datetime/Date/chrono dates, Decimal, nested models, null/undefined/None, and int-vs-float; (2) schema round-trip parity — a schema-valid prompt document is accepted identically and a schema-invalid one is rejected identically across all three languages. Explicitly OUT of scope: comprehensive render-parity fixtures (render parity is structural via Principle I / C-01, guaranteed by the single shared Rust core — the corpus must NOT re-test it; only the small engine-regression render-fixture set from spec 002 remains, unchanged). No engine logic may live in any binding (C-02). Governed by C-01 and C-07. Depends on specs 004 (Python binding) and 005 (TypeScript binding), both implemented."

## Overview

This feature is the **conformance corpus**: a shared, language-neutral set of fixtures plus a thin
per-language runner and a CI gate, proving that the three bindings over the shared Rust core behave
identically at the two seams the shared core does **not** itself guarantee.

Prompting Press renders **once, in Rust** (the `prompting-press-core` kernel), and every language binds
that same engine. Because rendering happens once, cross-language render byte-identity is a **structural
property of the shared core** (constitution Principle I / roadmap C-01) — it is true by construction and
is **not** something this corpus re-tests. What the shared core does *not* guarantee on its own are the
two **per-binding** seams:

1. **Marshaling** — each binding converts its language-native input values (a Python `datetime`, a JS
   `Date`, a `Decimal`, a nested model, `null`/`undefined`/`None`, an integer vs a float) into the
   kernel's value type. That conversion is binding-owned code. If two bindings marshal the "same" logical
   value differently, they render different text and stamp different hashes — and the shared core cannot
   catch it, because each binding hands the core an already-marshaled value.
2. **Schema acceptance** — each binding accepts prompt documents through its own loader path. Whether a
   schema-valid document is accepted and a schema-invalid one is rejected **identically** across the three
   languages is the round-trip guarantee behind "one JSON Schema, N generated shapes" (Principle VII /
   C-07).

The corpus turns both into a **CI gate**: the same logical input pushed through each binding MUST yield
identical rendered text and identical `template_hash`/`render_hash`, and the same prompt document MUST be
accepted-or-rejected identically across Rust, Python, and TypeScript. A divergence fails the build.

This is the spec the roadmap always pointed the second binding at: with two FFI bindings now real (Python
spec 004, TypeScript spec 005) over the same core plus the Rust consumer (spec 003), the marshaling
boundary can finally be tested for parity rather than asserted.

**What this feature is NOT** (boundary defense): it is not new library capability. It adds no I/O, no LLM
calls, no token counting, no request-body assembly, no new public API surface, and no engine logic in any
binding (Principle III / C-02). It adds **fixtures, three thin test runners, and a CI gate**. It does not
re-test render parity (structural per C-01); the only render fixtures that exist remain spec 002's small
engine-regression set, unchanged and kernel-owned.

## Clarifications

### Session 2026-06-28

- Q: How should the conformance corpus be laid out so all three bindings test the *same* logical
  inputs? → A: **Shared corpus + 3 thin runners** — one language-neutral fixture set (proposed top-level
  `conformance/` directory) is the single source of truth; one thin runner per binding (Rust, Python,
  TypeScript) reads it and drives that binding's real path. No per-language fixture copies (they drift);
  no generated-from-master pipeline (a second codegen + freshness gate is overhead the runtime-readable
  fixtures don't need). Matches the existing `schemas/jsonschema/fixtures/` shared-set pattern.
- Q: How should marshaling parity be asserted — what plays the role of the "expected" render text and
  `template_hash`/`render_hash`? → A: **Cross-check + golden tripwire** — the primary assertion is
  cross-binding equality (the three bindings' render + hashes equal each other = pure parity); PLUS a
  small committed golden value per fixture as a regression tripwire that also catches a kernel change
  moving all three in lockstep. The golden set stays tiny and MUST NOT grow into a comprehensive
  render-parity set (that would violate C-01 / FR-016).
- Q: How should fixture values for types without a universal native equivalent (dates, Decimal) be
  defined? → A: **Canonical serialized form** — each such value is defined by the serialized form the
  kernel sees (e.g. ISO-8601 string for dates, decimal-as-string), and each binding's runner constructs
  the native type (Python `datetime`/`Decimal`, JS `Date`, Rust `chrono`) that MUST marshal to that one
  form. This exercises the real binding marshaling code; a binding that currently diverges is exactly the
  hardening finding the corpus exists to surface.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Prove marshaling parity across all three bindings (Priority: P1)

A maintainer changes a binding's marshaling bridge (or bumps a binding-layer dependency such as
`pythonize`, the napi value codec, or a serde adapter) and needs certainty that the same logical input
still produces byte-identical rendered text and byte-identical provenance hashes through every binding —
so a marshaling regression in one language is caught before merge rather than discovered as a silent
cross-language divergence in production.

**Why this priority**: This is the corpus's reason to exist and the headline guarantee of Principle VII's
marshaling half. Without it, a binding can silently coerce a date, drop a nested field, or round a float
differently from its siblings and nothing fails. It is the one property the shared core architecturally
cannot self-verify, because each binding feeds the core a pre-marshaled value.

**Independent Test**: Take one shared conformance fixture (a logical input value + a prompt definition +
the expected rendered text and expected `template_hash`/`render_hash`), drive it through each binding's
render path, and assert every binding produces (a) the same rendered text as every other binding and
(b) the same two hashes as every other binding (and, where pinned, as the committed expectation). Fully
testable with no other story present.

**Acceptance Scenarios**:

1. **Given** a fixture whose input contains a date value (a Python `datetime`, the equivalent JS `Date`,
   the equivalent Rust `chrono` value), **When** the fixture is rendered through each binding, **Then**
   all three bindings produce identical rendered text and identical `template_hash`/`render_hash`.
2. **Given** a fixture whose input contains a `Decimal`/high-precision numeric value, **When** rendered
   through each binding, **Then** all three produce identical rendered text and identical hashes (no
   binding silently rounds, truncates, or reformats the value differently from another).
3. **Given** a fixture whose input is a nested model (object within object, list of objects), **When**
   rendered through each binding, **Then** all three produce identical rendered text and identical hashes.
4. **Given** a fixture exercising `null` vs `undefined`/`None` vs an absent field, **When** rendered
   through each binding, **Then** all three agree: explicit `null`/`None` renders as the kernel's `null`
   value, and `undefined`/absent triggers the identical strict-undefined behavior — same text, same
   hashes, across bindings.
5. **Given** a fixture pairing an integer (`1`) with a **fractional** float (`2.5`), **When** rendered
   through each binding, **Then** all three agree: the integer renders integer-form (`1`) and the
   fractional float renders float-form (`2.5`) — no binding silently widens, narrows, or reformats the
   number differently from another. (The `1.0`-vs-`1` distinction is NOT tested — it is unrepresentable in
   JavaScript's single number type; see Edge Cases.)
6. **Given** any marshaling fixture, **When** a single binding is deliberately made to marshal that value
   differently (a seeded mutation), **Then** the corpus gate fails and names the diverging binding, the
   fixture, and whether the divergence is in the rendered text or in a hash.

---

### User Story 2 - Prove schema round-trip parity across all three bindings (Priority: P1)

A maintainer changes the JSON Schema, a generated per-language shape, or a binding's loader path and needs
certainty that a schema-valid prompt document is still accepted identically and a schema-invalid one is
still rejected identically through every binding — so "one JSON Schema, N generated shapes" cannot drift
into one language quietly accepting a document another rejects.

**Why this priority**: This is the corpus's second guarantee (Principle VII / C-07 round-trip half) and is
co-equal with marshaling parity — both are the verified scope of spec 006. A schema/shape drift that makes
the bindings disagree on validity breaks the portability promise of a canonical, language-neutral prompt
document.

**Independent Test**: Take each shared schema fixture (a prompt document plus the expected accept/reject
verdict), feed it through each binding's loader, and assert every binding reaches the same verdict as every
other binding and as the committed expectation; for rejected documents, assert each binding surfaces a
structured rejection rather than partially loading or crashing.

**Acceptance Scenarios**:

1. **Given** a schema-**valid** prompt document, **When** it is loaded through each binding's loader,
   **Then** all three bindings accept it and produce an equivalent loaded prompt definition.
2. **Given** a schema-**invalid** prompt document (e.g. missing a required field, an extra root key, a bad
   role, a variant named `default`, malformed JSON/YAML), **When** it is loaded through each binding,
   **Then** all three bindings reject it with a structured error — none partially loads, silently coerces,
   or crashes.
3. **Given** the full schema-fixture set, **When** the round-trip runs in every binding, **Then** the
   accept/reject verdict is identical across all three bindings for 100% of fixtures.
4. **Given** a fixture where one binding is deliberately made to disagree (a seeded acceptance of an
   invalid doc, or rejection of a valid one), **When** the gate runs, **Then** it fails and names the
   diverging binding and fixture.

---

### User Story 3 - Run the conformance corpus as a CI gate, locally reproducible (Priority: P2)

A maintainer (or CI) runs the conformance corpus as a merge-gating check that is reproducible on a local
machine with one command per binding, consistent with how every other gate in the repo is wired (gate
logic in moon tasks / scripts, called by the CI workflow), so that a marshaling or schema-round-trip
divergence blocks the merge and can be diagnosed locally.

**Why this priority**: A corpus that is not wired as an enforced, locally-runnable gate is documentation,
not a guarantee. It depends on US1/US2 fixtures existing first, hence P2.

**Independent Test**: From a clean checkout, run the corpus gate command(s) locally and confirm they
execute the shared fixtures through all three bindings and pass; introduce a seeded divergence and confirm
the same command fails with a diagnostic naming the binding and fixture; confirm the CI workflow invokes
the same gate.

**Acceptance Scenarios**:

1. **Given** a clean checkout, **When** the maintainer runs the conformance gate command(s) locally,
   **Then** the shared fixtures execute through the Rust consumer, the Python binding, and the TypeScript
   binding, and the gate passes.
2. **Given** the CI workflow, **When** a pull request runs, **Then** the conformance gate runs and a
   marshaling-parity or schema-round-trip divergence fails the build.
3. **Given** a failing fixture, **When** the gate reports, **Then** the failure identifies the binding,
   the fixture, and the kind of divergence (rendered-text, a specific hash, or accept/reject verdict),
   without leaking raw bound-value content beyond what the fixture itself already contains.

---

### Edge Cases

- **Decimal has no native JS type**: JavaScript has no built-in decimal type (only `number`/`bigint`). The
  corpus's logical "Decimal" case MUST be defined as a value representable consistently in all three
  languages (see FR-006 / Assumptions), so "Decimal parity" is a meaningful, testable claim rather than a
  comparison of three incompatible representations.
- **Date representations differ per language**: Python `datetime`, JS `Date`, and Rust `chrono` are
  distinct types. The corpus's logical date case MUST pin one expected kernel value (and therefore one
  expected rendered string + hashes) that all three marshal to, so the test asserts convergence rather
  than encoding three separate expectations.
- **`null` vs `undefined`/`None` vs absent**: the already-fixed contract (specs 004/005: `undefined`/absent
  → field-not-present → strict-undefined; explicit `null`/`None` → JSON `null`) is *pinned* by the corpus,
  not redesigned. A fixture exercising all three forms must produce identical cross-binding behavior.
- **`1.0`-vs-`1` is unrepresentable in JavaScript**: JS has a single IEEE-754 number type, so `1.0` is
  indistinguishable from `1` (`JSON.stringify(1.0) === "1"`; the napi bridge reads an integral JS number
  as an integer). A "float that renders `1.0`" therefore cannot be constructed in the TS binding, while
  Python's `float(1.0)` would render `1.0` — an *inherent* property of JS's number model, NOT a marshaling
  defect the corpus should flag. The int-vs-float case therefore tests distinctions JS CAN represent: an
  integer (`1` → `1`) vs a **fractional** float (`2.5` → `2.5`). The `1.0`-vs-`1` distinction is excluded
  with this rationale (the feature-gap escape hatch below).
- **Hash determinism**: `template_hash`/`render_hash` are SHA-256 over strings; a fixture's expected hashes
  must be stable across OS and architecture (no locale/line-ending/float-formatting dependence). The
  corpus must not encode an OS-dependent expectation.
- **Engine-regression render fixtures are out of scope**: the corpus MUST NOT add comprehensive
  render-parity fixtures. Spec 002's small engine-regression render set
  (`crates/prompting-press-core/tests/fixtures/render/`) remains the only render-fixture set and is
  unchanged and kernel-owned. A reviewer who finds the corpus re-testing "does the renderer produce the
  right text for templates in general" has found scope creep.
- **A binding feature gap**: if a fixture exercises a capability one binding cannot express natively (a
  genuinely language-specific type with no representable logical equivalent), that case is excluded from
  the corpus with a recorded rationale rather than forcing an unfaithful mapping — the corpus tests
  *logical* parity, not type-system identity.

## Requirements *(mandatory)*

### The conformance corpus (shared fixtures)

- **FR-001**: The feature MUST provide a **shared, language-neutral conformance corpus** — a single set of
  fixtures consumed by all three bindings — so that "parity" is measured against one canonical expectation
  rather than three independently-authored fixture sets. Each binding MUST exercise the *same* logical
  inputs via a **thin per-language runner that reads the one shared fixture set** (clarified: shared
  corpus + 3 runners). The feature MUST NOT keep per-language fixture copies (they drift) and MUST NOT
  introduce a generated-from-master fixture pipeline (a second codegen + freshness gate is unnecessary
  for runtime-readable fixtures). The shared set follows the existing `schemas/jsonschema/fixtures/`
  pattern (proposed top-level `conformance/` directory; exact path/format finalized at plan time).
- **FR-002**: The corpus MUST contain **marshaling fixtures**, each defining a logical input value, the
  prompt definition it is rendered through, and the expected outcome (rendered text and the expected
  `template_hash`/`render_hash`). The marshaling fixtures MUST cover, at minimum: **dates**
  (datetime/Date/chrono), **Decimal/high-precision numerics**, **nested models** (objects and lists of
  objects), **`null` vs `undefined`/`None` vs absent**, and **integer vs fractional float** (an integer
  `1` vs a fractional float `2.5` — distinctions representable in all three languages; `1.0`-vs-`1` is
  excluded as JS-unrepresentable, see Edge Cases).
- **FR-003**: The corpus MUST contain **schema round-trip fixtures**, each defining a prompt document and
  its expected accept-or-reject verdict, covering both schema-valid and schema-invalid documents
  (including the existing invalid cases: missing-required, extra-root-key, bad-role, variant-named-default,
  malformed JSON/YAML). The corpus MUST reuse / build on the existing schema fixtures at
  `schemas/jsonschema/fixtures/{valid,invalid}/` rather than forking a parallel set.
- **FR-004**: The corpus's expected values MUST be defined so they are **stable across operating system and
  architecture** (no locale-, line-ending-, or platform-float-formatting dependence). A fixture MUST NOT
  encode an OS-dependent expectation.

### Marshaling-parity verification

- **FR-005**: For every marshaling fixture, the feature MUST verify that **all three bindings** (the Rust
  consumer, the Python binding, the TypeScript binding) produce **identical rendered text** and
  **identical `template_hash` and `render_hash`** for that fixture's logical input. The **primary**
  assertion MUST be a cross-binding equality (each binding agrees with the others), so a divergence in any
  one binding fails (clarified: cross-check). The feature MUST **additionally** pin a small **committed
  golden** render/hash value per fixture as a regression tripwire — catching a kernel-level change that
  shifts all three bindings in lockstep, which a pure cross-check would miss. The golden set MUST stay
  small and MUST NOT grow into a comprehensive render-parity set (that would violate C-01 / FR-016).
- **FR-006**: The feature MUST define each language-native input type's **logical equivalent** so the
  "same logical input" is unambiguous across languages. Each such value MUST be defined by its **canonical
  serialized form** — the form the kernel sees (clarified: e.g. ISO-8601 string for dates,
  decimal-as-string for high-precision numerics) — and each binding's runner MUST construct the native
  type (Python `datetime`/`Decimal`, JS `Date`, Rust `chrono`, nested model, the three null-forms,
  integer-vs-float) that marshals to that one form. This drives each binding's **real marshaling code**;
  a binding that fails to marshal its native type to the canonical form is a hardening finding the corpus
  is meant to surface, not a fixture to weaken. For a type with no native equivalent in a language (e.g.
  Decimal in JS), the canonical form MUST be representable in all three without a third-party dependency
  in the shipped library (Assumptions records the chosen representation).
- **FR-007**: The marshaling-parity check MUST exercise each binding's **real render path** (the same
  public render entry the spec-003/004/005 surfaces expose), driving the value through that binding's
  actual marshaling bridge — not a bypass that hand-builds the kernel value. The point is to test the
  binding-owned marshaling code, not to re-test the kernel.
- **FR-008**: The feature MUST pin the already-fixed `null`/`undefined`/`None`/absent contract (specs
  004/005: explicit `null`/`None` → JSON `null`; `undefined`/absent → field-not-present → kernel
  strict-undefined) as a cross-binding equality, so any binding diverging from it fails. The feature MUST
  NOT redesign this contract.

### Schema-round-trip verification

- **FR-009**: For every schema round-trip fixture, the feature MUST verify that **all three bindings**
  reach the **same accept-or-reject verdict** as each other and as the fixture's expected verdict, by
  driving the document through each binding's **own loader path** (not a single standalone schema
  validator standing in for all three).
- **FR-010**: For a rejected (schema-invalid) document, each binding MUST surface a **structured
  rejection** (the binding's normalized error contract), and MUST NOT partially load, silently coerce, or
  crash — verified identically across the three bindings.
- **FR-011**: The schema round-trip verification MUST cover YAML and JSON document forms where the bindings
  accept both, confirming a binding's YAML and JSON acceptance of the same logical document agree (YAML↔
  JSON parity is structural via the shared loader, but the corpus pins that each binding actually routes
  through it).

### CI gate wiring (C-01 boundary, repo conventions)

- **FR-012**: The conformance corpus MUST be wired as a **CI gate** that runs on pull requests and fails
  the build on any marshaling-parity or schema-round-trip divergence, consistent with the repo's existing
  gate pattern (gate logic in moon tasks / `scripts/ci/*.sh`, invoked by the GitHub Actions workflow).
- **FR-013**: The conformance gate MUST be **locally reproducible** with documented `mise exec -- moon run
  …` command(s), mirroring how `ci:check-ffi`, `ci:test-python`, and `ci:test-node` are run, so a
  divergence can be diagnosed without CI.
- **FR-014**: The gate's failure output MUST identify the **diverging binding, the fixture, and the kind of
  divergence** (rendered text, a named hash, or accept/reject verdict). Failure output MUST NOT leak raw
  bound-value content beyond what the fixture file itself already contains (consistent with the SEC-004
  scrub posture; corpus fixtures contain only non-secret test data by construction).
- **FR-015**: The feature MUST include the **Rust consumer as a first-class binding** in the corpus
  (alongside Python and TypeScript), since cross-language parity is meaningless without the reference Rust
  surface participating. A Rust conformance runner MUST drive the shared fixtures through the
  `prompting-press` consumer.

### Boundary & scope guards (C-01, C-02, Principle I/III)

- **FR-016**: The feature MUST NOT add **comprehensive render-parity fixtures**. Render byte-identity is a
  structural property of the single shared core (Principle I / C-01) and MUST NOT be re-tested by the
  corpus. Spec 002's small engine-regression render-fixture set
  (`crates/prompting-press-core/tests/fixtures/render/`) remains the only render-fixture set, unchanged.
- **FR-017**: The feature MUST NOT introduce **any engine logic into a binding** (rendering, agreement
  analysis, variant resolution, hashing) — these live once in the Rust core (C-02 / Principle II). The
  corpus adds tests and a gate, not capability. The existing `ci:check-ffi` gate MUST stay green.
- **FR-018**: The feature MUST NOT expand the library boundary: no I/O in the library, no LLM calls, no
  request-body assembly, no token counting, no new public API surface on any binding (Principle III). The
  corpus's own runners may read fixture files (they are test harnesses, not the library).
- **FR-019**: The corpus MUST treat the marshaling expectation as derived from the **shared core's**
  behavior — i.e. the expected rendered text and hashes are what the kernel produces for the correctly
  marshaled value — so the corpus measures *binding marshaling fidelity*, never proposing an alternative
  rendering of its own.

## Key Entities *(include if feature involves data)*

- **Conformance fixture (marshaling)**: a language-neutral case carrying a logical input value, the prompt
  definition it renders through, and the expected outcome (rendered text + expected
  `template_hash`/`render_hash`). Authored once; consumed by all three bindings' runners.
- **Conformance fixture (schema round-trip)**: a language-neutral case carrying a prompt document and its
  expected accept-or-reject verdict. Builds on the existing `schemas/jsonschema/fixtures/{valid,invalid}/`
  set rather than forking it.
- **Logical-type mapping**: the per-language construction recipe for each fixture's input value (how the
  canonical date / Decimal / nested model / null-form / int-vs-float is built in Python, TypeScript, and
  Rust), so "same logical input" is unambiguous.
- **Corpus runner (per language)**: a thin test harness — one for the Rust consumer, one for the Python
  binding, one for the TypeScript binding — that loads the shared corpus, drives each fixture through that
  binding's real render/loader path, and asserts equality against the canonical expectation (and/or the
  other bindings).
- **Conformance CI gate**: the moon task(s) + workflow wiring that runs the runners on PRs, is locally
  reproducible, and fails the build (naming binding + fixture + divergence kind) on any parity or
  round-trip divergence.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For 100% of marshaling fixtures, the Rust consumer, the Python binding, and the TypeScript
  binding produce **identical rendered text** and **identical `template_hash`/`render_hash`**; any
  divergence fails the gate.
- **SC-002**: The marshaling fixtures cover **all five** named hard cases — dates (datetime/Date/chrono),
  Decimal/high-precision numerics, nested models, `null`/`undefined`/`None`/absent, and integer-vs-
  fractional-float (`1` vs `2.5`; `1.0`-vs-`1` excluded as JS-unrepresentable) — with at least one fixture
  per case exercised through all three bindings.
- **SC-003**: For 100% of schema round-trip fixtures, all three bindings reach the **same accept/reject
  verdict** as each other and as the expected verdict; a schema-invalid document is rejected with a
  structured error (no partial load, no crash) in every binding.
- **SC-004**: A **seeded divergence** in any single binding (a deliberately wrong marshaling of one fixture
  value, or a wrong accept/reject) causes the gate to **fail** and name the diverging binding, the
  fixture, and the divergence kind — in 100% of seeded cases — demonstrating the gate actually detects
  drift rather than passing vacuously.
- **SC-005**: The conformance gate is **locally reproducible**: from a clean checkout, the documented
  `moon run` command(s) execute the shared corpus through all three bindings and reproduce the CI result.
- **SC-006**: The corpus adds **zero** comprehensive render-parity fixtures and **zero** engine logic to
  any binding; the spec-002 render-fixture set is byte-unchanged and the `ci:check-ffi` gate stays green
  (scope is held to marshaling + schema round-trip).
- **SC-007**: The conformance gate runs on pull requests and is enforced (a divergence blocks merge),
  taking its place alongside the existing FFI, codegen-freshness, advisory, and binding-test gates.

## Assumptions

- **Shared corpus, three thin runners (RESOLVED — clarify Q1)**: the corpus is a single language-neutral
  fixture set (proposed top-level `conformance/` directory) consumed by one thin runner per binding,
  rather than per-language fixture copies (which drift) or a generated-from-master pipeline (an
  unnecessary second codegen + freshness gate for runtime-readable fixtures). A single source is what
  makes "parity" meaningful. Only the exact directory layout and fixture file format (JSON/YAML) are
  finalized at plan time.
- **Hash-pinning strategy (RESOLVED — clarify Q2)**: the corpus asserts **cross-binding equality** (each
  binding's render/hashes equal the others') as the **primary** parity guarantee, **and additionally**
  pins a small **committed golden** value per fixture as a regression tripwire. Cross-check proves parity;
  the golden also catches a kernel-level regression that moves all three in lockstep. The golden set is
  small and MUST NOT become a comprehensive render-parity set (that would violate C-01 / FR-016).
- **Logical Decimal representation (RESOLVED — clarify Q3, canonical serialized form)**: because JS has no
  native decimal type, the canonical "Decimal" fixture value is defined by its **serialized form** —
  expressible in all three languages without a third-party decimal dependency in the shipped library
  (e.g. a decimal-as-string, or a JSON number with a pinned interpretation). Each runner constructs its
  native high-precision value (Python `Decimal`, a Rust equivalent, a JS value) that must marshal to that
  one form. The exact serialized representation is finalized at plan time. The intent is to surface any
  binding that silently reformats high-precision numerics.
- **Logical date representation (RESOLVED — clarify Q3, canonical serialized form)**: the canonical date
  fixture pins one **serialized form** (e.g. ISO-8601 string) that the kernel sees — and therefore one
  expected rendered string + hashes. The logical-type-mapping records how Python `datetime`, JS `Date`,
  and Rust `chrono` are each constructed to marshal to it. The exact serialized representation is
  finalized at plan time.
- **CI job shape (DEFERRED to plan — execution detail, not a spec ambiguity)**: the conformance runners
  are wired as moon tasks following the existing `ci/moon.yml` + `scripts/ci/*.sh` pattern and invoked
  from `.github/workflows/ci.yml`. Whether they are a dedicated `conformance` job or folded into the
  existing `test-python` / `test-node` jobs plus a new Rust leg is a plan-time wiring decision (it does
  not change what is tested or any acceptance criterion); either way the Rust consumer participates
  (FR-015) and the gate is locally reproducible (FR-013).
- **The null/undefined/None contract is already fixed and consistent** (specs 004/005, FR-003a): the corpus
  pins it as a cross-binding equality; it does not redesign it. Explicit `null`/`None` → JSON `null`;
  `undefined`/absent → field-not-present → kernel strict-undefined.
- **Render parity is structural and excluded** (Principle I / C-01): the single shared Rust core guarantees
  byte-identical rendering across bindings by construction; the corpus tests marshaling and schema
  round-trip, never render parity. The only render fixtures are spec 002's engine-regression set,
  unchanged.
- **Both bindings are implemented and merged** (specs 004 + 005): Python (`prompting-press-py` →
  `packages/python`, PyO3 + Pydantic) and TypeScript (`prompting-press-node` → `packages/typescript`,
  napi-rs + Zod), each over the same kernel (spec 002) + Rust consumer (spec 003). Cross-binding
  `templateHash`/`renderHash` parity was already proven empirically (TS == Python); this spec makes it a
  permanent, enforced gate.
- **No new dependencies are expected** beyond test-harness wiring; if a runner needs a fixture-loading
  helper, it MUST be pinned exact (no floating versions — the `ci:check-floating-versions` gate scans
  manifests) and MUST NOT introduce a second YAML parser or a JS decimal library into the shipped library
  (test-only is acceptable if unavoidable, but the preference is none).
- **No publish here**: registry publication + release tooling is spec 007. This spec produces enforced
  cross-language gates, not a release.

## Dependencies

- **Spec 004 (Python binding) — satisfied/merged**: `prompting-press-py` → `packages/python`. The Python
  render/loader paths the corpus drives, and the `pythonize`-based marshaling bridge whose fidelity the
  corpus verifies.
- **Spec 005 (TypeScript binding) — satisfied/merged**: `prompting-press-node` → `packages/typescript`.
  The TS render/loader paths the corpus drives, and the napi value codec whose fidelity the corpus
  verifies.
- **Spec 003 (Rust consumer) — satisfied/merged**: `prompting-press`. The reference binding the corpus
  includes as a first-class participant (FR-015), and the dual-input loader + `check` + error contract the
  other bindings reuse across FFI.
- **Spec 002 (Engine kernel) — satisfied/merged**: `prompting-press-core`. The single shared core whose
  behavior defines the corpus's expected render + hashes, and whose existing render-regression fixture set
  (`tests/fixtures/render/`) the corpus leaves unchanged.
- **Spec 001 (Foundations) — satisfied/merged**: the prompt-definition JSON Schema and the existing schema
  fixtures (`schemas/jsonschema/fixtures/{valid,invalid}/`) the round-trip corpus builds on, plus the
  established CI gate pattern (moon tasks + `scripts/ci/*.sh` + `ci.yml`) the conformance gate follows.

## Governance Alignment

Governed by constitution Principles **I** (shared core — render parity is structural and NOT re-tested;
the corpus tests the per-binding marshaling and schema-acceptance seams the core cannot self-verify),
**II** (FFI isolation — no engine logic enters a binding; `ci:check-ffi` stays green), **III** (minimal
boundary — the corpus adds tests + a gate, never I/O, model calls, token counting, or new public API), and
**VII** (JSON Schema single source — schema round-trip parity is the corpus's second guarantee), and by
roadmap decisions **C-01** (structural render parity) and **C-07** (conformance corpus scoped to FFI
marshaling + schema round-trip). No new pluggable interface; no boundary-expanding capability. This is the
spec the roadmap reserved for the moment a second binding made the FFI marshaling boundary real.
