# Quickstart: Auto-generated language API references

Runnable validation that the generated reference pages are produced from source and stay fresh.
References [plan.md](./plan.md), [research.md](./research.md), and
[contracts/api-doc-ir.md](./contracts/api-doc-ir.md) for detail.

## Prerequisites

- `mise` toolchains installed, including the pinned **nightly** Rust toolchain (for rustdoc JSON) in
  addition to the existing stable `1.95.0`.
- `docs/site` dev deps installed (TypeDoc `0.28.19`); griffe `2.1.0` available via `uv`.

## Generate the three reference pages (latest)

```bash
# Orchestrator — extracts all three languages and renders reference/{rust,python,typescript}.mdx.
# Runs as part of the Astro prebuild; can be invoked directly:
node docs/site/scripts/gen-api-refs.mjs            # defaults: --version latest, --out the live reference dir
```

Expected: `reference/rust.mdx`, `reference/python.mdx`, `reference/typescript.mdx` are written, each
with the AUTO-GENERATED marker, covering the full public surface of that binding in the canonical
group order, with no internal-jargon strings.

## Validate US1 — accuracy without manual mirroring

```bash
# 1. Edit a public doc comment in source (e.g. a /// on a method in crates/prompting-press/src/prompt.rs).
# 2. Regenerate and confirm the page reflects it with no .mdx hand-edit:
node docs/site/scripts/gen-api-refs.mjs
git diff --stat docs/site/src/content/docs/reference/rust.mdx   # shows the change

# 3. Freshness gate catches an un-regenerated change:
#    (edit source, do NOT regenerate, run the gate — it must FAIL and name the stale page)
bash docs/site/scripts/check-api-refs-fresh.sh   # mirrors schemas/scripts/codegen-check.sh
echo "exit: $?"   # non-zero on drift
```

## Validate US2 — complete + consistent coverage

```bash
# Every public symbol present, no internal symbol leaked:
#  - Rust: compare against crates/prompting-press/src/lib.rs re-exports
#  - TS:   compare against packages/typescript/src/index.ts exports
#  - Python: compare against prompting_press.__all__
# The three pages share the canonical group order (FR-009) — eyeball parallel structure.

# No internal-governance jargon on any page (SC-005):
rg -n 'C-[0-9]|FR-[0-9]|SC-[0-9]|SEC-[0-9]|Principle [IVX]|spec [0-9]' \
  docs/site/src/content/docs/reference/{rust,python,typescript}.mdx || echo "clean"
```

## Validate determinism (SC-003)

```bash
node docs/site/scripts/gen-api-refs.mjs && git add -A
node docs/site/scripts/gen-api-refs.mjs
git diff --stat docs/site/src/content/docs/reference/   # MUST be empty (byte-identical second run)
```

## Validate missing-doc-comment policy (R6 / FR-008)

```bash
# Remove a doc comment from a public symbol, regenerate → the gate FAILS naming the symbol:
node docs/site/scripts/gen-api-refs.mjs   # errors: "<language>: public symbol `<name>` is undocumented"
```

## Validate the boundary (Principle II/III / FR-011 / SC-006)

```bash
# The shipped runtime dependency sets are unchanged — extractors are dev/build-time only:
mise exec -- moon run ci:check-ffi          # kernel + consumer still FFI-free
# Confirm typedoc/griffe/nightly-rustdoc appear ONLY in docs/dev tooling, not in any
# crate Cargo.toml [dependencies] or package runtime deps.
```

## Validate version-awareness (R8, for spec 012)

```bash
# The generator accepts a version + output dir (012 will drive this per snapshot):
node docs/site/scripts/gen-api-refs.mjs --version 1.2 --out /tmp/v1.2-reference
ls /tmp/v1.2-reference/   # rust.mdx python.mdx typescript.mdx — same content, different target
```

## Done when

- All three pages generate from source and carry the AUTO-GENERATED marker.
- The freshness gate fails on un-regenerated source changes and on undocumented public symbols.
- Two consecutive runs are byte-identical.
- No internal jargon on any page; public-surface-only; shape types link out (not duplicated).
- `ci:check-ffi` passes; no runtime dependency added.
