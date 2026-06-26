# Issue Map — Spec 002 (Engine kernel)

GitHub issues created from `tasks.md` on 2026-06-26 (repo `srobroek/prompting-press`).

Titles are namespaced `[002] T###:` to avoid collision with spec-001's `T001`–`T035` issues
(bare `T###`). Labelled `spec-002`. Issues #37–#72.

| Task | Issue | # | Description |
|------|-------|---|-------------|
| T001 | [https://github.com/srobroek/prompting-press/issues/37](https://github.com/srobroek/prompting-press/issues/37) | #37 | Add kernel dependencies to `crates/prompting-press-core/Cargo.toml`: `minijinja … |
| T002 | [https://github.com/srobroek/prompting-press/issues/38](https://github.com/srobroek/prompting-press/issues/38) | #38 | Run `mise exec -- cargo build -p prompting-press-core` and `mise exec -- moon ru… |
| T003 | [https://github.com/srobroek/prompting-press/issues/39](https://github.com/srobroek/prompting-press/issues/39) | #39 | `git mv crates/prompting-press/src/generated.rs crates/prompting-press-core/src/… |
| T004 | [https://github.com/srobroek/prompting-press/issues/40](https://github.com/srobroek/prompting-press/issues/40) | #40 | `git mv crates/prompting-press/scripts/codegen.sh crates/prompting-press-core/sc… |
| T005 | [https://github.com/srobroek/prompting-press/issues/41](https://github.com/srobroek/prompting-press/issues/41) | #41 | Move the moon `codegen` task from `crates/prompting-press/moon.yml` to `crates/p… |
| T006 | [https://github.com/srobroek/prompting-press/issues/42](https://github.com/srobroek/prompting-press/issues/42) | #42 | Wire the kernel modules in `crates/prompting-press-core/src/lib.rs`: add `pub mo… |
| T007 | [https://github.com/srobroek/prompting-press/issues/43](https://github.com/srobroek/prompting-press/issues/43) | #43 | Update the consumer `crates/prompting-press/src/lib.rs`: delete `pub mod generat… |
| T008 | [https://github.com/srobroek/prompting-press/issues/44](https://github.com/srobroek/prompting-press/issues/44) | #44 | Update the freshness gate `schemas/moon.yml` `codegen-check` task: change dep `p… |
| T009 | [https://github.com/srobroek/prompting-press/issues/45](https://github.com/srobroek/prompting-press/issues/45) | #45 | Verify the relocation end-to-end: `mise exec -- cargo build --workspace`, `mise … |
| T010 | [https://github.com/srobroek/prompting-press/issues/46](https://github.com/srobroek/prompting-press/issues/46) | #46 | Create `crates/prompting-press-core/src/error.rs` with `KernelError` enum: `Unkn… |
| T011 | [https://github.com/srobroek/prompting-press/issues/47](https://github.com/srobroek/prompting-press/issues/47) | #47 | Create `crates/prompting-press-core/src/engine.rs` skeleton: a private `build_en… |
| T012 | [https://github.com/srobroek/prompting-press/issues/48](https://github.com/srobroek/prompting-press/issues/48) | #48 | Create the test fixtures dir `crates/prompting-press-core/tests/fixtures/` and a… |
| T013 | [https://github.com/srobroek/prompting-press/issues/49](https://github.com/srobroek/prompting-press/issues/49) | #49 | Variant-resolution + render tests in `crates/prompting-press-core/tests/render.r… |
| T014 | [https://github.com/srobroek/prompting-press/issues/50](https://github.com/srobroek/prompting-press/issues/50) | #50 | Determinism + hashing tests in `crates/prompting-press-core/tests/hashing.rs`: V… |
| T015 | [https://github.com/srobroek/prompting-press/issues/51](https://github.com/srobroek/prompting-press/issues/51) | #51 | Error-path tests in `crates/prompting-press-core/tests/render_errors.rs`: V1.7 (… |
| T016 | [https://github.com/srobroek/prompting-press/issues/52](https://github.com/srobroek/prompting-press/issues/52) | #52 | Implement variant resolution in `crates/prompting-press-core/src/engine.rs`: `re… |
| T017 | [https://github.com/srobroek/prompting-press/issues/53](https://github.com/srobroek/prompting-press/issues/53) | #53 | Implement `get_source(def, Option<&str>) -> Result<&str, KernelError>` in `engin… |
| T018 | [https://github.com/srobroek/prompting-press/issues/54](https://github.com/srobroek/prompting-press/issues/54) | #54 | Create `crates/prompting-press-core/src/hashing.rs`: `sha256_hex(&str) -> String… |
| T019 | [https://github.com/srobroek/prompting-press/issues/55](https://github.com/srobroek/prompting-press/issues/55) | #55 | Implement `RenderResult` struct + `render(def, variant, values, guard) -> Result… |
| T020 | [https://github.com/srobroek/prompting-press/issues/56](https://github.com/srobroek/prompting-press/issues/56) | #56 | Run `mise exec -- cargo test -p prompting-press-core --test render --test hashin… |
| T021 | [https://github.com/srobroek/prompting-press/issues/57](https://github.com/srobroek/prompting-press/issues/57) | #57 | Agreement-analysis tests in `crates/prompting-press-core/tests/agreement.rs`: V2… |
| T022 | [https://github.com/srobroek/prompting-press/issues/58](https://github.com/srobroek/prompting-press/issues/58) | #58 | Purity test in `crates/prompting-press-core/tests/agreement_purity.rs`: V2.5 — c… |
| T023 | [https://github.com/srobroek/prompting-press/issues/59](https://github.com/srobroek/prompting-press/issues/59) | #59 | Create `crates/prompting-press-core/src/agreement.rs`: `Agreement { variant, req… |
| T024 | [https://github.com/srobroek/prompting-press/issues/60](https://github.com/srobroek/prompting-press/issues/60) | #60 | In `agreement.rs`, build the globals allowlist DYNAMICALLY from the kernel `Envi… |
| T025 | [https://github.com/srobroek/prompting-press/issues/61](https://github.com/srobroek/prompting-press/issues/61) | #61 | In `agreement.rs`, guard the parse-error footgun (FR-016a): `undeclared_variable… |
| T026 | [https://github.com/srobroek/prompting-press/issues/62](https://github.com/srobroek/prompting-press/issues/62) | #62 | Run `mise exec -- cargo test -p prompting-press-core --test agreement --test agr… |
| T027 | [https://github.com/srobroek/prompting-press/issues/63](https://github.com/srobroek/prompting-press/issues/63) | #63 | Provenance + guard tests in `crates/prompting-press-core/tests/provenance.rs`: V… |
| T028 | [https://github.com/srobroek/prompting-press/issues/64](https://github.com/srobroek/prompting-press/issues/64) | #64 | Create `crates/prompting-press-core/src/provenance.rs`: `ProvenanceView { untrus… |
| T029 | [https://github.com/srobroek/prompting-press/issues/65](https://github.com/srobroek/prompting-press/issues/65) | #65 | In `provenance.rs`, add `GuardConfig { enabled: bool, template: Option<String> }… |
| T030 | [https://github.com/srobroek/prompting-press/issues/66](https://github.com/srobroek/prompting-press/issues/66) | #66 | Wire guard into `engine::render` (T019): when `guard.enabled`, set `RenderResult… |
| T031 | [https://github.com/srobroek/prompting-press/issues/67](https://github.com/srobroek/prompting-press/issues/67) | #67 | Run `mise exec -- cargo test -p prompting-press-core --test provenance`; confirm… |
| T032 | [https://github.com/srobroek/prompting-press/issues/68](https://github.com/srobroek/prompting-press/issues/68) | #68 | Excluded-feature regression tests in `crates/prompting-press-core/tests/excluded… |
| T033 | [https://github.com/srobroek/prompting-press/issues/69](https://github.com/srobroek/prompting-press/issues/69) | #69 | Crate-level rustdoc in `crates/prompting-press-core/src/lib.rs`: document the fo… |
| T034 | [https://github.com/srobroek/prompting-press/issues/70](https://github.com/srobroek/prompting-press/issues/70) | #70 | Full local gate suite — `mise exec -- moon run :build`, `mise exec -- cargo test… |
| T035 | [https://github.com/srobroek/prompting-press/issues/71](https://github.com/srobroek/prompting-press/issues/71) | #71 | Walk quickstart.md end-to-end (all V-scenarios mapped to tests T013–T032) and co… |
| T036 | [https://github.com/srobroek/prompting-press/issues/72](https://github.com/srobroek/prompting-press/issues/72) | #72 | Add a pinned dependency-advisory gate (security SEC-001): a `cargo audit` (or `c… |
