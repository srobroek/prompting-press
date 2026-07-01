# Open questions & ambiguities — autonomous run 2026-06-30

User away ~8h; working autonomously on Tier 3 then Tier 4. These need a decision when back.
Default choices I made are marked **[default: …]** — flag any you'd reverse.

## Cross-cutting

- **Q1 — Commits / PR.** Nothing has been committed or pushed (commits/PRs are outward-facing
  and I don't have standing authorization). All work sits in the working tree on
  `fix/rm-rf-guard-bash32`. **[default: leave uncommitted for review]** — but that branch is
  misnamed for this work (it was the rm-rf-guard hook fix). When back: decide branch/commit/PR
  strategy. NOTE: the audit work is large and unrelated to the branch name — likely wants its own
  branch + PR (or several: docs-audit-fixes, spec-012-iteration, spec-014).
- **Q2 — `fix/rm-rf-guard-bash32` still carries the original hook fix** (`rm-rf-guard.sh`) as the
  one pre-existing uncommitted change. It's now mixed with the entire docs audit. Probably split:
  the hook fix is its own concern (the resume-session thread that started all this).

## Tier 3 — dropdown freshness DECIDED: rebuild-all every deploy (Option 2)

**RESOLVED 2026-06-30 by user** (criterion: "if rebuild isn't long, 2 [fewest moving parts] is fine"):
Measured a bare frozen-version `astro build` = ~9s wall (prebuild/codegen SKIPPED for frozen trees —
refs already baked in `src/versions/vX.Y/`). So **rebuild EVERY version on EVERY deploy** from the
current manifest. Old versions' dropdowns are therefore baked fresh and include newer versions
released later — solving "older pages must show newer versions" with ZERO runtime JS / fetch /
fallback. Docusaurus-style. Cost: 1 codegen-heavy `next` build + N×~9s frozen builds (≈20–30s now,
≈90s at 10 versions — acceptable). No `next.slugs` JS concern; dropdown is static from the manifest
in every build. (Supersedes the hybrid/runtime-fetch options.)

## Tier 3 — ARCHITECTURE DECIDED (Model C: per-version whole-site builds)

**RESOLVED 2026-06-30 by user**: Build the ENTIRE native Starlight site once PER VERSION, each
under its own Astro `base` prefix, assembled into one deploy dir, with a thin redirect at root.
- `dist/index.html` = meta-refresh + canonical → `/v{latest}/` (latest from versions.json; GH Pages
  can't 301, so meta-refresh is the static ceiling).
- `dist/next/` ← build `src/content/docs/` (main) with `base:/next/`.
- `dist/v0.1/`, `dist/v0.2/`, … ← build each `src/versions/vX.Y/` with `base:/vX.Y/`.
- Each build is FULLY NATIVE Starlight (native sidebar, native per-version Pagefind search, native
  routing) — no StarlightPage custom routes, no `<pre>`, no content relocation, no empty-collection hack.
- Mechanism: an orchestration script loops versions, stages each tree into `src/content/docs/`
  (docsLoader is hardcoded there), runs `astro build --outDir dist/<prefix>` with `base`, restores,
  writes root redirect. The existing docs.yml publish step carries the assembled `dist/` over unchanged.
- Confirmed feasible: Astro `base` prefixes whole site + assets; `astro build --outDir` exists;
  Pagefind indexes each build's own dir → correct per-version search.
- **SUPERSEDES the StarlightPage iteration I applied earlier** — re-iterating spec 012 to this model
  (FR-017 → root redirects to latest, every version prefixed; FR-019 → each version is an independent
  full Starlight build; FR-020 banner baked into each non-latest build).
- Build time scales ×N versions — fine for a handful; **[default: accept]**.

### (prior, now superseded) Tier 3 — ARCHITECTURE STILL IN FLUX (decision needed)

- **Q-ARCH (the big one)** — Two candidate models for "root shows latest released":
  - **Model A (chosen in the iterate, (d)+(c))**: make `src/content/docs/` the RELEASED tree served
    natively at bare `/`; author main/next elsewhere, served at `/next/` via StarlightPage. Inverts
    the authoring model (where you edit in-progress docs changes).
  - **Model B (user's newer idea, 2026-06-30)**: EVERYTHING prefixed — `/v0.1/…`, `/next/…` — and
    bare `/` is ONLY a redirect to the latest version (manifest-driven, auto-tracks latest), or a
    dropdown-driven chooser defaulting to latest. Docusaurus-style; symmetric; no version privileged
    at root. CONTRADICTS current FR-017 ("latest at the unprefixed bare path") → needs a spec tweak.
  - Research agent (ac08120412…) is enumerating Model B feasibility on Starlight 0.41.1 + comparing.
    **If Model B is chosen, the spec-012 iteration FR-004/FR-017 wording I just applied needs
    revising** (bare path = redirect, not canonical content). I will re-iterate the spec if so.
  - **[default if user stays away: pick the research agent's recommended option, implement it,
    log which was chosen]**. Both keep the manifest/snapshot machinery + full chrome + manifest-driven
    latest; they differ in whether root hosts content or just redirects.

## Tier 3 (spec 012 iteration — StarlightPage + root=released + banner)

- **Q3 — Pagefind search scoping.** Research R9 flagged that `StarlightPage` pages on `/next/` and
  pinned `/vX.Y/` aren't auto-indexed by Starlight's Pagefind the way the `docs` collection is.
  **[default: scope search to the canonical `/` only]** (simplest, correct for SEO since non-latest
  is noindex anyway). If you want per-version search, that's extra wiring.
- **Q4 — Pre-release root content.** 0.1 is a real released tag, so `/` = released 0.1. The 0.1
  snapshot was backfilled from main and TC07 re-snapshots it from the *corrected* main, so `/` and
  `/next/` render near-identically until main diverges. **[default: accept]** — expected pre-1.0.
- **Q5 — `/next/` vs keeping next at a `/v/next/`-style path.** Spec iteration says `/next/`.
  **[default: `/next/`]** as the clean channel for main.

## Tier 4 — BUILDABILITY CONSTRAINT (affects verification scope)

- Only **Rust** is locally buildable right now: cargo 1.95.0 present; the consumer crate is pure Rust.
- **Python** extension is NOT built (no `.so` in packages/python; `maturin` not on PATH though mise
  can provide it — the Tier-2 coder ran `maturin develop` successfully earlier, so it's achievable).
- **TS** napi addon is NOT built (no `.node`; buildable via `pnpm -C packages/typescript run
  build:addon`, which the Tier-2 coder also ran).
- **Decision for autonomous run**: write all three consumer apps (code is the deliverable), fully
  verify the **Rust** app (`cargo run`/`cargo test`); for **Python/TS**, build the extension +
  verify IF it builds cleanly in reasonable time, else write against the known-correct API surface
  (from the reference pages / bindings) and mark verification PENDING here. Do NOT fake test output.
- Cargo `members=["crates/*"]` will NOT auto-include `samples/rust/*` → I add a workspace `members`
  entry (or a separate exclude) myself. No pnpm-workspace.yaml yet → I create one incl. samples/ts.

## Tier 4 — STATUS: WU-C DONE+VERIFIED; WU-A/B scoped-not-built (deliberate slice)

**DONE + verified in the main tree (2026-07-01):** WU-C — the three consumer sample apps under
`samples/{rust,python,typescript}/greeter-cli/`, each walking the FULL FR-014 feature surface
end-to-end with passing tests: **Rust 13/13** (`cargo test`), **Python 15/15** (`pytest`),
**TypeScript 13/13** (`node --test`). Registered: Cargo `members` += `samples/rust/*`; new root
`pnpm-workspace.yaml` (+ `samples/typescript/*`); Python app is a `uv` editable dep on packages/python.
`samples/README.md` documents the launch-flip (FR-019). This is the highest-value spec-014 deliverable
(proves "examples show full functionality end-to-end + are tested" — concerns 3/6).

**NOT built (deliberately scoped out for this autonomous run — too large to finish cleanly):**
- **WU-A (T001–T007)**: the doc-sample INJECTION harness + coverage audit — migrating the ~108
  existing MDX code blocks into source-canonical tested files injected at build. This is the big one
  (marker grammar, classifier, inject-samples.mjs, per-language runners, then the 108-block migration).
- **WU-B (T010–T011)**: auto-promote `// =>` output annotations to assertions in those migrated sources.
- **WU-D partial**: `samples:test` moon gate + ci.yml wiring (T017/T021) — the apps are verified
  manually but NOT yet wired into a CI gate. **[TODO next]** add a moon `samples:test` task (3 legs)
  + a ci.yml job so a consumed-API break fails the PR.
**Why sliced:** WU-A's 108-block migration is a multi-hour mechanical effort with lower value-density
than the consumer apps (which alone satisfy "end-to-end tested examples"). Tier-1 already fixed the
doc-sample CORRECTNESS defects (4-arg render, undefined Ask, etc.) directly. Recommend WU-A/B as a
follow-up spec-014 implementation pass. The spec-014 branch (pushed) holds the full plan/tasks.

## Tier 4 (spec 014 — tested doc samples + consumer apps) — original notes

- **Q6 — Scope/size.** Spec 014 is large (migrate ~108 MDX blocks to source-canonical tested
  files + 3 consumer apps + CI gate). Implementing it fully is a multi-hour effort. **[plan: drive
  via SpecKit agent-assign; checkpoint between phases]**. If it proves too large to finish cleanly
  autonomously, I'll implement the highest-value slice (the doc-sample test harness + the Rust/Py/TS
  consumer apps that prove the API end-to-end) and leave the full 108-block migration tracked.
- **Q7 — 014 branch.** Spec 014's spec/plan/tasks live on `014-tested-doc-samples` (now pushed).
  Implementing on the current working tree vs that branch is a branch-strategy question tied to Q1.

## Resolved this session (for reference)
- Docusaurus migration → NOT happening (Astro/Starlight stays; research concluded).
- Context7 key → fixed via chezmoi (Option C); future `claude`-wrapper launches auto-resolve.
- starlight-versions → rejected (R10): stability + inverted root model + would reverse a spec decision.
