# Memory Synthesis — Spec 003 (Rust consumer `prompting-press`)

> Markdown-first (SQLite optimizer cache + `speckit_memory` MCP not configured). Draws on the
> governance layer (constitution v1.0.0 Principles I–VII + roadmap C-01…C-10), the as-built spec-002
> kernel, and `docs/research/feature-scope.md` §4.1/§4.6/§5/§6. Phase: **Specify → Plan**.

## Current Scope

Spec 003 — the **Rust consumer crate** (`prompting-press`): the first full consumer layer over the
spec-002 kernel and the public API `cargo add prompting-press` yields. It proves the kernel/consumer
split before any FFI binding (004/005) exists. Capabilities: a **garde 0.23** typed-Vars facade +
custom validators (one `validate()` at render); a **dual-input loader** (YAML/JSON or constructed
object → the one `PromptDefinition` representation); **`check(registry)`** = the agreement +
provenance lint as a CI entry point (the consumer owns the `referenced ⊆ declared` comparison; the
kernel only returns the per-variant required-roots set); ergonomic `render()`/`get_source()` +
composition (`Vec` + `append_*`, never `.chain()`); **error normalization** to `[{field, code,
message}]` (garde `Report` + `KernelError` wrapped, never leaked); a pluggable `count_tokens(text,
model) -> int` **hook** (no built-in counter). No rendering/agreement/variant/hashing logic
duplicated — all wrapped from the kernel.

## Relevant Decisions

- **C-01 Shared core / structural parity** (active, constitution): rendering/agreement/variant/hashing
  live ONCE in the kernel. 003 WRAPS them — it MUST NOT reimplement any of that logic.
- **C-02 FFI isolation** (active; CI-enforced by `check-ffi`): the consumer crate is FFI-free — no
  `pyo3`/`napi`. The spec-001 gate already covers `prompting-press`; 003 adds garde + serde_yaml etc.
  and must keep it green.
- **C-03 Minimal boundary** (active): token counting is a pluggable HOOK only; NO built-in counter
  ships. No I/O, no LLM calls, no request-body assembly, no output parsing. The output-model reference
  is metadata only, never parsed.
- **C-06 Per-language idiom** (active — THE 003 driver): typed Vars via **garde 0.23**
  (`#[garde(custom(...))]`), validators run in ONE `validate()` at render. Composition = explicit
  ordered `Vec` + `append_*`, NEVER `.chain()` (collides with `Iterator::chain`, can't cross FFI).
  Errors normalized to `[{field, code, message}]`; garde `Report` / kernel `KernelError` never leak.
- **C-07 JSON Schema SSoT** (active): prompt data pushed as YAML or JSON or a constructed shape
  object; ONE internal dual-input loader normalizes both into the generated `PromptDefinition` shape
  (which the kernel owns and the consumer re-exports — do NOT redefine it).
- **C-04 / C-09** (active — surfaced via `check(registry)`): the agreement check (`referenced ⊆
  declared`) and provenance lint (untrusted/external fields in declared guard positions) are pure,
  pass/fail, never mutate. The kernel exposes `required_roots`/`provenance_view`; 003 does the
  comparison.

## Active Architecture Constraints

- **Dependency direction** (C-01/C-02): kernel ← consumer ← bindings. The consumer depends on the
  kernel; nothing in 003 may invert that. Validation lives HERE, never in the kernel (kernel stays
  validation-blind).
- **No logic duplication** (C-01): `render`/`get_source`/agreement/variant/hash are kernel calls.
  003's `render()`/`get_source()`/`check()` are thin idiomatic wrappers + the validation/loader/
  normalization layer the kernel deliberately omits.
- **Error normalization is the consumer's boundary job** (C-06): the kernel returns a CLOSED
  `KernelError` enum (5 variants — its closedness gives 003's normalization match exhaustiveness).
  003 maps both `KernelError` and garde `Report` to `[{field, code, message}]`.

## Relevant Security Constraints

- The provenance tags are declarative metadata; the provenance LINT (part of `check(registry)`) flags
  untrusted/external fields used outside declared guard positions — pure analysis, never mutates, no
  sanitization (C-09). The guard expansion itself is the kernel's (002) opt-in additive feature; 003
  surfaces it, doesn't re-implement it.
- SEC-004 carryover (002): the kernel's `KernelError` `Parse`/`Render` detail strings may carry
  bound-value content. 003's error-normalization layer MUST scrub/avoid logging raw detail before it
  reaches logs — the spot the 002 security review flagged for the consumer.

## Related Historical Lessons (from 002 + worklog)

- **garde version/API: verify-at-spec-time** — roadmap says garde 0.23; re-confirm the current
  version + the `#[garde(custom(...))]` + `Validate`/`Report` API at PLAN time (same discipline that
  corrected the MiniJinja version in 002).
- **Subagent tool-channel glitch** (`tool_uses: 0`): when a fresh-context audit subagent reports 0
  tool uses / empty reads, re-run on the main thread against objective evidence. Check tool_uses
  before trusting a subagent verdict.
- **`constitution C-NN` is wrong** — C-0N are roadmap decisions; cite "roadmap decision C-NN". A
  `grep -rn "constitution C-0"` catches it (regressed in 001 + 002).
- **`KernelError` is a closed enum** (002) — keep 003's normalization match exhaustive so a future
  kernel variant is a compile error, not a silent wildcard.
- moon cache hits can mask an unrun gate (`--force` to prove); `rm` blocked (use `git mv`/`rmdir`);
  single-quote `git commit -m` with backticks; `dgit push`; `Closes #N` one per line.

## Conflict Warnings

- **No hard conflicts.** 003 is the spec that makes C-06/C-07 executable in Rust, fully consistent
  with the constitution and the 002 kernel API. Soft watch items: (a) confirm garde 0.23's actual
  current version/API at plan time; (b) the dual-input loader needs a YAML parser (e.g. serde_yaml or
  a maintained successor) — verify it's maintained + pure-Rust (FFI gate); (c) keep the consumer
  FFI-free; (d) `check(registry)` and the validators are PURE — no mutation (C-04/C-09).

## Retrieval Notes

- Sources: governance layer (constitution + roadmap C-01..C-10, 003 entry), as-built 002 kernel
  (`crates/prompting-press-core/src/`), feature-scope §4.1/§4.6/§5/§6, auto-memory
  spec-002-engine-kernel + speckit-workflow-gotchas. Durable decisions/bugs/architecture dirs empty
  (fresh). MCP unavailable → markdown-first. Within budget.
