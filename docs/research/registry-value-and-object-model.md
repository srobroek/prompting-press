# Research note — Does the `Registry` earn its keep? (and the prompt-as-object model)

**Status:** design research / decision input — NOT a decision. Feeds the spec-008 clarify phase
(pre-publish API shape). No code changed by this note.
**Date:** 2026-06-28
**Origin:** a design conversation while building the examples repo. The user pushed on a chain of
questions: why is `render`/`check` keyed off a `Registry` by name? shouldn't `render` live on the
*prompt*? what is the registry even worth over a hashmap? — culminating in a candidate
**prompt-as-object** redesign of the public API. This note captures the reasoning so it is not lost,
and researches the core question: **what value does the `Registry` add over users just creating and
holding `Prompt` objects themselves?**

---

## 1. What exists today (verified against the code)

- **"A prompt" is `PromptDefinition`** — a *pure data struct* codegen'd from the JSON Schema (C-07).
  It has fields (`name`, `role`, `body`, `variables`, `variants`, `meta`, …) and **no behavior**.
  There is **no `Prompt` type** in the codebase at all.
- **`Registry`** = `BTreeMap<String, PromptDefinition>` + `{ new, insert, get, iter, load_yaml,
  load_json }`. Duplicate name → silent overwrite (no dedup error).
- **All operations are free functions keyed by name against the registry:**
  - `render(reg, name, vars, variant, guard)` — internally just `reg.get(name)` then render that one def.
  - `get_source(reg, name, variant)` — `reg.get(name)` then look up the source.
  - `check(reg)` — `for (name, def) in reg.iter() { check_agreement(def); check_provenance(def); }`.
  - `Composition::resolve(reg)` — for each entry, `reg.get(entry.name)` then render.
- **Verified facts that drive the analysis:**
  - `render`/`get_source` use the registry for **one thing only**: `reg.get(name)` to fetch the single
    def. Everything else operates on that def.
  - `check` has **zero cross-prompt state** — it is per-prompt logic (`check_agreement` +
    `check_provenance`, both calling the kernel's `required_roots` / `provenance_view`) run in a loop.
    The *iteration* is trivial; the *per-prompt analysis* is the real, non-DIY-able value (sound
    MiniJinja AST analysis — Principle IV).
  - `Composition::resolve` uses the registry only for `reg.get(entry.name)`; it then renders each and
    emits role-tagged `Message { role, text }` in order.

**Net:** every operation's use of the registry is **name → def lookup**. The registry is a
name-keyed collection + a dual-input loader (+ sorted iteration for `check`'s deterministic output).

---

## 2. The candidate redesign: prompt-as-object

The user's instinct, worked through to a coherent end-state:

| Concept | Role |
|---|---|
| **`Prompt`** | the object. Created via `loadPrompt(yaml/json)` or constructed. `render({schema?,data,variant?,guard?})`, `getSource({variant?})`, `check()` live ON it (single-prompt ops). |
| **`Composition`** | aggregates `Prompt` **objects** (+ vars/variant) → renders to ordered `[{role,text}]`. No registry needed (it holds the objects). |
| **`Registry`** | OPTIONAL — a named collection for managing many prompts + a `checkAll()` fan-out. Not on the critical path of any operation. |

Why this is attractive:
- It matches **what the code already does** — `render` needs one def, not the registry; `check` is
  per-prompt; `Composition` aggregates prompts. The object model would mirror the actual data flow.
- It **dissolves** several smaller smells discussed the same session: the `reg`/`name` positional
  placement question, and the TS `render` schema-vs-data **duck-typing** (`isSchema()` `.safeParse`
  sniff — which C-11 was partly meant to kill but which the schema-vs-data overload still relies on).
  `prompt.render({ schema?, data, … })` has a named `schema` key — no sniff, no positional ambiguity.
- It is **more idiomatic** in all three ecosystems (receiver-style; discoverable via autocomplete).

The split it implies (and the user should confirm they accept it): **single-prompt ops on the
`Prompt`; genuinely-multi-prompt ops elsewhere** (`Composition` over objects; `reg.checkAll()` as a
fan-out). That is correct domain modeling — "the operation lives where its data lives" — but it is no
longer "one uniform call shape."

---

## 3. The core research question: what is the registry worth?

### 3a. What the registry genuinely adds over a raw hashmap (honest accounting)

1. **The dual-input loader** (`load_yaml`/`load_json`) — the *real* value, but it is **loader value,
   not collection value**: it parses text → `PromptDefinition` through the shared serde path, giving
   YAML/JSON parity + shape-validation (malformed → structured `LoadError`, no partial load). A bare
   hashmap user cannot get this from `map[name] = …`. **But it does not require a registry** — it can
   be a free function `loadPrompt(text) -> Prompt`.
2. **Deterministic (sorted) iteration** — `BTreeMap` makes `check`'s output order-stable and
   identical across languages (a `dict`/`Map`/`HashMap` would not). Real — **but only matters because
   `check` iterates it**; in the object model it matters only for a `checkAll()` aggregator's ordering.
3. **The name-key invariant** — `insert` always keys by `def.name`, so you cannot file a prompt under
   the wrong key. Mild bug-prevention vs. `map["typo"] = greetingDef`.

**Conclusion:** strip the loader out as a free function and the residual `Registry` is "a hashmap with
two small conveniences (sorted iteration, name-key rule)."

### 3b. How comparable libraries model this (researched 2026-06-28)

- **BAML** (the library this project explicitly benchmarks — feature-scope §3, §10) — fetched from
  its docs: invocation is through a **generated client object with one typed method per function**:
  `from baml_client import b; b.UseTool(...)` (same shape in TS/Ruby). **No name-keyed registry
  lookup; no string-addressed collection.** Each prompt/function is a typed member, not a `get("name")`.
- **LangChain** (established knowledge; doc fetch redirected/404'd, not re-confirmed this session):
  a single prompt is a **`PromptTemplate` / `ChatPromptTemplate` object** the user creates and then
  `.format()` / `.invoke()`s **directly**; there is **no central name-registry** — users hold the
  template objects themselves (or in their own dict/variables). Treat as widely-known-but-unverified.

**Signal:** neither comparable library uses a string-name-keyed registry as the primary access path.
Both are object-centric (BAML: generated methods; LangChain: held template objects). This supports the
prompt-as-object direction and *weakens* the registry-as-core-abstraction case.

### 3c. What the project's own prior design already said

`docs/research/feature-scope.md` already:
- **Dropped the brief's `PromptStore` seam** (§5, §7, §11/G7 — push model, no I/O, eliminated).
- **Superseded the brief's "thin in-repo registry" framing** (§10) — but **never re-justified
  *keeping* a registry** vs. plain prompt objects. The registry survived as the implicit "home for
  pushed-in prompts," not as a deliberately-argued abstraction.
- Eliminated all five original pluggable interfaces under **Scope Discipline (C-08/R1)** — "no seam
  until a second concrete consumer needs it." The user's question is the same discipline applied one
  level deeper: *is the registry itself an unearned abstraction?*

---

## 4. Options (for spec-008 clarify)

- **(A) Keep the registry, honestly thin.** Named collection + dual-input loader + `check`-all +
  name-key invariant. Accept it is hashmap-ish; its value is "the one obvious place to put + load +
  lint prompts." Lowest churn; current shape mostly stands.
- **(B) Prompt-as-object; registry optional.** `loadPrompt(yaml) -> Prompt`; `prompt.render(...)` /
  `getSource` / `check` on the object; `Composition` aggregates objects; the registry becomes optional
  sugar for managing many (+ `checkAll()`). Usable with **zero** registry. Most aligned with the
  user's instincts and with BAML/LangChain. **Largest change** — a new `Prompt` wrapper type in all
  three bindings, methods migrate onto it, missing-prompt handling moves, touches every example +
  binding test + the conformance runners.
- **(C) Make the registry earn its keep.** Give it value a hashmap cannot: error (not overwrite) on
  duplicate names; enforce `validation_required` at the boundary; namespacing; a single audited
  load+check entry point. Heaviest; only worth it if the registry should be an opinionated component.

---

## 5. Linked decisions (same session, same spec-008 clarify)

These are not independent — they are one decision about the library's core object model:
- **Prompt-as-object** (§2/§4) — the spine.
- **`provenance` → `origin`** field rename (already roadmap spec 008).
- **`validation_required: true`** — a schema boolean letting a prompt mandate a validator (closes the
  Python/TS static-render bypass). Enforces *a validator was supplied*, NOT *that it checks anything*
  (an empty Zod/Pydantic model still passes) — a tripwire, not a proof. Pairs naturally with
  `origin: untrusted`. Honors C-06 (native validators), keeps the kernel validation-blind.
- **TS `render` duck-typing** — resolved for free by the named `schema` key in `prompt.render({...})`.

## 6. Open questions to resolve at clarify

1. **Is the registry kept at all in the v1 API** (optional sugar, Option B) **or dropped** in favor of
   "hold `Prompt` objects + a `loadPrompt` function"? This is the highest-leverage v1 API decision.
2. If kept: is it Option A (thin) or Option C (opinionated)?
3. Does the user accept the **single-prompt-ops-on-`Prompt` / multi-prompt-ops-elsewhere** split?
4. Cross-binding symmetry: prompt-as-object must be done in **all three** bindings (C-01 "feels like
   the same library"), including Rust (a `Prompt` handle wrapping the generated `PromptDefinition`,
   with a generic `render::<V>`). Confirm appetite for the all-bindings reshape pre-publish.
5. Does `check` move to `prompt.check()` (per-prompt) + `reg.checkAll()` (aggregator), or stay a
   registry-level free function?

## 7. Recommendation

Lean **(B) prompt-as-object with an optional registry** — it matches the actual data flow, dissolves
the duck-typing + reg/name smells, aligns with BAML/LangChain, and is the more idiomatic v1 surface.
But it is a foundational, all-bindings reshape that MUST be settled before v1 publish (007) and decided
in a spec, not ad hoc. Fold it into spec 008's scope (already opening the schema + binding API) or give
it its own spec; either way, resolve §6's open questions at clarify with this note as input.

---

## 8. Design-pattern catalogue research (refactoring.guru — full GoF sweep, 2026-06-28)

Researched the **entire** GoF catalogue (22 patterns across creational/structural/behavioral) via three
parallel subagents, each fetching the refactoring.guru pages live, quoting Intent + Applicability
verbatim, and reporting per-URL fetch status (integrity-guarded; all fetches succeeded). The three
load-bearing verdicts were then **re-verified against the code main-thread**: Composition is
`entries: Vec<Entry>` (flat); `render<V: Serialize + Validate>` invokes an injected validator; a
`Variant` carries only `body`+`meta`. The pattern fit, by design concern:

### 8a. Construction of an immutable `Prompt`
- **Verdict: a validating static factory / "smart constructor" — NOT a GoF creational pattern.**
- **Factory Method / Abstract Factory: POOR** — both vary the *product type* via subclasses/families
  ("alter the type of objects", "families of related products"); we have ONE `Prompt` type. Abstract
  Factory's "future extensibility … unknown beforehand" rationale also collides with Scope Discipline
  (C-08).
- **Builder: PARTIAL, and only the lightweight half** — its "get rid of a telescoping constructor" +
  "doesn't expose the unfinished product … prevents the client code from fetching an incomplete result"
  applicability *does* fire for an ~8-field object whose construction must validate-then-freeze. But the
  full Director+multi-Builder apparatus (for "different representations … stone and wooden houses") is
  OVERKILL — one product type, no director. **Decision: do NOT emit a Builder** (incl. Rust — see 8e);
  use a single validating terminal constructor. (refactoring.guru's own Rust Builder example
  `.expect()`-panics on `build()` — the wrong shape for our no-panic consumer.)
- **Singleton: POOR (anti-fit)** — we create *many* immutable prompts; its "single instance / a single
  database object" use case is the opposite + I/O-coupled (Principle III).

### 8b. The two creation paths (load-from-text vs construct-from-object)
- **Verdict: NO GoF pattern models this** — they vary product *type* (Factory), *assembly* (Builder),
  *copying* (Prototype), or *instance count* (Singleton); none addresses "same product, two input
  *formats*." Correct shape: **two named static factories** (`Prompt.fromYaml` / `Prompt.fromJson` /
  `Prompt.fromObject`) over the one internal validating constructor. (Explicitly *not* GoF Factory
  Method, which is about subclassing.)

### 8c. Immutable derive (clone-with-changes; e.g. "add a variant to an existing prompt")
- **Verdict: Prototype-flavored "copy-with-changes" (PARTIAL — right intent, GoF rationale doesn't fire).**
  Prototype's intent ("copy existing objects") is the only one aimed at derive; the value is the
  copy-with-changes *semantics* (`with_variant(...) -> Result<Prompt>` returns a NEW validated, frozen
  object), not GoF's decoupling-from-unknown-classes rationale.
- **Memento: POOR (misfit)** — it is save/restore/undo of prior state; immutable derive creates a *new*
  value and never restores an old one. Functional copy-on-write, not Memento.
- **Scope note (C-08):** derive may be *deferred* — "variants declared at construction" covers the common
  case; build the `with_*` derive API only when a real consumer needs to transform a prompt it didn't
  author.

### 8d. Structural relationships
- **`Prompt` wraps the codegen'd shape → FACADE (primary).** Facade = "a limited but straightforward
  interface to a complex subsystem." The `Prompt` exposes a small controlled surface (render/getSource/
  check + validated construction) over the generated shape + render/validate machinery. Rivals fail on
  their own applicability: **Adapter** needs *incompatible* interfaces (the same-language generated shape
  isn't incompatible); **Proxy** needs a transparent same-interface placeholder (Prompt exposes a
  *different* interface and IS the real object); **Decorator** needs runtime-stacked behavior preserving
  the wrapped interface (neither holds). → Facade.
- **Binding (PyO3/napi) over the core → ADAPTER (primary), Facade secondary.** Adapter = "objects with
  incompatible interfaces to collaborate" / "translator between your code and a … class with a weird
  interface" — exactly FFI marshaling + error normalization across the boundary. (Bridge is a defensible
  *architecture-level* label for the facade↔core split, but the "switch implementations at runtime" motive
  is weak — the core is never swapped.)
- **`Composition` → NOT Composite; a plain ordered aggregation.** Composite is *explicitly tree-shaped*
  ("nested recursive object structure that resembles a tree", leaf/container sharing one interface).
  Composition is a flat `Vec<Entry>` resolving to `[{role,text}]`, no recursion, and it does **not** share
  the `Prompt` interface — so even "treat simple/complex uniformly" fails. Naming it "Composite" is a
  misfit driven by the shared word "compose." It's an aggregation (a small builder producing an ordered
  sequence of objects). **Flyweight: POOR/overkill** — its applicability self-gates to "**only** when … a
  huge number of objects … barely fit into available RAM"; immutable prompts are shareable for free.

### 8e. Behavioral relationships
- **Caller-supplied validator → STRATEGY (STRONG — the one genuine behavioral fit).** "A family of
  algorithms … interchangeable" (garde/Pydantic/Zod), invoked at the render boundary, isolating the
  validation-blind engine from the validation algorithm. Confirmed by the `V: Validate` injected bound.
- **Variants → NOT Strategy, NOT State (both POOR).** Both swap *behavior/algorithm*; a variant swaps the
  *template body string* — DATA, same render algorithm. (Confirmed: `Variant` = `body`+`meta` only.) State
  additionally needs transitions variants never undergo. Variant selection is **data selection**.
- **`check()` walk → NOT Visitor (the headline misfit); Iterator only as a plain loop.** Visitor needs
  double-dispatch over a *heterogeneous class hierarchy* ("a behavior … in some classes … not others");
  `check()` applies fixed analyses over ONE homogeneous node type — no dispatch. (The heterogeneous AST
  Visitor lives *inside* MiniJinja, not in our `check()`.) It's a native loop applying two fixed analyses;
  name no pattern.
- **Render pipeline (validate→marshal→render→hash) → NOT Template Method, NOT Chain of Responsibility.**
  Fixed unconditional sequence: CoR needs unknown/runtime-variable handler sets that choose to pass-or-
  handle; Template Method's load-bearing mechanism is subclass step-override, which R1 forbids (the only
  pluggable step, the validator, is already Strategy).
- **Command / Observer / Mediator → POOR** — no operation reification/queuing/undo, no events/subscribers
  (Principle V rejects a telemetry sink), no object mesh. Directly contradicted by the minimal boundary.

### 8f. Consolidated pattern map (the resolved vocabulary for the spec)

| Concern | Pattern | Fit |
|---|---|---|
| Build an immutable `Prompt` | validating **static factory / smart constructor** (returns `Result`/throws, never panics) | the shape — NOT GoF Builder/Factory-Method |
| Two input formats (text vs object) | **named static factories** over one validating constructor | no GoF pattern |
| Derive a modified prompt / add a variant | **Prototype-flavored copy-with-changes** (`with_*` → new validated `Prompt`) | PARTIAL; defer per C-08 |
| `Prompt` wraps the codegen'd shape | **Facade** | STRONG |
| Binding over the shared core | **Adapter** (Facade secondary) | STRONG |
| `Composition` aggregates prompts | **plain ordered aggregation** | NOT Composite |
| Pluggable validator (garde/Pydantic/Zod) | **Strategy** | STRONG |
| Variant selection | **data selection** | NOT Strategy/State |
| `check()` traversal | **native loop** + two fixed analyses | NOT Visitor |

**Headline:** the design is deliberately a narrow input→output transform (Principle III), so most of GoF
doesn't apply — and the few that do (Facade for the wrapper, Adapter for the binding, Strategy for the
validator) are *descriptive labels for what already exists*, not new machinery to add. The construction
shape is a **validating static factory yielding an immutable `Prompt`**, with named per-format factories
and a generic immutable-derive (`with`). This is the pattern foundation for spec-008 clarify.

---

## 9. RESOLVED object model (user decisions, 2026-06-28)

The user resolved the core object-model questions during the design conversation. These are **decisions**
(not open questions) for spec 008 to implement; the remaining open items are noted at the end.

### 9a. Prompts are immutable; one generic copy-with-overlay derive — no `withVariant`, no setters

- **`Prompt` is immutable.** No in-place mutation, no public setters, ever.
- **The ONLY way to "change" a prompt is `prompt.with(overlay) -> Result<Prompt>`** — a generic
  copy-with-overlay (the generalized Prototype-flavored derive from §8c). It takes the original + a
  **partial overlay of ANY field(s)**, produces the merged definition, runs the **same validating
  constructor** over the merged result, and returns a **NEW immutable `Prompt`** (or an error). **The
  original is never touched.** `with` REPLACES `withVariant` and every other per-field mutator — "add a
  variant" is just `prompt.with({ variants: { ...prompt.variants, terse: {body} } })`.
- **This is a CORE primitive, not deferred.** (Supersedes §8c's C-08 "defer derive" caveat — since `with`
  is the *only* way to vary a prompt, it is in-scope for 008, not speculative.)
- **Mental model:** a prompt is a *template you stamp variations from* — `fromObject`/`fromYaml` to create,
  `with` to derive variations, all immutable.

### 9b. Merge semantics: SHALLOW replace per top-level field (DECIDED)

- An overlay field **replaces that whole top-level field** (no deep/recursive merge). `with({ variants:
  {...} })` replaces the entire `variants` map; `with({ body: "..." })` replaces `body`; etc.
- **Rationale:** predictable; matches `dataclasses.replace`/Pydantic `model_copy(update=)` (Python),
  object spread (TS), struct-update `..base` (Rust); and it sidesteps deep-merge's delete-expressibility
  problem (a merge can't express "remove variant X" without a sentinel).
- **Ergonomics for "add one item":** the caller spreads the current value themselves —
  `prompt.with({ variants: { ...prompt.getVariants(), terse: {body} } })`. This **requires read
  accessors** (see 9d).

### 9c. The overlay MAY change `name` (DECIDED)

- `with({ name: "other", ... })` is allowed → yields a new, differently-named prompt (a distinct
  identity, consistent with the "template you stamp from" framing). The derived prompt is a *new* prompt,
  not "the same prompt renamed."

### 9d. Read accessors (implied by 9b)

- `Prompt` exposes **read-only accessors** for its fields (e.g. `getVariants()`/`.variables`/`.body` per
  language idiom) so callers can read the current value to build a spread overlay. Read accessors + `with`
  + immutability = a complete value-object surface with **no setters anywhere**.

### 9e. Validation invariant (load-bearing)

- `with` validates the **whole merged object**, not the overlay alone — so overlaying a `body` that
  references an undeclared variable fails, even though the overlay looked fine in isolation. This is
  exactly why `with` routes through the **one validating constructor**, never per-field logic. (Pairs with
  §8a: construction enforces every *decidable* invariant; the un-analyzable-template residue is still
  reported by `check()`, which can be run after construction.)

### 9f. Still open for 008 clarify (NOT resolved here)

- Keep an **optional registry** (name→Prompt, for data-driven name resolution + `checkAll()`) or drop it
  entirely? (§4 Option B vs. fully dropping — the user leans toward prompts-as-objects; the registry's
  only irreplaceable feature is string-name resolution, needed only if Composition/flows address prompts
  by name. Composition holding objects removes that need — see §5 + the "Composition holds objects"
  conclusion.)
- TS shape: switch codegen `interface` → **Zod schema** (for runtime enforcement) so the TS validating
  constructor has something to enforce against (Python has Pydantic; Rust the struct + hand-written
  checks). Strong lean yes; confirm at clarify.
- `validation_required: true` schema field — ship in 008 or defer?
- Confirm the all-bindings appetite (Rust `Prompt` wrapping the generated struct with a generic
  `with`/`render::<V>`; same shape in Python/TS).
