# Kernel test fixtures (spec 002)

Reusable fixture data for the engine-kernel test suites. The harness that loads
these lives in `../common/mod.rs`.

## Self-referential-grep mitigation (spec-001 lesson)

CI runs forbidden-pattern greps over **Rust source** to defend the v1 template
boundary (no `{% include %}` / `{% import %}` / `{% extends %}` / `{% macro %}` /
`{% block %}`) and, in places, over interpolation markers. Those scans must not be
tripped by the test corpus's *own* example templates.

Mitigation: **every fixture template body lives in a data file under this
directory** (`*.json`), never inline in a `.rs` test or helper. Rust sources only
carry the loader logic and `Path` strings — no template bodies, no `{{ … }}` or
`{% … %}` literals. A grep scoped to `**/*.rs` therefore never matches a fixture's
template, while the data files remain plain UTF-8 the loader deserializes at runtime.

When a later task adds an *excluded-feature rejection* fixture (a template that
SHOULD fail to parse), keep it the same way: the offending `{% include %}` etc.
goes in a data file, and the test asserts the kernel rejects it — the forbidden
token never appears in scannable Rust source.

## Layout

- `render/*.json` — `(template, values) -> expected` render regression cases
  (`RegressionCase` in the harness). These are an engine regression guard only
  (constitution Principle VII); cross-language parity is structural, not retested.
- Spec-001 schema fixtures are NOT duplicated here. The harness loads them in place
  from `schemas/jsonschema/fixtures/valid/*.json` as `PromptDefinition` inputs.
