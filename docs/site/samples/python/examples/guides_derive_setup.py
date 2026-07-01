"""Derive guide — the starting pair: a ``greet`` prompt (a ``name`` + ``count`` body)
and a matching ``GreetVars``. Every later example on the page derives from this.

Standalone — the docs page displays this file verbatim; run it directly to check.
"""

from prompting_press import Prompt
from pydantic import BaseModel, Field

greet_yaml = """
name: greet
role: user
body: "Hi {{ name }}, you have {{ count }} messages."
variables:
  name: { type: string, trusted: true }
  count: { type: integer, trusted: true }
"""


class GreetVars(BaseModel):
    name: str = Field(min_length=1)
    count: int = Field(ge=0)


def main() -> None:
    # The pair parses and validates: the body's {{ name }}/{{ count }} agree with GreetVars.
    greet = Prompt.from_yaml(greet_yaml)
    assert greet.name == "greet"

    # GreetVars is a plain Pydantic model — construct one to prove the shape.
    vars = GreetVars(name="Ada", count=3)
    assert vars.name == "Ada"
    assert vars.count == 3


if __name__ == "__main__":
    main()
