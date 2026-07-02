"""Construct a Prompt from a typed PromptDefinition shape (same validation as from_*)."""

from prompting_press import Prompt, PromptDefinition

# PromptDefinition is the generated Pydantic shape — an editor/type-checker
# checks the fields, the role enum, and each variable's `trusted` flag at author time.
definition = PromptDefinition(
    name="assistant",
    role="system",
    body="You are a support assistant for {{ company }}. Keep your replies under {{ max_words }} words.",
    variables={
        "company": {"type": "string", "trusted": True},
        "max_words": {"type": "integer", "trusted": True},
    },
)

assistant = Prompt(definition)  # same validation as the from_* factories
assert assistant.name == "assistant"
