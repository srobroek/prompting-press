"""Discovering the selectable variants.

The ``variants`` accessor returns the declared variant map; read its keys to see
what is selectable (the default arm is not listed — it is the root body, name
``"default"``). Standalone program.
"""

from __future__ import annotations

from prompting_press import Prompt

SUMMARY_YAML = """
name: summary
role: user
body: "Summarise the following article in {{ max_words }} words:\\n\\n{{ article }}"
variables:
  article:
    type: string
    trusted: false
  max_words:
    type: integer
    trusted: true
variants:
  concise:
    body: "In one sentence, summarise: {{ article }}"
  structured:
    body: "Summarise {{ article }} as a title, three bullets, and a one-line conclusion."
"""


def main() -> None:
    summary = Prompt.from_yaml(SUMMARY_YAML)

    assert sorted(summary.variants) == ["concise", "structured"]
    assert "concise" in summary.variants  # True


if __name__ == "__main__":
    main()
