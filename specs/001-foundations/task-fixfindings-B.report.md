# task-fixfindings-B.report.md

Code-review findings fix report for spec 001 "Foundations" (agent B scope).
Branch: 001-foundations. Date: 2026-06-26.

---

## FIX 1 (CR-H1) — validate_fixtures.py: empty dir hard-errors

**File:** `schemas/jsonschema/scripts/validate_fixtures.py`

**Bug:** with `fixtures/valid/` or `fixtures/invalid/` empty the script printed
`Summary: 0/0 expectations met ALL PASS` and exited 0. Empty contract = fake pass.

**Change:** in `check_dir()`, count non-directory files after `sorted(directory.iterdir())`.
If zero, print an `ERROR:` message naming the required fixture class and call `sys.exit(1)`
immediately. The old `WARNING:` print is replaced entirely.

```diff
-    if not files:
-        print(f"  WARNING: no fixtures found in {directory}")
+    non_dir_files = [f for f in files if not f.is_dir()]
+    if not non_dir_files:
+        label = "valid" if expect_valid else "invalid"
+        print(f"  ERROR: no fixtures found in {directory} — at least one {label}/ fixture is required.")
+        sys.exit(1)
```

**Fail proof:**
```
$ mv schemas/jsonschema/fixtures/valid schemas/jsonschema/fixtures/valid.bak
$ mkdir schemas/jsonschema/fixtures/valid
$ mise exec -- uv run --with 'jsonschema==4.26.0' python3 schemas/jsonschema/scripts/validate_fixtures.py
  ERROR: no fixtures found in .../fixtures/valid — at least one valid/ fixture is required.
EXIT CODE: 1
```

**Restore-to-green:**
```
$ rmdir schemas/jsonschema/fixtures/valid
$ mv schemas/jsonschema/fixtures/valid.bak schemas/jsonschema/fixtures/valid
$ mise exec -- uv run --with 'jsonschema==4.26.0' python3 schemas/jsonschema/scripts/validate_fixtures.py
Summary: 10/10 expectations met  ALL PASS
EXIT CODE: 0
```

---

## FIX 2 (CR-H2) — codegen-check.sh: fails when generated file is deleted

**File:** `schemas/scripts/codegen-check.sh`

**Bug:** `git diff --exit-code -- <file>` returns 0 for a deleted-but-still-indexed file
(the unstaged diff only sees modifications, not unstaged deletions before `git rm`).
Deleting a generated file let the gate report `codegen-check PASSED`.

**Change:** added an existence pre-check loop before the `git add -N` + diff logic.
If any of the three tracked files is missing on disk the gate exits 1 immediately,
naming every missing file.

```diff
+# Assert every generated file EXISTS before checking for drift.
+FAILED=()
+for f in "${RS}" "${PY}" "${TS}"; do
+  if [[ ! -f "${f}" ]]; then
+    FAILED+=("${f} (file missing — not regenerated or accidentally deleted)")
+  fi
+done
+if [[ ${#FAILED[@]} -gt 0 ]]; then
+  echo "ERROR: codegen-check FAILED — the following generated files are MISSING:"
+  ...
+  exit 1
+fi
+
 # Register any newly-created (untracked) files so git diff can see them.
 git add -N "${RS}" "${PY}" "${TS}" 2>/dev/null || true
```

**Fail proof:**
```
$ mv crates/prompting-press/src/generated/prompt_definition.rs{,.bak}
$ bash schemas/scripts/codegen-check.sh
ERROR: codegen-check FAILED — the following generated files are MISSING:
  - crates/prompting-press/src/generated/prompt_definition.rs (file missing ...)
EXIT CODE: 1
```

**Restore-to-green:**
```
$ mv crates/prompting-press/src/generated/prompt_definition.rs{.bak,}
$ bash schemas/scripts/codegen-check.sh
codegen-check PASSED — all three generated files are up-to-date.
EXIT CODE: 0
```

---

## FIX 3 (CR-M2) — check-ffi-isolation.sh: crate preflight (exit 2 on typo)

**File:** `scripts/ci/check-ffi-isolation.sh`

**Bug:** `cargo tree -p <nonexistent> -i pyo3` falls back to the workspace-wide tree
and exits 0, so the gate incorrectly reported `<badcrate> depends on pyo3`.

**Change:** added a preflight loop over `COVERED_CRATES` that runs `cargo tree -p "$crate"`
before the FFI scan. If the crate is unknown: print `gate misconfigured — unknown crate ...`
and `exit 2` (distinct from the normal gate-failure exit 1).

```diff
+for crate in "${COVERED_CRATES[@]}"; do
+  if ! cargo tree -p "${crate}" > /dev/null 2>&1; then
+    echo "ERROR: gate misconfigured — unknown crate '${crate}' (not in workspace)."
+    echo "Fix COVERED_CRATES in $(basename "${BASH_SOURCE[0]}") or add the crate to the workspace."
+    exit 2
+  fi
+done
```

Also cleaned up the duplicated/garbled comment block at the `cargo tree -i` call site
(N3 — two consecutive comment blocks saying the same thing, collapsed to one).

**Fail proof (M2):**
```
$ sed 's/"prompting-press-core"/"nonexistent-crate"/' scripts/ci/check-ffi-isolation.sh | bash
ERROR: gate misconfigured — unknown crate 'nonexistent-crate' (not in workspace).
EXIT CODE: 2
```
(BASH_SOURCE artefact in pipe context is cosmetic; exit code 2 is correct.)

**Real script green:**
```
$ bash scripts/ci/check-ffi-isolation.sh
FFI-isolation gate PASSED.
  prompting-press-core: no pyo3, no napi in dependency tree
  prompting-press: no pyo3, no napi in dependency tree
EXIT CODE: 0
```

---

## FIX 4 (CR-M3 + N1) — check-floating-versions.sh: grep quiet + safe comment stripping

**File:** `scripts/ci/check-floating-versions.sh`

### N1 — grep stdout leak
All five `grep -En` calls printed matching lines to stdout (noisy in CI logs).
Replaced `-En` with `-Eq` (quiet) throughout. The script builds its own `FAILED[]`
summary; only the boolean exit status is needed.

```diff
-  if grep -En '"[\^~][^"]*"' "${tmp_scan}" 2>/dev/null; then
+  if grep -Eq '"[\^~][^"]*"' "${tmp_scan}" 2>/dev/null; then
```
(applied to all 5 pattern checks)

### CR-M3 — comment stripping: full-line only
Old sed: `'s/(^|[[:space:]])#.*$//'` — strips any whitespace-preceded `#`, which
would truncate a legitimate value containing ` #` (e.g. a future
`pkg @ git+https://host/repo.git#egg=x` or URL fragment anchor).

New sed: `'/^[[:space:]]*#/d'` — deletes only lines whose first non-whitespace
character is `#` (full-line comments). This is both necessary and sufficient:
the SEC-003 explanatory comments that caused the original false-positive are all
full-line comments.

```diff
-      scan_content="$(sed -E 's/(^|[[:space:]])#.*$//' "${manifest}")"
+      scan_content="$(sed -E '/^[[:space:]]*#/d' "${manifest}")"
```

Header comment updated to document scope accurately: covers npm/pipx/mise shorthand
floats; Cargo crate deps use semver ranges pinned by Cargo.lock (not by this lint).

**4 acceptance cases verified:**
```
CASE 1 (clean tree):   EXIT 0  — PASSED
CASE 2 (caret "^1"):   EXIT 1  — caught: caret or tilde range
CASE 3 (latest + *):   EXIT 1  — caught: literal 'latest'; wildcard '*'
CASE 4 (full-line #):  EXIT 0  — comment ignored, value "1.2.3" not flagged
```

---

## FIX 5 (CR-M1) — moon codegen inputs: add script + mise.toml

**Files:** `crates/prompting-press/moon.yml`, `packages/typescript/moon.yml`

**Bug:** moon only listed the schema as a `codegen` task input. Changing the
codegen script or pinned tool versions (mise.toml) would not invalidate the
cache, so moon could replay stale codegen output.

**Changes:**

`crates/prompting-press/moon.yml`:
```diff
     inputs:
       - '/schemas/jsonschema/prompt-definition.schema.json'
+      - '/crates/prompting-press/scripts/codegen.sh'
+      - '/mise.toml'
```

`packages/typescript/moon.yml`:
```diff
     inputs:
       - '/schemas/jsonschema/prompt-definition.schema.json'
+      - '/packages/typescript/scripts/codegen.mjs'
+      - '/mise.toml'
```

(packages/python/moon.yml is owned by the parallel agent and was not touched.)

---

## FIX 6 (L1) — schemas/moon.yml: pin jsonschema version

**File:** `schemas/moon.yml`

**Bug:** `uv run --with jsonschema` resolved the latest available jsonschema at
run time — floating, and ironic given SEC-003.

**Change:** pinned to `jsonschema==4.26.0` (current stable, verified via
`uv run --with jsonschema python3 -c "import jsonschema; print(jsonschema.__version__)"`
→ `4.26.0`) in both the `check-schema` and `validate-fixtures` task commands.

```diff
-    command: 'uv run --with jsonschema python3 jsonschema/scripts/meta_validate.py'
+    command: "uv run --with 'jsonschema==4.26.0' python3 jsonschema/scripts/meta_validate.py"

-    command: 'uv run --with jsonschema python3 jsonschema/scripts/validate_fixtures.py'
+    command: "uv run --with 'jsonschema==4.26.0' python3 jsonschema/scripts/validate_fixtures.py"
```

---

## FIX 7 — Cosmetics (N2, N3, L2, L3)

### N2 — Stale comments

`.moon/tasks/all.yml` header: removed "US3 will add it" (US3 has landed).
Replaced with an accurate note that `:codegen` is intentionally per-project
(different script + output path per crate/package), not a global inherited task.

`Cargo.toml` top comment: removed "member crates introduced in a later phase
(US1) — empty members today is intentional". Crates exist; replaced with a
concise accurate description.

### N3 — Duplicated comment block in check-ffi-isolation.sh

Lines ~47-51 restated the `cargo tree -i` exit-code behaviour twice in
back-to-back comment blocks. Collapsed to a single coherent comment.

### L2 — mktemp BSD/macOS portability in codegen.sh

`mktemp -t pp-typify-schema.XXXXXX.json` — on BSD/macOS the `.json` suffix is
appended literally (not substituted), so the template expansion is correct but
the `.json` suffix is misplaced (e.g. `pp-typify-schema.AbCdEf.json` vs
`pp-typify-schema.AbCdEfjson`).

Fix: use a plain `mktemp "${TMPDIR:-/tmp}/pp-typify.XXXXXX"` then `mv` the result
to `${base}.json`. Uniqueness is preserved; `.json` suffix is present for jq.

```diff
-TMP_SCHEMA="$(mktemp -t pp-typify-schema.XXXXXX.json)"
-trap 'rm -f "${TMP_SCHEMA}"' EXIT
+TMP_SCHEMA_BASE="$(mktemp "${TMPDIR:-/tmp}/pp-typify.XXXXXX")"
+TMP_SCHEMA="${TMP_SCHEMA_BASE}.json"
+mv "${TMP_SCHEMA_BASE}" "${TMP_SCHEMA}"
```

### L3 — Consolidated EXIT trap in codegen.sh

The original script set `trap '... TMP_SCHEMA ...' EXIT`, then later declared
`TMP_OUT` and overwrote the trap with `trap '... TMP_SCHEMA ... TMP_OUT ...' EXIT`.
This left a one-line window where `TMP_OUT` existed but was not covered by the
trap.

Fix: declare both `TMP_SCHEMA` and `TMP_OUT` near the top, before any trap is
set, and install a single cleanup trap covering both from the start.

```diff
+TMP_OUT="${TMPDIR:-/tmp}/pp-typify-out.$$.rs"
+# Single cleanup trap covering all temporaries.
+trap 'rm -f "${TMP_SCHEMA}" "${TMP_OUT}"' EXIT
 ...
-TMP_OUT="$(mktemp -t pp-typify-out.XXXXXX.rs)"
-trap 'rm -f "${TMP_SCHEMA}" "${TMP_OUT}"' EXIT
```

---

## Determinism verification

```
$ bash crates/prompting-press/scripts/codegen.sh
Generated crates/prompting-press/src/generated/prompt_definition.rs
$ git diff crates/prompting-press/src/generated/prompt_definition.rs
(empty)
$ bash crates/prompting-press/scripts/codegen.sh
Generated crates/prompting-press/src/generated/prompt_definition.rs
$ git diff crates/prompting-press/src/generated/prompt_definition.rs
(empty)
```

Byte-identical output across two runs after L2/L3 edits — determinism intact.

---

## Full gate sweep (clean tree)

```
moon run schemas:validate-fixtures   → 10/10 ALL PASS
moon run schemas:check-schema        → OK (cached)
moon run schemas:codegen-check       → PASSED (3/3 up-to-date)
moon run ci:check-ffi                → PASSED (prompting-press-core, prompting-press clean)
moon run ci:check-floating-versions  → PASSED (8 manifests OK)
```

---

## Summary

| Finding | Fix | Hardened gate proven |
|---------|-----|----------------------|
| CR-H1 | validate_fixtures exits 1 on empty valid/ dir | FAIL on empty dir → restore → 10/10 PASS |
| CR-H2 | codegen-check exits 1 on missing generated file | FAIL naming missing RS → restore → PASSED |
| CR-M2 | FFI gate exits 2 on unknown crate (not 0/phantom) | exit 2 + "gate misconfigured" message |
| CR-M3 | Full-line-only comment stripping (safe for `#` in values) | 4 acceptance cases verified |
| N1 | grep -Eq (quiet) — no stdout leak | verified no output on passing manifests |
| CR-M1 | codegen inputs include script + mise.toml | both moon.yml files updated |
| L1 | jsonschema pinned to 4.26.0 in schemas/moon.yml | both task commands updated |
| N2 | Stale comments in all.yml + Cargo.toml | updated to reflect current state |
| N3 | Deduped comment in check-ffi-isolation.sh | single coherent comment |
| L2 | Portable mktemp in codegen.sh | determinism verified |
| L3 | Single EXIT trap in codegen.sh | no window between declarations |
