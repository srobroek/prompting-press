"""Render, and read the result — hand the Prompt the Vars class plus a data dict;
it validates + renders in one call, returning a RenderResult."""

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


greet = Prompt.from_yaml("""\
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
""")

result = greet.render(GreetVars, data={"name": "Ada", "count": 3})

assert result.text == "Hi Ada, you have 3 messages."
assert result.variant == "default"  # same arm greet.body showed in Step 1
assert re.fullmatch(
    r"[0-9a-f]{64}", result.template_hash
)  # 64-char lowercase-hex SHA-256 of the template
assert re.fullmatch(
    r"[0-9a-f]{64}", result.render_hash
)  # 64-char lowercase-hex SHA-256 of result.text
assert result.guard is None  # no guard requested
