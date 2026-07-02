"""Construct a Prompt from a definition file with a from_* factory (validates immediately)."""

from pathlib import Path

from prompting_press import Prompt

# The caller reads the definition; the library does no file I/O itself.
# Resolve the file next to this program (a real app uses its own path).
_HERE = Path(__file__).parent

assistant = Prompt.from_yaml(
    (_HERE / "assistant.yaml").read_text()
)  # validates here, or raises
# The same definition in JSON or TOML parses into an identical Prompt:
# assistant = Prompt.from_json((_HERE / "assistant.json").read_text())
# assistant = Prompt.from_toml((_HERE / "assistant.toml").read_text())

assert assistant.name == "assistant"
assert (
    assistant.body
    == "You are a support assistant for {{ company }}. Keep your replies under {{ max_words }} words."
)
