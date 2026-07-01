"""Derive guide — add a variant at runtime: spread the current ``variants`` map (read via
the accessor) into the overlay, then add one — so existing arms survive. ``derive`` is the
sole mutator; the original is untouched.

Standalone — the docs page displays this file verbatim; run it directly to check.
"""

from prompting_press import Prompt

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

    # READ the current variants (spread), then add one — so existing arms survive.
    derived = greet.derive(
        {
            "variants": {
                **greet.variants,  # keep what's already there
                "formal": {
                    "body": "Good day, {{ name }}. You have {{ count }} messages."
                },
            }
        }
    )
    # greet is unchanged; derived is a new, fully-validated Prompt.

    assert dict(greet.variants) == {}, "original is untouched"
    assert "formal" in derived.variants


if __name__ == "__main__":
    main()
