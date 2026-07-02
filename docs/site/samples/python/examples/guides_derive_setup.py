"""Derive guide — the starting pair: an ``assistant`` prompt (a ``company`` + ``max_words``
system body) and a matching ``AssistantVars``. Every later example on the page derives from
this.

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
    # The pair parses and validates: the body's {{ company }}/{{ max_words }} agree with
    # AssistantVars.
    assistant = Prompt.from_yaml(assistant_yaml)
    assert assistant.name == "assistant"

    # AssistantVars is a plain Pydantic model — construct one to prove the shape.
    vars = AssistantVars(company="Acme Robotics", max_words=50)
    assert vars.company == "Acme Robotics"
    assert vars.max_words == 50


if __name__ == "__main__":
    main()
