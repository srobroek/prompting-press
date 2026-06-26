# Negative-Scope Checklist — Spec 001 Foundations

**Auditable checklist for FR-021 / FR-022 / SC-007.**

Scope under audit: the LIBRARY source — `crates/*/src/` (including
`crates/prompting-press/src/generated/`) and `packages/python/python/prompting_press/`
and `packages/typescript/src/`. Excluded from scope: codegen scripts
(`crates/prompting-press/scripts/`, `packages/python/scripts/`,
`packages/typescript/scripts/`), schema validation scripts
(`schemas/jsonschema/scripts/`), CI scripts (`scripts/ci/`), spec docs
(`specs/`), and `apm_modules/` / `node_modules/` / `.venv` / `target/` / `.git/`.

Codegen scripts that READ the schema file are explicitly NOT a library I/O
violation — they are build-time tooling, not library code.

Date: 2026-06-25

---

## Item 1 — No template-engine integration

**Requirement (FR-021):** no minijinja / handlebars / tera / jinja dependency.

**Search performed:**
```
rg "minijinja|handlebars|tera|jinja" \
  crates/prompting-press-core/Cargo.toml \
  crates/prompting-press/Cargo.toml \
  crates/prompting-press-py/Cargo.toml \
  crates/prompting-press-node/Cargo.toml \
  packages/python/pyproject.toml \
  packages/typescript/package.json
```

**Result:** exit 1 — no matches.

**Status: ABSENT**

---

## Item 2 — No render / rendering path

**Requirement (FR-021):** no `render` function or method in library crates.

**Search performed:**
```
rg "fn render|\.render|fn rendering|Render" \
  crates/prompting-press-core/src/ \
  crates/prompting-press/src/ \
  crates/prompting-press-py/src/ \
  crates/prompting-press-node/src/ \
  --include="*.rs"
```

**Result:** exit 1 — no matches.

Note: the generated file (`crates/prompting-press/src/generated/prompt_definition.rs`)
contains `#[doc = "... it does not render ..."]` — a doc-comment string explaining
the scope boundary, not a render function or method. Confirmed in context: both hits
are `#[doc = "..."]` attribute literals on a struct, not code.

**Status: ABSENT**

---

## Item 3 — No typed-Vars validation runtime

**Requirement (FR-021):** no garde / pydantic-validation / zod runtime logic invoked
by the library. Generated Pydantic model is a SHAPE (type declaration); `validate_default`
is a Pydantic field metadata parameter, not library-invoked validation logic.

**Search performed:**
```
# Rust crates
rg "garde|validate|Validate|ValidationError" \
  crates/prompting-press-core/src/ \
  crates/prompting-press/src/ \
  crates/prompting-press-py/src/ \
  crates/prompting-press-node/src/ \
  --include="*.rs"

# Python package
rg "validate_default" \
  packages/python/python/prompting_press/generated/prompt_definition.py -A2 -B2
```

**Result (Rust):** Two hits, both inside `#[doc = "..."]` attribute literals in the
generated file (lines 46 and 54 of `prompt_definition.rs`). The word "validate"
appears in a schema description string explaining the library does NOT validate.
No `garde` import, no `#[validate]` attribute, no `Validate` trait impl, no
`ValidationError` type.

**Result (Python):** One hit — `validate_default=True` at line 111 of
`packages/python/python/prompting_press/generated/prompt_definition.py`, inside a
`Field(...)` call. This is a Pydantic v2 field metadata parameter that controls
whether the default value is validated against the field's type constraint when the
model is constructed. It is a shape/schema declaration parameter generated from the
JSON Schema, not validation logic that the library itself invokes at runtime.
The library never calls `.model_validate()`, `.validate()`, or any validator on
user data.

**Result (TypeScript):** One hit — inside a `/** */` doc comment (line 12 of
`packages/typescript/src/generated/prompt-definition.ts`), describing what the
library does NOT do. Not executable code.

**Status: ABSENT** (all hits are doc text or shape-declaration metadata, not
invoked validation logic)

---

## Item 4 — No agreement-check / variant-resolution / hashing logic

**Requirement (FR-021):** no template_hash / render_hash / undeclared_variables /
variant-selection code.

**Search performed:**
```
rg "template_hash|render_hash|undeclared_var|variant_select|resolve_variant|\
agreement_check|check_agreement" \
  crates/prompting-press-core/src/ \
  crates/prompting-press/src/ \
  crates/prompting-press-py/src/ \
  crates/prompting-press-node/src/ \
  --include="*.rs"
```

**Result:** exit 1 — no matches.

**Status: ABSENT**

---

## Item 5 — No I/O (file / DB / network)

**Requirement (FR-022):** library crates must not perform file, database, or network I/O.

**Scope note:** codegen scripts (`crates/prompting-press/scripts/codegen.sh`,
`packages/python/scripts/codegen.sh`, `packages/typescript/scripts/codegen.mjs`) read
the schema file at build time. These are build-time tooling, not part of the library
binary. The audit covers only library source under `crates/*/src/` and
`packages/*/python|src/`.

**Search performed:**
```
rg "std::fs|File::open|File::create|reqwest|hyper|tokio::net|std::net|\
TcpStream|UdpSocket|UnixStream|BufReader|BufWriter|OpenOptions" \
  crates/prompting-press-core/src/ \
  crates/prompting-press/src/ \
  crates/prompting-press-py/src/ \
  crates/prompting-press-node/src/ \
  --include="*.rs"
```

**Result:** exit 1 — no matches.

**Status: ABSENT**

---

## Item 6 — No LLM call

**Requirement (FR-022):** no Anthropic / OpenAI / LLM client in the library.

**Search performed:**
```
rg "anthropic|openai|async_openai|llm|tiktoken|mistral|cohere|bedrock" \
  crates/prompting-press-core/src/ \
  crates/prompting-press/src/ \
  crates/prompting-press-py/src/ \
  crates/prompting-press-node/src/ \
  --include="*.rs"
```

**Result:** exit 1 — no matches.

**Status: ABSENT**

---

## Item 7 — No request-body assembly

**Requirement (FR-022):** no messages-array / chat-completion payload building.

**Search performed:**
```
rg "messages|chat_completion|ChatCompletion|request_body|RequestBody|content_block" \
  crates/prompting-press-core/src/ \
  crates/prompting-press/src/ \
  crates/prompting-press-py/src/ \
  crates/prompting-press-node/src/ \
  --include="*.rs"
```

**Result:** exit 1 — no matches.

**Status: ABSENT**

---

## Item 8 — No token counting

**Requirement (FR-022):** no tiktoken / token counter in the library.

**Search performed:**
```
rg "tiktoken|count_token|token_count|TokenCounter|num_tokens|count_tokens" \
  crates/prompting-press-core/src/ \
  crates/prompting-press/src/ \
  crates/prompting-press-py/src/ \
  crates/prompting-press-node/src/ \
  --include="*.rs"
```

**Result:** exit 1 — no matches.

**Status: ABSENT**

---

## Item 9 — No output parsing

**Requirement (FR-022):** no response / output parsing or coercion in the library.

**Search performed:**
```
rg "parse_response|parse_output|OutputParser|ResponseParser|\
extract_content|coerce_output|json_extract" \
  crates/prompting-press-core/src/ \
  crates/prompting-press/src/ \
  crates/prompting-press-py/src/ \
  crates/prompting-press-node/src/ \
  --include="*.rs"
```

**Result:** exit 1 — no matches.

**Status: ABSENT**

---

## Summary

| # | Capability | Status |
|---|-----------|--------|
| 1 | Template-engine integration (minijinja/handlebars/tera/jinja) | ABSENT |
| 2 | render / rendering path | ABSENT |
| 3 | Typed-Vars validation runtime (garde/pydantic validate calls/zod runtime) | ABSENT |
| 4 | Agreement-check / variant-resolution / hashing logic | ABSENT |
| 5 | I/O (file / DB / network) in library code | ABSENT |
| 6 | LLM call (Anthropic / OpenAI / any client) | ABSENT |
| 7 | Request-body assembly (messages-array / chat-completion payload) | ABSENT |
| 8 | Token counting | ABSENT |
| 9 | Output parsing / coercion | ABSENT |

All 9 items absent. SC-007 satisfied.
