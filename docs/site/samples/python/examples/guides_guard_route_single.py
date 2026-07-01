"""Routing the guard text for a single render: the advisory goes to a system
message, the body to a user message. The library never concatenates them.
Standalone — run directly or under pytest."""

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

    messages = [
        {"role": "system", "content": result.guard},
        {"role": "user", "content": result.text},
    ]

    assert messages[0]["role"] == "system"
    assert "<untrusted>" in messages[0]["content"]
    assert messages[1]["role"] == "user"
    assert messages[1]["content"] == "Tell me about <untrusted>rivers</untrusted>."


if __name__ == "__main__":
    main()
