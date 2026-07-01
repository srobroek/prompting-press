"""Complete example — construct (validates), then render with typed, Pydantic-validated vars."""

import re

from prompting_press import Prompt
from pydantic import BaseModel, field_validator


class GreetVars(BaseModel):
    name: str
    count: int

    @field_validator("count")
    @classmethod
    def _non_negative(cls, v: int) -> int:
        if v < 0:
            raise ValueError("count must be non-negative")
        return v


# 1. Construct (validates here).
# The caller reads the text; this program embeds it so it runs standalone.
GREET_YAML = """\
name: greet
role: user
body: "Hi {{ name }}, you have {{ count }} messages."
variables:
  name:
    type: string
    trusted: true
  count:
    type: integer
    trusted: true
"""
greet = Prompt.from_yaml(GREET_YAML)

# 2 + 3. Render with the typed, Pydantic-validated vars.
result = greet.render(GreetVars, data={"name": "Ada", "count": 3})
print(result.text)  # Hi Ada, you have 3 messages.
print(result.template_hash)  # 64-char hex

assert result.text == "Hi Ada, you have 3 messages."
assert re.fullmatch(r"[0-9a-f]{64}", result.template_hash)
