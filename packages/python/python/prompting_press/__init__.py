"""Prompting Press — a typed, variant-aware prompt-template library.

This package is the **Python binding** over the shared Rust core: prompts are parsed,
validated, rendered, hashed, and lint-checked once in Rust (Principle I), and Python only
marshals typed values across the FFI boundary — the binding contains no rendering, hashing,
or analysis logic of its own (Principle II / roadmap decision C-02).

The public API is re-exported here from the compiled Rust extension (the PyO3 binding crate
``crates/prompting-press-py``), built and merged into this package by maturin. In the mixed
Rust/Python layout the extension lands as the submodule ``prompting_press.prompting_press``;
this ``__init__`` re-exports its public names so callers use ``prompting_press.Prompt`` etc.
``PromptDefinition`` is the Pydantic prompt-definition shape, code-generated from the published
JSON Schema (decision C-07).

See ``packages/python/README.md`` for a runnable quickstart.
"""

from __future__ import annotations

from importlib.metadata import PackageNotFoundError, version

from .prompting_press import (  # the compiled extension submodule
    CheckReport,
    Composition,
    FieldError,
    Finding,
    GuardConfig,
    LoadError,
    Message,
    Prompt,
    PromptingPressError,
    PromptRenderError,
    PromptValidationError,
    RenderResult,
    core_version,
)

# The generated Pydantic prompt-definition shapes (codegen'd from the JSON Schema — C-07).
from .generated import PromptDefinition, PromptVariable, PromptVariant

try:
    # The PyPI distribution name is ``prompting-press`` (the import name is ``prompting_press``).
    __version__ = version("prompting-press")
except PackageNotFoundError:  # pragma: no cover — editable / unbuilt source tree
    __version__ = "0.0.0"

__all__ = [
    # Primary public type (spec 008 Phase 4).
    "Prompt",
    # Result + config types.
    "RenderResult",
    "GuardConfig",
    # Structured error row.
    "FieldError",
    # Lint report types.
    "CheckReport",
    "Finding",
    # Multi-message composition.
    "Composition",
    "Message",
    # Kernel version accessor.
    "core_version",
    # Generated prompt-definition shapes.
    "PromptDefinition",
    "PromptVariable",
    "PromptVariant",
    # Exception hierarchy.
    "PromptingPressError",
    "PromptValidationError",
    "PromptRenderError",
    "LoadError",
]
