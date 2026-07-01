"""Construct a Prompt from a typed PromptDefinition shape (same validation as from_*)."""

from prompting_press import Prompt, PromptDefinition

# PromptDefinition is the generated Pydantic shape — an editor/type-checker
# checks the fields, the role enum, and each variable's `trusted` flag at author time.
definition = PromptDefinition(
    name="greet",
    role="user",
    body="Hi {{ name }}, you have {{ count }} messages.",
    variables={
        "name": {"type": "string", "trusted": True},
        "count": {"type": "integer", "trusted": True},
    },
)

greet = Prompt(definition)  # same validation as the from_* factories
assert greet.name == "greet"
