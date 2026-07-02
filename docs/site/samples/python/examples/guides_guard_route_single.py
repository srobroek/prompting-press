"""Routing the guard text for a single render: the advisory goes to a system
message, the body to a user message. The library never concatenates them.
Standalone — run directly or under pytest."""

from pathlib import Path

from prompting_press import Prompt, GuardConfig
from pydantic import BaseModel, Field

# The caller reads the definition; the library does no file I/O itself.
# Resolve the file next to this program (a real app uses its own path).
_HERE = Path(__file__).parent
ask = Prompt.from_yaml((_HERE / "ask.yaml").read_text())


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
