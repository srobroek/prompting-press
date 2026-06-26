#!/usr/bin/env bash
# T030a — Floating-version lint (SEC-003).
#
# Rejects floating version specifiers (^, ~, "latest", "*") in the manifests
# that govern the codegen/tool chain. The current tree is clean — this script
# acts as a regression guard.
#
# Scoped manifests (anything else, e.g. lockfiles, is EXCLUDED):
#   mise.toml
#   packages/*/package.json          (devDependencies — the codegen/napi tools)
#   packages/*/pyproject.toml        (build-system requires + dependency-groups)
#   crates/*/Cargo.toml              (crate manifests)
#   Cargo.toml                       (workspace manifest)
#
# IMPORTANT nuances:
#   - `maturin>=1.14,<2.0` in pyproject.toml is a BOUNDED range, NOT a floating
#     specifier (SEC-003 targets the shorthand floats). Do NOT flag `>=x,<y`.
#   - Lockfiles (pnpm-lock.yaml, uv.lock) are intentionally EXCLUDED.
#   - The check is for literal `^`, `~`, `"latest"`, and `"*"` as version values
#     (e.g. `"^1.0.0"`, `~1`, `= "latest"`, version = "*").
#   - Cargo crate dependencies use semver ranges (e.g. serde = "1" means ^1);
#     they are pinned by Cargo.lock (the authoritative lock), NOT by this lint.
#     This script does NOT enforce Cargo crate pinning — it only catches the
#     explicit shorthand floats (^/~/latest/*) in npm/pipx/mise manifests.
#
# COMMENT STRIPPING (prevents false positives):
#   TOML files (.toml) carry explanatory comments that legitimately reference the
#   forbidden patterns (e.g. "# no floating "latest"/"^"/"~"/"*" per SEC-003").
#   Only FULL-LINE comments are stripped (lines whose first non-whitespace char
#   is '#'). Inline / trailing comments are NOT stripped to avoid truncating
#   legitimate values that contain ' #' (e.g. URLs with fragment anchors, git
#   egg references). The SEC-003 explanatory comments that triggered the original
#   false-positive are all full-line comments, so full-line stripping is both
#   necessary and sufficient. JSON has no comment syntax.
#
# PORTABILITY NOTE:
#   Uses grep -E (ERE), not grep -P (PCRE). -P is unreliable on BSD/ugrep
#   environments when invoked from a non-interactive bash script context.
#   All required patterns are expressible as ERE; -E works on GNU grep (CI)
#   and ugrep (macOS local) alike.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
cd "${REPO_ROOT}"

# Manifests to scan — explicit list, not a glob that could pull in lockfiles.
MANIFESTS=(
  "mise.toml"
  "Cargo.toml"
)

# Collect package.json files — exclude node_modules and pnpm-lock.yaml.
# Only scan the project-root package.json, not transitive dependency manifests.
while IFS= read -r f; do
  MANIFESTS+=("${f}")
done < <(find packages -maxdepth 2 -name "package.json" ! -path "*/node_modules/*" 2>/dev/null)

# Collect pyproject.toml files
while IFS= read -r f; do
  MANIFESTS+=("${f}")
done < <(find packages -name "pyproject.toml" 2>/dev/null)

# Collect Cargo.toml files (crates and workspace root already added above)
while IFS= read -r f; do
  MANIFESTS+=("${f}")
done < <(find crates -name "Cargo.toml" 2>/dev/null)

FAILED=()

for manifest in "${MANIFESTS[@]}"; do
  [[ -f "${manifest}" ]] || continue

  # Strip FULL-LINE comments before scanning to avoid false positives from
  # documentation that references the forbidden patterns (SEC-003 explanations).
  # TOML comment syntax: '#' to end-of-line. JSON has no comments.
  #
  # Only full-line comments (lines whose first non-whitespace char is '#') are
  # removed. Inline/trailing comments are NOT stripped — removing whitespace-
  # preceded '#' could truncate legitimate values that contain ' #' (e.g. a
  # future `pkg @ git+https://host/repo.git#egg=x` or a URL fragment anchor).
  # The SEC-003 explanatory comments that caused the original false-positive are
  # all full-line comments, so this is both necessary and sufficient.
  case "${manifest}" in
    *.toml)
      scan_content="$(sed -E '/^[[:space:]]*#/d' "${manifest}")"
      ;;
    *)
      scan_content="$(cat "${manifest}")"
      ;;
  esac

  # Write to a temp file so grep reads from a file descriptor, not a pipeline.
  # This sidesteps a portability trap: `printf '%s\n' "$var" | grep -E ...`
  # can behave differently from `grep -E ... <(printf '%s\n' "$var")` under
  # some bash+ugrep combinations (grep -P in particular exits 1 silently in
  # the pipeline context on macOS/ugrep even when a match exists). Writing to
  # a temp file and grepping the file is unambiguous on all targets.
  tmp_scan="$(mktemp)"
  # shellcheck disable=SC2064
  trap "rm -f '${tmp_scan}'" EXIT
  printf '%s\n' "${scan_content}" > "${tmp_scan}"

  # --- Pattern checks (grep -Eq, portable ERE, quiet — no stdout leak) ---
  #
  # grep -Eq returns exit status only; matching lines are NOT printed. The script
  # builds its own FAILED[] summary, so we need only the boolean result.
  #
  # Patterns to detect:
  #   "^..."     — npm caret range in JSON
  #   "~..."     — npm tilde range in JSON
  #   "latest"   — the literal string latest as a version value
  #   "*"        — wildcard version in JSON
  #   = "*"      — wildcard in TOML
  #   = "~"...   — tilde in TOML (e.g. version = "~1.0")
  #   = "^"...   — caret in TOML (e.g. version = "^1.0")
  #
  # Explicitly NOT flagged:
  #   >=x,<y     — bounded range (acceptable per SEC-003; maturin in pyproject.toml)
  #   ">=..."    — floor bound only (also acceptable)

  if grep -Eq '"[\^~][^"]*"' "${tmp_scan}" 2>/dev/null; then
    FAILED+=("${manifest}: caret or tilde range in JSON/TOML string")
  fi
  if grep -Eq '"latest"' "${tmp_scan}" 2>/dev/null; then
    FAILED+=("${manifest}: literal 'latest' version")
  fi
  if grep -Eq '"[*]"' "${tmp_scan}" 2>/dev/null; then
    FAILED+=("${manifest}: wildcard '*' version in JSON")
  fi
  # TOML: version = "*"
  if grep -Eq '=\s*"[*]"' "${tmp_scan}" 2>/dev/null; then
    FAILED+=("${manifest}: wildcard '*' version in TOML")
  fi
  # TOML: version = "^x" or version = "~x"
  # (also caught by the first pattern for JSON, but explicit for TOML context)
  if grep -Eq '=\s*"[\^~][^"]*"' "${tmp_scan}" 2>/dev/null; then
    FAILED+=("${manifest}: caret or tilde range in TOML assignment")
  fi

  rm -f "${tmp_scan}"
  trap - EXIT
done

if [[ ${#FAILED[@]} -gt 0 ]]; then
  echo ""
  echo "ERROR: Floating-version lint FAILED (SEC-003)."
  echo "The following manifests contain floating version specifiers:"
  for msg in "${FAILED[@]}"; do
    echo "  - ${msg}"
  done
  echo ""
  echo "Pin all versions explicitly. Floating specifiers (^, ~, 'latest', '*')"
  echo "are not allowed in codegen/tool manifests per SEC-003."
  echo ""
  exit 1
fi

echo "Floating-version lint PASSED — all manifests use pinned versions."
for m in "${MANIFESTS[@]}"; do
  [[ -f "${m}" ]] && echo "  OK: ${m}"
done
