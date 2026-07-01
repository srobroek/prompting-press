"""Run every docs sample under ``examples/`` as a standalone program.

Each file in ``examples/`` is a COMPLETE, standalone Python program that a docs
page displays verbatim (Astro ``?raw`` import) and that this gate executes. The
file the reader sees IS the tested artifact — there is no marker injection and
no separate assertion file. Assertions live inside each example (module-level
or under an ``if __name__ == "__main__"`` guard), so the only faithful way to
run them is to execute the file as a script; importing it would skip the
guarded examples' assertions.

A parametrized case per example runs ``python <file>`` in a subprocess and
fails — citing that file — on any non-zero exit.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

import pytest

EXAMPLES_DIR = Path(__file__).resolve().parent.parent / "examples"

EXAMPLES = sorted(
    p for p in EXAMPLES_DIR.glob("*.py") if not p.name.startswith("_")
)

assert EXAMPLES, f"no example programs found under {EXAMPLES_DIR}"


@pytest.mark.parametrize("example", EXAMPLES, ids=lambda p: p.name)
def test_example_runs(example: Path) -> None:
    result = subprocess.run(
        [sys.executable, str(example)],
        cwd=EXAMPLES_DIR,
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, (
        f"{example.name} exited {result.returncode}\n"
        f"--- stdout ---\n{result.stdout}\n"
        f"--- stderr ---\n{result.stderr}"
    )
