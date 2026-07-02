"""Complete example — construct (validates), then render with typed, Pydantic-validated vars."""

import re
from pathlib import Path

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
# The caller reads the definition; the library does no file I/O itself.
# Resolve the file next to this program (a real app uses its own path).
_HERE = Path(__file__).parent
assistant = Prompt.from_yaml((_HERE / "assistant.yaml").read_text())

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
