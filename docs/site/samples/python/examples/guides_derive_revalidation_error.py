"""Derive guide — re-validation on overlay: overlaying a body that references an
undeclared variable raises ``PromptRenderError`` (agreement failure over the merged whole).

Standalone — the docs page displays this file verbatim; run it directly to check.
"""

from prompting_press import Prompt, PromptRenderError

greet_yaml = """
name: greet
role: user
body: "Hi {{ name }}, you have {{ count }} messages."
variables:
  name: { type: string, trusted: true }
  count: { type: integer, trusted: true }
"""


def main() -> None:
    greet = Prompt.from_yaml(greet_yaml)

    try:
        bad = greet.derive({"body": "Hi {{ ghost }}"})
    except PromptRenderError as exc:
        print(exc.errors[0].code)  # "undefined_variable"
        print(exc.errors[0].field)  # "ghost"
        assert exc.errors[0].code == "undefined_variable"
        assert exc.errors[0].field == "ghost"
    else:
        raise AssertionError(f"expected PromptRenderError, got {bad!r}")


if __name__ == "__main__":
    main()
