# Specification Quality Checklist: Tested Documentation Samples

**Purpose**: Validate specification completeness and quality before planning
**Created**: 2026-06-29
**Feature**: [spec.md](../spec.md)

## Content Quality

- [x] No implementation details beyond the intrinsic (per-language test tools named in
      Assumptions/FR-004 as grounding facts, like prior specs named garde/Pydantic/Zod — not
      leaked design decisions)
- [x] Focused on user value (a developer trusting that every snippet they copy from the docs
      actually compiles and runs against the version they are using)
- [x] Written for non-technical stakeholders (page purposes and rot-prevention rationale are
      plain-language)
- [x] All mandatory sections completed

## Requirement Completeness

- [ ] **[NEEDS CLARIFICATION] Q1, Q2, Q3 remain open** — see Notes section below; these three
      items are marked in FR-003, FR-006, FR-007 and must be resolved before planning
- [x] All resolved requirements are testable and unambiguous
- [x] Success criteria are measurable (100% coverage audit; gate-blocks-publish; per-version
      library pin; assertions pass; no published runtime deps)
- [x] Success criteria are technology-agnostic where it matters (phrased as outcomes, not tool
      invocations)
- [x] All acceptance scenarios are defined
- [x] Edge cases identified (definition-only blocks, Tabs nesting, fragment-only snippets,
      hash-field assertions, version-agnostic pinning)
- [x] Scope is clearly bounded (build-time/dev-time only; no library behavior change; no
      cross-language parity testing via samples — that is spec 006's job)
- [x] Dependencies and assumptions identified (010, 011, 012 coordination; moon orchestrator)

## Feature Readiness

- [x] All resolved functional requirements have clear acceptance criteria
- [x] User scenarios cover primary flows (P1 gate blocks broken sample; P2 full coverage; P3
      expected-output assertions)
- [x] Feature meets measurable outcomes defined in Success Criteria
- [x] No implementation details leak beyond intrinsic tool choices (e.g. per-language test
      idioms are "intrinsic" — there is no non-idiomatic alternative in each ecosystem)
- [ ] **Blocked on Q1/Q2/Q3 for planning** — spec is draft-complete but three clarifications
      must be resolved before the plan/tasks phase

## Notes

### Open Clarification Questions

**Q1 — Architecture A (source-canonical) vs hybrid** [FR-003, marked `[NEEDS CLARIFICATION]`]

The recommended architecture is "source-canonical": real tested source files live in the repo and
are injected into MDX at build time, rather than extracting fenced blocks from MDX and testing
them. This matches the `gen-shape-table.mjs` (spec 011) pattern exactly. However, the ~108
existing docs blocks are currently hand-written in MDX (not yet injected from source). The open
question is: **does the user confirm source-canonical as the architecture for ALL samples (new
and migration of existing blocks), or is a hybrid acceptable** — source-canonical for new samples,
extract-and-test for existing blocks until they are migrated? The answer affects the injection
harness scope (FR-003) and whether a migration plan belongs in this spec's tasks.

**Q2 — Expected-output assertions: all `// =>` annotations or curated subset?** [FR-007, marked
`[NEEDS CLARIFICATION]`]

The docs site has many `// => "..."` and `# => ...` inline annotations. FR-007 says these SHOULD
be promoted to real assertions. The question is whether to promote **all** such annotations
automatically (the injection tool parses them and emits `assert_eq!` / `assert` / `expect().toBe`
for every one) or only a **curated subset** (the author marks which ones to assert, leaving
illustrative ones as comments). Automatic promotion is simpler and more complete; curated gives
more control over which values are contractual. The hash-field exemption (64-char hex, no
exact-match) applies in either case.

**Q3 — Per-version library-pin mechanism** [FR-006, SC-004, marked `[NEEDS CLARIFICATION]`]

The version-agnostic requirement (each docs version tests against its matching library version)
falls out structurally in architecture A if the frozen docs tree (spec 012) snapshots the source
tree: the sample source files in the snapshot reference the library version that was current when
that snapshot was taken (via Cargo.lock / requirements.txt / package-lock), so `cargo test` in
that snapshot tests against the right version automatically. But the explicit mechanism needs to
be confirmed:

- **Option A** (git-branch-implicit, architecture A): the frozen docs version branch carries its
  own `Cargo.lock` / `requirements.txt` / `package.json` lockfiles that pin the library version
  — no extra manifest needed. This is the minimal approach and is consistent with spec 012.
- **Option B** (explicit version manifest entry): a `docs/versions.json` (or equivalent) maps
  each docs version tag to a library SemVer range and the sample-test runner resolves the
  library version from it at test time.

Option A is preferred (simpler, consistent with C-05/git-owns-versioning), but confirmation from
the spec 012 owner is needed before locking this in FR-006.
