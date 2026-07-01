"""Derive guide — render a named variant: after adding a variant with ``derive``, select
it by name at render time. Variant selection is caller-owned.

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
    derived = greet.derive(
        {
            "variants": {
                **greet.variants,
                "formal": {
                    "body": "Good day, {{ name }}. You have {{ count }} messages."
                },
            }
        }
    )

    result = derived.render(
        GreetVars, data={"name": "Ada", "count": 3}, variant="formal"
    )
    print(result.text)  # "Good day, Ada. You have 3 messages."
    print(result.variant)  # "formal"
    assert result.text == "Good day, Ada. You have 3 messages."
    assert result.variant == "formal"


if __name__ == "__main__":
    main()
