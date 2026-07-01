"""Declare the typed Vars model — a caller-owned Pydantic model whose validators
run before the kernel is touched."""

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


# The model matches the prompt's `variables` (name, count) and its validators run first.
ok = GreetVars(name="Ada", count=3)
assert ok.name == "Ada"
assert ok.count == 3

# A negative count is rejected by the field validator.
try:
    GreetVars(name="Ada", count=-1)
    raise AssertionError("expected a validation error for a negative count")
except ValueError as exc:
    assert "count must be non-negative" in str(exc)
