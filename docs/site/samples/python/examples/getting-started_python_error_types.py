"""Error types — a rejected render value raises PromptValidationError, whose
`.errors` are normalized FieldError rows (.field, .code, .message)."""

from prompting_press import (
    Prompt,
    PromptingPressError,
    PromptValidationError,
    PromptRenderError,
    LoadError,
)
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

# The specific exceptions all derive from PromptingPressError.
assert issubclass(PromptValidationError, PromptingPressError)
assert issubclass(PromptRenderError, PromptingPressError)
assert issubclass(LoadError, PromptingPressError)

try:
    result = greet.render(GreetVars, data={"name": "Ada", "count": -1})
    raise AssertionError("expected a validation error for a negative count")
except PromptValidationError as exc:
    for row in exc.errors:
        print(row.field, row.code, row.message)
        # "count"  "validation"  "count must be non-negative"
    row = exc.errors[0]
    assert row.field == "count"
    assert row.code == "validation"
    assert "count must be non-negative" in row.message
