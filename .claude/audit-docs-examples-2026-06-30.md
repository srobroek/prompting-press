# Docs + Examples Audit — Prompting Press (2026-06-30)

Method: 36-agent adversarially-verified workflow (`wumaonav8`, 2.27M tokens) + main-thread
spot-checks against live source. Each finding below carries a verifier verdict:
**confirmed** / **partially_confirmed** (correct substance, evidence sharpened).

The docs site is **Astro + Starlight**, NOT Docusaurus. The "migrate to Docusaurus" idea was
researched in the prior (dropped) session and **rejected** — Starlight versions natively. This
audit is of the existing Astro site. No migration is in scope.

---

## Spec status (what's done vs open)

| Spec | Title | tasks.md | Reality |
|------|-------|----------|---------|
| 010-docs-site | Astro/Starlight docs site | **3/17 checked** | Site is built & substantial — checkboxes are **STALE**. T004–T017 work (index, getting-started, guides, reference, templates, README slim, CI deploy) demonstrably landed but is unchecked. Spec marked "Draft". |
| 011-autogen-api-refs | Auto-generated API refs | 20/21 | Implemented & CI-gated (`gen-api-refs.mjs` + `check-api-refs-fresh.sh`). Reference pages ARE built from live code. |
| 012-docs-versioning | Snapshot-per-minor + dropdown | 19/19 | Mechanism implemented; **but see Concern 1** — "latest" routing conflates 0.1 with main/next. Spec marked "Draft". |
| 013-unsafe-render-detail | Opt-in unsafe render detail | 0/11 | The `reveal_render_detail` 4th arg IS in live code (so partly implemented in the library), but tasks unchecked. |
| 014-tested-doc-samples | Tested doc samples + consumer apps | branch only | **UNMERGED, UNIMPLEMENTED.** Lives only on local branch `014-tested-doc-samples` (5 commits, ahead 5 / behind 23, on NO remote). This spec is the home of Concerns 2, 3, 5, 6, 7. |
| 015-guard-delimiting | Guard delimiting redesign | 0/0 | Implemented on `origin/015-guard-delimiting-impl` + partly on main (guard.mdx reflects the new `<untrusted>` model). |

**Key insight:** Concerns 2/3 (build-from-code + sample tests) and much of 5/6/7 are essentially
**"spec 014 is not implemented."** That spec already prescribes the exact fix: source-canonical
samples injected at build, auto-promoted output assertions, per-version pinning via the frozen
tree, and one tested consumer app per language under `samples/`.

---

## The 9 concerns

### 1. "Latest" must point to latest RELEASED version, not main/next — **VIOLATED** (verdict: confirmed)
- `VersionSelect.astro:51-57` — `hrefFor()` routes BOTH the `isLatest` (0.1) entry AND the `next`
  (main) entry through the same branch → identical bare canonical URL. The two dropdown options
  are byte-identical links.
- `content.config.ts:7-8` — the Starlight `docs` collection loads from `src/content/docs/` (the
  working tree that tracks main/next), not the frozen `src/versions/v0.1`. With `base:"/"` and no
  redirects, the root serves working/main content under the "0.1 (latest)" label.
- `versions/README.md:16` — design intent stated verbatim: "Latest docs stay at the unprefixed
  canonical paths in src/content/docs/." So this is working-as-designed, but the design conflates
  "latest" with "main/next".
- Verifier corrections (both make it WORSE): (a) the trees are NOT byte-identical — the v0.1
  snapshot has an extra `changelog.mdx` the working tree lacks; (b) `/v/0.1/` currently generates
  **zero pages** (getStaticPaths filters out `isLatest`), so the released 0.1 snapshot is
  effectively **unreachable** — there is no distinct URL serving released-0.1 docs at all.
- Maps to prior task: **"flip default version to 0.1 at root."**

### 2. Examples built/integrated from code, not static — **PARTIAL** (verdict: partially_confirmed)
- Reference pages: genuinely built from live code, CI-gated. ✅
- getting-started + guides + templates: **104 hand-typed static fenced blocks**, no build-time
  sourcing, no `<Code file=>`/`?raw` import mechanism anywhere, no `examples/` dir.
- `getting-started/rust.mdx:6` imports Starlight `<Code>` but never uses it — dead import,
  signals an abandoned intent.
- This is the spec-014 gap (source-canonical migration of all ~108 blocks).

### 3. All examples/samples have unit tests — **VIOLATED** (verdict: confirmed)
- **No harness anywhere** compiles/runs/type-checks doc code samples (~251 fenced blocks: 89 rust,
  87 ts, 75 python). Library has its own solid tests; the doc EXAMPLES have none.
- `docs.yml:73` runs only `astro build`. `justfile:5` — `test:` is a TODO stub.
- `package.json:15` references a `check-stale-surface.mjs` that **does not exist on disk**.
- No `samples/` consumer apps exist. This is the spec-014 deliverable.

### 4. Cross-language parity + 3-lang selector — **PARTIAL** (mixed; see per-page below)
- Getting-started & reference are per-language files **by design** (not a defect). Guides use
  `<Tabs syncKey="construct">` with all 3 languages and DO render a selector.
- Pages with **no** language tabs (by design, but worth confirming intent): `index.mdx` (prose
  landing), `faq.mdx` (prose Q&A, Rust-flavored API notation only), `templates.mdx` (jinja-only;
  constructor/error names are Rust-first), `reference/prompt-definition.mdx` (auto-generated).

### 4b. Intra-page CONTENT parity (user-flagged: guard Rust tab) — **VIOLATED** (verdict: confirmed/partial)
Real per-language content gaps WITHIN tabbed pages:
- **guard.mdx** (the user's example, CONFIRMED): Rust tab is missing — (a) the non-conforming
  override → `PromptRenderError` rejection example that Python/TS both show; (b) concrete input
  data in the success example; (c) a compiling render call (3-arg, see Concern 7); and the
  "Where to route the guard text" section shows message assembly **only in Python**.
- **composition.mdx**: Rust missing `from_messages` bulk-constructor example; failed-append +
  non-default-variant-in-composition shown only in Python.
- **lint-in-ci.mdx**: TS missing construction-failure handling; Rust missing `Finding.kind` demo.
- **metadata.mdx**: Rust missing concrete `# => {...}` output annotations (Python/TS have them).
  (Verifier downgraded these to stylistic — full 3-lang coverage exists; asymmetry is pedagogical.)
- **variants.mdx**: Rust render omits the 4th arg; Rust closes with a different read op than Py/TS.
- **getting-started triplet**: TS RenderResult table missing the `name` row; Python missing the
  "From a plain dict" construction route Rust/TS-equivalents have; Rust Step-1 prose omits the
  malformed-doc/LoadError case Python/TS mention.
- **reference triplet**: Python missing `Composition.len()/is_empty()` docs (Rust/TS have them);
  TS render() doc missing the unsafe-render-detail opt-in note. (Auto-generated → fix at generator.)

### 5. Examples end-to-end complete (no undefined "Ask" prompt) — **VIOLATED** (verdict: confirmed)
- **guard.mdx (high):** every render snippet uses `ask` (Prompt), `Ask` (Vars schema), `vars`
  (Rust struct) — **none ever defined/constructed on the page**. The `ask.yaml` shown late is
  never wired into a `Prompt`. This is EXACTLY the user's "starts with an Ask prompt without
  defining it." Duplicated verbatim in the v0.1 snapshot.
- **composition.mdx (medium x3):** Python uses `Field(...)` without importing it; uses JS/YAML
  `true`/`false` instead of Python `True`/`False` (lines 108 & 114); `from_messages` selects
  variant `"detailed"` on a prompt declaring no variants → `unknown_variant` at resolve.
- **variants.mdx / derive.mdx (low):** reference `vars`/`Vars`/`data`/`yaml_text`/`GreetVars`
  never defined on the page.

### 6. Examples show FULL functionality, not cut halfway — **PARTIAL** (verdict: confirmed)
- **variants.mdx:** the core "select a variant at render" example shows only the resolved
  `.variant` NAME, never the rendered `.text` body — never reaches the payoff that proves a
  variant changes output. (`.text` count on page = 0.)
- **composition.mdx:** `from_messages` example truncates at `resolve()` with no shown output AND
  would raise `unknown_variant`.
- Getting-started (x3), guard, derive, metadata, lint-in-ci DO reach a meaningful end result.

### 7. Examples validated against live code for that version — **VIOLATED** (verdict: confirmed)
- **Every Rust sample** (working tree AND shipped v0.1 snapshot) calls `render` with the stale
  **3-arg** signature; live `render` is **4-arg** (`reveal_render_detail: bool`, no default) —
  verified at `crates/prompting-press/src/prompt.rs:234-240`. **Does not compile**, even against
  the released `prompting-press-v0.1.0` tag.
- `guard.mdx:141` — `GuardConfig { enabled: true }` omits the required `advisory` field (E0063;
  `GuardConfig` is not `#[non_exhaustive]`, fields at `origin.rs:54,65`). (Note: `GuardConfig::default()`
  IS valid — so getting-started's only defect there is the render arity.)
- Python & TypeScript samples are accurate.
- Root cause: no CI step compiles/typechecks hand-written samples (only the auto-gen refs).

### 8. Stale worktrees / dangling code — **PARTIAL** (verdict: confirmed) — mostly DONE
- Worktrees clean: `synchronous-gathering-tiger` holds 0 unmerged commits, no uncommitted files;
  main checkout's only change is the intended `rm-rf-guard.sh` fix. Prior 3 stale worktrees were
  already removed.
- **At-risk (high):** `specs/014-tested-doc-samples/` (816 lines, 5 commits) lives ONLY on local
  branch `014-tested-doc-samples`, pushed to **no remote** — this clone is the only copy.
- **Low:** orphaned `stash@{0}` ("WIP on 011-autogen-api-refs") hand-edits a now-auto-generated
  file; base is stale → effectively dead. Local `015-guard-delimiting` has 1 unpushed but
  superseded commit. ~28 local branches, many merged-but-undeleted.

---

---

## Additional dimensions (scanned 2026-06-30, adversarially verified)

### C. Greenfield / "docs-are-product" voice — VIOLATIONS FOUND (verdict: partially_confirmed)
Three classes, all duplicated in working tree + v0.1 snapshot:
- **(a) Pre-release voice (hand-written, high):** all 3 getting-started pages carry a
  `<Aside title="Pre-release: install from source">` ("not published to PyPI/npm/crates.io yet…
  Until the first release… once published"). Fix: delete the asides; the published install tabs
  above them are the real path. **Also a broken instruction:** `rust.mdx:14` pins
  `prompting-press = "1"` (resolves `>=1.0.0,<2.0.0`) — won't match the `0.1.0` release; must be `"0.1"`.
- **(b/c) Archaeology + process IDs (auto-generated, medium×~14):** the reference pages are
  saturated with internal artifacts pulled from **source doc-comments + the JSON schema**:
  spec numbers (`spec 002`), task IDs (`T045/T046`), decision/critique/US/FR/SC/SEC IDs
  (`critique E1`, `C-01`, `C-11`, `US3`, `[FR-022]`, `D2`, `SEC-004`, `F5/F7`, `R7/Q4`, `CR-1`,
  `TY-4`), plus prose archaeology: "pre-reshape `render(reg,…)` path", "Retained from the spec-001
  stub", "Trivial placeholder", "Replaces the former `origin` enum".
- **Root cause + systematic fix:** `docs/site/scripts/lib/strip-jargon.mjs` (`ln()`) already strips
  *parenthetical* `(FR-/SC-/SEC-/C-NN/spec/Principle)` citations, but **misses** bracketed `[FR-022]`,
  bare IDs (`T045`, `US3`, `F5`, `critique E1`, `D2`, `R7/Q4`, `CR-1`, `TY-4`), and whole
  archaeological **sentences**. Fix = (1) extend `ln()` to catch the missing ID forms, (2) hand-fix
  the archaeological **sentences** at their source doc-comment / the JSON schema (an ID-strip leaves
  the sentence; these sentences must be rewritten/removed), (3) regenerate the reference pages.
  Hand-written `index/faq/templates/guides` are otherwise clean.

### D. Constitution v2.0.0 consistency — MOSTLY CLEAN, 2 residues (verdict: confirmed)
Docs correctly use the `trusted` boolean, fixed `<untrusted>` delimiters, advisory-only override,
guard-on-mutates-body framing, no version axis, no `vars_hash`. Residues:
- `templates.mdx:118` (+ v0.1 copy): "an `untrusted`/`external` variable…" — `external` is the
  removed 3-value-enum value; → "an untrusted (`trusted: false`) variable".
- `reference/prompt-definition.mdx:30` (auto-gen): "input-trust **origin**" — stale `origin` term;
  fix at `schemas/jsonschema/prompt-definition.schema.json:26` + regenerate.

### E. Removed-surface references — VIOLATIONS FOUND (verdict: confirmed)
Live surface is the immutable `Prompt` object; no `Registry`/`vars_hash`/`version=`/`.chain()`/
eliminated pluggable interfaces (all grep-clean). But auto-generated reference pages carry stale
**Registry** prose from source doc-comments:
- `reference/typescript.mdx:295` (auto-gen): CI example calls removed free-fn `check(reg)` →
  fix `crates/prompting-press-node/src/check.rs:55` to `prompt.check().passed()`.
- `reference/python.mdx:172` (auto-gen): "mirrors the **current** module-level `render(reg, name…)`"
  — that surface is removed; fix `crates/prompting-press-py/src/prompt.rs:288`.
- `reference/typescript.mdx:329` "registry name" → "name"; `reference/rust.mdx:49` "registry render
  paths" (low). All fix-at-source-doc-comment + regenerate.

### Completeness critic — full requirement inventory
All 8 goal points + 4b + spec-status + greenfield(C) + constitution(D) + removed-surfaces(E) +
handover items are **covered by a scan**. The only non-scanned items are correctly **actions/gates**,
not scan targets: **"publish docs live for 0.1.0"** (a deploy step, runs after fixes) and the
**adversarial-challenger agreement** (the acceptance gate, already satisfied for the audit). Two
handover items have only partial standalone coverage: H2 (Starlight versioning-plugin decision —
resolved by spot-check: native `@astrojs/starlight` + custom `VersionSelect.astro`, no 3rd-party
plugin) and H4 (guard-metadata-YAML schema-validity — only spot-checked).

---

## Recommended fix ordering (NOT yet done — awaiting direction)
1. **Push `014-tested-doc-samples` branch** (protect the only copy) — trivial, removes data-loss risk.
2. **Concern 7 (Rust compile breaks)** — correct render arity + GuardConfig literal in working tree
   AND re-snapshot v0.1. Highest user-visible severity (shipped docs don't compile).
3. **Concern 5 (guard.mdx undefined Ask) + composition.mdx Python errors** — make examples runnable.
4. **Concern 1 (latest→0.1 routing)** — decide: redirect root to /v/0.1, or accept "latest=working".
5. **Concern 4b parity gaps** — backfill Rust/missing-lang content.
6. **Concerns 2/3/6 (build-from-code + tests + full-functionality)** — implement spec 014 via SpecKit
   (large; source-canonical migration + consumer apps + CI gate).
