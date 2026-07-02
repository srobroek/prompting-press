"""Complete example — construct (validates), then render with typed, Pydantic-validated vars."""

import re

from prompting_press import Prompt
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


# 1. Construct (validates here).
# The caller reads the text; this program embeds it so it runs standalone.
ASSISTANT_YAML = """\
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
"""
assistant = Prompt.from_yaml(ASSISTANT_YAML)

# 2 + 3. Render with the typed, Pydantic-validated vars.
result = assistant.render(
    AssistantVars, data={"company": "Acme Robotics", "max_words": 50}
)
print(
    result.text
)  # You are a support assistant for Acme Robotics. Keep your replies under 50 words.
print(result.template_hash)  # 64-char hex

assert (
    result.text
    == "You are a support assistant for Acme Robotics. Keep your replies under 50 words."
)
assert re.fullmatch(r"[0-9a-f]{64}", result.template_hash)
