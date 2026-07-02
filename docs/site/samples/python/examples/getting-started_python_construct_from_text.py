"""Construct a Prompt from YAML text with a from_* factory (validates immediately)."""

from prompting_press import Prompt

# The caller reads the text; the library does no file I/O itself.
# (This program embeds the document so it runs standalone; a real caller would
#  read it from a file, e.g. `with open("assistant.yaml") as f: f.read()`.)
ASSISTANT_YAML = """\
name: assistant
role: system
body: "You are a support assistant for {{ company }}. Keep your replies under {{ max_words }} words."
variables:
  company:
    type: string
    trusted: true
  max_words:
    type: integer
    trusted: true
"""

assistant = Prompt.from_yaml(ASSISTANT_YAML)  # validates here, or raises
# from_json / from_toml parse the JSON / TOML forms into the same Prompt.

assert assistant.name == "assistant"
assert (
    assistant.body
    == "You are a support assistant for {{ company }}. Keep your replies under {{ max_words }} words."
)
