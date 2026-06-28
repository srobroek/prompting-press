<!--
SYNC IMPACT REPORT
==================
Version change: 1.2.0 → 1.3.0
Bump rationale: MINOR — spec 008 materially broadened from "schema rename + fixture move"
  to "Pre-publish API & schema reshape": adds the prompt-as-object API redesign + the
  candidate `validation_required` schema field, bundled because they are one object-model
  decision (research note docs/research/registry-value-and-object-model.md). 008 now blocks
  010 too. Material scope change → MINOR. No constraint reversed; C-08/C-11 newly cited by
  008's expanded scope.

Changes this revision (1.3.0, 2026-06-28):
  - Spec 008 retitled "…schema & repo refinement" → "Pre-publish API & schema reshape";
    scope expanded to bundle (a) provenance→origin, (b) fixture move, (c) prompt-as-object
    API redesign (first-class Prompt; render/getSource/check migrate onto it; Composition
    aggregates Prompt objects; Registry kept-thin/optional/dropped — decided at clarify;
    resolves the TS render duck-typing), (d) candidate `validation_required` schema boolean.
    Clarify-gated open questions recorded. Now blocks 007 AND 010. Governed-by extended to
    C-01/C-06/C-08/C-11.
  - Added docs/research/registry-value-and-object-model.md (008 clarify input): researches
    the registry's value vs. plain Prompt objects (BAML method-per-prompt client; LangChain
    held template objects — neither uses name-keyed lookup) + the full object model + linked
    decisions + open questions.

Prior revision (1.2.0, 2026-06-28):
  Bump rationale: MINOR — three NEW planned specs added (008/009/010) + spec 007 scope narrowed
  (docs site → 010; READMEs slimmed) + depends-on extended. Structural additions → MINOR.

Changes in revision (1.2.0, 2026-06-28):
  - Added spec 008 — Pre-publish schema & repo refinement [planned]: rename the schema
    `provenance` field → `origin` (single source + 3 codegen'd shapes + all consumers;
    enum values unchanged) and move the schema accept/reject fixtures into a tests/
    namespace. Must land before 007 (don't rename a published contract). Governed C-07/C-09.
  - Added spec 009 — Adversarial hardening & fuzzing [planned]: robustness + property-based
    (proptest/hypothesis/fast-check) fuzzing, an honest injection/guard demo (guard is
    advisory, no LLM), and secret-scrub verification, in the library's own suites + CI.
    Should land before 007. Governed C-03/C-09.
  - Added spec 010 — Documentation site (Astro/Starlight) [planned]: end-user docs site
    (overview/getting-started/howto/FAQ/API-ref) derived from the codebase + JSON Schema +
    template feature set; README slimmed to a pointer. Subsumes the spec-007 user-docs +
    template-feature-doc gap. Depends on 008 (final field name). Governed C-07/C-05.
  - Spec 007 — scope narrowed (docs site → 010; per-package READMEs stay short pointers)
    and depends-on extended to 008 + 009 + 010 (sequencing: those three land before 007).
    Outcome/decisions otherwise unchanged.

Prior revision (1.1.4, 2026-06-28):
  Bump rationale: PATCH — spec 006 status transition (planned → implemented) from its
  post-implementation debrief. Outcome met, scope clean, C-01/C-07 upheld, zero
  Must-Address findings. Notes line added citing the debrief + memories D1/A1 + the
  int-vs-float critique refinement. No new decision; C-01..C-11 untouched.

Changes in revision (1.1.4, 2026-06-28):
  - Spec 006 status: planned → implemented (PR #185, squash b2092c0; 21 issues
    #164–#184 auto-closed; full Phase-3 QA — verify-tasks/verify/review/qa/
    code-review/security-review/cleanup/sync.analyze/sync.conflicts/retro — all
    passed clean; CI green incl. the new conformance gate, verified on a real runner).
    Debrief: specs/006-conformance-corpus/roadmap-reviews/debrief-2026-06-28.md.
  - Spec 006 Notes: recorded the as-built shape (shared corpus + 3 runners +
    ci:conformance gate that also closes the Rust-consumer CI gap), the two captured
    memories (D1 canonical-serialized-form marshaling; A1 loader-vs-schema-validator
    layers), and the int-vs-float critique refinement (1 vs 1.0 → 1 vs 2.5). Outcome/
    scope UNCHANGED — the debrief found the §278 entry accurate.

Prior revision (1.1.3, 2026-06-27):
  Bump rationale: PATCH — spec 003 status transition (planned → implemented) plus a
  scope refresh of the 003 entry from its post-implementation cycle: the token-count
  hook was DROPPED at the analyze gate (refinement F4) and folded into the existing
  "Token budgeting / truncation" Deferred entry. No new decision; C-01..C-10 untouched.

Changes in revision (1.1.3, 2026-06-27):
  - Spec 003 status: planned → implemented (code complete + SC-verified; full Phase-3
    QA — verify-tasks/verify/review/qa/code-review/security-review/cleanup/sync.analyze/
    sync.conflicts — all passed clean; 44 consumer tests, 94 workspace, all CI gates green
    incl. FFI-isolation). Debrief: specs/003-rust-consumer/retros/retro-2026-06-27.md.
  - Spec 003 Outcome/Scope: struck the token-count hook (analyze-gate refinement F4 —
    the bare seam added little; per Principle III token counting is deferred). Net 003
    surface: 24 FR + 8 SC (was 26 FR + 9 SC); FR-021/022 + SC-009 + task T024 dropped.
  - Spec 003 lint: the provenance half reframed to "declares untrusted/external + no
    guard configured" (`UntrustedWithoutGuard`) — the kernel has no in-template
    guard-position concept (refinement F1); the agreement half (FR-016/017) unchanged.
  - Deferred "Token budgeting / truncation": noted the 003 hook drop as its origin.

Changes in revision (1.1.2, 2026-06-26):
  - Spec 002 status: planned → implemented (code complete + SC-verified; full Phase-3
    QA — verify-tasks/verify/review/qa/code-review/security-review/cleanup/sync — all
    passed clean; 50 tests + 7/7 CI gates green).
  - Spec 002 Scope (in): "implicit/explicit default" → "root body is always the default
    arm; caller-named selection; unknown-variant the only resolution error" (debrief D1,
    roadmap-stale — the explicit-default path was structurally unreachable under the 001
    schema and removed via the FR-010 refine; implementation is correct).
  - Spec 002 Notes: recorded the implemented MiniJinja pin (2.21, default-features=false,
    macros/multi_template off, adjacent_loop_items kept), env-derived allowlist, and the
    separate-field guard (debrief D2; roadmap Q3 re-confirmed against the 2.21.0 tag).

Prior revision (1.1.1, 2026-06-25):
  - Spec 001 status: planned → implemented (code complete + SC-verified; moves to
    'verified' after the Phase-3 QA gates run).

Prior revision (1.1.0, 2026-06-25):
  - Added decision C-10 — release-please (unified linked library-package version)
    + native per-ecosystem build/publish (cargo publish / maturin / @napi-rs/cli);
    GoReleaser evaluated and rejected (binary builder, not library/wheel/addon).
    Reaffirms the two distinct version axes (package vs prompt-content/C-05).
  - Spec 007: added C-10 to Governed-by; recorded release-tooling decision in Notes.

Prior revision (1.0.0, 2026-06-25):
  Bump rationale: MAJOR — initial ratification of the spec roadmap. Created from
    the resolved post-grilling design (feature-scope.md) and constitution v1.0.0.
  - Added specs 001–007 (Phase 0 Foundations … Phase 6 v1 Release)
  - Added decisions C-01 … C-09 (the load-bearing constitution principles + R1)
  - Added Deferred section (Go binding, inline partials, token budgeting, etc.)
  - Added Never section (boundary defense — requires constitution amendment)

Specs affected: 008 (this revision, scope broadened → API reshape); 008/009/010 (1.2.0, added [planned]) + 007 (scope narrowed);
  006 (1.1.4, → implemented); 003 (1.1.3); 002 (1.1.2); 001 (1.1.1); 007 (1.1.0);
  001–007 (initial).
Open questions added/resolved: none this revision; 3 added at 1.0.0.

Notes: Supersedes the informal docs/research/roadmap.md (which remains as a
  human-readable narrative; this ledger is the governance artifact of record).
-->

# Prompting Press — Spec Roadmap

Living, non-binding map of the specs planned for Prompting Press. It is **not a
commitment to order or scope** — it captures the spec-specific discussion,
decisions, technology choices, outcomes, and constraints surfaced during the
constitution and grilling phases so they are not lost before the spec that needs
them is written. Specs are scoped and clarified when they are actually started.
Foundations: the project [constitution](constitution.md) and the resolved scope
in `docs/research/feature-scope.md`.

Status legend (lifecycle): **undecided** · **needs-info** · **planned** ·
**specced** · **in-progress** · **implemented** · **verified** · **deferred** ·
**abandoned**.

---

## Vision & End States

- **A typed prompt-template library that catches input errors before render, not
  after.** Calling a prompt with missing/mistyped variables is a caught error, not
  a silent empty render — the BAML-equivalent static guarantee no file-based
  library provides. This (typed input + the sound agreement check) is the reason
  the library exists.
- **One prompt, byte-identical across Python, TypeScript, and Rust** — by
  construction (one shared Rust engine), not by per-language test. Go reaches the
  same core later, never via reimplementation.
- **A minimal, orthogonal library that never grows into a framework.** It
  parses/validates/renders text and stamps provenance; it does no I/O, no LLM
  calls, no request-body assembly, no token counting, no output parsing. It stays
  drop-in alongside any call layer.
- **Repo-canonical prompts with content-addressed provenance.** Prompts live
  in-repo, PR-gated; git owns evolution; `template_hash`/`render_hash` give traces
  content identity. Consumer #1 (Bellwether/`claudebroker`) integrates end-to-end.

## Constraints & Decisions

- **C-01 — Shared core, structural parity:** one compiled Rust engine
  (`prompting-press-core`) bound into each language; rendering is byte-identical
  by construction. No per-language reimplementation. _See constitution Principle I._
- **C-02 — FFI isolation:** `pyo3`/`napi` appear only in binding crates, never in
  the kernel or the Rust consumer crate; kernel is binding-agnostic and
  validation-blind. CI-enforced. _Principle II._
- **C-03 — Minimal boundary (non-negotiable):** no I/O, no LLM calls, no
  request-body assembly, no token counting (hook only), no output parsing; the
  user pushes data in. _Principle III._
- **C-04 — Sound agreement check:** template referenced-vars ⊆ declared Vars
  fields, via MiniJinja stable `Template::undeclared_variables(nested=false)` +
  globals allowlist; CI lint, never mutates. Stays sound because
  includes/imports/extends/macros/inheritance are excluded from v1 templates.
  _Verified against MiniJinja 2.21 source. Principle IV._
- **C-05 — Repo-canonical; git owns versioning:** no managed version axis;
  variants are named, caller-selected alternatives (implicit default for one,
  explicit default required for many); provenance carries per-variant
  `template_hash` + `render_hash` (over strings). No `vars_hash`. _Principle V._
- **C-06 — Per-language idiom over forced uniformity:** typed Vars + custom
  validators via Pydantic / Zod / garde 0.23; composition via ordered array (never
  `.chain()`); errors normalized to `[{field, code, message}]`, native error types
  never cross FFI. _Principle VI._
- **C-07 — JSON Schema is the single source of truth:** prompt-definition shapes
  codegen'd per language from one schema; YAML+JSON push input via a dual-input
  loader; conformance corpus guards the FFI boundary + schema round-trip, not
  render parity. _Principle VII._
- **C-08 — Scope discipline (R1):** all five of the original design brief's
  pluggable interfaces (Store, Loader, VariantSelector, ProvenanceSink, type
  system) are eliminated as v1 public seams. No new pluggable interface until a
  second concrete implementation exists. _Constitution §Scope Discipline._
- **C-09 — Var provenance is metadata + lint + opt-in guard, never silent
  mutation:** 3-way tag (`trusted | untrusted | external`); configurable,
  opt-in, additive guard expansion; sanitization/stripping rejected. _Principle
  IV / feature-scope §4.4._
- **C-10 — Release tooling & version axes (for 007):** two distinct version
  axes, kept separate. (a) **Library-package version** — managed by
  **release-please** in monorepo manifest mode with the `linked-versions`
  plugin: ONE unified version across all three published packages
  (crate / wheel / npm), driven by conventional commits. Rationale: C-01 (one
  shared core, byte-identical) makes independent per-package versions
  incoherent. (b) **Prompt-content version** — git-owned and content-addressed
  via `template_hash`/`render_hash` (C-05/Principle V); release tooling MUST NOT
  touch this axis. Artifact build+publish uses **native per-ecosystem tools, not
  a single multi-language releaser**: `cargo publish` (crates.io), maturin /
  maturin-action (PyPI wheels — manylinux/universal2/abi3 for the PyO3 cdylib),
  `@napi-rs/cli` platform-package split (npm native addon). **GoReleaser
  evaluated and rejected** (2026-06-25): its builders emit *binaries* (rust =
  cargo-zigbuild binaries w/ limited workspace support; python = "coming soon";
  node/bun/deno = single-executable apps), but all three of our deliverables are
  libraries/wheels/native-addons — it would contribute only a trivial
  `cargo publish` after-hook and nothing to the hard wheel + napi-prebuild
  paths. Exact tool versions verified at spec-007 time (verify-at-spec-time
  discipline). _Spec-007 governance; does not affect 001._
- **C-11 — Options objects over long/optional positional parameters (per-binding API shape):** a public
  function with optional or >~2 meaningful parameters takes its optional/config tail as a single named
  **options object** (TS/JS) or **keyword-only args** (Python `*, kw=...`) / options struct (Rust), never
  a positional list of optionals. Required positional operands (registry, name, schema+data) stay
  positional. Also kills positional-shape duck-typing (schema-vs-data by `.safeParse` sniff). Codifies
  the constitution **v1.1.0** Principle VI amendment (see `DECISIONS.md` 2026-06-28). **Per-language
  threshold:** TS/JS + Python are strict (any optional → options object / keyword-only); **Rust** keeps a
  **single** `Option<T>` positional (idiomatic, self-documenting) and only needs an options struct at
  **2+** optional params. **Origin:** the spec-005 TS-binding review — `render` couldn't select a variant
  without colliding with `guard`, and composition entries were duck-typed tuples (Long Parameter List +
  Primitive Obsession, refactoring.guru). **Applied:** 005 TS (`render`/`getSource`/`Composition` →
  options objects, `329cd20`); Python binding (`render`/`get_source`/`Composition.append`/`GuardConfig`
  → keyword-only via PyO3 `signature` `*,`). **Rust** (kernel + consumer): no change — below the Rust
  threshold. _Governs all binding specs (004/005 + future); does not change the workflow, only the
  per-language public call shape._

## Planned Specs

### 001 — Foundations: crate layout, JSON Schema, codegen, CI guardrails  [status: implemented]

- **Description:** The project spine — restructure to the load-bearing crate
  layout, define the prompt-definition JSON Schema, build the schema→shape codegen
  pipeline, and wire the constitution's structural invariants into CI.
- **Outcome:** A buildable workspace with `crates/{prompting-press-core,
  prompting-press,prompting-press-py,prompting-press-node}` + `packages/
  {python,typescript}` (reserved `packages/go`); a published prompt-definition
  JSON Schema in `schemas/jsonschema/`; codegen producing Pydantic models, TS
  types, and Rust structs; CI that fails on FFI deps in the kernel/consumer crate
  or stale codegen.
- **Scope (in):** crate/package reorg + moon wiring; the JSON Schema; the codegen
  pipeline (tooling pinned at spec time, not assumed); CI guardrails for C-02 and
  codegen freshness.
- **Scope (out):** any rendering, validation, or agreement logic (later specs).
- **Depends on:** none.
- **Governed by:** C-01, C-02, C-07, C-08.
- **Notes:** The bootstrap scaffold created a flat `packages/{python,typescript,
  go,rust}`; this reorg replaces it. Codegen-tool selection (e.g.
  datamodel-code-generator / json-schema-to-typescript / typify) is an open thread
  — **verify current tooling at spec time, do not assume** (see Open Questions).

### 002 — Engine kernel (`prompting-press-core`)  [status: implemented]

- **Description:** The binding-agnostic, validation-blind Rust engine: MiniJinja
  render path, sound agreement analysis, variant resolution, hashing, and
  var-provenance plumbing.
- **Outcome:** A kernel that, given already-validated values + a prompt
  definition, renders text, reports required root variables, resolves variants,
  emits `template_hash`/`render_hash`, and supports the opt-in guard expansion —
  with no FFI and no typed-Vars knowledge.
- **Scope (in):** MiniJinja integration restricted to interpolation/conditionals/
  loops; render path; `undeclared_variables(nested=false)` + globals allowlist;
  variant resolution (root body is always the default arm; caller-named selection;
  unknown-variant the only resolution error — per C-05); `template_hash` +
  `render_hash`; 3-way provenance plumbing + configurable additive guard
  expansion; small engine-regression render fixtures.
- **Scope (out):** `{% include %}`/`{% import %}`/`{% extends %}`, macros,
  inheritance; any FFI; any typed-Vars validation.
- **Depends on:** 001.
- **Governed by:** C-01, C-03, C-04, C-05, C-09.
- **Notes:** Soundness verified against MiniJinja 2.21 source — the stable API is
  strictly more sound than `jinja2.meta`. Excluding includes is what keeps the
  check airtight with zero `unstable_machinery` dependency. _Implemented (2026-06-26):_
  MiniJinja pinned at `2.21` with `default-features=false` — `macros`/`multi_template`
  OFF is the parse-time exclusion mechanism (excluded tags fail at `add_template`),
  `adjacent_loop_items` kept; re-confirmed against the 2.21.0 source tag (roadmap Q3
  satisfied). Globals allowlist derived dynamically from the kernel's own
  `Environment` (drift-proof). Provenance guard is a separate result field via plain
  `{fields}` substitution (never re-rendered).

### 003 — Rust consumer crate (`prompting-press`)  [status: implemented]

- **Description:** The first full consumer layer over the kernel — proves the
  kernel/consumer split before any FFI.
- **Outcome:** `cargo add prompting-press` gives a typed-Vars facade (garde),
  dual-input loader, the agreement check + provenance lint as CI entry points, and
  ergonomic `render()`/`get_source()` + composition — all over the kernel, no
  rendering logic duplicated.
- **Scope (in):** garde 0.23 Vars facade + custom validators; dual-input loader
  (YAML/JSON or constructed object); `check(registry)` agreement + provenance
  lint; error normalization to the common shape; composition (`Vec` + `append`).
- **Scope (out):** any built-in token counter; ~~token-count hook interface~~
  (DROPPED at the analyze gate — refinement F4; folded into the Deferred "Token
  budgeting / truncation" entry); `.chain()` composition.
- **Depends on:** 002.
- **Governed by:** C-03, C-06, C-07.
- **Notes:** Validation lives here (consumer layer), never in the kernel. garde
  `Report` is wrapped, never leaked.

### 004 — Python binding (`prompting-press-py` → `packages/python`)  [status: implemented]

- **Description:** PyO3 + Pydantic binding — consumer #1's language
  (Bellwether/`claudebroker`).
- **Outcome:** A `pip install prompting-press` package: PyO3 marshaling over the
  kernel (maturin wheel), a Pydantic Vars facade with custom validators, the
  agreement check + provenance lint wired to Pydantic fields, `from_messages`
  composition, and normalized errors as Python exceptions.
- **Scope (in):** PyO3 marshaling (marshaling + Pydantic facade only); Pydantic
  Vars + validators; agreement/lint wiring; dual-input loader; `from_messages`
  composition; error normalization; a Python dependency advisory gate
  (`ci:check-advisories-py`, security review SEC-101).
- **Scope (out):** any rendering/hashing/analysis logic in the binding (C-02);
  ~~token hook~~ (struck — the token surface was dropped in spec 003, refinement
  F4, and deferred to the "Token budgeting / truncation" Deferred entry; never
  re-introduced at the binding layer).
- **Depends on:** 002 (kernel); informed by 003 (consumer pattern).
- **Governed by:** C-02, C-06.
- **Notes:** Marshaling + facade only — zero engine logic (Principle II).

### 005 — TypeScript binding (`prompting-press-node` → `packages/typescript`)  [status: implemented]

- **Description:** napi-rs + Zod binding — proves the *second* binding pattern and
  exercises the FFI seam the conformance corpus targets.
- **Outcome:** An `npm i prompting-press` package: napi-rs marshaling over the
  kernel (platform-binary packaging), a Zod Vars facade with `.refine()`
  validators, agreement/lint wired to Zod, array-literal composition, and
  normalized errors as JS errors.
- **Scope (in):** napi-rs marshaling; Zod Vars + validators; agreement/lint
  wiring; dual-input loader; **options-object** composition entries
  (`{ name, schema?, data, variant? }` — per C-11; was "array-literal/builder");
  error normalization.
- **Scope (out):** any engine logic in the binding; ~~token hook~~ (struck — same
  F4 reason as 004; the token surface is deferred, not a binding concern); a fluent `.chain()` API
  (cannot cross napi; collides with idiom).
- **Depends on:** 002 (kernel); informed by 003/004.
- **Governed by:** C-02, C-06, C-11 (the options-object convention originated in this spec's review).
- **Notes:** Second binding makes the FFI boundary real — surfaces marshaling
  divergences that 006 then locks down.

### 006 — Conformance corpus + cross-language hardening  [status: implemented]

- **Description:** The corpus in its verified-correct scope — FFI-boundary
  marshaling and schema round-trip, **not** render parity (structural via C-01).
- **Outcome:** A CI gate proving that the same logical input through each binding
  yields identical render + identical `template_hash`/`render_hash`, and that
  schema-valid/invalid docs are accepted/rejected identically across languages.
- **Scope (in):** FFI-marshaling fixtures (datetime/Date/chrono, Decimal, nested
  models, null/undefined/None, int-vs-float); schema round-trip fixtures; wiring
  as a CI gate across the three packages.
- **Scope (out):** comprehensive render-parity fixtures (render parity is
  structural; only a small engine-regression set exists, in 002).
- **Depends on:** 004, 005.
- **Governed by:** C-01, C-07.
- **Notes:** Corpus pivoted from "prove renderers match" (now guaranteed) to
  "prove bindings marshal identically + schema round-trips." **Implemented**
  (PR #185, squash `b2092c0`; 21 issues #164–#184 auto-closed): shared
  `conformance/` corpus + 3 thin runners (Rust consumer / Python pytest / TS
  node:test) + a `ci:conformance` gate (the Rust consumer leg also closes a CI gap
  — its tests ran nowhere before). Full Phase-3 QA passed clean; CI green incl. the
  conformance gate. Two implementation decisions captured to `docs/memory/`:
  **D1** (cross-binding type parity uses the *canonical serialized form*, not native
  objects — Pydantic emits `Z`/`1E-17`, JS `Date` emits `.000Z`) and **A1** (the
  binding loaders do serde *shape* validation, not full JSON-Schema — so
  `variant-named-default` is loader-accepted and excluded from the loader round-trip
  set). The int-vs-float case was refined at the critique gate (`1 vs 1.0` was
  JS-unrepresentable → `1` vs fractional `2.5`). Debrief:
  `specs/006-conformance-corpus/roadmap-reviews/debrief-2026-06-28.md` (outcome met,
  scope clean, C-01/C-07 upheld, zero Must-Address).

### 007 — v1 release  [status: planned]

- **Description:** Documentation, packaging, registry reservation, publish, and
  the Bellwether end-to-end integration validation.
- **Outcome:** `prompting-press` published on crates.io, PyPI, and npm under
  Apache-2.0; docs carrying both halves of the tagline (type-safety AND
  press/provenance); Bellwether using it end-to-end (in-repo prompts, provenance
  → its traces, output models referenced).
- **Scope (in):** package metadata (description/keywords/repository/license),
  per-package READMEs, registry-name reservation; license/NOTICE; publish;
  Bellwether integration validation. (The full end-user **docs site** moved to
  spec **010**; the per-package READMEs here stay short + point at it.)
- **Scope (out):** anything in the Deferred/Never lists below; the docs site (010);
  the schema rename + fixture move (008); adversarial hardening (009).
- **Depends on:** 006; **008** (publish the renamed contract, not a renamed-post-publish
  contract); **009** (don't publish an un-hardened library); **010** (docs site live at
  publish). Sequencing: 008 + 009 + 010 land before 007.
- **Governed by:** C-03, C-05, C-10.
- **Notes:** README must carry the type-safety half prominently — the press
  imagery must not bury the actual differentiator (brief R6). **Release tooling
  (C-10):** release-please (manifest mode + `linked-versions`) for the unified
  library-package version + changelogs from conventional commits, paired with
  native build/publish — `cargo publish`, maturin-action (wheels),
  `@napi-rs/cli` (npm prebuilds). GoReleaser evaluated and rejected (binary
  builder, not a library/wheel/native-addon publisher — see C-10). Verify exact
  tool versions when 007 is specced.

### 008 — Pre-publish API & schema reshape  [status: planned]

- **Description:** The pre-publish reshape of the public contract — the LAST chance to change the API
  shape + schema before packages go on the registries (post-publish these are breaking changes). Bundles
  the schema rename, the fixture move, and the **prompt-as-object** API redesign because they are ONE
  decision about the library's core object model (research note:
  `docs/research/registry-value-and-object-model.md`). Resolve the object-model open questions at this
  spec's **clarify** phase.
- **Outcome:** The published v1 contract uses (a) the final field name `origin`; (b) the resolved object
  model (prompt-as-object vs. registry-keyed — clarify decides); (c) the resolved validator-attachment
  shape. CI + conformance stay green throughout. Every example + binding test + the conformance runners
  reflect the final shape.
- **Scope (in):**
  - **Schema rename** — `provenance` → `origin` in `schemas/jsonschema/prompt-definition.schema.json`
    (enum values `trusted|untrusted|external` unchanged) → regenerate the 3 per-language shapes → update
    kernel `ProvenanceView`/check + consumer + both bindings + all fixtures + conformance + docs.
  - **Fixture move** — `schemas/jsonschema/fixtures/{valid,invalid}/` →
    `schemas/jsonschema/tests/fixtures/{valid,invalid}/`; fix the 3 refs (`validate_fixtures.py`, the
    `schemas:validate-fixtures` moon-task inputs, `conformance/schema/manifest.json`).
  - **Prompt-as-object API redesign** (clarify-gated — see open questions) — a first-class `Prompt`
    object across all three bindings; `render`/`getSource`/`check` migrate onto it (single-prompt ops);
    `Composition` aggregates `Prompt` objects; the `Registry` is re-decided (kept-thin / optional sugar +
    `checkAll()` / dropped). Resolves the TS `render` schema-vs-data duck-typing (named `schema` key, no
    `isSchema()`/`.safeParse` sniff) and the `reg`/`name` positional questions.
  - **`validation_required` (candidate, clarify-gated)** — an optional schema boolean letting a prompt
    mandate that a validator was supplied (closes the Python/TS static-render bypass). Tripwire, NOT a
    proof of meaningful validation (an empty Zod/Pydantic model still passes); pairs with `origin:
    untrusted`. Keeps the kernel validation-blind (C-06).
- **Scope (out):** new rendering/agreement/hashing behavior (the kernel is untouched — Principle I); the
  fuzzing/hardening (009); the docs site (010); runtime enforcement of `origin`/`validation_required`
  beyond "a validator ran".
- **Depends on:** 006 (conformance references the fixtures + the field). **Blocks 007, 010** (010
  documents the FINAL API + field name). All-bindings reshape — must precede publish.
- **Governed by:** C-01 (kernel unchanged; cross-binding symmetry — the object model lands in all three),
  C-06 (native validators; errors normalized), C-07 (JSON Schema single source — codegen), C-08 (Scope
  Discipline — the registry keep/drop decision IS this discipline applied to the registry abstraction),
  C-09 (origin tag declarative), C-11 (the options-object/receiver call-shape). Schema + API contract —
  firmly SpecKit-worthy.
- **Notes:** Bundled per the 2026-06-28 design conversation (see the research note). `origin` chosen over
  `provenance` (more descriptive; fits all three values; a boolean can't express the 3rd). Object-model
  lean = prompt-as-object + optional registry (matches the actual data flow — `render` needs one def,
  `check` is per-prompt, `Composition` aggregates prompts; aligns with BAML's method-per-prompt client +
  LangChain's held template objects, neither of which uses name-keyed registry lookup). ~180+ files; do
  as ONE coordinated change, full conformance + CI re-run. **Open questions for clarify** (from the
  research note §6): keep the registry at all? prompt.check() vs reg.checkAll()? accept the single-prompt-
  on-Prompt / multi-prompt-elsewhere split? confirm the all-bindings (incl. Rust generic `render::<V>`)
  appetite? ship `validation_required` now or defer?

### 009 — Adversarial hardening & fuzzing  [status: planned]

- **Description:** A library-hardening test pass: fuzzing + hostile-input + scrub-verification tests in
  the library's OWN suites (not the examples repo), proving the boundary holds under abuse.
- **Outcome:** Each binding's test suite (+ the kernel) gains adversarial coverage that asserts the
  library never panics, always returns a structured error, never leaks bound-value content, and upholds
  its invariants under generated/hostile input — wired into CI.
- **Scope (in):** (a) **robustness fuzzing** — malformed/huge/deeply-nested/Unicode/control-char inputs
  to load/render/check; assert no panic, structured error, no leak; (b) **property-based / generative
  fuzzing** — Rust `proptest`/`arbitrary`, Python `hypothesis`, TS `fast-check` asserting invariants
  (never-panic, validate-before-render, hash-determinism); (c) **injection/guard demo** — injection-
  shaped untrusted input shows `check()` flags the unguarded field + the guard text wraps it, with
  EXPLICIT notes that the guard is advisory text, NOT enforcement (no LLM in the library); (d)
  **secret-scrub verification** — secret-looking values that trigger parse/render errors must never
  appear in the error message/stack (SEC-004 holds end to end).
- **Scope (out):** any "jailbreak the model" claim (the library has no model — Principle III); turning
  the advisory guard into runtime enforcement (that's a future capability spec, if ever); changing the
  boundary.
- **Depends on:** 002–005 (the surfaces under test). Independent of 008's rename (rebase if 008 lands
  first). **Should land before 007** (don't publish un-hardened).
- **Governed by:** C-03 (boundary — fuzzing proves it, doesn't expand it), C-09 (provenance is
  metadata + advisory guard, never silent mutation — the injection demo states this honestly).
- **Notes:** new dev-deps per ecosystem (proptest/hypothesis/fast-check) pinned exact (the
  floating-version gate). Origin: user asked for fuzzing + failure-mode + "break the model" coverage
  2026-06-28; the honest framing (library robustness, not LLM jailbreak) is load-bearing.

### 010 — Documentation site (Astro/Starlight)  [status: planned]

- **Description:** A published end-user documentation site for the library, built with Astro (Starlight
  docs theme), with content derived from the codebase + the JSON Schema + the supported template
  feature set. Subsumes the spec-007 "user docs" + the template-feature-set doc gap.
- **Outcome:** A GitHub-hosted Astro docs site with: overview, getting-started, how-to guides, FAQ, and
  an **API reference** (per-language: Rust/Python/TS) generated/derived from the codebase + schema; the
  repo README is slimmed to overview + getting-started + a pointer to the site. The
  template-engine + supported-feature documentation (MiniJinja; interpolation/conditionals/loops in,
  includes/macros/inheritance out) lives here.
- **Scope (in):** Astro + Starlight site scaffold; hand-written overview/getting-started/howto/FAQ;
  API-reference generation per language (cargo doc / schema-driven shape docs / TS types); the template
  feature-set page; doc the `variables[].type` is carried-metadata-for-codegen (not runtime-enforced)
  and the variant/default shared-variables guarantee; README slim-down + pointer; CI build/deploy of the
  site.
- **Scope (out):** auto-generating prose (the narrative pages are hand-written); a docs *framework*
  beyond Starlight; versioned/multi-release docs (a later concern); hosting choice bikeshedding
  (GitHub Pages assumed unless decided otherwise at specify time).
- **Depends on:** 008 (document the FINAL `origin` field name, not a renamed-after-docs name); informed
  by 002–006 (the API it documents). **Should land before/with 007** (docs live at publish).
- **Governed by:** C-07 (schema is the single source the API ref derives from), C-05 (document the
  git-canonical/provenance model). Doc-only — no library behavior change.
- **Notes:** SUBSTANTIAL deliverable (a real site). Decide at specify time: how much API ref is truly
  auto-generated vs hand-curated, and the hosting/deploy path. Origin: user request 2026-06-28 (Astro
  docs with overview/howto/FAQ/getting-started/API-ref, schema-derived; README points to it). The
  spec-007 "package READMEs + quickstart" scope was narrowed to short per-package READMEs that point
  here.

## Deferred

<!-- Gated on real demand (C-08 / R1). Not planned specs until a trigger fires. -->

- **Go binding** — `[status: deferred]` build via cgo-over-C-ABI or WASM-via-wazero
  against the **same** core, never an independent reimplementation (C-01). Trigger:
  a concrete Go consumer + a solved binding path. `packages/go` placeholder + a
  conformance target reserved.
- **Inline source-partials (`{{> name }}`)** — `[status: deferred]` source-splice
  *before* MiniJinja parses, so the agreement check stays sound on the stable API.
  Additive, non-breaking. Trigger: static-boilerplate fan-out friction proven
  painful.
- **Token budgeting / truncation** — `[status: deferred]` depends on a wired
  `count_tokens` hook; per-vendor tokenizer parity is the hard part. The hook itself
  was scoped into spec 003 then DROPPED at its analyze gate (F4) — a bare seam with no
  consumer added little; the whole token surface (hook + budgeting) waits for a later
  spec where an accurate counter justifies it.
- **`nested=true` strict agreement mode** — `[status: deferred]` verifies deep
  attribute paths; partially duplicates the type system; MiniJinja recovers full
  paths only for trivial chains.
- **Langfuse delivery backend** — `[status: deferred]` push-to-SaaS as *delivery*
  only; repo stays canonical, SaaS never source of truth.
- **Python binding DX follow-ups (from spec 004 review/debrief)** — `[status: deferred]`
  three non-blocking items surfaced in the 004 Phase-3 reviews, none in any 004 FR/SC:
  (a) **TD001** — bound or document `depythonize` recursion depth on the `insert(dict)`
  path (`crates/prompting-press-py`; render/compose are already pydantic-depth-bounded);
  (b) **value-equality** (`__eq__`/`__hash__`) on `RenderResult`/`Finding` — they are
  content-addressed (carry hashes) but currently compare by identity; (c) **`.pyi` type
  stubs** for downstream `mypy`/IDE typing of the compiled extension. Trigger: a real
  consumer need (e.g. Bellwether) or the spec-005 TS binding wanting parity. Apply the
  same pattern to spec 005 (napi) if adopted.
- **Any new pluggable interface** — `[status: deferred]` introduced only when a
  second concrete implementation actually exists to exercise it (C-08).
- **Variable-context render modes (WISHLIST — user-raised 2026-06-27, during spec 004)** —
  `[status: deferred — needs boundary review]` three related ideas for giving the
  downstream agent more structured context about a prompt's variables:
  (a) a **placeholder-preserving render** (template skeleton with `{{ var }}` intact) plus
  a **variable legend** section explaining each variable; (b) a **typed filled-variable
  manifest** (name → declared `type` → `provenance` → value) surfaced for agent context;
  (c) an option (a flag/helper) to return a **final composed output that includes the `guard`
  text** (e.g. guard prepended to the body). All three are *rendering / output-composition*
  behavior, so they are **kernel-level** (Principle I — must preserve cross-language parity),
  NOT binding-level (C-02 forbids engine logic in a binding).
  - **(c) — DECIDED 2026-06-27: NOT building a composed field.** Reasoning: the `guard` is
    already a *separate* field and is semantically a **system-prompt addendum**, while `text` is
    the user-level body. Gluing them (`guard + body`) is the *wrong* split for the common chat
    case (it jams a system instruction into a user turn) and only helps a single-blob/completion
    send. Since the caller routes `guard` → their system prompt and sends `text` as the user
    message with zero library help needed, a `composed` field earns nothing and risks nudging the
    wrong usage. Resolution: **docs-only** — document the guard-as-system-addendum doctrine (in the
    kernel `guard` rustdoc + each binding's quickstart): *single render → route `guard` to the
    system prompt, send `text`; multi-message → place `guard` as its own `system` message*. No
    kernel change, no `composed`, no per-binding helper (a helper would be per-binding, against
    parity). `render_hash = SHA256(text)` (body only, `engine.rs:173`) is noted for the record;
    a future composed feature, if ever wanted, is provenance-safe but currently has no consumer
    justifying it.
  - (a)/(b) are heavier: they brush the Minimal-Boundary line (Principle III: no request-body
    assembly) and may need a constitution amendment.
  **Today, all three are one-liners in the consuming app**: `get_source()` already returns the
  placeholder-intact template; `def.variables` already carries `type`/`provenance`; and
  `result.guard` is a separate field the caller can compose (`f"{r.guard}\n\n{r.text}"`).
  Trigger: a concrete consumer (e.g. Bellwether) finding the caller-side assembly repetitive
  enough to justify a kernel spec.

## Never (boundary defense)

<!-- Out of scope by constitution; each requires a constitution amendment to revisit. -->

- LLM calls · provider request-body assembly · output parsing/coercion · built-in
  token counting · a managed version axis (git owns versioning) · I/O / storage
  adapters · sanitization/stripping of untrusted vars · a SaaS authoring backend
  as source of truth.

## Open Questions

- **Q1 — Codegen tooling (for 001):** which three generators (schema → Pydantic /
  TS types / Rust structs) are current and mutually consistent in 2026? Resolve by
  verifying live tooling at spec time; do not assume. _Surfaced in the grill;
  deferred to spec 001._
- **Q2 — PyO3/napi receiver constraints (for 004/005):** confirmed by research
  that owned-`self` builders can't cross the boundary, so the kernel stays plain
  data — needs a real compile-check at binding time as proof.
- **Q3 — MiniJinja minor-version drift:** we depend only on the stable
  `undeclared_variables` + render path (no `unstable_machinery`), so drift risk is
  bounded; re-confirm on each MiniJinja bump.

## Cross-Cutting Notes

- **The library is variant-*aware*, not variant-*selecting*.** This reverses the
  original brief, which pitched deterministic variant selection as *the*
  differentiator; the grill concluded selection is the caller's experiment
  framework's job. The real differentiator is typed input + the sound agreement
  check.
- **Verification research is banked** (don't redo): cross-language template engines
  (MiniJinja shared-core viable Py/TS/Rust; Go is the break), multi-vendor
  tokenizers (no offline Claude/Gemini tokenizer → hook only), MiniJinja AST
  (`undeclared_variables` stable + sound; includes stop at boundary), Rust
  validation (garde 0.23), fluent-API idioms (array, never chain).
- **Codegen + conformance corpus are CI gates from day one**, not afterthoughts —
  they enforce C-07 and C-01 respectively.

---

**Version**: 1.3.0 | **Ratified**: 2026-06-25 | **Last Amended**: 2026-06-28
