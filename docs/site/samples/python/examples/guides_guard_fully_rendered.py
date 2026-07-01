"""Fully-rendered example: rendering the `ask` prompt with the guard enabled
carries the untrusted value wrapped in delimiters in `result.text`, and returns
the advisory separately in `result.guard`. Standalone — run directly or under
pytest."""

from prompting_press import Prompt, GuardConfig
from pydantic import BaseModel, Field

ask = Prompt.from_yaml("""
name: ask
role: user
body: "Tell me about {{ topic }}."
variables:
  topic:
    type: string
    trusted: false
""")


class Ask(BaseModel):
    topic: str = Field(min_length=1)


def main() -> None:
    result = ask.render(Ask, data={"topic": "rivers"}, guard=GuardConfig(enabled=True))

    # result.text  = "Tell me about <untrusted>rivers</untrusted>."
    # result.guard = "User-supplied inputs are wrapped in <untrusted>…</untrusted> tags below; …"
    assert result.text == "Tell me about <untrusted>rivers</untrusted>."
    assert result.guard is not None
    assert "<untrusted>" in result.guard

    # The two fields are never concatenated by the library.
    assert result.guard not in result.text


if __name__ == "__main__":
    main()
