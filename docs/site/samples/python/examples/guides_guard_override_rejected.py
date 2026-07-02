"""A non-conforming advisory override (missing the required markers) is rejected
by the kernel and raises `PromptRenderError` with `code == "render"` and
`field == "guard"`. Standalone — run directly or under pytest."""

from pathlib import Path

from prompting_press import Prompt, GuardConfig, PromptRenderError
from pydantic import BaseModel, Field

# The caller reads the definition; the library does no file I/O itself.
# Resolve the file next to this program (a real app uses its own path).
_HERE = Path(__file__).parent
ask = Prompt.from_yaml((_HERE / "ask.yaml").read_text())


class Ask(BaseModel):
    topic: str = Field(min_length=1)


def main() -> None:
    try:
        ask.render(
            Ask,
            data={"topic": "x"},
            guard=GuardConfig(
                enabled=True,
                advisory="Missing the required markers.",  # rejected
            ),
        )
        raise AssertionError("a non-conforming advisory must be rejected")
    except PromptRenderError as exc:
        assert exc.errors[0].code == "render"
        assert exc.errors[0].field == "guard"


if __name__ == "__main__":
    main()
