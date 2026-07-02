"""Reading prompt-level and per-variant metadata back out.

The library stores the opaque ``metadata`` maps and echoes them through
accessors; it never interprets them. The accessors return the maps as-is;
application code interprets them. Standalone — run it directly, or the doc-sample
test harness executes it and its assertions.
"""

from __future__ import annotations

from pathlib import Path

from prompting_press import Prompt

# The caller reads the definition; the library does no file I/O itself.
# Resolve the file next to this program (a real app uses its own path).
_HERE = Path(__file__).parent

p = Prompt.from_yaml((_HERE / "summary_metadata.yaml").read_text())

p.metadata  # => {"model_hint": "claude-sonnet-4-6", "max_tokens": 512, "owner": "team-content"}
p.metadata["model_hint"]  # application code decides what to do with it

# per-variant metadata (each variant is a plain dict):
p.variants["terse"]["metadata"]  # => {"weight": 0.2, "group": "experiment-q4"}

# The accessors return the maps as-is; nothing is interpreted or mutated.
assert p.metadata == {
    "model_hint": "claude-sonnet-4-6",
    "max_tokens": 512,
    "owner": "team-content",
}
assert p.metadata["model_hint"] == "claude-sonnet-4-6"
assert p.variants["terse"]["metadata"] == {"weight": 0.2, "group": "experiment-q4"}
