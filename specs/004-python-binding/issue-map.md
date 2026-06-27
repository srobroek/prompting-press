# Issue Map — Spec 004 (Python binding `prompting-press-py`)

GitHub issues for each task in `tasks.md`. Titles are prefixed `[004]` to disambiguate the
reused `T0NN` IDs from specs 001/002/003 (which share the same numbering namespace).

Issues #101–#128 on https://github.com/srobroek/prompting-press . Created 2026-06-27 via
`/speckit.taskstoissues`. On PR squash-merge, list each as its own `Closes #N` line in the PR body.

| Task | Issue | # | Title |
|------|-------|---|-------|
| T001 | [#101](https://github.com/srobroek/prompting-press/issues/101) | #101 | In `crates/prompting-press-py/Cargo.toml` |
| T002 | [#102](https://github.com/srobroek/prompting-press/issues/102) | #102 | In `packages/python/pyproject.toml` |
| T003 | [#103](https://github.com/srobroek/prompting-press/issues/103) | #103 | Baseline build + FFI/codegen gates |
| T004 | [#104](https://github.com/srobroek/prompting-press/issues/104) | #104 | Wire the binding module tree in `crates/prompting-press-py/src/lib.rs` |
| T005 | [#105](https://github.com/srobroek/prompting-press/issues/105) | #105 | Create `crates/prompting-press-py/src/marshal.rs` (the FFI value brid… |
| T006 | [#106](https://github.com/srobroek/prompting-press/issues/106) | #106 | Create `crates/prompting-press-py/src/error.rs` |
| T007 | [#107](https://github.com/srobroek/prompting-press/issues/107) | #107 | Create `crates/prompting-press-py/src/registry.rs` |
| T008 | [#108](https://github.com/srobroek/prompting-press/issues/108) | #108 | Rust-side marshaling + scrub unit tests in `crates/prompting-press-py… |
| T009 | [#109](https://github.com/srobroek/prompting-press/issues/109) | #109 | Python-side render tests in `packages/python/tests/test_render.py` (p… |
| T010 | [#110](https://github.com/srobroek/prompting-press/issues/110) | #110 | Create `crates/prompting-press-py/src/render.rs` |
| T011 | [#111](https://github.com/srobroek/prompting-press/issues/111) | #111 | In `render.rs`, add `#[pyfn] get_source(reg, name, variant=None) -> s… |
| T012 | [#112](https://github.com/srobroek/prompting-press/issues/112) | #112 | Build + run: `mise exec -- cargo test -p prompting-press-py` (T008); … |
| T013 | [#113](https://github.com/srobroek/prompting-press/issues/113) | #113 | Python-side loader tests in `packages/python/tests/test_loader.py` |
| T014 | [#114](https://github.com/srobroek/prompting-press/issues/114) | #114 | In `registry.rs`, add `load_yaml(&mut self, text |
| T015 | [#115](https://github.com/srobroek/prompting-press/issues/115) | #115 | Build + run: `mise exec -- maturin develop ...`; `mise exec -- pytest… |
| T016 | [#116](https://github.com/srobroek/prompting-press/issues/116) | #116 | Python-side check tests in `packages/python/tests/test_check.py` |
| T017 | [#117](https://github.com/srobroek/prompting-press/issues/117) | #117 | Create `crates/prompting-press-py/src/check.rs` |
| T018 | [#118](https://github.com/srobroek/prompting-press/issues/118) | #118 | Build + run: `mise exec -- maturin develop ...`; `mise exec -- pytest… |
| T019 | [#119](https://github.com/srobroek/prompting-press/issues/119) | #119 | Python-side composition tests in `packages/python/tests/test_compose.py` |
| T020 | [#120](https://github.com/srobroek/prompting-press/issues/120) | #120 | Create `crates/prompting-press-py/src/compose.rs` |
| T021 | [#121](https://github.com/srobroek/prompting-press/issues/121) | #121 | Build + run: `mise exec -- maturin develop ...`; `mise exec -- pytest… |
| T022 | [#122](https://github.com/srobroek/prompting-press/issues/122) | #122 | Python package facade in `packages/python/python/prompting_press/__in… |
| T023 | [#123](https://github.com/srobroek/prompting-press/issues/123) | #123 | Docs: a module docstring / `packages/python/README.md` quickstart doc… |
| T024 | [#124](https://github.com/srobroek/prompting-press/issues/124) | #124 | Build the distributable wheel + fresh-env import (SC-009) |
| T025 | [#125](https://github.com/srobroek/prompting-press/issues/125) | #125 | Full local gate suite — `mise exec -- moon run :build`; `mise exec --… |
| T026 | [#126](https://github.com/srobroek/prompting-press/issues/126) | #126 | Walk quickstart.md end-to-end and confirm every SC-001…SC-010 has a p… |
| T027 | [#127](https://github.com/srobroek/prompting-press/issues/127) | #127 | Reconcile the stale roadmap "token hook" line (analyze U1; spec Assum… |
| T028 | [#128](https://github.com/srobroek/prompting-press/issues/128) | #128 | Python dependency advisory gate (FR-025 / SC-011; security review SEC… |
