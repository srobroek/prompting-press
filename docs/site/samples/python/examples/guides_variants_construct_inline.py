"""Constructing a three-arm prompt inline from a shape object.

The ``variants`` map mirrors the document form one-to-one and construction runs
the same validation as ``Prompt.from_yaml``. Standalone program; run it directly
or under the example test harness.
"""

from __future__ import annotations

from prompting_press import Prompt


def main() -> None:
    summary = Prompt(
        {
            "name": "summary",
            "role": "user",
            "body": "Summarise the following article in {{ max_words }} words:\n\n{{ article }}",
            "variables": {
                "article": {"type": "string", "trusted": False},
                "max_words": {"type": "integer", "trusted": True},
            },
            "variants": {
                "concise": {"body": "In one sentence, summarise: {{ article }}"},
                "structured": {
                    "body": "Summarise {{ article }} as a title, three bullets, and a one-line conclusion.",
                    "metadata": {"group": "experiment-q4"},
                },
            },
        }
    )  # same validation as Prompt.from_yaml

    assert sorted(summary.variants) == ["concise", "structured"]


if __name__ == "__main__":
    main()
