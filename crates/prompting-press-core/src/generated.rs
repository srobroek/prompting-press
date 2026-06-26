//! Segregated home for code-generated shape modules (FR-016).
//!
//! Everything under this module is emitted from the single source of truth
//! `schemas/jsonschema/prompt-definition.schema.json` by `cargo-typify`. The
//! files here are MARKED-GENERATED and freshness-gated in CI (US4); never
//! hand-edit them. Regenerate with `crates/prompting-press-core/scripts/codegen.sh`.
//!
//! This wrapper module is hand-written (it is the `mod` declaration, not generated
//! content) so the generated file stays a clean module file carrying its own
//! `#![allow(...)]` inner attributes, rather than being `include!`d mid-file.

pub mod prompt_definition;
