"""Routing the guard text for a multi-message composition: prepend the advisory
as a system message, then the resolved body messages. Standalone — run directly
or under pytest."""

from prompting_press import Prompt, GuardConfig, Composition
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
    # A single-render result supplies the guard advisory to prepend.
    result = ask.render(Ask, data={"topic": "rivers"}, guard=GuardConfig(enabled=True))

    # The body messages come from a composition.
    comp = Composition()
    comp.append(ask, Ask(topic="rivers"))

    guard_msg = {"role": "system", "content": result.guard}
    body_messages = [{"role": m.role, "content": m.text} for m in comp.resolve()]
    request_messages = [guard_msg] + body_messages

    assert request_messages[0]["role"] == "system"
    assert "<untrusted>" in request_messages[0]["content"]
    assert request_messages[1]["role"] == "user"
    # Composition never applies the guard, so its body is the plain render.
    assert request_messages[1]["content"] == "Tell me about rivers."


if __name__ == "__main__":
    main()
