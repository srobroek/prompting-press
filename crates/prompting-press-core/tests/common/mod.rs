//! Shared test-fixture harness for the engine-kernel suites (spec 002, T012).
//!
//! This is the reusable loader the per-user-story suites (later tasks) build on. It
//! does two things:
//!
//! 1. Loads `(template, values) -> expected` **render regression cases** from JSON
//!    data files under `tests/fixtures/render/` ([`load_regression_case`]).
//! 2. Loads **spec-001 schema fixtures** from `schemas/jsonschema/fixtures/valid/*.json`
//!    and deserializes them into the kernel's [`PromptDefinition`]
//!    ([`load_prompt_definition`]).
//!
//! ## Self-referential-grep mitigation (spec-001 lesson)
//!
//! CI greps Rust **source** for forbidden v1 template features and interpolation
//! markers. To keep the fixture corpus's own templates out of that scan, **no template
//! body is ever inlined in a `.rs` file** — they live exclusively in the JSON data
//! files this module reads at runtime. This module carries only loader logic and path
//! strings; it contains no `{{ … }}` / `{% … %}` literals. See `tests/fixtures/README.md`.

#![allow(dead_code)] // Shared harness: each tests/*.rs compiles as its own crate including this
                     // module, so a helper used by only some suites reads as dead in the others.

use std::path::PathBuf;

use prompting_press_core::PromptDefinition;

/// A single render regression case loaded from `tests/fixtures/render/*.json`.
///
/// The `template` and `expected` fields are intentionally *data* (deserialized at
/// runtime), never Rust string literals, so a forbidden-pattern grep over `**/*.rs`
/// never sees a fixture template.
#[derive(Debug, serde::Deserialize)]
pub struct RegressionCase {
    /// Human-readable note describing what the case exercises.
    #[serde(default)]
    pub description: String,
    /// The template source to render.
    pub template: String,
    /// The render context, as an arbitrary JSON object.
    pub values: serde_json::Value,
    /// The expected rendered output (byte-for-byte).
    pub expected: String,
}

/// Absolute path to this crate's root (`crates/prompting-press-core`).
fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Absolute path to the repository root (two levels above the crate root:
/// `crates/prompting-press-core` -> `crates` -> repo root).
fn repo_root() -> PathBuf {
    crate_root()
        .ancestors()
        .nth(2)
        .expect("crate is nested at <repo>/crates/<crate>; repo root is two levels up")
        .to_path_buf()
}

/// Load a render regression case by file stem from `tests/fixtures/render/<name>.json`.
///
/// # Panics
/// Panics (a test-only contract) if the fixture is missing or malformed.
pub fn load_regression_case(name: &str) -> RegressionCase {
    let path = crate_root()
        .join("tests")
        .join("fixtures")
        .join("render")
        .join(format!("{name}.json"));
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read regression fixture {}: {e}", path.display()));
    serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("parse regression fixture {}: {e}", path.display()))
}

/// Load a spec-001 valid schema fixture by file stem from
/// `schemas/jsonschema/fixtures/valid/<name>.json` and deserialize it into the
/// kernel's [`PromptDefinition`] (FR-027 — the kernel consumes the spec-001 shape).
///
/// # Panics
/// Panics (a test-only contract) if the fixture is missing or does not deserialize
/// into a `PromptDefinition`.
pub fn load_prompt_definition(name: &str) -> PromptDefinition {
    let path = repo_root()
        .join("schemas")
        .join("jsonschema")
        .join("fixtures")
        .join("valid")
        .join(format!("{name}.json"));
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read schema fixture {}: {e}", path.display()));
    serde_json::from_str(&raw).unwrap_or_else(|e| {
        panic!(
            "deserialize schema fixture {} as PromptDefinition: {e}",
            path.display()
        )
    })
}

/// Load a kernel-local `PromptDefinition` fixture by file stem from
/// `tests/fixtures/defs/<name>.json`.
///
/// These are kernel-test-owned prompt definitions (distinct from the spec-001 schema
/// fixtures loaded by [`load_prompt_definition`]) used where a render suite needs a
/// specific template body that the spec-001 corpus does not provide. Like the render
/// regression cases, their template bodies live **only** in JSON data files — never
/// inlined in a `.rs` source — so the forbidden-pattern grep over `**/*.rs` never sees
/// them (see `tests/fixtures/README.md`).
///
/// # Panics
/// Panics (a test-only contract) if the fixture is missing or does not deserialize
/// into a `PromptDefinition`.
pub fn load_def_fixture(name: &str) -> PromptDefinition {
    let path = crate_root()
        .join("tests")
        .join("fixtures")
        .join("defs")
        .join(format!("{name}.json"));
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read def fixture {}: {e}", path.display()));
    serde_json::from_str(&raw).unwrap_or_else(|e| {
        panic!(
            "deserialize def fixture {} as PromptDefinition: {e}",
            path.display()
        )
    })
}
