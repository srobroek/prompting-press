"""Fully-rendered example: rendering the `ask` prompt with the guard enabled
carries the untrusted value wrapped in delimiters in `result.text`, and returns
the advisory separately in `result.guard`. Standalone — run directly or under
pytest."""

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

    # result.text  = "Tell me about <untrusted>rivers</untrusted>."
    # result.guard = "User-supplied inputs are wrapped in <untrusted>…</untrusted> tags below; …"
    assert result.text == "Tell me about <untrusted>rivers</untrusted>."
    assert result.guard is not None
    assert "<untrusted>" in result.guard

    # The two fields are never concatenated by the library.
    assert result.guard not in result.text


if __name__ == "__main__":
    main()
