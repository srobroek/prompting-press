# prompting-press (Python)

Python distribution of [Prompting Press](https://github.com/srobroek/prompting-press) —
a thin wrapper around the shared Rust core, exposed via a PyO3 binding crate and
packaged with [maturin](https://www.maturin.rs/).

> **Status: skeleton (spec 001 / FR-004).** No runtime logic ships yet. The
> importable `prompting_press` module is the compiled Rust extension, which
> arrives once the binding crate (US1) and code generation (US3) land. Nothing
> is published from spec 001 (version `0.0.0`).

## Layout

```
packages/python/
├── pyproject.toml                  # maturin build backend; points at the binding crate
├── README.md
└── python/
    └── prompting_press/
        └── __init__.py             # package marker (logic-free)
```

The PyO3 binding crate lives outside this directory at
`crates/prompting-press-py/` (a workspace member). `pyproject.toml`'s
`[tool.maturin].manifest-path` references it across the repo.
