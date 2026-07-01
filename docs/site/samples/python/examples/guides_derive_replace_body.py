"""Derive guide — replace only the root body (the default arm) with ``derive``.

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
    greet = Prompt.from_yaml(greet_yaml)

    brief_greet = greet.derive({"body": "Hi {{ name }}!"})

    result = brief_greet.render(GreetVars, data={"name": "Ada", "count": 3})
    assert result.text == "Hi Ada!"


if __name__ == "__main__":
    main()
