"""Derive guide — re-validation on overlay: overlaying a body that references an
undeclared variable raises ``PromptRenderError`` (agreement failure over the merged whole).

Standalone — the docs page displays this file verbatim; run it directly to check.
"""

from pathlib import Path

from prompting_press import Prompt, PromptRenderError

# The caller reads the definition; the library does no file I/O itself.
# Resolve the file next to this program (a real app uses its own path).
_HERE = Path(__file__).parent


def main() -> None:
    assistant = Prompt.from_yaml((_HERE / "assistant.yaml").read_text())

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
