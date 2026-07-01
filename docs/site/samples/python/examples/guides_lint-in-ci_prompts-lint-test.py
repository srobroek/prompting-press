"""Wiring ``Prompt.check()`` as a CI gate under pytest.

A CI gate is a test that fails the build: load every ``*.yaml`` under a ``prompts/``
directory, construct each prompt, and assert ``check()`` returns no findings — naming
the offender otherwise. Standalone: this program first materializes a ``prompts/``
directory of shipped fixtures in a temp dir and ``chdir``s into it (a real repo keeps
its own ``prompts/`` under version control), then runs the documented parametrized test.

Run it directly (``python guides_lint-in-ci_prompts-lint-test.py``) or under pytest.
"""

from __future__ import annotations

import os
import tempfile
from pathlib import Path

import pytest
from prompting_press import Prompt, PromptingPressError

# ── Materialize the `prompts/` directory a real repo would keep under version control. ──
# A clean, shipped prompt: its untrusted-free variable needs no guard, so check() passes.
_TMP = Path(tempfile.mkdtemp(prefix="pp_lint_in_ci_"))
(_TMP / "prompts").mkdir()
(_TMP / "prompts" / "greet.yaml").write_text(
    """
name: greet
role: user
body: "Hi {{ name }}, you have {{ count }} messages."
variables:
  name:
    type: string
    trusted: true
  count:
    type: integer
    trusted: true
"""
)
os.chdir(_TMP)

# ── The CI gate itself. ──
# tests/test_prompts_lint.py  — runs under pytest
PROMPT_FILES = sorted(Path("prompts").glob("*.yaml"))


@pytest.mark.parametrize("path", PROMPT_FILES, ids=lambda p: p.name)
def test_shipped_prompt_passes_check(path: Path) -> None:
    try:
        prompt = Prompt.from_yaml(path.read_text())  # construction enforces hard invariants
    except PromptingPressError as e:
        pytest.fail(f"{path.name}: construction failed: {e}")

    report = prompt.check()
    findings = [f"{f.kind}: {f.detail}" for f in report.findings]
    assert report.passed(), f"{path.name} lint findings:\n" + "\n".join(findings)


if __name__ == "__main__":
    # Run the documented per-file check directly (no pytest runner needed).
    for _path in PROMPT_FILES:
        _prompt = Prompt.from_yaml(_path.read_text())
        _report = _prompt.check()
        assert _report.passed(), f"{_path.name} lint findings: {_report.findings}"
    print(f"prompt lint: clean — checked {len(PROMPT_FILES)} prompt(s)")
