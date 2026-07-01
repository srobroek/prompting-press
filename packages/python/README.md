# prompting-press

A typed, variant-aware **prompt-template library**. It turns typed inputs and a template into
rendered text plus content-addressed provenance — nothing else (no I/O, no LLM calls, no request
assembly). Rust, Python, and TypeScript all bind one compiled Rust engine, so rendered output is
byte-identical across every language.

This is the Python distribution: a [Pydantic](https://docs.pydantic.dev)-friendly
[PyO3](https://pyo3.rs) binding to that engine. Import name `prompting_press`; distribution name
`prompting-press`.

## Install

```bash
pip install prompting-press
```

## Quick start

```python
from prompting_press import Prompt
from pydantic import BaseModel

class Vars(BaseModel):
    name: str

greet = Prompt.from_yaml("""
name: greet
role: user
body: "Hi {{ name }}"
variables:
  name: { type: string, trusted: true }
""")

result = greet.render(Vars, data={"name": "Ada"})
result.text           # "Hi Ada"
result.template_hash  # 64-char SHA-256 of the template source
result.render_hash    # 64-char SHA-256 of the rendered output
```

## Documentation

Full docs — getting started, API reference, template features, guides, and the CI agreement
lint — are at **<https://prompting-press.github.io/>**.

## License

[Apache-2.0](https://github.com/prompting-press/prompting-press/blob/main/LICENSE).
