"""Reading prompt-level and per-variant metadata back out.

The library stores the opaque ``metadata`` maps and echoes them through
accessors; it never interprets them. The accessors return the maps as-is;
application code interprets them. Standalone — run it directly, or the doc-sample
test harness executes it and its assertions.
"""

from __future__ import annotations

from prompting_press import Prompt

# A real consumer would read this from a file; inlined here so the sample is standalone.
SUMMARY_YAML = """
name: summary
role: user
body: "Summarise {{ article }}."
variables:
  article:
    type: string
    trusted: false
metadata:
  model_hint: claude-sonnet-4-6
  max_tokens: 512
  owner: team-content
variants:
  terse:
    body: "TL;DR of {{ article }}."
    metadata:
      weight: 0.2
      group: experiment-q4
"""

p = Prompt.from_yaml(SUMMARY_YAML)

p.metadata           # => {"model_hint": "claude-sonnet-4-6", "max_tokens": 512, "owner": "team-content"}
p.metadata["model_hint"]   # application code decides what to do with it

# per-variant metadata (each variant is a plain dict):
p.variants["terse"]["metadata"]   # => {"weight": 0.2, "group": "experiment-q4"}

# The accessors return the maps as-is; nothing is interpreted or mutated.
assert p.metadata == {
    "model_hint": "claude-sonnet-4-6",
    "max_tokens": 512,
    "owner": "team-content",
}
assert p.metadata["model_hint"] == "claude-sonnet-4-6"
assert p.variants["terse"]["metadata"] == {"weight": 0.2, "group": "experiment-q4"}
