"""append validates eagerly: a passing model instance is stored, but an invalid
one raises PromptValidationError and NOTHING is stored (no partial state).
Standalone."""

from prompting_press import Composition, Prompt, PromptValidationError
from pydantic import BaseModel, Field


class SysVars(BaseModel):
    instructions: str


sys_prompt = Prompt({
    "name": "system-preamble",
    "role": "system",
    "body": "{{ instructions }}",
    "variables": {"instructions": {"type": "string", "trusted": True}},
})
user_prompt = Prompt({
    "name": "user-turn",
    "role": "user",
    "body": "{{ query }}",
    "variables": {"query": {"type": "string", "trusted": False}},
})

# Successful append — model instance passes Pydantic validation:
comp = Composition()
comp.append(sys_prompt, SysVars(instructions="Be concise."))

# Failed append — the vars fail validation; nothing is stored:
class StrictVars(BaseModel):
    query: str = Field(min_length=1)

# model_construct() bypasses Pydantic so the invalid model reaches append,
# where validation runs — this is what raises PromptValidationError.
invalid = StrictVars.model_construct(query="")
raised = False
try:
    comp.append(user_prompt, invalid)   # raises before storage
except PromptValidationError as exc:
    raised = True
    print(exc.errors[0].field)    # "query"
    print(exc.errors[0].code)     # "validation"
    # comp is unchanged — the failed entry was never stored
    assert exc.errors[0].field == "query"
    assert exc.errors[0].code == "validation"

assert raised, "the invalid append must raise PromptValidationError"
assert len(comp) == 1, "the rejected append must store nothing (no partial state)"
