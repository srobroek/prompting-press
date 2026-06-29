# Feature Specification: Opt-in unsafe render-error detail

**Feature Branch**: `013-unsafe-render-detail`

**Created**: 2026-06-29

**Status**: Draft

**Input**: User description: "013 — Opt-in unsafe Render-error detail. Add an explicit, off-by-default mode in which the caller opts in to receive the full render-error detail (currently scrubbed by SEC-004), accepting responsibility for any bound-value/PII content it may carry."

## Clarifications

### Session 2026-06-29

- Q: Granularity — where is the opt-in set? → A: **Per-render-call** — a flag on the render options object (alongside `variant`/`guard`), matching the existing options-object call shape (C-11) and how the guard is already passed per render. Deciding to expose detail is contextual to a specific debugging render, not a property of the prompt. Explicitly NOT a per-Prompt/construction setting and NOT global.
- Q: Scope of detail surfaced? → A: **Render detail only.** The opt-in surfaces ONLY the scrubbed `Render`-error detail (the one PII-sensitive scrub). `ExcludedFeature` and `Parse` are unaffected (Parse already preserved per D2). Minimal blast radius; the opt-in maps 1:1 to the single risky scrub.
- Q: Governance — this touches the SEC-004 *Render*-scrub half. Amendment or recorded decision? → A: **Recorded decision (D3) + an explicit SEC-004 carve-out note**, like D2 — NOT a full constitution amendment. Rationale: the default (scrub-by-default) is unchanged; this adds a sanctioned, OFF-BY-DEFAULT, caller-opt-in escape hatch, not a redefinition of the principle. The boolean flag is NOT a "new pluggable interface" under Scope Discipline (no seam, no second implementation), so it does not trip the boundary-defense amendment trigger.

## Context

By default the library **scrubs** render-error detail: when the rendering engine rejects a render, the
normalized error carries a fixed, templated message and the raw engine detail is discarded, because that
detail can contain bound input values (untrusted data, PII, secrets). This is the SEC-004 guarantee. A
recent refinement (decision D2) preserved **parse**-error detail (it is pre-binding template syntax, with
no bound values) while keeping **render**-error detail scrubbed.

That default is correct for most callers, but it removes information an operator may genuinely need to
debug a render failure. This feature adds a deliberate, **off-by-default** escape hatch: a caller that
controls its own log destination and accepts the risk can opt in to receive the **full** render-error
detail. It is the inverse tradeoff from the default — debuggability over automatic scrubbing — and it must
always be an explicit choice, never silently on.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - An operator opts in to full render-error detail for debugging (Priority: P1)

A developer debugging a render failure in a controlled environment explicitly enables the unsafe-detail
option and receives the full underlying render-error detail in the returned error, accepting that the
detail may contain bound input values. Without the opt-in, the same failure yields the scrubbed message.

**Why this priority**: This is the entire feature — the opt-in path that surfaces detail. It is the MVP:
the option plus the surfaced-vs-scrubbed behavior delivers the value on its own.

**Independent Test**: Trigger a render error twice — once with the opt-in enabled (full detail present in
the error) and once without (scrubbed message). Confirm the two differ exactly in render-detail exposure.

**Acceptance Scenarios**:

1. **Given** the unsafe-detail opt-in is enabled, **When** a render fails, **Then** the returned error's
   message contains the full underlying render detail.
2. **Given** the opt-in is NOT enabled (the default), **When** a render fails, **Then** the returned error
   carries the fixed scrubbed message and no bound-value content.
3. **Given** the opt-in is enabled, **When** a render *succeeds*, **Then** behavior is unchanged (the
   option only affects error detail; it never alters rendered output, hashes, or success paths).

### User Story 2 - The default stays safe and the opt-in is never implicit (Priority: P1)

A caller who does nothing — or who is unaware of the option — always gets the scrubbed default. The opt-in
cannot be turned on by accident, by a global side effect, or by any default value.

**Why this priority**: The safety guarantee is co-equal with the feature itself; a feature that could
silently disable scrubbing would be a regression of SEC-004. Tied P1.

**Independent Test**: With no opt-in anywhere in the call, confirm every render error is scrubbed (the
existing scrub corpus stays green); confirm no construction path, environment variable, or global toggle
can enable it implicitly.

**Acceptance Scenarios**:

1. **Given** a caller that never sets the option, **When** any render fails, **Then** the detail is
   scrubbed (the existing scrub test corpus passes unchanged).
2. **Given** the codebase, **When** searched, **Then** there is no global/ambient way to enable the opt-in
   (it is set only at the explicit, documented call/construction site).

### User Story 3 - Behavior and opt-in shape are consistent across all three languages (Priority: P2)

A developer using Rust, Python, or TypeScript enables the same option in the same place with the same
semantics, and the surfaced detail arrives through each binding's normalized error shape
(`{field, code, message}`), not as a leaked native error type.

**Why this priority**: Cross-language consistency is a project invariant (Principle VI). Subordinate to the
behavior + safety slices but required for a coherent release.

**Independent Test**: Enable the opt-in in each binding, trigger a render error, and confirm the full
detail arrives through that binding's normalized error contract identically.

**Acceptance Scenarios**:

1. **Given** the opt-in enabled in any binding, **When** a render fails, **Then** the detail is delivered
   via the normalized `{field, code, message}` shape (no native error type leaks across the boundary).
2. **Given** the three bindings, **When** the opt-in is set, **Then** it is set the same way (same option
   name/shape, adapted to each language's idiom) and produces equivalent detail exposure.

### Edge Cases

- The opt-in is enabled but the render **succeeds** → no error is produced; the option is inert (it only
  affects the error path; success output/hashes are identical with or without it).
- A **parse** error occurs while the opt-in is enabled → unchanged from the default (parse detail is
  already preserved per D2; the opt-in concerns the render-scrub half only).
- The opt-in is enabled and the render detail contains an obvious secret-shaped value → the detail IS
  surfaced (that is the opted-into behavior); responsibility for handling it is the caller's, and the
  API/doc must have made that explicit at the opt-in site.
- The opt-in interacts with composition (multi-message render) → the same per-call/per-construction opt-in
  rule applies to composed renders; it does not become a global once set on one entry.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The library MUST provide an explicit, **off-by-default** option that, when enabled, causes a
  render-error's full underlying detail to appear in the returned/raised error's message instead of the
  fixed scrubbed string.
- **FR-002**: With the option absent or disabled (the default), render-error detail MUST remain scrubbed
  exactly as today — the existing scrub guarantee and its test corpus are unchanged.
- **FR-003**: The option MUST NOT be enableable implicitly — no default-on value, no environment variable,
  no global/ambient toggle. It is set only at an explicit, documented site.
- **FR-004**: The option MUST affect ONLY render-error detail. It MUST NOT change rendered output, content
  hashes, success-path behavior, validation behavior, or parse-error handling (parse detail already
  preserved per D2).
- **FR-005**: When enabled, the surfaced detail MUST be delivered through each binding's normalized
  `{field, code, message}` error contract; native error types MUST NOT leak across the FFI boundary
  (Principle VI).
- **FR-006**: The opt-in MUST be available and behave identically across the Rust, Python, and TypeScript
  bindings, set in the same place with the same semantics (adapted to each language's idiom).
- **FR-007**: This feature MUST NOT change kernel behavior (the kernel already produces the detail; this is
  purely the consumer/binding choice of whether to surface it — Principle I) and MUST add no I/O or logging
  (the library does not log; it only chooses whether detail reaches the returned error value — Principle III).
- **FR-008**: The documentation (the error-reference pages + relevant guides) MUST explain the tradeoff and
  warn, at the opt-in site, that enabling it may surface bound-value content (untrusted input / PII /
  secrets) into the caller's logs or stack traces.
- **FR-009**: The opt-in MUST be set **per-render-call** — a flag on the render options object (alongside
  `variant`/`guard`), matching the options-object call shape (C-11) and how the guard is already passed per
  render. It MUST NOT be a per-Prompt/construction-bound setting and MUST NOT be a global/ambient setting.
- **FR-010**: The opt-in MUST surface **render-error detail only** — exactly the `Render`-error detail that
  is otherwise scrubbed. It MUST NOT change `Parse` handling (already preserved per D2) and MUST NOT alter
  `ExcludedFeature` detail; the opt-in maps 1:1 to the single PII-sensitive scrub.
- **FR-011**: The governance treatment is a **recorded decision (D3)** plus an explicit SEC-004 carve-out
  note (the same path as D2), NOT a full constitution amendment: the default scrub-by-default guarantee is
  unchanged, so this adds a sanctioned, off-by-default, caller-opt-in escape hatch rather than redefining
  the principle. The boolean opt-in is NOT a "new pluggable interface" under Scope Discipline (no seam, no
  second implementation), so it does not trip the boundary-defense amendment trigger.
- **FR-012**: The opt-in's name/signal MUST make the risk explicit at the call site (e.g. an
  `unsafe`/`unredacted`/`include_error_detail`-style name plus a doc-comment warning), so enabling it reads
  as a deliberate, risk-acknowledging choice rather than an innocuous flag.

### Key Entities

- **Unsafe-detail opt-in**: the explicit, off-by-default flag on the **render options** (per-call, FR-009)
  that selects render-error-detail surfacing over scrubbing. Carries no data beyond "on/off"; not a
  pluggable interface.
- **Normalized error** (`{field, code, message}`): the existing cross-binding error contract; the opt-in
  changes only what the `message` carries for a render error (full detail vs. scrubbed), never the shape.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: With the opt-in enabled, 100% of render errors surface the full underlying detail in the
  returned error; with it disabled, 0% do (the scrubbed message only).
- **SC-002**: With no opt-in set anywhere, the existing render-scrub test corpus passes unchanged (zero
  regressions to the default safety guarantee).
- **SC-003**: There is no code path — global, environmental, or default-valued — that enables the opt-in
  without an explicit call/construction-site choice (verifiable by inspection + a test).
- **SC-004**: The opt-in behaves identically across all three bindings and delivers detail only through the
  normalized `{field, code, message}` shape (no native error leakage).
- **SC-005**: Enabling the opt-in changes nothing on the success path — rendered text and content hashes
  are byte-identical with the option on or off.
- **SC-006**: The opt-in site carries an explicit risk warning in its documentation/signature, so a reader
  cannot enable it without encountering the bound-value-exposure caveat.

## Assumptions

- The kernel already produces the full render detail today (the scrub happens in the consumer/binding error
  normalization); this feature only changes whether that detail is surfaced, so no kernel change is needed.
- Parse-error detail is already preserved (decision D2) and is out of scope here; this feature concerns the
  render-scrub half only.
- The default remains scrubbed; the off-by-default guarantee is the load-bearing safety property and is
  non-negotiable regardless of how granularity/governance resolve.
- The library still performs no logging or I/O; "surfacing detail" means placing it on the returned error
  value — what the caller does with it (log, redact, drop) is the caller's responsibility.
- The opt-in is a simple on/off option, not a configurable redaction policy or a pluggable sink (those
  would be separate, larger features and are out of scope).
- Builds on branch state where decision D2 (parse preserved / render scrubbed) is already implemented.
