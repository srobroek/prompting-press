"""Construct a Prompt from YAML text with a from_* factory (validates immediately)."""

from prompting_press import Prompt

# The caller reads the text; the library does no file I/O itself.
# (This program embeds the document so it runs standalone; a real caller would
#  read it from a file, e.g. `with open("greet.yaml") as f: f.read()`.)
GREET_YAML = """\
name: greet
role: user
body: "Hi {{ name }}, you have {{ count }} messages."
variables:
  name:
    type: string
    trusted: true
  count:
    type: integer
    trusted: true
"""

greet = Prompt.from_yaml(GREET_YAML)     # validates here, or raises
# from_json / from_toml parse the JSON / TOML forms into the same Prompt.

assert greet.name == "greet"
assert greet.body == "Hi {{ name }}, you have {{ count }} messages."
