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
