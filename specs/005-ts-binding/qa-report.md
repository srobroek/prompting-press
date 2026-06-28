# T031 — SC Coverage Walk (spec 005 TypeScript binding)

Every Success Criterion maps to a passing backing test or CI gate. Verified 2026-06-28 (impl phase,
T026–T031). Toolchain via `mise exec --`.

| SC | Criterion | Backing evidence | Status |
|----|-----------|------------------|--------|
| SC-001 | Idiomatic validate-then-render path, no native types on the API | `test/render.test.mjs` (valid render → text + name + variant + 64-hex templateHash/renderHash) | ✅ |
| SC-002 | Invalid input rejected before render, every offending field named | `test/render.test.mjs` (Zod `.refine()` fails → `PromptValidationError` naming field, code `validation`, no render) | ✅ |
| SC-003 | YAML/JSON/object parity (identical render + provenance) | `test/loader.test.mjs` (loadYaml/loadJson/insert → identical text + both hashes) | ✅ |
| SC-004 | Agreement check detects undeclared-variable reference | `test/check.test.mjs::undeclared` (kind `undeclared_variable` naming prompt/variant/var) | ✅ |
| SC-005 | Provenance lint flags untrusted/external-without-guard | `test/check.test.mjs::untrusted` (kind `untrusted_without_guard` naming prompt/field) | ✅ |
| SC-006 | No native error type on the public API | `test/render.test.mjs` (thrown error is `instanceof PromptValidationError`, NOT a `ZodError`) | ✅ |
| SC-007 | `napi` only in `-node`; no engine logic in the binding | `ci:check-ffi` PASS (`FFI_CRATES=("pyo3" "napi")`; `cargo tree -i napi` empty for kernel + consumer); render/check/compose delegate to `prompting_press_core`/`prompting_press` | ✅ |
| SC-008 | N composition entries → N ordered `{role,text}` messages | `test/compose.test.mjs` (order + roles via `fromMessages`/`append`; no-partial; empty → `[]`; no `.chain()`) | ✅ |
| SC-009 | napi addon builds; fresh-env import + render works | T026: `pnpm pack` → `prompting-press-0.0.0.tgz`; installed in a clean `/tmp` dir (only zod added), ESM `import 'prompting-press'` + render → `'Hi Ada'`, 64-hex hashes | ✅ |
| SC-010 | Generated TS shape byte-identical to fresh regen; no token surface | `schemas:codegen-check` PASS; narrowed token grep (`count_tokens|tokenCount|countTokens|count-tokens`) finds nothing | ✅ |
| SC-011 | CI advisory gate scans Node deps (pnpm-lock) for CVEs | `ci:check-advisories-node` PASS (`pnpm audit >= high`, clean) | ✅ |

## Full gate suite (T027) — all green

- `cargo test -p prompting-press-node` → 36 passed.
- `pnpm -C packages/typescript test` (node:test) → 57 passed (15 render + 17 loader + 11 check + 14 compose).
- `cargo clippy -p prompting-press-node --all-targets -- -D warnings` → clean.
- `cargo fmt -p prompting-press-node -- --check` → clean.
- `moon run :build` → completed.
- `ci:check-ffi`, `ci:check-floating-versions`, `schemas:codegen-check`, `ci:check-advisories`,
  `ci:check-advisories-node`, `ci:test-node` → all PASS (`--force`, not cache-masked).

## Notes

- SEC-004 secret-scrub is pinned both Rust-side (`error.rs` `#[cfg(test)]`) and TS-side
  (`test/render.test.mjs`): a seeded secret never appears in the thrown error's `message`, `.stack`, or
  any `.errors[*]` row — on BOTH the Zod-reject path and the real `{{ token + 1 }}` kernel-render path.
- Cross-binding parity (SC-009 / Principle I): the provenance hashes from a TS render are byte-identical
  to the Python binding's + Rust consumer's for the same logical prompt + inputs — structural, via the
  shared `minijinja::Value::from_serialize` marshaling path; not re-tested here.
- The headline value is US3 (`check()`); US1 is the MVP that proves the napi marshaling path on Node.
