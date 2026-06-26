# Phase 0 Research: Foundations

Resolves the spec's one genuine unknown — roadmap **Q1: schema→shape codegen tooling** — plus the
adjacent toolchain/layout choices. All tool claims verified against current registries/source
(2026-06-25); cite-and-verify, not assumption (constitution's stated discipline).

---

## D1 — Codegen: best-in-class tool per language (NOT one multi-target tool)

**Decision:** generate the per-language prompt-definition shapes from the single JSON Schema using a
dedicated tool per language:

| Language | Tool | Version | Determinism flags (load-bearing) |
|---|---|---|---|
| Python (Pydantic v2) | `datamodel-code-generator` | 0.65.1 | `--disable-timestamp`, `--formatters builtin` (decouple from black/isort versions), version-header OFF; has built-in `--check` |
| TypeScript | `json-schema-to-typescript` (`json2ts`) | 15.0.4 | `--bannerComment ''` (static banner otherwise; eliminates upgrade churn); pin Prettier |
| Rust (serde) | `cargo-typify` (typify) | 0.7.0 | CLI mode (NOT the `import_types!` macro); rustfmt pinned via `rust-toolchain.toml`; install `--locked` |

**Rationale:** the make-or-break property is **deterministic, byte-stable output** (the codegen
freshness CI gate, FR-019, fails otherwise). Each tool above is byte-stable *with the listed flags*
(empirically verified across 3 runs in research). `datamodel-code-generator` is the only mature tool
emitting true Pydantic v2; `typify` is the only actively maintained serde-native option.

**Alternatives considered:**
- **quicktype (one tool, all 3 targets) — REJECTED.** Two disqualifiers: (1) its Pydantic output is
  not idiomatic v2 (no `model_config`/validators; Python typing capped at ≤3.7, issue #2848); (2)
  open unfixed ordering-determinism bug (#2698 — `alphabetizeProperties=false` still sorts), which
  directly threatens the freshness gate. Simpler pipeline, but worst-in-class on the two things that
  matter most here (Python fidelity + determinism).
- **schemafy / json_typegen (Rust) — REJECTED**, unmaintained (2021) and proc-macro shaped (no
  committed file to diff).

**Tradeoff accepted:** three tools = three version pins + three determinism configs (timestamp/banner
suppression, formatter pinning) instead of one. But the *gate mechanism* is identical either way, and
the determinism + Pydantic-v2 fidelity requirements make per-language the only viable choice.

**JSON-Schema feature support (all three handle the shapes our schema needs):**
- string enum → Python `Enum` / TS union / Rust serde enum
- `const` → Python `Literal` / TS literal / (typify: confirm on sample)
- `oneOf`/`anyOf` → union/enum (note: typify's `anyOf` is its weak area — avoid `anyOf`, prefer
  `oneOf` or a discriminated shape in the schema)
- `additionalProperties:false` → Pydantic `extra='forbid'` / TS sealed type / Rust
  `#[serde(deny_unknown_fields)]`
- **free-form opaque object** (the `meta` + `metadata` fields) → `dict[str, Any]` /
  `{ [k:string]: unknown }` / `HashMap<String, serde_json::Value>` — all clean. **This is why `meta`
  must be modeled as an open object in the schema.**
- `pattern`/`format`/`minimum` → emitted where the type system allows (Pydantic `constr`/`confloat`);
  silently dropped by TS/Rust type systems (expected — validation lives in the consumer layer, not
  the generated shape).

## D2 — Codegen freshness CI gate (FR-019)

**Decision:** regenerate to committed paths, then assert no diff.
- Python: `datamodel-codegen ... --check` (built-in; exits 1 on drift).
- TS + Rust: regenerate → `git add -N . && git diff --exit-code -- <generated paths>` (the `-N`
  catches new untracked files, i.e. partial regeneration, per spec edge case).
- Determinism prerequisites enforced in CI: pin every tool version exactly (no `^`), suppress
  timestamps/banners, pin every formatter (rustfmt via toolchain, Prettier, black/isort or
  `--formatters builtin`), `LC_ALL=C`.

## D3 — FFI-isolation CI gate (FR-018)

**Decision:** a CI check asserts `pyo3` and `napi` (and any binding/FFI crate) do not appear in the
dependency trees of `prompting-press-core` or `prompting-press`.
- Mechanism: `cargo metadata` / `cargo tree -p prompting-press-core -i pyo3` (and `napi`) → expect
  "not found"; fail if present. Cheaper alternative: grep the two crates' `Cargo.toml` for the
  forbidden deps. Prefer `cargo tree` (catches transitive introduction too).
- **Rationale:** C-02 is the constitution's enforceable structural invariant; a `Cargo.toml`-only
  grep misses a transitive pull-in, so the dependency-graph query is the sound check.

## D4 — Workspace orchestration

**Decision:** Cargo workspace (`crates/*`) + moon (already bootstrapped) orchestrates cross-language
build/test/codegen. Python packaging = maturin (PyO3 standard); TS = napi-rs CLI packaging. **These
are reserved/known from the scaffold; 001 wires them to the new layout, introducing no new
orchestration tech** (per the spec's Assumptions).
- Go: reserved placeholder dir only, excluded from the moon project globs and the Cargo workspace
  members (FR-005/006).

## D5 — Schema dialect & identity

**Decision:** author the prompt-definition schema in **JSON Schema draft 2020-12** (current; supported
by all three generators), with a stable `$id`. "Published" = committed with a stable `$id`, not an
external URL endpoint (per spec Assumptions). One root schema; the `variants` map and the
default-vs-named structure modeled with `oneOf`/object composition (avoiding `anyOf` for typify's
sake — see D1).

## Residual unknowns (carried to implementation, low-risk)

- typify's exact serde-derive line, string-enum `#[serde(rename)]` mapping, and `const` handling were
  not source-quotable — **confirm by generating a sample** during the Rust-codegen task (cheap,
  first-task verification).
- Exact moon task wiring for the 3 codegen steps + the 2 CI gates — mechanical, decided at task time.
