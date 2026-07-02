"""Discovering the selectable variants.

The ``variants`` accessor returns the declared variant map; read its keys to see
what is selectable (the default arm is not listed — it is the root body, name
``"default"``). Standalone program.
"""

from __future__ import annotations

from pathlib import Path

from prompting_press import Prompt

# The caller reads the definition; the library does no file I/O itself.
# Resolve the file next to this program (a real app uses its own path).
_HERE = Path(__file__).parent


def main() -> None:
    summary = Prompt.from_yaml((_HERE / "summary.yaml").read_text())

    assert sorted(summary.variants) == ["concise", "structured"]
    assert "concise" in summary.variants  # True


if __name__ == "__main__":
    main()
