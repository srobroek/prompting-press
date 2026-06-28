# Phase 0 Research — 006 Conformance corpus

All decisions below were derived by **reading the as-built source directly** (the project's
systemic-subagent-fabrication guard), not via a research subagent. Each `D` resolves a Technical-Context
unknown so Phase 1 design has no open questions.

---

## D1 — Canonical serialized forms for date and Decimal

**Decision**: Date → **ISO-8601 string** (e.g. `"2026-06-28T12:00:00+00:00"`, or a date-only
`"2026-06-28"` for a `date`). Decimal → **decimal-as-string** (e.g. `"3.14159265358979"`). Each marshaling
fixture pins this serialized form as the value the kernel sees; each runner constructs its native type and
asserts it marshals to that form.

**Rationale**: Reading `crates/prompting-press-py/src/marshal.rs` confirms the Python path is
`pythonize::depythonize → serde_json::Value → minijinja::Value::from_serialize`, and the module doc states
Pydantic validation runs in `mode="json"`, which **already stringifies `datetime`/`Decimal`
deterministically** before `to_kernel_value` is reached. So on the Python side a `datetime`/`Decimal`
*already* becomes a string in the kernel value. Reading `crates/prompting-press-node/src/marshal.rs`
confirms the Node path is napi `serde-json` → `from_serialize` with **no `Date`-specific handling** — a JS
`Date` is serialized by napi's serde conversion. For the two bindings to converge on one kernel value (and
therefore one render + one hash), the corpus must define the value at the **serialized string form** both
arrive at, and have each runner feed/produce that form. JS has no native decimal type at all, so a string
is the only cross-language-representable decimal (clarified Q3). This is not a weakening: it is precisely
the contract that makes "no binding silently reformats a high-precision number / a date" testable.

**Alternatives considered**:
- *Pass native date/Decimal objects and compare whatever each produces* — rejected: there is no single
  expected value to pin (each binding's serializer could differ), and JS has no Decimal at all.
- *A JSON number for Decimal* — rejected: floats lose precision and reintroduce the int-vs-float ambiguity
  the corpus separately tests; a string is exact.

**Hardening note**: if the TS runner's `Date`→napi-serde output does NOT match Python's `mode="json"` ISO
string, that is a **real divergence the corpus is built to catch** — the runner constructs the `Date` and
passes the ISO string form the fixture pins (a binding that can't reach it fails loudly, which is the
intended finding, per spec FR-006).

**int-vs-float scope limit (critique E1)**: the int-vs-float case tests only distinctions JavaScript can
represent. JS has a single IEEE-754 number type, so `1.0` is indistinguishable from `1`
(`JSON.stringify(1.0) === "1"`; confirmed both by the `prompting-press-node` `marshal.rs` test comment —
"there is no int-vs-float JS distinction to preserve" — and by direct execution), and the napi bridge
reads an integral JS number as an integer. The fixture therefore pairs an **integer `1`** (renders `1`)
with a **fractional float `2.5`** (renders `2.5`) — both representable and distinct in all three
languages. The `1.0`-vs-`1` distinction is **excluded** as a JS-unrepresentable boundary (spec Edge
Cases / the feature-gap escape hatch), not a marshaling defect to flag.

---

## D2 — Marshaling fixture file schema (per-field `type` tag)

**Decision**: each marshaling fixture is a JSON file with this shape:

```jsonc
{
  "case": "date",                       // fixture id
  "definition": { /* spec-001 prompt definition: name, role, body, variables, … */ },
  "variant": null,                       // optional variant selector
  "input": {                             // the logical Vars, each field tagged with its logical type
    "when": { "type": "datetime", "value": "2026-06-28T12:00:00+00:00" }
  },
  "expected": {
    "text": "…rendered output…",
    "template_hash": "<64-hex>",
    "render_hash": "<64-hex>"
  }
}
```

Each runner reads `input`, and for each field constructs the **native type named by `type`** carrying
`value` (e.g. `type:"datetime"` → Python `datetime.fromisoformat`, JS `new Date(...)`, Rust
`chrono::DateTime::parse_from_rfc3339`), then drives its binding's real render path with the constructed
Vars. Plain types (`string`/`int`/`float`/`bool`/`null`/`object`/`array`) carry their JSON value directly.

**Rationale**: the whole point is to exercise each binding's marshaling of a **native** date/Decimal, so
the fixture must tell the runner which native type to build — a bare JSON value would only ever test
JSON-native marshaling (the weaker "JSON-native only" option the spec rejected). The `type` tag is the
minimum machinery that lets one shared fixture drive three different native constructions. A small, fixed
tag vocabulary keeps each runner's constructor a simple match.

**Alternatives considered**: *infer the type from the JSON shape* — rejected: a date and a string are both
JSON strings; inference can't distinguish them. *Separate per-language fixtures* — rejected at clarify
(Q1: drift). 

---

## D3 — Golden authoring & the cross-check mechanism

**Decision**: the `expected.{text, template_hash, render_hash}` golden values are **generated by a small
committed script** (run on demand, not in CI) that renders each fixture through the **Rust consumer** (the
reference binding) and writes the goldens back into the fixture files. Each runner then asserts its
binding's output **equals the committed golden**. Cross-binding parity is therefore transitive: all three
assert equality to the one committed value ⇒ all three agree. A divergence in any binding fails its
runner; a lockstep kernel change that moves all three is caught because the committed golden no longer
matches (until regenerated, which is a reviewable diff). The golden set stays bounded to the named hard
cases (FR-016 — never a comprehensive render set).

**Rationale**: live cross-process comparison (one runner spawning the other two languages) is brittle,
slow, and couples the three toolchains in one process. A committed golden gives the same parity guarantee
(transitively) while keeping each runner single-language and deterministic, and the golden doubles as the
regression tripwire (clarified Q2). Generating goldens from the reference Rust binding (not hand-typing
64-hex SHAs) keeps them correct and regenerable; the generator is the same render path the corpus tests,
so it cannot encode a different rendering than the kernel produces (FR-019).

**Alternatives considered**:
- *Hand-author the hashes* — rejected: 64-hex SHA-256 values can't be authored by hand; error-prone.
- *Pure cross-check, no golden* — rejected at clarify (Q2): misses a lockstep kernel regression.
- *A CI step that regenerates and diffs* — deferred: a freshness-gate over goldens is possible but adds a
  gate; for v1 the generator is a documented manual step (quickstart) and the goldens are reviewed in PR.
  (Recorded as a possible follow-up, not built — Scope Discipline.)

---

## D4 — The Rust passthrough-Vars newtype

**Decision**: a **test-only** newtype in `crates/prompting-press/tests/conformance.rs`:

```rust
struct RawVars(serde_json::Value);
impl serde::Serialize for RawVars { /* delegate to self.0 */ }
impl garde::Validate for RawVars { type Context = (); fn validate_into(&self, _: &Self::Context, _: &mut garde::Path, _: &mut garde::Report) {} }
```

so the runner can call `prompting_press::render(&reg, name, &RawVars(value), variant, &guard)` for any
fixture's already-`serde_json`-decoded input.

**Rationale**: reading `crates/prompting-press/src/render.rs` confirms `render<V>` requires `V: Serialize +
Validate, V::Context: Default`. The corpus is **data-driven** (it holds a `serde_json::Value`, not a
compile-time struct), so it needs a `Serialize + Validate` wrapper. A no-op `Validate` is **correct** for
the corpus: validation is a higher-binding concern (the corpus tests *marshaling*, not validation), and
the input is already a well-formed value. This adds **zero engine logic** (C-02) — `Serialize` delegates,
`Validate` is empty — and lives only in the test file, never in the shipped consumer. `from_serialize` over
the delegated value reproduces exactly the kernel value the Python/TS bindings reach via their serde hops.

**Alternatives considered**: *use the kernel `render` directly from the Rust runner* (the kernel takes a
`minijinja::Value`, no garde) — viable, but the consumer's `render` is the **public Rust binding** the
corpus is meant to exercise for parity (FR-007 — drive the real path), so the runner uses the consumer and
pays the one-newtype cost.

---

## D5 — CI gate topology + local reproducibility

**Decision**: add a dedicated **`ci:conformance` moon task** (`scripts/ci/conformance.sh`) that runs all
three runners: `cargo test -p prompting-press --test conformance`, the Python runner (via the existing
extension build path used by `test-python`), and the TS runner (via the existing addon build path used by
`test-node`). Wire it as a step in CI. The Python and TS runners ALSO live in their binding's existing test
dir, so `ci:test-python` / `ci:test-node` pick them up too; the dedicated task guarantees the **Rust** leg
runs (the found gap) and gives one locally-reproducible command (`mise exec -- moon run ci:conformance`).

**Rationale**: a single `conformance` task makes the gate a named, locally-runnable unit (FR-013) and
unambiguously includes the Rust consumer (FR-015), which `build`/`test-*` don't cover today. Folding the
Python/TS runners into their existing suites costs nothing extra and avoids rebuilding the extensions
twice; the dedicated task orchestrates and adds the Rust leg. Mirrors the established
`scripts/ci/*.sh` + `ci/moon.yml` + `ci.yml` pattern (spec-001/004/005).

**Alternatives considered**: *fold entirely into `test-python`/`test-node`* — rejected: leaves the Rust
consumer unexercised (FR-015 unmet). *A separate GitHub job* — viable; decide at task time whether
`ci:conformance` is its own `ci.yml` job or a step in an existing one (both satisfy the FRs; a job gives a
clearer red/green signal). Recorded as a task-level wiring choice.

---

## D6 — YAML twin for the schema round-trip

**Decision**: the schema round-trip reuses the existing JSON fixtures in
`schemas/jsonschema/fixtures/{valid,invalid}/` via a `conformance/schema/manifest.json` mapping each to
its expected verdict, **and** adds a **minimal YAML twin** of one valid and one invalid document so each
binding's `load_yaml` path is exercised alongside `load_json` (FR-011). The manifest records, per fixture,
the file, the input form (json/yaml), and the expected accept/reject.

**Rationale**: the existing fixtures are JSON (`not-json.txt` is the one malformed-text case). FR-011
requires confirming each binding routes YAML through the shared loader and agrees with JSON on the same
logical document. A single YAML twin per verdict class is enough to prove the `load_yaml` entry point is
wired and agrees — YAML↔JSON parity itself is structural (the consumer's one serde loader), so the corpus
pins routing, not the parser. Keeping it to one twin per class avoids forking a parallel fixture set
(FR-003 — build on, don't fork).

**Alternatives considered**: *YAML-twin every fixture* — rejected: redundant (parity is structural); one
twin per verdict class proves the routing. *No YAML at all* — rejected: FR-011 explicitly wants the
YAML path covered where bindings accept both.

---

## Consolidated decisions

| # | Decision | Rationale (short) |
|---|---|---|
| D1 | Date = ISO-8601 string; Decimal = decimal-as-string canonical forms | Python `mode="json"` already stringifies; JS has no Decimal; one string both converge on |
| D2 | Marshaling fixture = definition + `type`-tagged input + golden expected | Runner needs the `type` tag to build the right NATIVE value; bare JSON tests only JSON-native |
| D3 | Goldens generated from the Rust reference binding; runners assert equality (transitive cross-check + regression tripwire) | Deterministic, single-language runners; golden also trips lockstep kernel drift |
| D4 | Test-only `RawVars(serde_json::Value)`: Serialize-delegate + no-op garde Validate | `render<V>` needs `Serialize+Validate`; corpus is data-driven; no-op validate is correct + zero engine logic |
| D5 | Dedicated `ci:conformance` moon task running all three runners (Rust leg closes a CI gap) | One locally-runnable gate; guarantees the Rust consumer participates (FR-015) |
| D6 | Reuse JSON schema fixtures via a manifest + one YAML twin per verdict class | FR-011 wants the YAML path; parity is structural so one twin proves routing without forking fixtures |

**All NEEDS CLARIFICATION resolved.** No open unknowns block Phase 1.
