"""Derive guide — add a variant at runtime: spread the current ``variants`` map (read via
the accessor) into the overlay, then add one — so existing arms survive. ``derive`` is the
sole mutator; the original is untouched.

Standalone — the docs page displays this file verbatim; run it directly to check.
"""

from pathlib import Path

from prompting_press import Prompt

# The caller reads the definition; the library does no file I/O itself.
# Resolve the file next to this program (a real app uses its own path).
_HERE = Path(__file__).parent


def main() -> None:
    assistant = Prompt.from_yaml((_HERE / "assistant.yaml").read_text())

    # READ the current variants (spread), then add one — so existing arms survive.
    derived = assistant.derive(
        {
            "variants": {
                **assistant.variants,  # keep what's already there
                "formal": {
                    "body": "You are the official support assistant for {{ company }}. "
                    "Please keep every reply under {{ max_words }} words."
                },
            }
        }
    )
    # assistant is unchanged; derived is a new, fully-validated Prompt.

    assert dict(assistant.variants) == {}, "original is untouched"
    assert "formal" in derived.variants


if __name__ == "__main__":
    main()
