# Issue Map — Spec 005 (TypeScript binding `prompting-press-node`)

GitHub issues for each task in `tasks.md`. Titles are prefixed `[005]` to disambiguate the
reused `T0NN` IDs from specs 001-004 (which share the same numbering namespace — issues #1-#32
are spec 001's T001-T031, #101-#128 are spec 004's). Each issue carries `spec:005` + a `phase:` label.

Issues #131-#161 on https://github.com/srobroek/prompting-press . Created 2026-06-27 via
`/speckit.taskstoissues`. On PR squash-merge, list each as its own `Closes #N` line in the PR body.

| Task | Issue | # | Phase | Title |
|------|-------|---|-------|-------|
| T001 | [#131](https://github.com/srobroek/prompting-press/issues/131) | #131 | setup | In `crates/prompting-press-node/Cargo.toml`: pin `napi` and |
| T002 | [#132](https://github.com/srobroek/prompting-press/issues/132) | #132 | setup | In `packages/typescript/package.json`: add `zod` (v4, exact `4.4.3` |
| T003 | [#133](https://github.com/srobroek/prompting-press/issues/133) | #133 | setup | Baseline build + FFI/codegen gates: `mise exec -- cargo build -p |
| T004 | [#134](https://github.com/srobroek/prompting-press/issues/134) | #134 | foundational | Wire the binding module tree in |
| T005 | [#135](https://github.com/srobroek/prompting-press/issues/135) | #135 | foundational | Create `crates/prompting-press-node/src/marshal.rs` (the FFI value |
| T006 | [#136](https://github.com/srobroek/prompting-press/issues/136) | #136 | foundational | Create `crates/prompting-press-node/src/error.rs`: the Rust side of |
| T007 | [#137](https://github.com/srobroek/prompting-press/issues/137) | #137 | foundational | Create `crates/prompting-press-node/src/registry.rs`: `#[napi] struct |
| T008 | [#138](https://github.com/srobroek/prompting-press/issues/138) | #138 | foundational | Create the TS facade scaffold in `packages/typescript/src/index.ts`: |
| T009 | [#139](https://github.com/srobroek/prompting-press/issues/139) | #139 | us1 | Rust-side marshaling + scrub unit tests in |
| T010 | [#140](https://github.com/srobroek/prompting-press/issues/140) | #140 | us1 | TS-side render tests in `packages/typescript/test/render.test.ts` |
| T011 | [#141](https://github.com/srobroek/prompting-press/issues/141) | #141 | us1 | Create `crates/prompting-press-node/src/render.rs`: `#[napi] fn |
| T012 | [#142](https://github.com/srobroek/prompting-press/issues/142) | #142 | us1 | In `render.rs`, add `#[napi] fn getSource(reg, name, variant?) -> |
| T013 | [#143](https://github.com/srobroek/prompting-press/issues/143) | #143 | us1 | In the TS facade (`packages/typescript/src/index.ts`), add the |
| T014 | [#144](https://github.com/srobroek/prompting-press/issues/144) | #144 | us1 | Build + run: `mise exec -- cargo test -p prompting-press-node` |
| T015 | [#145](https://github.com/srobroek/prompting-press/issues/145) | #145 | us2 | TS-side loader tests in `packages/typescript/test/loader.test.ts`: |
| T016 | [#146](https://github.com/srobroek/prompting-press/issues/146) | #146 | us2 | In `registry.rs`, add `#[napi]` `loadYaml(text)` and `loadJson(text)` |
| T017 | [#147](https://github.com/srobroek/prompting-press/issues/147) | #147 | us2 | Build + run: `mise exec -- pnpm -C packages/typescript build`; run |
| T018 | [#148](https://github.com/srobroek/prompting-press/issues/148) | #148 | us3 | TS-side check tests in `packages/typescript/test/check.test.ts`: |
| T019 | [#149](https://github.com/srobroek/prompting-press/issues/149) | #149 | us3 | Create `crates/prompting-press-node/src/check.rs`: `#[napi] fn |
| T020 | [#150](https://github.com/srobroek/prompting-press/issues/150) | #150 | us3 | Build + run: `mise exec -- pnpm -C packages/typescript build`; run |
| T021 | [#151](https://github.com/srobroek/prompting-press/issues/151) | #151 | us4 | TS-side composition tests in |
| T022 | [#152](https://github.com/srobroek/prompting-press/issues/152) | #152 | us4 | Create `crates/prompting-press-node/src/compose.rs`: `#[napi]` |
| T023 | [#153](https://github.com/srobroek/prompting-press/issues/153) | #153 | us4 | Build + run: `mise exec -- pnpm -C packages/typescript build`; run |
| T024 | [#154](https://github.com/srobroek/prompting-press/issues/154) | #154 | polish | TS package facade finalize in `packages/typescript/src/index.ts`: |
| T025 | [#155](https://github.com/srobroek/prompting-press/issues/155) | #155 | polish | Docs: `packages/typescript/README.md` quickstart documenting the |
| T026 | [#156](https://github.com/srobroek/prompting-press/issues/156) | #156 | polish | Build the distributable package + fresh-env import (SC-009): `mise |
| T027 | [#157](https://github.com/srobroek/prompting-press/issues/157) | #157 | polish | Full local gate suite — `mise exec -- moon run :build`; `mise exec -- |
| T028 | [#158](https://github.com/srobroek/prompting-press/issues/158) | #158 | polish | VERIFY the FFI-isolation gate covers napi (FR-022 / SC-007) — |
| T029 | [#159](https://github.com/srobroek/prompting-press/issues/159) | #159 | polish | Node dependency advisory gate (FR-025 / SC-011): create |
| T030 | [#160](https://github.com/srobroek/prompting-press/issues/160) | #160 | polish | Wire `ci:test-node` into CI (the spec-004 I1 lesson): create |
| T031 | [#161](https://github.com/srobroek/prompting-press/issues/161) | #161 | polish | Walk quickstart.md end-to-end and confirm every SC-001…SC-011 has a |
