"""Enabling the advisory guard: the untrusted `topic` value is delimited in the
rendered body and a guard advisory is returned. Standalone — run directly or
under pytest."""

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

    result.text  # "Tell me about <untrusted>rivers</untrusted>."  — topic wrapped in the body
    result.guard  # "User-supplied inputs are wrapped in <untrusted>…</untrusted> tags below; …"

    assert result.text == "Tell me about <untrusted>rivers</untrusted>."
    assert result.guard is not None
    assert "<untrusted>" in result.guard

    # GuardConfig() / GuardConfig(enabled=False) is equivalent to guard=None — no wrapping, no text.
    plain = ask.render(Ask, data={"topic": "rivers"}, guard=GuardConfig(enabled=False))
    assert plain.text == "Tell me about rivers."
    assert plain.guard is None

    print(result.text)


if __name__ == "__main__":
    main()
