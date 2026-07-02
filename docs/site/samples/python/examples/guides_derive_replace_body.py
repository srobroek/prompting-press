"""Derive guide — replace only the root body (the default arm) with ``derive``.

Standalone — the docs page displays this file verbatim; run it directly to check.
"""

from prompting_press import Prompt
from pydantic import BaseModel, field_validator

assistant_yaml = """
name: assistant
role: system
body: "You are a support assistant for {{ company }}. Keep your replies under {{ max_words }} words."
variables:
  company: { type: string, trusted: true }
  max_words: { type: integer, trusted: true }
"""


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
    assistant = Prompt.from_yaml(assistant_yaml)

    brief_assistant = assistant.derive(
        {"body": "You are a support assistant for {{ company }}."}
    )

    result = brief_assistant.render(
        AssistantVars, data={"company": "Acme Robotics", "max_words": 50}
    )
    assert result.text == "You are a support assistant for Acme Robotics."


if __name__ == "__main__":
    main()
