"""Render, and read the result — hand the Prompt the Vars class plus a data dict;
it validates + renders in one call, returning a RenderResult. The same Vars class
also rejects bad data at render, before the kernel ever sees it."""

import re

from prompting_press import Prompt, PromptValidationError
from pydantic import BaseModel, field_validator


class AssistantVars(BaseModel):
    company: str
    max_words: int

    @field_validator("max_words")
    @classmethod
    def _at_least_one(cls, v: int) -> int:
        if v < 1:
            raise ValueError("max_words must be at least 1")
        return v


assistant = Prompt.from_yaml("""\
name: assistant
role: system
body: "You are a support assistant for {{ company }}. Keep your replies under {{ max_words }} words."
variables:
  company:
    type: string
    trusted: true
  max_words:
    type: integer
    trusted: true
""")

result = assistant.render(
    AssistantVars, data={"company": "Acme Robotics", "max_words": 50}
)

assert (
    result.text
    == "You are a support assistant for Acme Robotics. Keep your replies under 50 words."
)
assert result.variant == "default"  # same arm assistant.body showed in Step 1
assert re.fullmatch(
    r"[0-9a-f]{64}", result.template_hash
)  # 64-char lowercase-hex SHA-256 of the template
assert re.fullmatch(
    r"[0-9a-f]{64}", result.render_hash
)  # 64-char lowercase-hex SHA-256 of result.text
assert result.guard is None  # no guard requested

# The same AssistantVars validates at render: bad data is rejected before the kernel.
try:
    assistant.render(AssistantVars, data={"company": "Acme Robotics", "max_words": 0})
    raise AssertionError("expected a validation error for max_words below 1")
except PromptValidationError as exc:
    row = exc.errors[0]
    assert row.field == "max_words"
    assert row.code == "validation"
    assert "max_words must be at least 1" in row.message
