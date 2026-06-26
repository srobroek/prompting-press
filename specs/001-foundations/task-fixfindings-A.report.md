# Fix-Findings A — Security Hardening Report
## Spec 001 "Foundations" — HIGH/MEDIUM findings

Agent: coder (subagent)
Branch: 001-foundations
Date: 2026-06-26

---

## Summary

All 7 fixes landed. Python codegen output changed by exactly one comment line
(the header now correctly names the uv.lock path instead of `mise`). The new
hash is stable across two consecutive runs. The freshness gate passes.

---

## FIX 1 (Sec H-2a) — cargo-typify --locked

**File:** `mise.toml`

**Diff:**
```diff
-"cargo:cargo-typify" = "0.7.0"                  # Rust JSON-Schema -> serde structs (T022/T025)
+"cargo:cargo-typify" = { version = "0.7.0", locked = true }  # ...
```

**Verification:**
- `mise install` → `all tools are installed`
- `mise exec -- cargo typify --version` → `cargo-typify 0.7.0`

---

## FIX 2 (Sec H-2b) — Move datamodel-code-generator off pipx, onto uv

**Files:** `mise.toml`, `packages/python/scripts/codegen.sh`

**mise.toml diff:**
```diff
-"pipx:datamodel-code-generator" = "0.65.1"      # Python JSON-Schema -> Pydantic v2 (T020/T023)
-"pipx:maturin" = "1.14.1"                        # PyO3 cdylib -> Python wheel (T009 + Python build)
+# datamodel-code-generator is managed via uv (packages/python/uv.lock, dependency-group codegen)
+# maturin is declared as a build-system requires in packages/python/pyproject.toml (>=1.14,<2.0);
+# the wheel build (spec 004/007) will resolve it via uv/build-system at that time.
```

**codegen.sh invocation diff:**
```diff
-mise exec -- datamodel-codegen \
+uv run --project packages/python --group codegen --no-sync datamodel-codegen \
```

### Maturin judgment call

maturin was removed from `pipx:` in `mise.toml`. Rationale: maturin is already
declared as a `build-system.requires` entry in `packages/python/pyproject.toml`
(`maturin>=1.14,<2.0`). No maturin invocation occurs anywhere in spec 001 CI
(only T009 "verified available" — which is satisfied by pyproject.toml declaring
it). When the wheel build lands (spec 004/007), maturin will be resolved by uv
at build time via the build-system mechanism. Removing the pipx pin eliminates
the unverified redundant mechanism the H-2 finding flags without breaking
anything in 001.

If the wheel build spec needs a specific pinned maturin version before uv
resolves it, the approach is `uv run --with maturin==1.14.1 maturin ...` at that
point, or adding maturin to a `[dependency-groups] build` group in pyproject.toml.
The current `>=1.14,<2.0` bound in build-system.requires is acceptable under
SEC-003 (bounded ranges are explicitly carved out; only `^`/`~`/`latest`/`*`
shorthands are prohibited).

---

## FIX 3 (Sec H-1) — Make CI hermetic

**File:** `.github/workflows/ci.yml`

**Diff:** Two steps added in the `gates` job before the gate tasks:

```yaml
- name: 'Install TS deps (pnpm)'
  run: mise exec -- pnpm -C packages/typescript install --frozen-lockfile

- name: 'Install Python codegen deps (uv)'
  run: mise exec -- uv sync --project packages/python --group codegen --no-install-project --frozen
```

The TS step ensures `json-schema-to-typescript` is present in `node_modules`
(gitignored, absent on cold checkout) before `schemas:codegen-check` regenerates
the TS shape. The Python step syncs the hash-locked codegen group so `codegen.sh`
can run `uv run --no-sync` hermetically.

---

## FIX 4 (Sec M-1) — Least-privilege token

**File:** `.github/workflows/ci.yml`

**Diff:**
```yaml
+permissions:
+  contents: read
```

Added at top-level (before `jobs:`). Restricts the default GITHUB_TOKEN to
read-only for all jobs. No job in this workflow writes or deploys.

---

## FIX 5 (Sec M-2) — cargo --locked in build job

**File:** `.github/workflows/ci.yml`

**Diff:**
```diff
-run: mise exec -- cargo build --workspace
+run: mise exec -- cargo build --workspace --locked
```

Fails fast if `Cargo.lock` is stale rather than silently regenerating it.

---

## FIX 6 (Sec I-1) — Fix comment drift in codegen.sh

**File:** `packages/python/scripts/codegen.sh`

Header comments now accurately describe what the script does (invokes
`uv run --no-sync` after the caller has pre-synced the locked group). Removed
the aspirational `mise exec -- uv sync` example that described a different
invocation path and was never on the execution path.

Also updated `packages/python/pyproject.toml` codegen group comment to remove
the stale reference to `mise.toml` as the "dev source of truth" (it is now the
pyproject.toml itself) and correct the CI install command to match the actual
`--project` flag form.

---

## FIX 7 (CR-M1) — moon codegen inputs

**File:** `packages/python/moon.yml`

**Diff:**
```diff
 inputs:
   - '/schemas/jsonschema/prompt-definition.schema.json'
+  - '/packages/python/scripts/codegen.sh'
+  - '/mise.toml'
```

Changing the codegen script or tool version now invalidates moon's cache for the
`codegen` task, preventing stale cached output from masking the change.

---

## Verification Results

### V1 — mise install
```
mise all tools are installed
cargo-typify 0.7.0  ✓
```

### V2 — Python codegen determinism
Run 1 hash: `edb99a80ebf115afc054d2ff1bf3b6f7297406135fd482d261de8088dda948bd`
Run 2 hash: `edb99a80ebf115afc054d2ff1bf3b6f7297406135fd482d261de8088dda948bd`

Stable across two runs. Changed from pre-fix `11f892...` by exactly one comment
line in the generated file header:
```diff
-# by datamodel-code-generator (pinned via mise / packages/python uv.lock).
+# by datamodel-code-generator (pinned via packages/python/uv.lock, group codegen).
```
This is the correct propagation of the FIX 6 comment correction into the
generated artifact. The generated file was updated to commit the new hash.

### V3 — Freshness gate
```
schemas:codegen-check | codegen-check PASSED — all three generated files are up-to-date.
  crates/prompting-press/src/generated/prompt_definition.rs
  packages/python/python/prompting_press/generated/prompt_definition.py
  packages/typescript/src/generated/prompt-definition.ts
Tasks: 4 completed (3 cached)
```

### V4 — ci.yml YAML validity
```
yaml ok
```

### V5 — Hermetic uv sync against committed uv.lock
```
uv sync --project packages/python --group codegen --no-install-project --frozen
Checked 22 packages in 38ms  ✓
```

---

## Files Changed

| File | Fixes |
|---|---|
| `mise.toml` | FIX 1, FIX 2 |
| `.github/workflows/ci.yml` | FIX 3, FIX 4, FIX 5 |
| `packages/python/scripts/codegen.sh` | FIX 2, FIX 6 |
| `packages/python/pyproject.toml` | FIX 6 (comment correction) |
| `packages/python/moon.yml` | FIX 7 |
| `packages/python/python/prompting_press/generated/prompt_definition.py` | regenerated (comment propagation) |

No files outside owned scope were touched.
