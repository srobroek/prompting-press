"""Derive guide — re-validation on overlay: overlaying a body that references an
undeclared variable raises ``PromptRenderError`` (agreement failure over the merged whole).

Standalone — the docs page displays this file verbatim; run it directly to check.
"""

from prompting_press import Prompt, PromptRenderError

assistant_yaml = """
name: assistant
role: system
body: "You are a support assistant for {{ company }}. Keep your replies under {{ max_words }} words."
variables:
  company: { type: string, trusted: true }
  max_words: { type: integer, trusted: true }
"""


def main() -> None:
    assistant = Prompt.from_yaml(assistant_yaml)

    try:
        bad = assistant.derive({"body": "You help {{ ghost }}."})
    except PromptRenderError as exc:
        print(exc.errors[0].code)  # "undefined_variable"
        print(exc.errors[0].field)  # "ghost"
        assert exc.errors[0].code == "undefined_variable"
        assert exc.errors[0].field == "ghost"
    else:
        raise AssertionError(f"expected PromptRenderError, got {bad!r}")


if __name__ == "__main__":
    main()
