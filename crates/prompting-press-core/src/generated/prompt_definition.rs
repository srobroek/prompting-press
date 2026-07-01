// GENERATED FILE — DO NOT EDIT.
//
// This module is code-generated from the single source of truth:
//   schemas/jsonschema/prompt-definition.schema.json
// by cargo-typify (pinned via mise: "cargo:cargo-typify" = "0.7.0").
//
// Regenerate with: crates/prompting-press-core/scripts/codegen.sh  (re-run on schema change).
// Hand edits are overwritten and will fail the US4 freshness gate. Edit the schema.
//
// NOTE: the schema's `variants.propertyNames` (reserved-"default" rejection,
// FR-011b) is a VALIDATION constraint with no representable Rust type; it is
// stripped before typify (which cannot parse its `not`/`const` form) and is
// enforced by the US2 validation gate, not by the types below. See codegen.sh.

#![allow(clippy::doc_markdown)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::needless_lifetimes)]
#![allow(clippy::match_single_binding)]
#![allow(clippy::clone_on_copy)]

#[doc = r" Error types."]
pub mod error {
    #[doc = r" Error from a `TryFrom` or `FromStr` implementation."]
    pub struct ConversionError(::std::borrow::Cow<'static, str>);
    impl ::std::error::Error for ConversionError {}
    impl ::std::fmt::Display for ConversionError {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
            ::std::fmt::Display::fmt(&self.0, f)
        }
    }
    impl ::std::fmt::Debug for ConversionError {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
            ::std::fmt::Debug::fmt(&self.0, f)
        }
    }
    impl From<&'static str> for ConversionError {
        fn from(value: &'static str) -> Self {
            Self(value.into())
        }
    }
    impl From<String> for ConversionError {
        fn from(value: String) -> Self {
            Self(value.into())
        }
    }
}
#[doc = "The single source of truth for a prompt's shape. Per-language shapes (Pydantic v2 / TypeScript types / Rust serde structs) are code-generated from this document; it is never hand-mirrored. The library parses, generates, and round-trips this shape; it does not render, validate, or resolve it. The $id is a stable identity URI, not a live endpoint."]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"$id\": \"https://prompting-press.dev/schemas/prompt-definition.schema.json\","]
#[doc = "  \"title\": \"PromptDefinition\","]
#[doc = "  \"description\": \"The single source of truth for a prompt's shape. Per-language shapes (Pydantic v2 / TypeScript types / Rust serde structs) are code-generated from this document; it is never hand-mirrored. The library parses, generates, and round-trips this shape; it does not render, validate, or resolve it. The $id is a stable identity URI, not a live endpoint.\","]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"body\","]
#[doc = "    \"name\","]
#[doc = "    \"role\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"body\": {"]
#[doc = "      \"description\": \"The DEFAULT variant's template source. The root body IS the default arm (FR-011); surfaced under reserved name 'default' with is_default=true.\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    },"]
#[doc = "    \"metadata\": {"]
#[doc = "      \"description\": \"Arbitrary prompt-level metadata; library-OPAQUE (may include uninterpreted model/param hints, selection labels like weight/group/tags, or a `guard` key). Stored and echoed; never interpreted by the library. The prompt and each variant each carry exactly one `metadata` bag.\","]
#[doc = "      \"type\": \"object\","]
#[doc = "      \"additionalProperties\": true"]
#[doc = "    },"]
#[doc = "    \"name\": {"]
#[doc = "      \"description\": \"Logical prompt name; the caller's reference key.\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"minLength\": 1"]
#[doc = "    },"]
#[doc = "    \"output_model\": {"]
#[doc = "      \"description\": \"Optional OPAQUE reference to the caller's output model (e.g. 'NodeOutput'). Stored and echoed; never resolved, loaded, or parsed (Principle III). Shared across variants.\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    },"]
#[doc = "    \"role\": {"]
#[doc = "      \"description\": \"Conversational role; first-class metadata the caller reads. Shared across all variants.\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"enum\": ["]
#[doc = "        \"system\","]
#[doc = "        \"user\","]
#[doc = "        \"assistant\""]
#[doc = "      ]"]
#[doc = "    },"]
#[doc = "    \"variables\": {"]
#[doc = "      \"description\": \"Declared input variables, shared across all variants. Each entry declares the variable's type and input-trust flag.\","]
#[doc = "      \"default\": {},"]
#[doc = "      \"type\": \"object\","]
#[doc = "      \"additionalProperties\": {"]
#[doc = "        \"$ref\": \"#/$defs/PromptVariable\""]
#[doc = "      }"]
#[doc = "    },"]
#[doc = "    \"variants\": {"]
#[doc = "      \"description\": \"Named alternative arms. Absent => the prompt has only the default (root body) arm. Each arm differs ONLY in body (+ optional opaque metadata).\","]
#[doc = "      \"type\": \"object\","]
#[doc = "      \"additionalProperties\": {"]
#[doc = "        \"$ref\": \"#/$defs/PromptVariant\""]
#[doc = "      }"]
#[doc = "    }"]
#[doc = "  },"]
#[doc = "  \"additionalProperties\": false"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct PromptDefinition {
    #[doc = "The DEFAULT variant's template source. The root body IS the default arm (FR-011); surfaced under reserved name 'default' with is_default=true."]
    pub body: ::std::string::String,
    #[doc = "Arbitrary prompt-level metadata; library-OPAQUE (may include uninterpreted model/param hints, selection labels like weight/group/tags, or a `guard` key). Stored and echoed; never interpreted by the library. The prompt and each variant each carry exactly one `metadata` bag."]
    #[serde(default, skip_serializing_if = "::serde_json::Map::is_empty")]
    pub metadata: ::serde_json::Map<::std::string::String, ::serde_json::Value>,
    #[doc = "Logical prompt name; the caller's reference key."]
    pub name: PromptDefinitionName,
    #[doc = "Optional OPAQUE reference to the caller's output model (e.g. 'NodeOutput'). Stored and echoed; never resolved, loaded, or parsed (Principle III). Shared across variants."]
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub output_model: ::std::option::Option<::std::string::String>,
    #[doc = "Conversational role; first-class metadata the caller reads. Shared across all variants."]
    pub role: PromptDefinitionRole,
    #[doc = "Declared input variables, shared across all variants. Each entry declares the variable's type and input-trust flag."]
    #[serde(
        default,
        skip_serializing_if = ":: std :: collections :: HashMap::is_empty"
    )]
    pub variables: ::std::collections::HashMap<::std::string::String, PromptVariable>,
    #[doc = "Named alternative arms. Absent => the prompt has only the default (root body) arm. Each arm differs ONLY in body (+ optional opaque metadata)."]
    #[serde(
        default,
        skip_serializing_if = ":: std :: collections :: HashMap::is_empty"
    )]
    pub variants: ::std::collections::HashMap<::std::string::String, PromptVariant>,
}
#[doc = "Logical prompt name; the caller's reference key."]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"Logical prompt name; the caller's reference key.\","]
#[doc = "  \"type\": \"string\","]
#[doc = "  \"minLength\": 1"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct PromptDefinitionName(::std::string::String);
impl ::std::ops::Deref for PromptDefinitionName {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<PromptDefinitionName> for ::std::string::String {
    fn from(value: PromptDefinitionName) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for PromptDefinitionName {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for PromptDefinitionName {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for PromptDefinitionName {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for PromptDefinitionName {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for PromptDefinitionName {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        ::std::string::String::deserialize(deserializer)?
            .parse()
            .map_err(|e: self::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
#[doc = "Conversational role; first-class metadata the caller reads. Shared across all variants."]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"Conversational role; first-class metadata the caller reads. Shared across all variants.\","]
#[doc = "  \"type\": \"string\","]
#[doc = "  \"enum\": ["]
#[doc = "    \"system\","]
#[doc = "    \"user\","]
#[doc = "    \"assistant\""]
#[doc = "  ]"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(
    :: serde :: Deserialize,
    :: serde :: Serialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub enum PromptDefinitionRole {
    #[serde(rename = "system")]
    System,
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
}
impl ::std::fmt::Display for PromptDefinitionRole {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::System => f.write_str("system"),
            Self::User => f.write_str("user"),
            Self::Assistant => f.write_str("assistant"),
        }
    }
}
impl ::std::str::FromStr for PromptDefinitionRole {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        match value {
            "system" => Ok(Self::System),
            "user" => Ok(Self::User),
            "assistant" => Ok(Self::Assistant),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for PromptDefinitionRole {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for PromptDefinitionRole {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for PromptDefinitionRole {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
#[doc = "A declared input variable: type, trust flag, and an optional human-readable description. Validation constraints belong in the per-language validator (Zod/Pydantic/garde); the kernel is validation-blind."]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"A declared input variable: type, trust flag, and an optional human-readable description. Validation constraints belong in the per-language validator (Zod/Pydantic/garde); the kernel is validation-blind.\","]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"trusted\","]
#[doc = "    \"type\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"description\": {"]
#[doc = "      \"description\": \"Optional human-readable description of the variable.\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    },"]
#[doc = "    \"trusted\": {"]
#[doc = "      \"description\": \"Per-field input-trust flag. `true` ⇒ a trusted input (never delimited). `false` ⇒ untrusted input: when the opt-in guard is enabled, the variable's interpolated value is wrapped in injection-resistant `<untrusted>…</untrusted>` delimiters in the rendered body, and the guard advisory references the markers. Use `check()` to detect untrusted variables (`trusted: false`) that lack a declared guard.\","]
#[doc = "      \"type\": \"boolean\""]
#[doc = "    },"]
#[doc = "    \"type\": {"]
#[doc = "      \"description\": \"JSON-Schema type keyword(s) for the variable.\","]
#[doc = "      \"oneOf\": ["]
#[doc = "        {"]
#[doc = "          \"type\": \"string\","]
#[doc = "          \"enum\": ["]
#[doc = "            \"string\","]
#[doc = "            \"integer\","]
#[doc = "            \"number\","]
#[doc = "            \"boolean\","]
#[doc = "            \"array\","]
#[doc = "            \"object\""]
#[doc = "          ]"]
#[doc = "        },"]
#[doc = "        {"]
#[doc = "          \"type\": \"array\","]
#[doc = "          \"items\": {"]
#[doc = "            \"type\": \"string\","]
#[doc = "            \"enum\": ["]
#[doc = "              \"string\","]
#[doc = "              \"integer\","]
#[doc = "              \"number\","]
#[doc = "              \"boolean\","]
#[doc = "              \"array\","]
#[doc = "              \"object\","]
#[doc = "              \"null\""]
#[doc = "            ]"]
#[doc = "          }"]
#[doc = "        }"]
#[doc = "      ]"]
#[doc = "    },"]
#[doc = "    \"validation_required\": {"]
#[doc = "      \"description\": \"When true, a validator covering this variable MUST be supplied when the Prompt is constructed (spec 008). Orthogonal to `trusted` — it MAY mark any variable, trusted or not. Declarative metadata; enforcement is per-language (constitution Principle VI v1.2.0): TypeScript (Zod) and Python (Pydantic) introspect the supplied validator and throw/raise at construction if this variable is uncovered, while Rust guarantees coverage structurally at compile time. The kernel never reads this field (validation-blind).\","]
#[doc = "      \"default\": false,"]
#[doc = "      \"type\": \"boolean\""]
#[doc = "    }"]
#[doc = "  },"]
#[doc = "  \"additionalProperties\": false"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct PromptVariable {
    #[doc = "Optional human-readable description of the variable."]
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub description: ::std::option::Option<::std::string::String>,
    #[doc = "Per-field input-trust flag. `true` ⇒ a trusted input (never delimited). `false` ⇒ untrusted input: when the opt-in guard is enabled, the variable's interpolated value is wrapped in injection-resistant `<untrusted>…</untrusted>` delimiters in the rendered body, and the guard advisory references the markers. Use `check()` to detect untrusted variables (`trusted: false`) that lack a declared guard."]
    pub trusted: bool,
    #[doc = "JSON-Schema type keyword(s) for the variable."]
    #[serde(rename = "type")]
    pub type_: PromptVariableType,
    #[doc = "When true, a validator covering this variable MUST be supplied when the Prompt is constructed (spec 008). Orthogonal to `trusted` — it MAY mark any variable, trusted or not. Declarative metadata; enforcement is per-language (constitution Principle VI v1.2.0): TypeScript (Zod) and Python (Pydantic) introspect the supplied validator and throw/raise at construction if this variable is uncovered, while Rust guarantees coverage structurally at compile time. The kernel never reads this field (validation-blind)."]
    #[serde(default)]
    pub validation_required: bool,
}
#[doc = "JSON-Schema type keyword(s) for the variable."]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"JSON-Schema type keyword(s) for the variable.\","]
#[doc = "  \"oneOf\": ["]
#[doc = "    {"]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"enum\": ["]
#[doc = "        \"string\","]
#[doc = "        \"integer\","]
#[doc = "        \"number\","]
#[doc = "        \"boolean\","]
#[doc = "        \"array\","]
#[doc = "        \"object\""]
#[doc = "      ]"]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"type\": \"array\","]
#[doc = "      \"items\": {"]
#[doc = "        \"type\": \"string\","]
#[doc = "        \"enum\": ["]
#[doc = "          \"string\","]
#[doc = "          \"integer\","]
#[doc = "          \"number\","]
#[doc = "          \"boolean\","]
#[doc = "          \"array\","]
#[doc = "          \"object\","]
#[doc = "          \"null\""]
#[doc = "        ]"]
#[doc = "      }"]
#[doc = "    }"]
#[doc = "  ]"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
#[serde(untagged)]
pub enum PromptVariableType {
    String(PromptVariableTypeString),
    Array(::std::vec::Vec<PromptVariableTypeArrayItem>),
}
impl ::std::convert::From<PromptVariableTypeString> for PromptVariableType {
    fn from(value: PromptVariableTypeString) -> Self {
        Self::String(value)
    }
}
impl ::std::convert::From<::std::vec::Vec<PromptVariableTypeArrayItem>> for PromptVariableType {
    fn from(value: ::std::vec::Vec<PromptVariableTypeArrayItem>) -> Self {
        Self::Array(value)
    }
}
#[doc = "`PromptVariableTypeArrayItem`"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"type\": \"string\","]
#[doc = "  \"enum\": ["]
#[doc = "    \"string\","]
#[doc = "    \"integer\","]
#[doc = "    \"number\","]
#[doc = "    \"boolean\","]
#[doc = "    \"array\","]
#[doc = "    \"object\","]
#[doc = "    \"null\""]
#[doc = "  ]"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(
    :: serde :: Deserialize,
    :: serde :: Serialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub enum PromptVariableTypeArrayItem {
    #[serde(rename = "string")]
    String,
    #[serde(rename = "integer")]
    Integer,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "boolean")]
    Boolean,
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "object")]
    Object,
    #[serde(rename = "null")]
    Null,
}
impl ::std::fmt::Display for PromptVariableTypeArrayItem {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::String => f.write_str("string"),
            Self::Integer => f.write_str("integer"),
            Self::Number => f.write_str("number"),
            Self::Boolean => f.write_str("boolean"),
            Self::Array => f.write_str("array"),
            Self::Object => f.write_str("object"),
            Self::Null => f.write_str("null"),
        }
    }
}
impl ::std::str::FromStr for PromptVariableTypeArrayItem {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        match value {
            "string" => Ok(Self::String),
            "integer" => Ok(Self::Integer),
            "number" => Ok(Self::Number),
            "boolean" => Ok(Self::Boolean),
            "array" => Ok(Self::Array),
            "object" => Ok(Self::Object),
            "null" => Ok(Self::Null),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for PromptVariableTypeArrayItem {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for PromptVariableTypeArrayItem {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for PromptVariableTypeArrayItem {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
#[doc = "`PromptVariableTypeString`"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"type\": \"string\","]
#[doc = "  \"enum\": ["]
#[doc = "    \"string\","]
#[doc = "    \"integer\","]
#[doc = "    \"number\","]
#[doc = "    \"boolean\","]
#[doc = "    \"array\","]
#[doc = "    \"object\""]
#[doc = "  ]"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(
    :: serde :: Deserialize,
    :: serde :: Serialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub enum PromptVariableTypeString {
    #[serde(rename = "string")]
    String,
    #[serde(rename = "integer")]
    Integer,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "boolean")]
    Boolean,
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "object")]
    Object,
}
impl ::std::fmt::Display for PromptVariableTypeString {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::String => f.write_str("string"),
            Self::Integer => f.write_str("integer"),
            Self::Number => f.write_str("number"),
            Self::Boolean => f.write_str("boolean"),
            Self::Array => f.write_str("array"),
            Self::Object => f.write_str("object"),
        }
    }
}
impl ::std::str::FromStr for PromptVariableTypeString {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        match value {
            "string" => Ok(Self::String),
            "integer" => Ok(Self::Integer),
            "number" => Ok(Self::Number),
            "boolean" => Ok(Self::Boolean),
            "array" => Ok(Self::Array),
            "object" => Ok(Self::Object),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for PromptVariableTypeString {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for PromptVariableTypeString {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for PromptVariableTypeString {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
#[doc = "A named alternative arm. May carry ONLY body and metadata; redefining role/variables/output_model is rejected (FR-011a)."]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"A named alternative arm. May carry ONLY body and metadata; redefining role/variables/output_model is rejected (FR-011a).\","]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"body\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"body\": {"]
#[doc = "      \"description\": \"The variant's template source — the only field that differs per variant.\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    },"]
#[doc = "    \"metadata\": {"]
#[doc = "      \"description\": \"Library-OPAQUE per-variant metadata (selection labels like weight/group/tags, or a `guard` key). Stored + exposed; never interpreted by the library (caller selects). No schema-enforced selection semantics (FR-011c). Mirrors the prompt-level `metadata` bag.\","]
#[doc = "      \"type\": \"object\","]
#[doc = "      \"additionalProperties\": true"]
#[doc = "    }"]
#[doc = "  },"]
#[doc = "  \"additionalProperties\": false"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct PromptVariant {
    #[doc = "The variant's template source — the only field that differs per variant."]
    pub body: ::std::string::String,
    #[doc = "Library-OPAQUE per-variant metadata (selection labels like weight/group/tags, or a `guard` key). Stored + exposed; never interpreted by the library (caller selects). No schema-enforced selection semantics (FR-011c). Mirrors the prompt-level `metadata` bag."]
    #[serde(default, skip_serializing_if = "::serde_json::Map::is_empty")]
    pub metadata: ::serde_json::Map<::std::string::String, ::serde_json::Value>,
}
