### 2026-06-28 — Cross-binding type parity is tested via canonical serialized form, not native objects

**ID**: D1

**Status**
Active

**Why this is durable**
Any time a value crosses the FFI boundary in more than one binding, the question "do the bindings marshal
the *same* logical value identically?" recurs. This decision is the reusable answer for types that have no
single cross-language representation (dates, decimals). It applies directly to the deferred Go binding and
to any future logical type the conformance corpus (spec 006) adds.

**Decision**
For a marshaling-parity fixture, pin the value by its **canonical serialized form** (the string the kernel
sees — e.g. ISO-8601 for a date, decimal-as-string for a high-precision number), and have each binding's
runner feed THAT string. Do **not** construct a native `datetime`/`Date`/`Decimal` object and expect it to
reproduce the golden.

**Evidence**
Discovered empirically during spec 006 (both the Python and TS runner agents hit it independently):
- Python `datetime.fromisoformat("…+00:00")` → Pydantic `model_dump(mode="json")` emits `…Z` (UTC
  canonicalized to a `Z` suffix), not `+00:00`.
- Python `Decimal("0.00000000000000001")` → Pydantic emits `1E-17` (scientific), not fixed-point.
- JS `new Date("…+00:00").toISOString()` → `….000Z` (millis + `Z`).
None of these reproduce the Rust reference golden byte-for-byte. A JSON string, by contrast, dumps
unchanged through every binding's serde/JSON hop, so all three converge.

**Tradeoffs**
- Gained: a meaningful, byte-exact cross-binding parity assertion for date/decimal that actually holds.
- Made harder: the corpus does NOT prove a native object round-trips (that a binding can *reach* the
  canonical form from a native `datetime`/`Decimal`); it proves the kernel renders the canonical form
  identically. The "native object → canonical form" step is a documented per-ecosystem limitation, not a
  defect — `1.0`-vs-`1` is likewise excluded (JS has one IEEE-754 number type).
- Reconsider: if a binding ever gains a lossless native-object marshaling path AND a future spec wants to
  assert it, add a separate fixture dimension rather than changing this one.

**Where to look next**
`conformance/README.md` (the type-tag table + DECISION note), `specs/006-conformance-corpus/research.md`
(D1), the TS runner's `assertDateDiverges()` probe (proves the workaround is still necessary at runtime).
A future maintainer tempted to "fix" a runner to build a native object will break the gate — this is why.
