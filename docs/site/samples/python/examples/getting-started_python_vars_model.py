"""Declare the typed Vars model — a caller-owned Pydantic model whose validators
run before the kernel is touched."""

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


# The model matches the prompt's `variables` (company, max_words) and its validators run first.
ok = AssistantVars(company="Acme Robotics", max_words=50)
assert ok.company == "Acme Robotics"
assert ok.max_words == 50

# A max_words below 1 is rejected by the field validator.
try:
    AssistantVars(company="Acme Robotics", max_words=0)
    raise AssertionError("expected a validation error for max_words below 1")
except ValueError as exc:
    assert "max_words must be at least 1" in str(exc)
