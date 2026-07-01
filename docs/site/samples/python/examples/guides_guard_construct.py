"""The one-variable `ask` prompt used throughout the guard guide: `topic` is
declared untrusted (trusted: false). Standalone — run directly or under pytest."""

from prompting_press import Prompt
from pydantic import BaseModel, Field

# The prompt: `topic` is declared untrusted (trusted: false).
ask = Prompt.from_yaml("""
name: ask
role: user
body: "Tell me about {{ topic }}."
variables:
  topic:
    type: string
    trusted: false
""")


# The typed vars model handed to render().
class Ask(BaseModel):
    topic: str = Field(min_length=1)


def main() -> None:
    # Without the guard the body renders plainly.
    result = ask.render(Ask, data={"topic": "rivers"})
    assert result.text == "Tell me about rivers."
    assert result.guard is None
    print(result.text)


if __name__ == "__main__":
    main()
