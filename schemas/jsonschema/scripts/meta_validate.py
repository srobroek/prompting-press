"""
T016 — Schema meta-validation (FR-008).

Loads prompt-definition.schema.json and asserts it is itself a valid
Draft 2020-12 document via jsonschema.Draft202012Validator.check_schema().
Exits 0 on success, 1 on any failure.
"""

import json
import pathlib
import sys

try:
    import jsonschema
except ImportError:
    print("ERROR: jsonschema not available. Run via: uv run --with jsonschema python3 meta_validate.py")
    sys.exit(1)

SCHEMA_PATH = pathlib.Path(__file__).parent.parent / "prompt-definition.schema.json"


def main() -> int:
    if not SCHEMA_PATH.exists():
        print(f"ERROR: Schema file not found: {SCHEMA_PATH}")
        return 1

    try:
        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        print(f"ERROR: Schema file is not valid JSON: {exc}")
        return 1

    try:
        jsonschema.Draft202012Validator.check_schema(schema)
    except jsonschema.SchemaError as exc:
        print(f"ERROR: Schema is NOT a valid Draft 2020-12 document:")
        print(f"  {exc.message}")
        return 1

    print(f"OK: {SCHEMA_PATH.name} is a valid JSON Schema Draft 2020-12 document.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
