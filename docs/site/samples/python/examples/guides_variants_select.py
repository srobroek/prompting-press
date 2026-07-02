"""Selecting a variant at render.

Omit the name (or pass ``variant=None``) for the default arm; pass a name for
that arm. The resolved name comes back on ``RenderResult.variant`` and the text
is that arm's rendered body. Standalone program.
"""

from __future__ import annotations

from pathlib import Path

from prompting_press import Prompt
from pydantic import BaseModel, Field

# The caller reads the definition; the library does no file I/O itself.
# Resolve the file next to this program (a real app uses its own path).
_HERE = Path(__file__).parent


class SummaryVars(BaseModel):
    article: str = Field(min_length=1)
    max_words: int = Field(ge=1)


def main() -> None:
    summary = Prompt.from_yaml((_HERE / "summary.yaml").read_text())
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
