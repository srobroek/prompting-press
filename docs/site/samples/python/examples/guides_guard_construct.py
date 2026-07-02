"""The one-variable `ask` prompt used throughout the guard guide: `topic` is
declared untrusted (trusted: false). Standalone — run directly or under pytest."""

from pathlib import Path

from prompting_press import Prompt
from pydantic import BaseModel, Field

# The prompt: `topic` is declared untrusted (trusted: false).
# The caller reads the definition; the library does no file I/O itself.
# Resolve the file next to this program (a real app uses its own path).
_HERE = Path(__file__).parent
ask = Prompt.from_yaml((_HERE / "ask.yaml").read_text())


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
