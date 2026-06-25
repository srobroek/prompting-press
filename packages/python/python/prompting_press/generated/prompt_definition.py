# GENERATED FILE — DO NOT EDIT.
#
# This module is code-generated from the single source of truth:
#   schemas/jsonschema/prompt-definition.schema.json
# by datamodel-code-generator (pinned via mise / packages/python uv.lock).
#
# Regenerate with: packages/python/scripts/codegen.sh  (re-run on schema change).
# Hand edits are overwritten and will fail the US4 freshness gate. Edit the schema.

from enum import Enum
from typing import Annotated, Any

from pydantic import BaseModel, ConfigDict, Field


class Role(Enum):
    system = 'system'
    user = 'user'
    assistant = 'assistant'


class Type(Enum):
    string = 'string'
    integer = 'integer'
    number = 'number'
    boolean = 'boolean'
    array = 'array'
    object = 'object'


class TypeEnum(Enum):
    string = 'string'
    integer = 'integer'
    number = 'number'
    boolean = 'boolean'
    array = 'array'
    object = 'object'
    null = 'null'


class Provenance(Enum):
    trusted = 'trusted'
    untrusted = 'untrusted'
    external = 'external'


class VariableDecl(BaseModel):
    model_config = ConfigDict(
        extra='forbid',
    )
    type: Annotated[
        Type | list[TypeEnum],
        Field(description='JSON-Schema type keyword(s) for the variable.'),
    ]
    provenance: Annotated[
        Provenance,
        Field(
            description='Per-field provenance tag (FR-010a). DECLARATIVE METADATA ONLY — there is NO runtime enforcement of this tag in the current library version; it is not a security guard by itself. Untrusted-input guarding (the opt-in, additive guard expansion + lint) is introduced in a later spec per roadmap decision C-09 (deriving from constitution Principle IV). Do not assume the library protects `untrusted`/`external` fields until that version.'
        ),
    ]
    format: str | None = None
    pattern: str | None = None
    enum: list[Any] | None = None
    minimum: float | None = None
    maximum: float | None = None
    minLength: Annotated[int | None, Field(ge=0)] = None
    maxLength: Annotated[int | None, Field(ge=0)] = None
    description: str | None = None


class Variant(BaseModel):
    model_config = ConfigDict(
        extra='forbid',
    )
    body: Annotated[
        str,
        Field(
            description="The variant's template source — the only field that differs per variant."
        ),
    ]
    meta: Annotated[
        dict[str, Any] | None,
        Field(
            description='Library-OPAQUE selection metadata (weight, group, tags, ...). Stored + exposed; never interpreted by the library (caller selects). No schema-enforced selection semantics (FR-011c).'
        ),
    ] = None


class PromptDefinition(BaseModel):
    model_config = ConfigDict(
        extra='forbid',
    )
    name: Annotated[
        str,
        Field(
            description="Logical prompt name; the caller's reference key.", min_length=1
        ),
    ]
    role: Annotated[
        Role,
        Field(
            description='Conversational role; first-class metadata the caller reads. Shared across all variants.'
        ),
    ]
    body: Annotated[
        str,
        Field(
            description="The DEFAULT variant's template source. The root body IS the default arm (FR-011); surfaced under reserved name 'default' with is_default=true."
        ),
    ]
    variables: Annotated[dict[str, VariableDecl] | None, Field(description='Declared input variables, shared across all variants. Rich enough to generate-then-extend a typed Vars model in a later spec.', validate_default=True)] = {

    }
    variants: Annotated[
        dict[str, Variant] | None,
        Field(
            description='Named alternative arms. Absent => the prompt has only the default (root body) arm. Each arm differs ONLY in body (+ optional opaque meta).'
        ),
    ] = None
    output_model: Annotated[
        str | None,
        Field(
            description="Optional OPAQUE reference to the caller's output model (e.g. 'NodeOutput'). Stored and echoed; never resolved, loaded, or parsed (Principle III). Shared across variants."
        ),
    ] = None
    metadata: Annotated[
        dict[str, Any] | None,
        Field(
            description='Arbitrary prompt-level metadata; library-OPAQUE (may include uninterpreted model/param hints). Never interpreted by the library.'
        ),
    ] = None
    meta: Annotated[
        dict[str, Any] | None,
        Field(
            description="The default (root) arm's selection metadata; library-opaque (weight, group, tags, ...). Symmetric with Variant.meta."
        ),
    ] = None
