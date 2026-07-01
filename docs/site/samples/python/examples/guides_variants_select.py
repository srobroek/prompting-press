"""Selecting a variant at render.

Omit the name (or pass ``variant=None``) for the default arm; pass a name for
that arm. The resolved name comes back on ``RenderResult.variant`` and the text
is that arm's rendered body. Standalone program.
"""

from __future__ import annotations

from prompting_press import Prompt
from pydantic import BaseModel, Field

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
"""


class SummaryVars(BaseModel):
    article: str = Field(min_length=1)
    max_words: int = Field(ge=1)


def main() -> None:
    summary = Prompt.from_yaml(SUMMARY_YAML)
    data = {"article": "The Nile floods yearly.", "max_words": 20}

    default = summary.render(SummaryVars, data=data)  # default (root body)
    concise = summary.render(SummaryVars, data=data, variant="concise")

    assert default.variant == "default"
    assert (
        default.text
        == "Summarise the following article in 20 words:\n\nThe Nile floods yearly."
    )
    assert concise.variant == "concise"
    assert concise.text == "In one sentence, summarise: The Nile floods yearly."


if __name__ == "__main__":
    main()
