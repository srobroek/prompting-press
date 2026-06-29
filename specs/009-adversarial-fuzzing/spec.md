# Feature Specification: Adversarial hardening & fuzzing

**Feature Branch**: `009-adversarial-fuzzing`

**Created**: 2026-06-29

**Status**: Draft

**Input**: User description: "009 — Adversarial hardening & fuzzing. A library-hardening test pass proving the minimal boundary holds under abuse: robustness fuzzing, property-based fuzzing, an injection/guard demo, and secret-scrub verification across the kernel and all three bindings."

## Overview

A **test-only hardening pass**: it adds adversarial coverage to the library's own suites and changes **no
library behavior** (Principle I — the kernel is untouched; Principle III — the minimal boundary is not
expanded). It proves, under generated and hostile input, that the library upholds the guarantees it already
claims: it never panics, always returns a structured error, never leaks bound-value content, and keeps its
invariants (validate-before-render, hash-determinism). It runs against the **post-008 `Prompt` object surface**
(construct / `from_yaml` / `from_json` / `from_toml` / `render` / `check` / `with`), since 008 removed the
registry.

The "users" served are the **library's own maintainers and downstream consumers**: the value is justified
confidence that the boundary is abuse-resistant before v1 publish (spec 007 should not ship an un-hardened
library). The honest framing is load-bearing: this hardens **library robustness**, it does **not** claim to
"jailbreak" or defend an LLM — the library has no model.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The library never panics on hostile input (Priority: P1)

A maintainer runs the suite against malformed, huge, deeply-nested, Unicode, and control-character inputs fed to
every entry point (construct, the text factories, render, check). The library always returns a **structured
error or a valid result** — it never panics, aborts, or crashes the host process/runtime.

**Why this priority**: "never panics / always structured error" is the foundational robustness guarantee of the
boundary; every other adversarial property builds on the process staying alive and the error path being taken.

**Independent Test**: Feed a corpus of malformed/oversized/nested/Unicode/control-char documents and var-sets
to each entry point in each binding; assert every call either returns a value or raises/returns the binding's
normalized structured error — zero panics, zero uncaught exceptions, zero aborts.

**Acceptance Scenarios**:

1. **Given** a malformed prompt document (truncated, wrong types, unknown keys, non-string body), **When**
   construction is attempted in any binding, **Then** a structured error is returned/raised — never a panic.
2. **Given** a pathological input (a multi-megabyte body, a deeply-nested variables/variants/metadata
   structure, a body of pure control characters or astral-plane Unicode), **When** fed to construct/render/
   check, **Then** the call terminates with a structured error or a valid result — never a hang-to-crash or
   panic.
3. **Given** a var-set whose values are hostile (huge strings, deep nesting, NaN/inf where a number is
   expected), **When** rendered, **Then** validation or the kernel rejects it with a structured error before
   or during render — never a panic.

---

### User Story 2 - Library invariants hold under generated input (Priority: P1)

A maintainer runs property-based tests (generated input, many cases) asserting the library's core invariants:
**never-panic**, **validate-before-render** (a render is reached only after validation passes), and
**hash-determinism** (the same definition + variables always yield the same `template_hash` / `render_hash`).

**Why this priority**: property-based fuzzing is what turns "we tested some cases" into "the invariant holds
across a generated space"; it is the headline of an adversarial pass and the strongest evidence the boundary is
sound.

**Independent Test**: Run the generative suite in each binding for a bounded number of generated cases; assert
no case panics, no case renders without first validating, and re-rendering any generated case reproduces
byte-identical hashes.

**Acceptance Scenarios**:

1. **Given** generated valid var-sets for a prompt, **When** rendered twice, **Then** both renders produce
   byte-identical `template_hash` and `render_hash` (determinism).
2. **Given** generated invalid var-sets, **When** rendered, **Then** the validator rejects them and the kernel
   render is **never reached** (validate-before-render).
3. **Given** a generated space of inputs to any entry point, **When** the property suite runs, **Then** no
   generated case panics (never-panic holds across the generated space).

---

### User Story 3 - The injection/guard demo is honest about what the guard does (Priority: P2)

A maintainer (and a reader of the docs/tests) sees a worked demonstration: an injection-shaped value in an
`untrusted`/`external` field is **flagged** by `check()` when unguarded, and the opt-in guard text **names** that
field — while the demo explicitly asserts the value passes through the render **unchanged** (the guard is
advisory text, not a sanitizer; the library has no LLM to enforce anything).

**Why this priority**: it documents the security posture truthfully and guards against the library being
mis-sold as injection-proof. It is P2 because it is a demonstration/assertion of existing behavior, not a new
robustness guarantee.

**Independent Test**: Run the demo test: construct a prompt with an `untrusted` field carrying injection-shaped
text; assert (a) `check()` flags the unguarded field, (b) the rendered output contains the injection text
**verbatim**, (c) the opt-in guard, when enabled, produces guard text naming the field and the body is
byte-identical with or without the guard.

**Acceptance Scenarios**:

1. **Given** a prompt with an `untrusted` field and no configured guard, **When** `check()` runs, **Then** it
   flags the field as untrusted-without-guard.
2. **Given** an injection-shaped untrusted value, **When** rendered, **Then** the value appears **verbatim** in
   the output — the library does not strip, escape, or alter it (C-09).
3. **Given** the opt-in guard enabled, **When** rendered, **Then** the separate guard text names the untrusted
   field and the rendered body is byte-identical to the unguarded render (the guard is additive, advisory text).

---

### User Story 4 - Secrets never leak through error paths (Priority: P1)

A maintainer feeds values that *look like secrets* (API-key-shaped, token-shaped, PII-shaped) into inputs that
trigger parse/render errors, and asserts the secret-looking content **never** appears in the resulting error
message, error rows, or stack trace — in any binding (SEC-004 holds end to end, adversarially verified).

**Why this priority**: a leaked secret in an error/log is a real security incident; the scrub is an existing
guarantee (SEC-004) that this spec proves holds under adversarial input before publish.

**Independent Test**: For each binding, construct inputs where a secret-shaped value triggers a parse/render
error; assert the secret substring is absent from the error's message, its `[{field, code, message}]` rows, and
the stack/`__str__`/`.stack` representation.

**Acceptance Scenarios**:

1. **Given** a secret-shaped value that triggers a kernel parse/render error, **When** the error surfaces in any
   binding, **Then** the secret substring appears **nowhere** in the message, rows, or stack.
2. **Given** a validation failure on a secret-shaped value, **When** the structured error is produced, **Then**
   the rows carry the field name + code + a value-free message, never the rejected value.

---

### Edge Cases

- **Resource bounds**: an input large/deep enough to be a denial-of-service concern terminates with a structured
  error within a bounded time/memory, not an unbounded hang. (The suite caps generated sizes so the gate itself
  stays bounded.)
- **Unicode/normalization**: astral-plane code points, combining marks, bidi/control characters, and lone
  surrogates in bodies/values are handled without panic and render deterministically.
- **Empty/degenerate**: empty body, empty variables, zero-length strings, a body that is only whitespace.
- **The guard is never a sanitizer**: no fuzz assertion may expect the guard to alter a value (that would
  contradict C-09); the injection demo asserts pass-through, not filtering.
- **Determinism across runs**: the property suite uses a fixed/recorded seed (or the framework's replay) so a
  failure is reproducible, not flaky.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The test suites MUST add **robustness fuzzing** over construct / `from_yaml` / `from_json` /
  `from_toml` / `render` / `check`, feeding malformed, oversized, deeply-nested, Unicode, and control-character
  inputs, and asserting the library returns a structured error or valid result — **never panics**.
- **FR-002**: The suites MUST assert the library **never leaks bound-value content** on any fuzzed error path
  (the SEC-004 scrub), across the kernel and all three bindings.
- **FR-003**: The test suites MUST add **property-based / generative fuzzing** using the per-ecosystem
  frameworks (Rust, Python, TypeScript) asserting the invariants: **never-panic**, **validate-before-render**,
  and **hash-determinism**.
- **FR-004**: The property suites MUST be **deterministic/replayable** (fixed or recorded seed) so any failure
  reproduces, and MUST be **bounded** (case count + input size capped) so the CI gate runs in bounded time.
- **FR-005**: The suite MUST include an **injection/guard demonstration** asserting: `check()` flags an
  unguarded `untrusted`/`external` field; the untrusted value renders **verbatim** (no sanitization — C-09);
  and the opt-in guard names the field while leaving the body byte-identical.
- **FR-006**: The injection/guard demo + its docs/comments MUST **explicitly state** the guard is advisory text,
  **not** enforcement, and that the library has no LLM (no "jailbreak"/"injection-proof" claim).
- **FR-007**: The suite MUST add **secret-scrub verification**: secret-shaped values that trigger parse/render
  errors MUST be absent from the error message, the `[{field, code, message}]` rows, and the stack trace, in
  every binding.
- **FR-008**: All new adversarial coverage MUST be **wired into CI** as a gate (it runs automatically, not
  ad hoc), in bounded time.
- **FR-009**: The new dev-dependencies MUST be **pinned exactly** (no floating ranges): Rust `proptest 1.11.0`
  (+ `arbitrary 1.4.2` where useful), Python `hypothesis 6.155.7`, TypeScript `fast-check 4.8.0`. They MUST be
  **dev/test dependencies only** — they MUST NOT enter the published library's runtime dependency set, and MUST
  NOT introduce an FFI dependency into the kernel or consumer crate.
- **FR-010**: This spec MUST NOT change any library behavior: no kernel rendering/agreement/hashing change, no
  new public API, no boundary expansion (no I/O, no LLM, no request-body assembly). It is **tests only**.

### Key Entities

- **Fuzz input corpus**: generated or enumerated hostile inputs (malformed docs, oversized/nested structures,
  Unicode/control-char strings, secret-shaped values) fed to the public entry points.
- **Invariant**: a property asserted across the generated space — never-panic, validate-before-render,
  hash-determinism, no-leak.
- **Injection/guard demo**: a worked test showing the advisory guard names an untrusted field without altering
  the rendered value.
- **Secret-shaped value**: a value resembling an API key / token / PII, used to prove the SEC-004 scrub holds.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Across the kernel and all three bindings, the adversarial suites run with **zero panics / zero
  uncaught crashes** over the full generated + enumerated corpus.
- **SC-002**: **100%** of fuzzed error paths return the binding's normalized structured error shape
  (`[{field, code, message}]`), never a native error type leaking across FFI and never a raw value.
- **SC-003**: In **every** secret-scrub case, the secret-shaped substring appears in **zero** of: the error
  message, the error rows, the stack trace.
- **SC-004**: Re-rendering any generated case yields **byte-identical** `template_hash` and `render_hash`
  (hash-determinism holds across the generated space).
- **SC-005**: In **zero** generated cases does a render occur without validation having passed first
  (validate-before-render holds).
- **SC-006**: The injection/guard demo shows the untrusted value rendered **verbatim** (byte-for-byte present in
  the output) and the guarded body byte-identical to the unguarded body — proving the guard never sanitizes.
- **SC-007**: The adversarial gate is wired into CI and completes in **bounded time** (a fixed case-count
  budget), and the existing CI gates (FFI isolation, conformance, the binding suites) remain green.
- **SC-008**: The published library's runtime dependency set is **unchanged** — the fuzzing frameworks appear
  only as dev/test dependencies.

## Assumptions

- **Targets the post-008 surface**: the suites exercise the `Prompt` object API (construct / factories / render
  / check / with); 009 is branched on top of 008 (rebase if 008's PR lands first).
- **Bounded gate**: the CI fuzz gate uses a capped case count + input-size ceiling so it is fast and
  deterministic; deeper/longer fuzzing (e.g. a nightly or `cargo fuzz` continuous job) is a future option, not
  required here.
- **`cargo fuzz`/libFuzzer (coverage-guided) is out of scope** for v1 — `proptest` property tests + enumerated
  hostile corpora meet the bar; a coverage-guided harness can be added later if a real need appears (Scope
  Discipline).
- **No new library behavior** — purely additive test coverage over the existing, unchanged boundary
  (Principle I/III).
- **Versions verified** 2026-06-29 (proptest 1.11.0, arbitrary 1.4.2, hypothesis 6.155.7, fast-check 4.8.0).

## Dependencies

- **Depends on** specs 002–005 (the kernel + bindings under test) and **008** (the `Prompt` surface the suites
  drive). Independent of 008's *rename* semantically, but branched on it to target the current API.
- **Should land before** spec 007 (do not publish an un-hardened library).

## Out of Scope

- Any "jailbreak the model" / "injection-proof" claim — the library has no model (Principle III).
- Turning the advisory guard into runtime enforcement (a future capability spec, if ever).
- Any change to the boundary, the kernel, or the public API.
- Coverage-guided continuous fuzzing (`cargo fuzz`/libFuzzer) as a required deliverable.
