# Memory Synthesis

_(Direct-read fallback: `speckit_memory_*` MCP tools + `.spec-kit-memory/` SQLite cache absent this session.
Sources read: `.specify/memory/constitution.md` (v1.2.0), `docs/research/{roadmap.md, release-please-notes.md}`,
`docs/memory/INDEX.md`, the spec + its Clarifications, spec 011's plan/synthesis (coordinating feature), and
this session's release-please + docs work. Token banner N/A — no cache.)_

## Current Scope

Spec 012 — per-version documentation on the existing Astro/Starlight site (native content-collection +
dynamic-routing versioning; no platform migration, no third-party plugin) with a nav version dropdown.
Architecture is pre-decided + fixed: snapshot logic = a moon task (`docs:snapshot`, cacheable, locally
runnable, idempotent); release-please = version oracle only; CI = trigger glue (every release snapshots;
**minor** = new bucket + dropdown entry, **patch** = overwrite its minor bucket / roll up; dropdown lists
minors only). Affected areas: `docs/site/` (versioned content structure + dynamic `[version]/[...slug]`
route + version-dropdown component + manifest + freshness footer), a new `docs:snapshot` moon task,
`.github/workflows/{release.yml, docs.yml}` (trigger + multi-version publish), and coordination with spec
011's version-aware API-ref generator (drive it per snapshot). Pre-publish: builds the mechanism; first
real snapshot at the first released minor.

## Relevant Decisions

- **Clarify (Session 2026-06-29) — patch rolls up into its minor bucket** (Reason: defines the core snapshot
  behavior; Status: active; Source: spec ## Clarifications + FR-010/FR-016). Every release snapshots; minor
  creates a new bucket (`1.2`) + dropdown entry, patch (`1.2.1`) overwrites the `1.2` bucket. Dropdown =
  minors only. SemVer guarantees same API across patches → no per-patch bucket; but patch doc-fixes still
  reach that line. A freshness footer surfaces the exact patch (FR-018).
- **Clarify — keep all minors forever; unprefixed-latest + `/vX.Y/` pinned; non-latest noindex** (Reason:
  retention + URL + SEO; Status: active; Source: spec ## Clarifications + FR-015/FR-017). Bare path = latest
  (canonical); pinned under `/vX.Y/`; older versions noindexed.
- **release-please lockstep config is the version source** (Reason: 012 consumes its release events, does not
  modify it; Status: active — built this session; Source: `release-please-config.json` + `docs/research/release-please-notes.md`).
  release.yml already has a gated-off `publish` job + a noted SHA-pin TODO; the docs-snapshot trigger wires
  into that workflow on the release event.
- **Snapshot = moon task, not inline CI** (Reason: pre-decided layering; Status: active; Source: spec Context).
  Mirrors how codegen/schema-validation/conformance live as moon tasks; CI only triggers it.

## Active Architecture Constraints

- **Principle V — repo canonical; git owns versioning** (Reason: the LIBRARY has no managed version axis; this
  feature versions only the DOCS site, driven by git/release-please tags, not a library-internal store; Source:
  constitution). Docs versioning must not leak a version axis back into the library API.
- **Principle VII / C-07 — generated artifacts + freshness** (Reason: the snapshot freezes generated content
  incl. the per-version API refs; the snapshot task must be deterministic/idempotent like the codegen gate;
  Source: constitution + this session's gen-shape-table/codegen-check pattern).
- **Spec 011 coordination — version-aware API-ref generator** (Reason: the snapshot MUST capture per-version
  API refs, not latest-only — FR-012; Source: spec 011 plan, `gen-api-refs.mjs --version/--out`). The snapshot
  task invokes 011's generator with the version being frozen.
- **Existing docs deploy is GitHub Pages artifact-based** (Reason: docs.yml builds `pnpm -C docs/site build`
  on push to main + uploads a Pages artifact; 012 changes WHAT is published (multi-version) not the deploy
  mechanism — FR-011; Source: `.github/workflows/docs.yml`). NOTE: docs.yml pins pnpm 10.11.0 / node 22.12.0
  (distinct from mise's node 25.3.0) — keep the snapshot task runnable under both, or align deliberately.
- **Astro/Starlight stays; native versioning** (Reason: migration + the `starlight-versions` plugin were both
  rejected in prior discussion; Source: spec Assumptions). Use content collections + a dynamic `[version]`
  route + a custom dropdown component.

## Accepted Deviations

- None specific to 012. (Spec 011's "version-aware before second consumer" relaxation is what MAKES 012's
  per-version generation clean — 012 is that second consumer, so the relaxation is now justified in hindsight.)

## Relevant Security Constraints

- **No publishing enabled** (Reason: 'everything except publish' — FR-014/SC-007; Source: user directive +
  release.yml gated `publish` job). The docs-versioning trigger must not flip on package publishing; it only
  snapshots + deploys docs.
- **noindex for non-latest** (Reason: SEO duplicate-content + not strictly security, but a correctness
  guard on what crawlers surface — FR-017; Source: spec Clarifications). Pinned `/vX.Y/` pages excluded from
  indexing.

## Related Historical Lessons

- **Stale dev server / build cache served old HTML twice this session** (the derive-page + metadata-404
  regressions were stale-server, not source): a versioned site has MORE caching surface — ensure the snapshot
  + deploy invalidate cleanly and the dropdown manifest is read fresh.
- **Strict version pins are gated** (specs 002/004/005/009/011): pin any new site/build tooling EXACTLY
  (e.g. a routing/manifest helper) and record it; verify current versions before pinning.
- **Idempotence must be proven by a twice-run diff** (codegen-check pattern): the snapshot task's idempotence
  (FR-008/SC-004) is validated the same way — run twice, assert zero diff.
- **Commit via local `commit.gpgsign=false` toggle** (this session): the precommit gate false-flags the inline
  `-c` form as `--no-verify`.

## Conflict Warnings

- **No HARD conflicts.** 012 versions only the docs site (Principle V intact — library versioning stays git +
  release-please); it adds no library surface and no publishing.
- **Soft watch — node/pnpm version skew**: docs.yml runs node 22.12.0 / pnpm 10.11.0 while mise pins node
  25.3.0; the new `docs:snapshot` moon task + the dynamic route must work under the deploy's toolchain, not
  just locally. Resolve in the plan (align or explicitly support both).
- **Soft watch — 011 dependency**: 012's FR-012 assumes 011's `gen-api-refs.mjs --version/--out` exists. 011
  is specced + planned but NOT yet implemented. The plan must state whether 012 implementation BLOCKS on 011
  landing, or stubs the per-version API-ref call until 011 ships.
- **Soft watch — minor-vs-patch detection**: FR-016 computes it by comparing the released version to the
  previous release tag; the plan must define the exact comparison + the loud-fail path (FR-013) for
  unparseable/missing-previous cases.

## Retrieval Notes

- Index entries considered: `docs/memory/INDEX.md` (A1, D1) — neither directly relevant to docs versioning
  (A1 = loader layers, D1 = marshaling parity); omitted from constraints except as background. Governance read
  directly (constitution Principle V/VII; roadmap — no 012 entry yet). Live sources: this session's
  release-please config + docs.yml + spec 011 plan. Budget: under limits (4 decisions, 5 constraints, 0
  deviations, 2 security, 4 lessons). Full-memory-read not required.
