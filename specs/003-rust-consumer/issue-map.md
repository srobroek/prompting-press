# Issue Map — Spec 003 (Rust consumer)

GitHub issues from `tasks.md`, 2026-06-26 (repo `srobroek/prompting-press`). Titles namespaced
`[003] T###:` (avoiding the spec-001 bare-`T###` + spec-002 `[002]` collisions). Labelled `spec-003`.
Issues #74–#99. (T024 token-hook task was dropped pre-issue — analyze F4.)

| Task | Issue | # | Description |
|------|-------|---|-------------|
| T001 | [https://github.com/srobroek/prompting-press/issues/74](https://github.com/srobroek/prompting-press/issues/74) | #74 | Add deps to `crates/prompting-press/Cargo.toml`: `garde = { version = "0.23", fe… |
| T002 | [https://github.com/srobroek/prompting-press/issues/75](https://github.com/srobroek/prompting-press/issues/75) | #75 | Run `mise exec -- cargo build -p prompting-press`, `mise exec -- moon run ci:che… |
| T003 | [https://github.com/srobroek/prompting-press/issues/76](https://github.com/srobroek/prompting-press/issues/76) | #76 | Wire the consumer module tree in `crates/prompting-press/src/lib.rs`: add `pub m… |
| T004 | [https://github.com/srobroek/prompting-press/issues/77](https://github.com/srobroek/prompting-press/issues/77) | #77 | Create `crates/prompting-press/src/error.rs`: `FieldError { field: String, code:… |
| T005 | [https://github.com/srobroek/prompting-press/issues/78](https://github.com/srobroek/prompting-press/issues/78) | #78 | In `error.rs`, implement the normalizers (FR-014/015): `From<garde::Report>` → `… |
| T006 | [https://github.com/srobroek/prompting-press/issues/79](https://github.com/srobroek/prompting-press/issues/79) | #79 | Create `crates/prompting-press/src/registry.rs`: `Registry { prompts: BTreeMap<S… |
| T007 | [https://github.com/srobroek/prompting-press/issues/80](https://github.com/srobroek/prompting-press/issues/80) | #80 | Render + validation tests in `crates/prompting-press/tests/render.rs`: define a … |
| T008 | [https://github.com/srobroek/prompting-press/issues/81](https://github.com/srobroek/prompting-press/issues/81) | #81 | Validation-failure tests in `crates/prompting-press/tests/render_validation.rs`:… |
| T009 | [https://github.com/srobroek/prompting-press/issues/82](https://github.com/srobroek/prompting-press/issues/82) | #82 | Create `crates/prompting-press/src/render.rs`: `pub fn render<V: serde::Serializ… |
| T010 | [https://github.com/srobroek/prompting-press/issues/83](https://github.com/srobroek/prompting-press/issues/83) | #83 | In `render.rs`, add `pub fn get_source<'a>(reg: &'a Registry, name: &str, varian… |
| T011 | [https://github.com/srobroek/prompting-press/issues/84](https://github.com/srobroek/prompting-press/issues/84) | #84 | Run `mise exec -- cargo test -p prompting-press --test render --test render_vali… |
| T012 | [https://github.com/srobroek/prompting-press/issues/85](https://github.com/srobroek/prompting-press/issues/85) | #85 | Loader tests in `crates/prompting-press/tests/loader.rs`: V2.1 (load_yaml → Prom… |
| T013 | [https://github.com/srobroek/prompting-press/issues/86](https://github.com/srobroek/prompting-press/issues/86) | #86 | In `registry.rs`, add `load_json(&mut self, doc: &str) -> Result<&PromptDefiniti… |
| T014 | [https://github.com/srobroek/prompting-press/issues/87](https://github.com/srobroek/prompting-press/issues/87) | #87 | Run `mise exec -- cargo test -p prompting-press --test loader`; confirm T012 pas… |
| T015 | [https://github.com/srobroek/prompting-press/issues/88](https://github.com/srobroek/prompting-press/issues/88) | #88 | Check tests in `crates/prompting-press/tests/check.rs`: V3.1 (clean registry → e… |
| T016 | [https://github.com/srobroek/prompting-press/issues/89](https://github.com/srobroek/prompting-press/issues/89) | #89 | Purity test in `crates/prompting-press/tests/check_purity.rs`: V3.4 — snapshot t… |
| T017 | [https://github.com/srobroek/prompting-press/issues/90](https://github.com/srobroek/prompting-press/issues/90) | #90 | Create `crates/prompting-press/src/check.rs`: `CheckReport { findings: Vec<Findi… |
| T018 | [https://github.com/srobroek/prompting-press/issues/91](https://github.com/srobroek/prompting-press/issues/91) | #91 | Implement the agreement lint (FR-016/017) in `check.rs`: for each prompt (iterat… |
| T019 | [https://github.com/srobroek/prompting-press/issues/92](https://github.com/srobroek/prompting-press/issues/92) | #92 | Implement the provenance lint (FR-018, reframed F1) in `check.rs`: `prompting_pr… |
| T020 | [https://github.com/srobroek/prompting-press/issues/93](https://github.com/srobroek/prompting-press/issues/93) | #93 | Run `mise exec -- cargo test -p prompting-press --test check --test check_purity… |
| T021 | [https://github.com/srobroek/prompting-press/issues/94](https://github.com/srobroek/prompting-press/issues/94) | #94 | Composition tests in `crates/prompting-press/tests/compose.rs`: V4.1 (N appended… |
| T022 | [https://github.com/srobroek/prompting-press/issues/95](https://github.com/srobroek/prompting-press/issues/95) | #95 | Create `crates/prompting-press/src/compose.rs`: `Message { role: String, text: S… |
| T023 | [https://github.com/srobroek/prompting-press/issues/96](https://github.com/srobroek/prompting-press/issues/96) | #96 | Run `mise exec -- cargo test -p prompting-press --test compose`; confirm T021 pa… |
| T025 | [https://github.com/srobroek/prompting-press/issues/97](https://github.com/srobroek/prompting-press/issues/97) | #97 | Crate-level rustdoc in `crates/prompting-press/src/lib.rs`: document the public … |
| T026 | [https://github.com/srobroek/prompting-press/issues/98](https://github.com/srobroek/prompting-press/issues/98) | #98 | Full local gate suite — `mise exec -- moon run :build`, `mise exec -- cargo test… |
| T027 | [https://github.com/srobroek/prompting-press/issues/99](https://github.com/srobroek/prompting-press/issues/99) | #99 | Walk quickstart.md end-to-end (V1.1–V4.3 + the boundary check mapped to tests T0… |
