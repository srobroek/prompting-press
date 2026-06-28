//! Shared test-support for the spec-006 conformance corpus runners (Rust side).
//!
//! This module is **test-only** (`crates/prompting-press/tests/common/`) and is shared by the golden
//! generator (`conformance_goldens.rs`) and the two Rust runners (`conformance_marshaling.rs`,
//! `conformance_schema.rs`) via `mod common;`.
//!
//! It contains NO engine logic (constitution Principle I / C-02). Its three jobs:
//!   1. `RawVars` — a data-driven `Serialize + garde::Validate` wrapper so the consumer's generic
//!      `render<V>` can render an arbitrary `serde_json::Value` (the corpus is data-driven; it has a
//!      value, not a compile-time struct). The `Validate` impl is a **no-op** — correct here because the
//!      corpus tests *marshaling*, not validation (validation is a higher-binding concern), and the
//!      fixture value is already well-formed (research D4).
//!   2. Fixture deserialization structs (`MarshalingFixture`, `TypedValue`, `Expected`, the schema
//!      `Manifest`/`SchemaEntry`).
//!   3. `build_value` — turn the `{type,value}` typed descriptor into a `serde_json::Value`, dropping
//!      `absent` fields (the field-not-present case; FR-008). This is the Rust arm of the D2 logical-type
//!      mapping. For Rust the "native construction" of date/decimal is just the canonical serialized
//!      string the fixture pins (the Rust reference binding marshals a string as a string — exactly the
//!      canonical form the other bindings must converge on).

#![allow(dead_code)] // each runner uses a subset of these helpers

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::{Map, Value};

// ---------------------------------------------------------------------------
// RawVars — data-driven Serialize + no-op Validate (research D4)
// ---------------------------------------------------------------------------

/// A test-only newtype wrapping an already-built `serde_json::Value`, so the consumer's
/// `render<V: Serialize + Validate>` can render any fixture's input. `Serialize` delegates to the inner
/// value; `Validate` is an intentional no-op (the corpus tests marshaling, not validation).
pub struct RawVars(pub Value);

impl serde::Serialize for RawVars {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl garde::Validate for RawVars {
    type Context = ();

    fn validate_into(
        &self,
        _ctx: &Self::Context,
        _parent: &mut dyn FnMut() -> garde::Path,
        _report: &mut garde::Report,
    ) {
        // No-op: validation is a higher-binding concern; the corpus tests marshaling only (D4 / C-02).
    }
}

// ---------------------------------------------------------------------------
// Fixture model
// ---------------------------------------------------------------------------

/// A typed value descriptor: `{ "type": <logical-type>, "value": <json> }`. `value` is optional so the
/// `absent` type (field-not-present) needs no value. `object`/`array` nest further `TypedValue`s.
#[derive(Debug, Deserialize)]
pub struct TypedValue {
    #[serde(rename = "type")]
    pub ty: String,
    #[serde(default)]
    pub value: Value,
}

/// The golden outcome committed in each marshaling fixture.
#[derive(Debug, Deserialize)]
pub struct Expected {
    pub text: String,
    pub template_hash: String,
    pub render_hash: String,
}

/// A marshaling fixture (`conformance/marshaling/<case>.json`).
#[derive(Debug, Deserialize)]
pub struct MarshalingFixture {
    pub case: String,
    #[serde(default)]
    pub description: String,
    /// The spec-001 prompt definition (kept as raw JSON; deserialized into the kernel shape on load).
    pub definition: Value,
    #[serde(default)]
    pub variant: Option<String>,
    /// Vars field name -> typed descriptor. `BTreeMap` for deterministic iteration.
    pub input: BTreeMap<String, TypedValue>,
    pub expected: Expected,
}

/// One schema round-trip fixture entry.
#[derive(Debug, Deserialize)]
pub struct SchemaEntry {
    pub path: String,
    pub form: String,    // "json" | "yaml"
    pub verdict: String, // "accept" | "reject"
    #[serde(default)]
    pub note: String,
}

/// The schema round-trip manifest (`conformance/schema/manifest.json`).
#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub fixtures: Vec<SchemaEntry>,
}

// ---------------------------------------------------------------------------
// Corpus location + IO (SEC-001: resolve within the repo root)
// ---------------------------------------------------------------------------

/// The repo root, derived from this crate's manifest dir (`crates/prompting-press` -> repo root).
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent() // crates/
        .and_then(Path::parent) // repo root
        .expect("repo root is two levels above the crate manifest dir")
        .to_path_buf()
}

/// The `conformance/` corpus directory.
pub fn corpus_dir() -> PathBuf {
    repo_root().join("conformance")
}

/// Resolve a manifest-relative path WITHIN the repo root, rejecting absolute paths and `..` escapes
/// (SEC-001 — defense-in-depth; the manifest is repo-committed but a runner must not open outside the
/// repo).
pub fn resolve_in_repo(rel: &str) -> PathBuf {
    assert!(
        !Path::new(rel).is_absolute(),
        "SEC-001: manifest path must be repo-relative, got absolute: {rel}"
    );
    assert!(
        !rel.split(['/', '\\']).any(|seg| seg == ".."),
        "SEC-001: manifest path must not escape the repo root via '..': {rel}"
    );
    repo_root().join(rel)
}

/// Load and deserialize every marshaling fixture, sorted by filename for deterministic order.
pub fn load_marshaling_fixtures() -> Vec<(PathBuf, MarshalingFixture)> {
    let dir = corpus_dir().join("marshaling");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read conformance/marshaling/ ({}): {e}", dir.display()))
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|p| {
            let text = std::fs::read_to_string(&p).expect("read fixture");
            let fx: MarshalingFixture = serde_json::from_str(&text)
                .unwrap_or_else(|e| panic!("parse {}: {e}", p.display()));
            (p, fx)
        })
        .collect()
}

/// Load the schema round-trip manifest.
pub fn load_schema_manifest() -> Manifest {
    let p = corpus_dir().join("schema").join("manifest.json");
    let text = std::fs::read_to_string(&p).expect("read schema manifest");
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse manifest: {e}"))
}

// ---------------------------------------------------------------------------
// build_value — typed descriptor -> serde_json::Value (the Rust arm of D2)
// ---------------------------------------------------------------------------

/// Build a `serde_json::Value` for one Vars field from its typed descriptor. Returns `None` for the
/// `absent` type so the caller omits the key entirely (field-not-present; FR-008).
///
/// For Rust the canonical-serialized-form choice means `datetime`/`date`/`decimal` are carried as their
/// pinned string (the reference binding marshals a string as a string — the exact canonical form the
/// Python/TS runners must converge on by constructing a native value that serializes to it).
pub fn build_value(tv: &TypedValue) -> Option<Value> {
    match tv.ty.as_str() {
        "absent" => None,
        // Primitives + canonical-string types pass their JSON value straight through.
        "string" | "int" | "float" | "bool" | "null" | "datetime" | "date" | "decimal" => {
            Some(tv.value.clone())
        }
        "object" => {
            // value is a map of field -> nested TypedValue; recurse, dropping absent fields.
            let obj = tv
                .value
                .as_object()
                .expect("object descriptor value must be a JSON object");
            let mut out = Map::new();
            for (k, v) in obj {
                let nested: TypedValue =
                    serde_json::from_value(v.clone()).expect("nested object descriptor");
                if let Some(built) = build_value(&nested) {
                    out.insert(k.clone(), built);
                }
            }
            Some(Value::Object(out))
        }
        "array" => {
            let arr = tv
                .value
                .as_array()
                .expect("array descriptor value must be a JSON array");
            let mut out = Vec::new();
            for v in arr {
                let nested: TypedValue =
                    serde_json::from_value(v.clone()).expect("nested array descriptor");
                if let Some(built) = build_value(&nested) {
                    out.push(built);
                }
            }
            Some(Value::Array(out))
        }
        other => panic!("unknown logical type tag in fixture input: {other:?}"),
    }
}

/// Build the full Vars object for a fixture's `input`, dropping `absent` fields.
pub fn build_vars(input: &BTreeMap<String, TypedValue>) -> Value {
    let mut map = Map::new();
    for (k, tv) in input {
        if let Some(v) = build_value(tv) {
            map.insert(k.clone(), v);
        }
    }
    Value::Object(map)
}
