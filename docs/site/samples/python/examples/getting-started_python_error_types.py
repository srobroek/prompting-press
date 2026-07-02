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

# The specific exceptions all derive from PromptingPressError.
assert issubclass(PromptValidationError, PromptingPressError)
assert issubclass(PromptRenderError, PromptingPressError)
assert issubclass(LoadError, PromptingPressError)

try:
    result = assistant.render(
        AssistantVars, data={"company": "Acme Robotics", "max_words": 0}
    )
    raise AssertionError("expected a validation error for max_words below 1")
except PromptValidationError as exc:
    for row in exc.errors:
        print(row.field, row.code, row.message)
        # "max_words"  "validation"  "max_words must be at least 1"
    row = exc.errors[0]
    assert row.field == "max_words"
    assert row.code == "validation"
    assert "max_words must be at least 1" in row.message
