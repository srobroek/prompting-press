"""Construct a Prompt from a plain dict (the Rust loader validates the shape on the way in)."""

from prompting_press import Prompt

# A plain dict works too — convenient when the shape comes from already-parsed
# config. The Rust loader validates the shape on the way in, the same as the typed form.
greet = Prompt(
    {
        "name": "greet",
        "role": "user",
        "body": "Hi {{ name }}, you have {{ count }} messages.",
        "variables": {
            "name": {"type": "string", "trusted": True},
            "count": {"type": "integer", "trusted": True},
        },
    }
)

assert greet.name == "greet"
