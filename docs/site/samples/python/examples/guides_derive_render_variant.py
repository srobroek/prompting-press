"""Derive guide — render a named variant: after adding a variant with ``derive``, select
it by name at render time. Variant selection is caller-owned.

Standalone — the docs page displays this file verbatim; run it directly to check.
"""

from pathlib import Path

from prompting_press import Prompt
from pydantic import BaseModel, field_validator

# The caller reads the definition; the library does no file I/O itself.
# Resolve the file next to this program (a real app uses its own path).
_HERE = Path(__file__).parent


class AssistantVars(BaseModel):
    company: str
    max_words: int

    @field_validator("max_words")
    @classmethod
    def _at_least_one(cls, v: int) -> int:
        if v < 1:
            raise ValueError("max_words must be at least 1")
        return v


def main() -> None:
    assistant = Prompt.from_yaml((_HERE / "assistant.yaml").read_text())
    derived = assistant.derive(
        {
            "variants": {
                **assistant.variants,
                "formal": {
                    "body": "You are the official support assistant for {{ company }}. "
                    "Please keep every reply under {{ max_words }} words."
                },
            }
        }
    )

    result = derived.render(
        AssistantVars,
        data={"company": "Acme Robotics", "max_words": 50},
        variant="formal",
    )
    print(
        result.text
    )  # "You are the official support assistant for Acme Robotics. ..."
    print(result.variant)  # "formal"
    assert (
        result.text == "You are the official support assistant for Acme Robotics. "
        "Please keep every reply under 50 words."
    )
    assert result.variant == "formal"


if __name__ == "__main__":
    main()
