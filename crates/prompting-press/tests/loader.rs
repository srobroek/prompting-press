//! US2 dual-input loader contract (spec 003, T012).
//!
//! The dual-input loader normalizes a YAML document, a JSON document, and a constructed
//! object into the **same** internal `PromptDefinition` (FR-005/006/008). These tests pin
//! that contract:
//!
//! - **V2.1** `Registry::load_yaml(doc)` → `Ok`, the def is now `get`-able and matches the
//!   expected fields.
//! - **V2.2 (SC-003 — load-bearing)** a JSON fixture AND a hand-written equivalent YAML doc
//!   parse to **structurally equal** `PromptDefinition`s. The generated `PromptDefinition`
//!   derives no `PartialEq`, so equality is asserted by re-serializing both to
//!   `serde_json::Value` and comparing those (canonical structural compare — NOT a smoke
//!   check).
//! - **V2.3** `insert(constructed_def)` works on equal footing — a def loaded from YAML and
//!   the same def inserted as an object are byte-identical once retrieved.
//! - **V2.4** malformed input → `Err(ConsumerError::Load(..))`, and **nothing is inserted**
//!   (FR-007: no partial load).
//! - **V2.5 (Norway-safe)** a YAML metadata value of `no` / `off` / `yes` parses as the
//!   STRING `"no"` etc., not a boolean — serde_yaml_ng is backed by yaml-rust2 (YAML 1.2),
//!   where those tokens are plain-scalar strings (research D2).

use prompting_press::{ConsumerError, PromptDefinition, Registry};

/// The spec-001 valid fixtures, reused as JSON loader inputs (FR-008: the crate consumes the
/// kernel's `PromptDefinition`, no parallel shape).
const SINGLE_BODY_JSON: &str =
    include_str!("../../../schemas/jsonschema/tests/fixtures/valid/single-body.json");
const MULTI_VARIANT_JSON: &str =
    include_str!("../../../schemas/jsonschema/tests/fixtures/valid/multi-variant.json");

/// Canonical structural compare: re-serialize a `PromptDefinition` to a `serde_json::Value`.
/// Two defs are structurally equal iff their `Value`s are equal — independent of input form
/// (YAML vs JSON vs object) and field ordering (`serde_json::Value` object compare is
/// order-insensitive). Used because the generated shape derives no `PartialEq`.
fn as_value(def: &PromptDefinition) -> serde_json::Value {
    serde_json::to_value(def).expect("PromptDefinition serializes to a JSON value")
}

/// V2.1 — `load_yaml` returns `Ok`, and the loaded def is retrievable and carries the
/// expected fields.
#[test]
fn load_yaml_makes_definition_gettable() {
    let yaml = "\
name: greeting
role: system
body: \"You are a helpful assistant. Today is {{date}}.\"
";
    let mut reg = Registry::new();

    let loaded = reg.load_yaml(yaml).expect("well-formed YAML loads");
    // The returned ref reflects the parsed fields.
    assert_eq!(loaded.name.to_string(), "greeting");
    assert_eq!(
        loaded.body,
        "You are a helpful assistant. Today is {{date}}."
    );

    // It is now resolvable by name through the registry.
    let got = reg.get("greeting").expect("present after load_yaml");
    assert_eq!(got.name.to_string(), "greeting");
    assert_eq!(
        as_value(got)["role"],
        serde_json::json!("system"),
        "role must round-trip through the loader"
    );
}

/// V2.2 (SC-003) — a JSON fixture and a hand-written EQUIVALENT YAML doc parse to
/// **structurally equal** `PromptDefinition`s (single-body fixture).
#[test]
fn yaml_and_json_parse_to_equal_definitions_single_body() {
    // The hand-written YAML below is the field-for-field equivalent of single-body.json.
    let yaml = "\
name: greeting
role: system
body: \"You are a helpful assistant. Today is {{date}}.\"
variables:
  date:
    type: string
    provenance: trusted
    format: date
    description: \"The current date injected by the server.\"
output_model: GreetingOutput
metadata:
  author: team-foundations
  version: \"1.0.0\"
";

    let mut yaml_reg = Registry::new();
    let mut json_reg = Registry::new();
    let from_yaml = yaml_reg.load_yaml(yaml).expect("YAML loads");
    let from_json = json_reg.load_json(SINGLE_BODY_JSON).expect("JSON loads");

    assert_eq!(
        as_value(from_yaml),
        as_value(from_json),
        "SC-003: YAML and the equivalent JSON must parse to structurally identical \
         PromptDefinitions"
    );
}

/// V2.2 (SC-003) — same parity over the richer multi-variant fixture (variants + nested
/// opaque `meta`, integer + array variable types), to prove parity holds beyond the trivial
/// shape.
#[test]
fn yaml_and_json_parse_to_equal_definitions_multi_variant() {
    // Field-for-field equivalent of multi-variant.json. Integers stay integers; nested
    // variant `meta` (weight/group/tags) and the prompt `metadata` (model_hint/max_tokens)
    // round-trip identically.
    let yaml = "\
name: content-summariser
role: user
body: \"Summarise the following article in {{max_words}} words or fewer:\\n\\n{{article}}\"
variables:
  article:
    type: string
    provenance: untrusted
    minLength: 1
    description: \"Raw article text supplied by the end user.\"
  max_words:
    type: integer
    provenance: trusted
    minimum: 10
    maximum: 500
    description: \"Maximum word count for the summary.\"
  style:
    type: string
    provenance: trusted
    enum:
      - bullet
      - prose
      - headline
    description: \"Output style preference.\"
variants:
  concise:
    body: \"In one sentence, summarise: {{article}}\"
  structured:
    body: \"Produce a structured summary with a title, three bullet points, and a one-sentence conclusion for:\\n\\n{{article}}\"
    meta:
      weight: 0.3
      group: experiment-2024-q4
      tags:
        - structured
        - verbose
output_model: SummaryOutput
metadata:
  model_hint: claude-3-5-sonnet
  max_tokens: 256
";

    let mut yaml_reg = Registry::new();
    let mut json_reg = Registry::new();
    let from_yaml = yaml_reg.load_yaml(yaml).expect("YAML loads");
    let from_json = json_reg.load_json(MULTI_VARIANT_JSON).expect("JSON loads");

    assert_eq!(
        as_value(from_yaml),
        as_value(from_json),
        "SC-003: multi-variant YAML and JSON must parse to structurally identical \
         PromptDefinitions (variants, nested meta, int/array types)"
    );
}

/// V2.3 — a constructed object is on equal footing with a loaded one. Loading the
/// single-body fixture via `load_json` and inserting the SAME def (parsed directly into a
/// `PromptDefinition` object) yield byte-identical retrieved values.
#[test]
fn constructed_object_is_on_equal_footing() {
    // Path A: through the loader.
    let mut loaded_reg = Registry::new();
    loaded_reg
        .load_json(SINGLE_BODY_JSON)
        .expect("JSON loads via loader");

    // Path B: parse the same fixture to an object, then `insert` it (the constructed-object
    // path — FR-005, US2 scenario 3).
    let constructed: PromptDefinition =
        serde_json::from_str(SINGLE_BODY_JSON).expect("fixture parses to a PromptDefinition");
    let mut object_reg = Registry::new();
    object_reg.insert(constructed);

    let via_loader = loaded_reg.get("greeting").expect("present via loader");
    let via_object = object_reg.get("greeting").expect("present via insert");

    assert_eq!(
        as_value(via_loader),
        as_value(via_object),
        "a loaded def and the same def inserted as an object must be structurally identical \
         (no second-class path)"
    );
}

/// V2.4 — malformed YAML → `ConsumerError::Load`, and NOTHING is inserted (FR-007).
#[test]
fn malformed_yaml_errors_and_inserts_nothing() {
    let mut reg = Registry::new();

    let err = reg
        .load_yaml("not: : valid: yaml")
        .expect_err("malformed YAML must error");
    assert!(
        matches!(err, ConsumerError::Load(_)),
        "expected ConsumerError::Load, got {err:?}"
    );

    // Nothing partially loaded: the registry is untouched.
    assert!(
        reg.get("not").is_none() && reg.get("").is_none(),
        "a failed load must insert nothing"
    );
}

/// V2.4 — malformed JSON → `ConsumerError::Load`, and NOTHING is inserted (FR-007).
#[test]
fn malformed_json_errors_and_inserts_nothing() {
    let mut reg = Registry::new();

    let err = reg
        .load_json("{bad json")
        .expect_err("malformed JSON must error");
    assert!(
        matches!(err, ConsumerError::Load(_)),
        "expected ConsumerError::Load, got {err:?}"
    );
    assert!(
        reg.get("bad").is_none(),
        "a failed load must insert nothing"
    );
}

/// V2.4 — shape-violating data (valid JSON, but missing the required `body`/`role` fields)
/// → `ConsumerError::Load`, and nothing inserted (FR-007: no partial/coerced load).
#[test]
fn shape_violating_json_errors_and_inserts_nothing() {
    let mut reg = Registry::new();

    // Valid JSON, but the PromptDefinition shape requires `role` and `body` too.
    let err = reg
        .load_json(r#"{ "name": "incomplete" }"#)
        .expect_err("shape-violating JSON must error");
    assert!(
        matches!(err, ConsumerError::Load(_)),
        "expected ConsumerError::Load, got {err:?}"
    );
    assert!(
        reg.get("incomplete").is_none(),
        "a shape-violating load must insert nothing"
    );
}

/// V2.5 (Norway-safe) — a YAML metadata value of `no` / `off` / `yes` parses as the STRING
/// `"no"` (etc.), not a boolean. serde_yaml_ng is backed by yaml-rust2 (YAML 1.2), where
/// those tokens are plain-scalar strings (research D2). The `metadata` map is free-form
/// (`serde_json::Map<String, Value>`), so it carries the value verbatim.
#[test]
fn norway_tokens_parse_as_strings_not_bools() {
    let yaml = "\
name: norway
role: system
body: \"hi\"
metadata:
  country: no
  toggle: off
  agree: yes
";
    let mut reg = Registry::new();
    let def = reg.load_yaml(yaml).expect("Norway-token YAML loads");

    // Each value must be a STRING, never a bool. (YAML 1.1 would have coerced these to
    // false/false/true — the bug serde_yaml_ng's YAML-1.2 backing avoids.)
    assert_eq!(
        def.metadata.get("country"),
        Some(&serde_json::Value::String("no".to_string())),
        "`no` must parse as the string \"no\", not boolean false"
    );
    assert_eq!(
        def.metadata.get("toggle"),
        Some(&serde_json::Value::String("off".to_string())),
        "`off` must parse as the string \"off\", not boolean false"
    );
    assert_eq!(
        def.metadata.get("agree"),
        Some(&serde_json::Value::String("yes".to_string())),
        "`yes` must parse as the string \"yes\", not boolean true"
    );
}
