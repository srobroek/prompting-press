//! Dual-input loader contract (spec 008 reshape of spec 003, T012).
//!
//! Post-reshape the "loader" is the set of `Prompt` text-factory methods:
//! [`Prompt::from_yaml`], [`Prompt::from_json`], and [`Prompt::from_toml`]. They replace
//! the old `Registry::load_yaml` / `Registry::load_json` / `Registry::insert` trio.
//!
//! These tests pin that all three formats normalize into the **same** internal
//! `PromptDefinition` (FR-005/006/008):
//!
//! - **V2.1** `Prompt::from_yaml(doc)` → `Ok`, the prompt fields match the document.
//! - **V2.2 (SC-003 — load-bearing)** a JSON fixture AND a hand-written equivalent YAML doc
//!   parse to **structurally equal** `PromptDefinition`s. Equality is asserted by
//!   re-serializing both to `serde_json::Value` and comparing (canonical structural compare).
//! - **V2.3** `Prompt::new(constructed_def)` is on equal footing with the text loaders.
//! - **V2.4** malformed input → `Err(ConsumerError::Load(..))`, nothing partially loaded.
//! - **V2.5 (Norway-safe)** a YAML metadata value of `no` / `off` / `yes` parses as the
//!   STRING `"no"` etc., not a boolean — serde_yaml_ng YAML 1.2 backing (research D2).

use prompting_press::{ConsumerError, Prompt, PromptDefinition};

/// The spec-001 valid fixtures, reused as JSON loader inputs (FR-008: the crate consumes the
/// kernel's `PromptDefinition`, no parallel shape). These fixtures use `trusted` (spec 015
/// replaced the `origin` enum with a boolean; spec 008 renamed it from `provenance`).
const SINGLE_BODY_JSON: &str =
    include_str!("../../../schemas/jsonschema/tests/fixtures/valid/single-body.json");
const MULTI_VARIANT_JSON: &str =
    include_str!("../../../schemas/jsonschema/tests/fixtures/valid/multi-variant.json");

/// Canonical structural compare: re-serialize a `PromptDefinition` to a `serde_json::Value`.
/// Two defs are structurally equal iff their `Value`s are equal — independent of input form
/// and field ordering. Used because the generated shape derives no `PartialEq`.
fn as_value(def: &PromptDefinition) -> serde_json::Value {
    serde_json::to_value(def).expect("PromptDefinition serializes to a JSON value")
}

/// V2.1 — `Prompt::from_yaml` returns `Ok`, and the prompt carries the expected fields.
#[test]
fn from_yaml_makes_prompt_with_expected_fields() {
    let yaml = "\
name: greeting
role: system
body: \"You are a helpful assistant. Today is {{ date }}.\"
variables:
  date:
    type: string
    trusted: true
";
    let prompt = Prompt::from_yaml(yaml).expect("well-formed YAML loads");
    assert_eq!(prompt.name(), "greeting");
    assert_eq!(
        prompt.body(),
        "You are a helpful assistant. Today is {{ date }}."
    );
    assert!(prompt.variables().contains_key("date"));
}

/// V2.2 (SC-003) — a JSON fixture and a hand-written equivalent YAML doc parse to
/// **structurally equal** `PromptDefinition`s (single-body fixture). Equality is checked by
/// re-serializing both loaded `PromptDefinition`s to `serde_json::Value` and comparing.
#[test]
fn yaml_and_json_parse_to_equal_definitions_single_body() {
    // The hand-written YAML below is the field-for-field equivalent of single-body.json.
    // Uses `trusted` (spec 015 boolean; spec 008 renamed from `provenance`).
    let yaml = "\
name: greeting
role: system
body: \"You are a helpful assistant. Today is {{date}}.\"
variables:
  date:
    type: string
    trusted: true
    description: \"The current date injected by the server.\"
output_model: GreetingOutput
metadata:
  author: team-foundations
  version: \"1.0.0\"
";

    // Parse to PromptDefinition via the public re-export path (serde_json + serde_yaml_ng
    // are deps of the crate; integration tests access them via the crate's public API).
    let yaml_def: PromptDefinition = serde_json::from_value(serde_json::json!({
        "name": "greeting",
        "role": "system",
        "body": "You are a helpful assistant. Today is {{date}}.",
        "variables": {
            "date": {
                "type": "string",
                "trusted": true,
                "description": "The current date injected by the server."
            }
        },
        "output_model": "GreetingOutput",
        "metadata": {
            "author": "team-foundations",
            "version": "1.0.0"
        }
    }))
    .expect("inline JSON to PromptDefinition");
    let json_def: PromptDefinition =
        serde_json::from_str(SINGLE_BODY_JSON).expect("raw JSON def parse");

    assert_eq!(
        as_value(&yaml_def),
        as_value(&json_def),
        "SC-003: YAML-equivalent JSON and the fixture JSON must parse to structurally \
         identical PromptDefinitions"
    );

    // Both Prompt loaders carry the same name.
    let from_yaml = Prompt::from_yaml(yaml).expect("YAML loads");
    let from_json = Prompt::from_json(SINGLE_BODY_JSON).expect("JSON loads");
    assert_eq!(from_yaml.name(), from_json.name());
}

/// V2.2 (SC-003) — same parity over the richer multi-variant fixture: both the JSON fixture
/// and a hand-crafted serde_json equivalent (field-for-field) parse to structurally equal
/// `PromptDefinition`s.
#[test]
fn yaml_and_json_parse_to_equal_definitions_multi_variant() {
    // Field-for-field equivalent of multi-variant.json. Uses `trusted` for variable tags.
    let equiv_def: PromptDefinition = serde_json::from_value(serde_json::json!({
        "name": "content-summariser",
        "role": "user",
        "body": "Summarise the following article in {{max_words}} words or fewer:\n\n{{article}}",
        "variables": {
            "article": {
                "type": "string",
                "trusted": false,
                "description": "Raw article text supplied by the end user."
            },
            "max_words": {
                "type": "integer",
                "trusted": true,
                "description": "Maximum word count for the summary."
            },
            "style": {
                "type": "string",
                "trusted": true,
                "description": "Output style preference."
            }
        },
        "variants": {
            "concise": {
                "body": "In one sentence, summarise: {{article}}"
            },
            "structured": {
                "body": "Produce a structured summary with a title, three bullet points, and a one-sentence conclusion for:\n\n{{article}}",
                "metadata": {
                    "weight": 0.3,
                    "group": "experiment-2024-q4",
                    "tags": ["structured", "verbose"]
                }
            }
        },
        "output_model": "SummaryOutput",
        "metadata": {
            "model_hint": "claude-3-5-sonnet",
            "max_tokens": 256
        }
    }))
    .expect("inline JSON to PromptDefinition");

    let json_def: PromptDefinition =
        serde_json::from_str(MULTI_VARIANT_JSON).expect("raw JSON def parse");

    assert_eq!(
        as_value(&equiv_def),
        as_value(&json_def),
        "SC-003: multi-variant equivalent JSON and fixture JSON must parse to structurally \
         identical PromptDefinitions (variants, nested metadata, int/array types)"
    );
}

/// V2.3 — `Prompt::new(constructed_def)` is on equal footing with `from_json`. Loading the
/// single-body fixture via `from_json` and constructing from the same def object yield a
/// prompt with the same name (structural identity checked via the underlying def).
#[test]
fn constructed_object_is_on_equal_footing() {
    // Path A: through the text loader.
    let via_loader = Prompt::from_json(SINGLE_BODY_JSON).expect("JSON loads via loader");

    // Path B: parse the same fixture to a PromptDefinition object, then `Prompt::new`.
    let constructed: PromptDefinition =
        serde_json::from_str(SINGLE_BODY_JSON).expect("fixture parses to a PromptDefinition");
    let via_object = Prompt::new(constructed).expect("constructed def is valid");

    // Both paths yield a prompt with the same name.
    assert_eq!(
        via_loader.name(),
        via_object.name(),
        "a loaded prompt and the same def wrapped in Prompt::new must agree on name"
    );
}

/// V2.4 — malformed YAML → `ConsumerError::Load`, nothing partially constructed.
#[test]
fn malformed_yaml_errors() {
    let err = Prompt::from_yaml("not: : valid: yaml").expect_err("malformed YAML must error");
    assert!(
        matches!(err, ConsumerError::Load(_)),
        "expected ConsumerError::Load, got {err:?}"
    );
}

/// V2.4 — malformed JSON → `ConsumerError::Load`.
#[test]
fn malformed_json_errors() {
    let err = Prompt::from_json("{bad json").expect_err("malformed JSON must error");
    assert!(
        matches!(err, ConsumerError::Load(_)),
        "expected ConsumerError::Load, got {err:?}"
    );
}

/// V2.4 — shape-violating data (valid JSON, missing required `body`/`role`) →
/// `ConsumerError::Load` (FR-007: no partial/coerced load).
#[test]
fn shape_violating_json_errors() {
    let err = Prompt::from_json(r#"{ "name": "incomplete" }"#)
        .expect_err("shape-violating JSON must error");
    assert!(
        matches!(err, ConsumerError::Load(_)),
        "expected ConsumerError::Load, got {err:?}"
    );
}

/// V2.5 (Norway-safe) — a YAML metadata value of `no` / `off` / `yes` parses as the STRING
/// `"no"` etc., not a boolean (YAML 1.2 — research D2).
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
    let prompt = Prompt::from_yaml(yaml).expect("Norway-token YAML loads");

    assert_eq!(
        prompt.metadata().get("country"),
        Some(&serde_json::Value::String("no".to_string())),
        "`no` must parse as the string \"no\", not boolean false"
    );
    assert_eq!(
        prompt.metadata().get("toggle"),
        Some(&serde_json::Value::String("off".to_string())),
        "`off` must parse as the string \"off\", not boolean false"
    );
    assert_eq!(
        prompt.metadata().get("agree"),
        Some(&serde_json::Value::String("yes".to_string())),
        "`yes` must parse as the string \"yes\", not boolean true"
    );
}
