# Phase 0 Research — Pre-publish API & schema reshape

All version facts below were **verified by direct registry/source fetch on 2026-06-28** (npm registry,
crates.io, upstream README). Two `speckit-research` subagent attempts returned fabricated, mutually
contradictory data (`tool_uses: 0`; one invented a nonexistent spec file) and were **discarded** — see
§Provenance. Nothing here rests on a subagent summary.

## R1 — TS codegen: `json-schema-to-zod`

- **Decision:** Replace `json-schema-to-typescript@15.0.4` with **`json-schema-to-zod@2.8.1`** for the TS shape.
- **Rationale:** The TS validating constructor needs a *runtime* enforcer (Q1). A generated `interface` is
  compile-time only; a generated Zod schema is both a runtime validator (`safeParse`) and a static type
  (`z.infer<typeof Schema>`), and exposes `.shape` for the per-variable `validation_required` coverage check.
- **Verified facts:** v2.8.1 ships a CLI (`bin: json-schema-to-zod`); `-i schema.json -o out.ts` emits
  committed `.ts`; **`--zodVersion` defaults to `4`** (matches the project's `zod@4.4.3`); `--type` exports the
  inferred type; `--module esm` (the package is ESM, Node ≥18); dev-tested against `zod ^4.1.3`.
- **Determinism:** unlike `json-schema-to-typescript`, it bundles **no formatter**. The `codegen.mjs` rewrite
  must add a deterministic format/normalize pass (the project already pins Biome 1.9.4) so the committed file
  stays byte-stable for the freshness gate. Validate with the existing twice-run byte-identical check.
- **`propertyNames` caveat:** the schema's `variants.propertyNames` (reserved-`default` rule, a `not`/`const`
  subschema) already needs a `jq` strip for Rust's `cargo-typify`. Likely the Zod codegen needs the same strip
  (it is a niche keyword). **Open verification for the schema+codegen task:** run the Zod codegen against the
  real schema and, if it errors/mis-emits on `propertyNames`, reuse the same `jq 'del(...)'` transform the Rust
  codegen uses. Either way the reserved-`default` rule stays enforced by the fixture gate + `check()`, not the
  generated type (consistent with architecture memory A1).
- **Alternatives rejected:** keep `interface` (no runtime enforcer — fails Q1); hand-write a Zod schema
  (violates C-07 codegen-from-schema).

## R2 — Zod v4 per-field introspection (the coverage check)

- **Decision:** Use **`ZodObject.shape`** to check per-variable validator coverage at construction.
- **Verified:** Zod 4.4.3 `ZodObject` exposes a `shape` record (`{ [field]: ZodType }`); `field in
  schema.shape` (or `Object.keys(schema.shape)`) lists covered fields. This is the runtime mechanism for
  FR-024's "throw at construction if a `validation_required` variable is uncovered" in TypeScript.
- **Note:** the facade already types schemas structurally (`ZodLikeSchema` with `safeParse`) to avoid a hard
  Zod-identity dependency. The coverage check needs `.shape`, which is a slightly richer structural contract;
  extend `ZodLikeSchema` to optionally carry `shape?: Record<string, unknown>` and treat its absence as "cannot
  introspect → cannot assert coverage" (documented limitation for a non-Zod schema object).

## R3 — Rust TOML

- **Decision:** Add **`toml@1.1.2`** (serde-native) for `Prompt::from_toml`.
- **Verified:** crates.io `max_stable_version = 1.1.2` (the crate is at 1.x, not 0.8.x); `toml::from_str::<T>()`
  with `T: DeserializeOwned` — same serde path the existing YAML/JSON loaders use, so the marshaling/canonical-
  form invariants (decision memory D1) carry over unchanged.
- **Boundary check (Principle III):** parsing caller-supplied text is not I/O — identical to the existing
  `serde_yaml`/`serde_json` loaders. No boundary breach.

## R4 — Node TOML

- **Decision:** Add **`smol-toml@1.7.0`** for the TS `Prompt.fromToml`.
- **Verified:** npm `latest = 1.7.0`; `type: module` (native ESM, Node ≥18 — matches the package's ESM-only,
  Node 20+ stance); TS types bundled; zero deps; `parse(str)` returns a plain object fed into the same Zod path.
- **Alternatives rejected:** `@iarna/toml` and `toml` are CJS-only and unmaintained (last publish 2019–2020).

## R5 — Python floor raised to 3.12 (user decision)

- **Decision:** Raise `requires-python` `>=3.10` → **`>=3.12`**; pyo3 `abi3-py310` → **`abi3-py312`**;
  `datamodel-codegen --target-python-version` `3.10` → **3.12**; CI build-matrix floor bumped; fix the stale
  `abi3-py39` CI comments.
- **Rationale (user-initiated):** pre-publish is the only free window (no published wheels to break); 3.10 is
  near EOL (Oct 2026); 3.12 is already the CI-tested interpreter; and at 3.12 **`tomllib` is stdlib**, so
  Python `from_toml` needs **no `tomli` backport dependency** — one fewer supply-chain pin. The floor IS part
  of the published contract, so this belongs in 008.
- **Note:** the floor config was already drifted (Cargo `abi3-py310`, pyproject `>=3.10`, CI comments
  `abi3-py39`); this standardizes all three on 3.12.

## R6 — `with(overlay)` + bound-validator carry-forward semantics (resolves open item A8-2)

The sole mutator `with(overlay) -> Result<Prompt>` shallow-replaces top-level fields and re-validates the merged
whole. Because validators are now **bound at construction** (Principle VI, v1.2.0), `with` must define what
happens to the *bound validators* of the original prompt. **Decision (the least-surprising, immutability-
consistent rule):**

1. **Validators carry forward by default.** The derived prompt inherits the source prompt's bound validator(s);
   the original is untouched (immutability). `with` re-runs the **same construction-time coverage check** over
   the *merged* definition using the *carried* validators.
2. **The overlay MAY supply new/replacement validators** as the same side input construction accepts (so a
   developer adding a `validation_required` variable can supply its validator in the same `with` call).
3. **Coverage is re-checked against the merged whole.** If the overlay adds a `validation_required` variable
   and neither the carried nor the supplied validators cover it, `with` fails (TS/Python throw; Rust = the
   type-level guarantee, since the Rust `with::<V>` is generic over the Vars type — see the contract).
4. **Rust specifics:** Rust's `with` is generic over the validator type `V` exactly as `render::<V>` is; there
   is no runtime validator object to "carry," so carry-forward is a non-issue in Rust — the caller names `V` at
   the `with`/`render` call and the compiler enforces coverage. Carry-forward is a TS/Python (runtime-validator)
   concern only. This is the same asymmetry codified in Principle VI v1.2.0.
- **Rationale:** anything else either silently drops validation on derive (unsafe — a derived prompt could
  render unvalidated) or forces re-supplying validators on every `with` (hostile ergonomics). Carry-with-
  override is what `dataclasses.replace`/object-spread users expect.
- **This is now the contract**; the spec edge case + data-model reflect it.

## R7 — Construction-vs-`check()` boundary (confirms Q4, grounded in the kernel)

- **Verified against the code:** `agreement.rs:88 required_roots` parses the template FIRST; with
  `macros`/`multi_template` disabled, an excluded `{% include/import/extends/macro/block %}` or a syntax error
  returns `Err(KernelError::ExcludedFeature | ::Parse)` BEFORE analysis. `check.rs:271` currently turns that
  `Err` into a `FindingKind::AnalysisError`.
- **Decision (Q4):** the validating constructor calls the same parse+`required_roots` path and treats that
  `Err` as a **construction failure** (normalized structured error). Therefore a constructed `Prompt` always
  parses + analyzes, and `check()`'s `AnalysisError` arm is **unreachable for a constructed `Prompt`** — it
  remains in the code (harmless; defensive) but the only *live* `check()` finding is the origin/guard advisory.
- **Kernel impact:** none — the constructor *reuses* `required_roots`; no kernel code changes (Principle I).

## Provenance (why no subagent findings are trusted here)

Two `speckit-research` spawns both returned `tool_uses: 0` with fabricated content: contradictory versions
(`json-schema-to-zod` "2.4.1" vs "2.6.0"; `toml` "0.8.22" vs "0.8.23"; zod-`latest` "3.24.4" vs "4.4.3"), and
one invented a nonexistent `.specify/specs/008-pre-publish-api-reshape.md` describing a `PromptDefinition→
PromptSpec` rename that is not this project. Per [[speckit-workflow-gotchas]], both were discarded and every
load-bearing pin re-verified main-thread by direct fetch. The verified numbers (2.8.1 / 4.4.3 / toml 1.1.2 /
smol-toml 1.7.0) appear in the plan's Dependency Pins table.
