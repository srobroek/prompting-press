"""
T019 — Fixture validation gate (FR-013 / US2 acceptance gate).

Validates every file in fixtures/valid/  → each MUST be accepted by the schema.
Validates every file in fixtures/invalid/ → each MUST be rejected by the schema.

Rejection sources:
  - JSON parse error   (e.g. not-json.txt)  → counts as a correct rejection.
  - jsonschema.ValidationError              → schema-level rejection.

A valid/ file that fails validation  → task FAILS (schema too strict).
An invalid/ file that passes validation → task FAILS (schema too permissive).

Exits 0 only when every expectation is met; exits 1 otherwise.
"""

import json
import pathlib
import sys
from typing import NamedTuple

try:
    import jsonschema
except ImportError:
    print("ERROR: jsonschema not available. Run via: uv run --with jsonschema python3 validate_fixtures.py")
    sys.exit(1)

ROOT = pathlib.Path(__file__).parent.parent
SCHEMA_PATH = ROOT / "prompt-definition.schema.json"
VALID_DIR = ROOT / "fixtures" / "valid"
INVALID_DIR = ROOT / "fixtures" / "invalid"

# ANSI colours (kept minimal; no external deps)
GREEN = "\033[32m"
RED = "\033[31m"
RESET = "\033[0m"


class Result(NamedTuple):
    fixture: pathlib.Path
    passed: bool      # True = expectation met
    note: str         # human-readable outcome


def load_schema() -> dict:
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    return schema


def validate_document(schema: dict, path: pathlib.Path) -> tuple[bool, str]:
    """
    Try to parse path as JSON and validate it against schema.
    Returns (is_valid, note).
    is_valid=True  → document accepted by schema.
    is_valid=False → document rejected (parse error OR validation error).
    The note distinguishes which kind of rejection occurred.
    """
    try:
        instance = json.loads(path.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, UnicodeDecodeError) as exc:
        return False, f"parse-error: {exc}"

    validator = jsonschema.Draft202012Validator(schema)
    errors = list(validator.iter_errors(instance))
    if errors:
        # Report the first error for brevity
        first = errors[0]
        path_str = " > ".join(str(p) for p in first.absolute_path) or "(root)"
        return False, f"schema-invalid at [{path_str}]: {first.message}"
    return True, "accepted"


def check_dir(schema: dict, directory: pathlib.Path, expect_valid: bool) -> list[Result]:
    results: list[Result] = []
    files = sorted(directory.iterdir())
    non_dir_files = [f for f in files if not f.is_dir()]
    if not non_dir_files:
        label = "valid" if expect_valid else "invalid"
        print(f"  ERROR: no fixtures found in {directory} — at least one {label}/ fixture is required.")
        sys.exit(1)
    for fixture in files:
        if fixture.is_dir():
            continue
        is_valid, note = validate_document(schema, fixture)
        expectation_met = is_valid == expect_valid
        results.append(Result(fixture=fixture, passed=expectation_met, note=note))
    return results


def print_results(label: str, results: list[Result]) -> int:
    """Print per-fixture report. Returns count of failures."""
    print(f"\n{label}")
    print("-" * len(label))
    failures = 0
    for r in results:
        icon = f"{GREEN}PASS{RESET}" if r.passed else f"{RED}FAIL{RESET}"
        print(f"  [{icon}] {r.fixture.name}  —  {r.note}")
        if not r.passed:
            failures += 1
    return failures


def main() -> int:
    if not SCHEMA_PATH.exists():
        print(f"ERROR: Schema not found: {SCHEMA_PATH}")
        return 1

    schema = load_schema()

    # Pre-validate the schema itself so a broken schema surfaces early.
    try:
        jsonschema.Draft202012Validator.check_schema(schema)
    except jsonschema.SchemaError as exc:
        print(f"ERROR: Schema file is not a valid Draft 2020-12 document: {exc.message}")
        return 1

    valid_results = check_dir(schema, VALID_DIR, expect_valid=True)
    invalid_results = check_dir(schema, INVALID_DIR, expect_valid=False)

    fail_valid = print_results(
        f"fixtures/valid/  (each MUST be ACCEPTED — {len(valid_results)} files)",
        valid_results,
    )
    fail_invalid = print_results(
        f"fixtures/invalid/  (each MUST be REJECTED — {len(invalid_results)} files)",
        invalid_results,
    )

    total_files = len(valid_results) + len(invalid_results)
    total_failures = fail_valid + fail_invalid
    total_passed = total_files - total_failures

    print(f"\n{'=' * 56}")
    print(f"Summary: {total_passed}/{total_files} expectations met", end="")
    if total_failures == 0:
        print(f"  {GREEN}ALL PASS{RESET}")
        return 0
    else:
        print(f"  {RED}{total_failures} FAILURE(S){RESET}")
        if fail_valid:
            print(f"  {fail_valid} valid fixture(s) were incorrectly rejected (schema too strict).")
        if fail_invalid:
            print(f"  {fail_invalid} invalid fixture(s) were incorrectly accepted (schema too permissive).")
        return 1


if __name__ == "__main__":
    sys.exit(main())
