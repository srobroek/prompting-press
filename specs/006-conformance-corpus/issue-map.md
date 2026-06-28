# Issue Map — 006 Conformance corpus + cross-language hardening

GitHub issues created from `tasks.md` on 2026-06-28, repo `srobroek/prompting-press`. Titles follow the
repo convention `[006] T0NN: <summary>`; each carries a `spec:006` label + its `phase:` label (enforced by
the `speckit-issue-label-guard` hook). Full task text, dependencies, and acceptance live in `tasks.md`.

| Task | Issue | Phase | Summary |
|------|-------|-------|---------|
| T001 | [#164](https://github.com/srobroek/prompting-press/issues/164) | setup | conformance/ dir structure + README contract |
| T002 | [#165](https://github.com/srobroek/prompting-press/issues/165) | foundational | five marshaling fixture inputs (type-tagged) |
| T003 | [#166](https://github.com/srobroek/prompting-press/issues/166) | foundational | schema/manifest.json + YAML twins |
| T004 | [#167](https://github.com/srobroek/prompting-press/issues/167) | foundational | Rust test-support module (RawVars, builders) |
| T005 | [#168](https://github.com/srobroek/prompting-press/issues/168) | foundational | golden generator (#[ignore]d test) |
| T006 | [#169](https://github.com/srobroek/prompting-press/issues/169) | foundational | run generator; commit goldens |
| T007 | [#170](https://github.com/srobroek/prompting-press/issues/170) | us1 | Rust marshaling runner |
| T008 | [#171](https://github.com/srobroek/prompting-press/issues/171) | us1 | Python marshaling runner |
| T009 | [#172](https://github.com/srobroek/prompting-press/issues/172) | us1 | TS marshaling runner |
| T010 | [#173](https://github.com/srobroek/prompting-press/issues/173) | us1 | confirm marshaling parity + seeded-divergence |
| T011 | [#174](https://github.com/srobroek/prompting-press/issues/174) | us2 | Rust schema runner |
| T012 | [#175](https://github.com/srobroek/prompting-press/issues/175) | us2 | Python schema runner |
| T013 | [#176](https://github.com/srobroek/prompting-press/issues/176) | us2 | TS schema runner |
| T014 | [#177](https://github.com/srobroek/prompting-press/issues/177) | us2 | confirm schema verdict parity (incl. YAML) |
| T015 | [#178](https://github.com/srobroek/prompting-press/issues/178) | us3 | scripts/ci/conformance.sh |
| T016 | [#179](https://github.com/srobroek/prompting-press/issues/179) | us3 | ci/moon.yml conformance task |
| T017 | [#180](https://github.com/srobroek/prompting-press/issues/180) | us3 | wire gate into ci.yml (Rust leg, FR-015) |
| T018 | [#181](https://github.com/srobroek/prompting-press/issues/181) | us3 | goldens-regeneration entry point |
| T019 | [#182](https://github.com/srobroek/prompting-press/issues/182) | polish | finalize README + cross-links |
| T020 | [#183](https://github.com/srobroek/prompting-press/issues/183) | polish | verify scope guards (SC-006) |
| T021 | [#184](https://github.com/srobroek/prompting-press/issues/184) | polish | full quickstart validation |

**Created**: 21 issues (#164–#184). **Skipped**: 0 (no pre-existing `[006]` issues). **Label**: `spec:006`
created this run (`fbca04`); `phase:*` labels pre-existed.

**Note on dedup**: this repo disambiguates per-spec task IDs with a `[NNN]` title prefix (004/005 issues use
`[004]`/`[005] T0NN:`). A re-run of `/speckit.taskstoissues` must match on the `[006]` prefix (not a bare
`T0NN`), or it would treat 004/005's `T001…` as collisions. The created issues all carry `[006]`.
