---
document_type: security-review
review_type: plan
assessment_date: 2026-06-27
codebase_analyzed: prompting-press / spec-004-python-binding
findings_total: 6
findings_critical: 0
findings_high: 0
findings_moderate: 1
findings_low: 2
findings_informational: 3
overall_risk: low
owasp_categories:
  - "A09:2021 Security Logging and Monitoring Failures — Rust→Python exception translation must not echo scrubbed-past bound-value content (SEC-004 carryover at a NEW boundary)"
  - "A04:2021 Insecure Design — provenance lint honest-boundary (not a runtime sanitizer); FFI marshaling fidelity"
  - "A06:2021 Vulnerable and Outdated Components — new FFI/Python deps (pyo3 0.29, pythonize 0.29, pydantic, maturin, datamodel-code-generator); advisory gate is Rust-only"
  - "A03:2021 Injection / A06 availability — depythonize of deeply nested / large untrusted Python objects (resource consumption); out of scope for v1 trust model but flagged"
cwe_ids:
  - "CWE-209: Generation of Error Message Containing Sensitive Information — SEC-004 carryover; the binding MUST surface the consumer's ALREADY-scrubbed rows and never reach past to raw KernelError detail"
  - "CWE-655: provenance lint mistaken for a runtime control / sanitizer — SEC-002 carryover into the Python lint surface"
  - "CWE-400 / CWE-674: Uncontrolled Resource Consumption / Uncontrolled Recursion — depythonize of deeply nested input; out of scope for v1 (repo-canonical / app-authored Vars) but flagged"
  - "CWE-1104: Use of Unmaintained Third-Party Components — Python deps fall OUTSIDE the Rust-only cargo-deny advisory gate; no equivalent Python CVE gate is specified"
field_summaries:
  scope: "The FIRST FFI binding (prompting-press-py): a PyO3 + maturin wheel that MARSHALS to the merged spec-002 kernel via the spec-003 Rust consumer. Adds a Pydantic v2 typed-Vars facade (validate-at-render — Q1), a dual-input loader REUSED from the consumer via FFI (Q3 — text marshaled in, parsed by the consumer's serde path), the check() lint, from_messages composition, and a PromptingPressError exception hierarchy normalizing native errors to [{field,code,message}] (Q2). Per FR-023 it does NO I/O, makes NO model calls, assembles NO request body, parses NO output, and ships NO token counter. Per FR-011/C-02 it contains ZERO render/agreement/variant/hash logic — those live once in Rust; render byte-parity is structural (Principle I), not re-tested here. authn/authz, sessions, SQLi, SSRF, transport, secrets-at-rest are absent by constitutional construction (Principle III / C-03), not unimplemented."
  authentication: "N/A — the binding performs no authentication. It exposes pure library functions (Registry load/insert, render, get_source, check, Composition.resolve) over pushed-in data. No credentials, tokens, or identity handling exist in scope."
  authorization: "N/A — no access-control decisions. Variant selection is caller-owned; render() resolves a name against the registry and delegates to the kernel. A name absent → UnknownPromptError (FR-008a). No privilege logic."
  data_protection: "No data at rest, no transport, no secrets storage. The Registry wraps the consumer's in-memory BTreeMap; validated Vars values live in-memory for one synchronous validate+marshal+render and are never persisted or sent to a sink (provenance is data on RenderResult). The two boundary-crossing paths for caller value content are (a) the normalized exception message — FR-015 mandates the SEC-004 scrub be preserved across the Rust→Python translation — and (b) the depythonize marshal step (FR-003a), which transports values into the kernel but emits nothing externally."
  input_validation: "The binding OWNS validation at the render boundary (Q1 / FR-002): model_validate runs ONCE before any templating; on failure no render is performed, the kernel never sees the values, and the Pydantic ValidationError is normalized to PromptValidationError (FR-004/014). Only validated values cross FFI (FR-003). Two input boundaries: (1) typed Vars → model_validate → depythonize → kernel value (FR-003a, lossless, no silent coercion); (2) the dual-input loader → text marshaled to the CONSUMER's serde path (Q3), so accept/reject + YAML↔JSON parity stay structural. Malformed input → LoadError, nothing partially loaded (FR-007). The new surface vs spec 003 is depythonize of arbitrary nested Python objects (SEC-005-PY) — out of scope under the app-authored-Vars trust model."
  injection: "Template/SSTI is the kernel's resolved risk class (values bound as data, never re-parsed; macros/includes excluded at parse). The binding adds NO new injection sink: render delegates to the consumer/kernel, the loader marshals text to the consumer's serde path (no Python-side eval, no second parser), check() is pure analysis, and depythonize produces a serde/minijinja Value (data), not code. The provenance lint is a LINT, not a runtime sanitizer (SEC-002 carryover)."
  cryptography: "N/A in this binding. template_hash/render_hash are computed once in the kernel (SHA-256) and surfaced 1:1 on RenderResult as opaque trace identifiers (FR-011 — no hashing logic here). data-model specifies them as lowercase SHA-256 hex; quickstart asserts len 64."
  error_handling: "The Rust→Python exception translation (error.rs, FR-014/015) is the new security-relevant surface and the home of the inherited SEC-004 concern at a NEW boundary. ConsumerError (already scrubbed by spec 003) and Pydantic ValidationError are mapped to the PromptingPressError hierarchy carrying [{field,code,message}] rows; native types never reach the public API (C-06/SC-006). T006 specifies the binding surface the consumer's ALREADY-scrubbed FieldError rows verbatim and never reach past the consumer to raw KernelError detail. The binding must also introduce no logging primitive that emits row content."
  secrets_management: "N/A — the binding handles no secrets, credentials, or connection strings and does no environment access (FR-023). Caller Vars values could contain secrets/PII, but the binding never persists or transmits them; the only boundary-crossing text path is the normalized exception message, which inherits the consumer's FR-015/SEC-004 scrub (T008 pins it with a seeded-secret test)."
---

# Security Review — Spec 004: Python binding (`prompting-press-py` → `packages/python`)

## Executive summary

**Overall risk: LOW.** Spec 004 is the first FFI binding — a PyO3 + maturin wheel that marshals to the
already-reviewed spec-002 kernel through the already-reviewed spec-003 Rust consumer. It adds a
Pydantic v2 typed-Vars facade (validate-at-render), a dual-input loader reused from the consumer via
FFI, the `check()` lint, `from_messages` composition, and a `PromptingPressError` exception hierarchy.
The classic web-app threat surface (authn/authz, sessions, SQLi, SSRF, transport, secrets-at-rest) is
**absent by constitutional construction (Principle III / C-03), not merely unaddressed** — FR-023
states "MUST NOT perform I/O (no file/network/database/environment access), make model calls, assemble
provider request bodies, parse model output, or count tokens." I did not manufacture findings for
those absent surfaces.

The design is sound and re-uses the spec-003 mitigations rather than re-implementing the risk surface.
Three load-bearing facts were verified against live repo state, not just spec text:

1. **The spec-003 scrub the binding relies on is real and tested.** `crates/prompting-press/src/error.rs`
   maps `KernelError::Parse`/`Render`/`ExcludedFeature` to a fixed message and **discards the raw
   `detail`** ("SEC-004: `detail` may embed bound-value content — DO NOT copy it. Fixed message."), with
   three passing scrub tests (`render_detail_secret_is_scrubbed`, `parse_detail_secret_is_scrubbed`,
   `excluded_feature_maps_to_stable_code_without_leaking_detail`). The 004 design correctly surfaces
   the consumer's **already-scrubbed** `FieldError` rows (T006: "surface the consumer's ALREADY-scrubbed
   `FieldError` rows verbatim — never reach past the consumer to raw `KernelError` detail").

2. **The FFI gate covers the kernel + consumer.** `scripts/ci/check-ffi-isolation.sh` `COVERED_CRATES`
   lists `prompting-press-core` and `prompting-press`, so `pyo3`/`pythonize` creeping into either is a
   CI failure (SC-007). The binding crate itself is *intended* to carry `pyo3`.

3. **The `abi3-py39` install-then-ImportError trap is real in the committed crate.**
   `crates/prompting-press-py/Cargo.toml` currently declares `features = ["extension-module",
   "abi3-py39"]`; the spec correctly bumps it to `abi3-py310` (T001). This is a correctness/availability
   fix, not a security vuln, but it is the one place the committed state diverges from the resolved
   design.

The headline carryover is **SEC-004 at a NEW boundary**: the value content that spec 003 scrubs at the
Rust normalizer now has to survive a second translation (ConsumerError → `PromptingPressError`).
The plan handles this correctly — FR-015 restates the scrub, and **T006/T008 pin it with a Rust-side
seeded-secret test** (`ConsumerError::Kernel(Render{detail with a seeded secret})` → `PromptRenderError`
whose `str()` and rows do NOT contain the secret). The one genuinely new gap worth recording is
supply-chain: the `ci:check-advisories` gate is **Rust-only** (`cargo deny check advisories` against
`Cargo.lock`) and does **not** scan the new Python deps (pydantic, maturin, datamodel-code-generator) —
no equivalent Python CVE gate is specified anywhere in the four artifacts.

Finding counts: **0 critical, 0 high, 1 moderate, 2 low, 3 informational.** None blocks implementation.
The two load-bearing items are SEC-004-PY (MODERATE — already pinned by a test; the remaining work is to
also assert the scrub survives at the *Python* `str()` surface, not only the Rust translation) and
SEC-101 (LOW — no Python-dependency advisory gate).

## Plan artifacts reviewed

- `specs/004-python-binding/spec.md` — FR-001..FR-024, SC-001..SC-010, US1–US4, edge cases (incl. the
  explicit "Secret in a bound value" and "Marshaling edge values" cases), Clarifications Q1–Q4,
  assumptions, dependencies, governance alignment.
- `specs/004-python-binding/plan.md` — Constitution Check (all PASS), structure, primary dependencies
  + versions, verified-this-cycle dependency notes (PyO3/pythonize/maturin/Pydantic/dmcg/CPython).
- `specs/004-python-binding/tasks.md` — T001..T026, guardrails, the SEC-004 task wording (T006/T008),
  the final-gate task (T025) and its FFI / floating-version / advisory / no-token checks.
- `specs/004-python-binding/research.md` — D1–D7 (PyO3 0.29 + abi3-py310, the pythonize marshal bridge,
  loader-via-FFI, exception hierarchy, maturin packaging, codegen freshness, composition).
- `specs/004-python-binding/data-model.md` — Python-facing surface + Rust-side `#[pyclass]` modules,
  validation & invariants (validate-then-render, only-validated-values-cross-FFI, native-types-never-leak,
  scrub preserved, check pure, codegen'd shape).
- `specs/004-python-binding/contracts/python-api.md` — the normative public Python API + boundary
  guarantees (pyo3 only here; no I/O/model/request-body/output-parse/token-count; output_model metadata).
- `specs/004-python-binding/quickstart.md` — US1–US4 + boundary/isolation scenarios (incl. the SEC-004
  seeded-secret assertion and the `rg -ri "count_tokens|token"` no-token-surface check).
- `specs/003-rust-consumer/security-review-report.md` — the PRIOR binding's review; SEC-004 / SEC-002 /
  SEC-005 / SEC-001 are the carryovers this spec must honor.
- `.specify/memory/constitution.md` — Principles I (shared core), II (FFI isolation), III (minimal
  boundary), IV (agreement check), VI (per-language idiom), VII (JSON Schema single source).
- **Corroborating live state** (read-only confirmation, not a 004 artifact):
  `crates/prompting-press/src/error.rs` (scrub + 3 passing scrub tests),
  `crates/prompting-press-py/Cargo.toml` (committed `abi3-py39`),
  `scripts/ci/check-ffi-isolation.sh` (COVERED_CRATES = core + consumer),
  `scripts/ci/check-advisories.sh` (Rust-only `cargo deny check advisories`),
  `scripts/ci/check-floating-versions.sh` (scans `pyproject.toml` + `Cargo.toml`, excludes lockfiles).

## Vulnerability findings

### SEC-004-PY — The SEC-004 scrub must survive the Rust→Python exception translation; assert it at the Python `str()` surface, not only the Rust translation (MODERATE) — inherited carryover at a new boundary

- **OWASP 2025:** A09:2021 Security Logging and Monitoring Failures (info leakage via error detail) /
  A04 Insecure Design.
- **CWE:** CWE-209 (Generation of Error Message Containing Sensitive Information).
- **Severity:** MODERATE. (Up one notch from the spec-003 LOW: in 003 the scrub lands at the single Rust
  normalizer; in 004 the *same* content must additionally survive a **second** translation hop
  (`ConsumerError` → `PromptingPressError`) and a Python `str(exc)` / logging surface that did not exist
  in 003. The blast radius is larger — a Python app that does `logging.exception(e)` or prints `str(e)`
  is the realistic exfil path — so the obligation is more than "implementation discipline.")
- **Location:** `crates/prompting-press-py/src/error.rs` (the `From<ConsumerError>` → exception
  translation), and the Python-facing `PromptingPressError.__str__` / `.errors` surface; contract
  §Exceptions; data-model §Exception hierarchy.
- **Evidence:** The spec states the requirement in normative language. FR-015: *"Error normalization MUST
  NOT echo raw, potentially sensitive bound-value content into exception messages or logs (the SEC-004
  scrub: `parse`/`render`/`excluded_feature` detail is replaced by a fixed message)."* The edge-case
  list reinforces it: *"a value triggering a kernel parse/render error must never appear in the raised
  exception's message or any log derived from it (SEC-004 scrub preserved)."* The mechanism is correctly
  designed to **reuse** the consumer's scrub rather than re-do it — T006: *"**SEC-004**: surface the
  consumer's ALREADY-scrubbed `FieldError` rows verbatim — never reach past the consumer to raw
  `KernelError` detail."* research D4 confirms the same: *"The binding surfaces the consumer's already
  scrubbed `FieldError` rows verbatim — it MUST NOT reach past the consumer to the raw `KernelError`
  detail."* I verified the consumer's scrub is real and tested (`error.rs` line 191:
  `"SEC-004: detail may embed bound-value content — DO NOT copy it. Fixed message."`, plus three passing
  scrub tests), so the source rows the binding consumes are already clean. **A task pins it:** T008 —
  *"a `ConsumerError::Kernel(Render{detail with a seeded secret})` → `PromptRenderError` whose `str()`
  and rows do NOT contain the secret (SEC-004)"* — and quickstart restates the Python-side assertion
  (*"a seeded secret in a render-error value never appears in the raised exception's `str()`"*).
  - **Residual:** the design is correct and a test is specified; the finding is to (a) ensure the test
    actually asserts against the **Python** `str(exc)` AND `exc.errors` (not only the Rust-side `str()`
    of the rows), because the Python exception's `__str__`/`__repr__` is a *binding-authored* surface
    (T006 builds the `#[pyclass(extends=PyException)]` base) that could re-introduce content the
    consumer scrubbed if `__str__` is implemented to interpolate, say, the requested name or the raw
    `detail` of any future non-scrubbed variant; and (b) confirm the Pydantic `ValidationError` →
    `PromptValidationError` mapper (T006: `.errors()` rows → `{field: loc-joined, code:"validation",
    message: msg}`) does not place the **offending input value** into `message`. Pydantic's
    `error["msg"]` is generally value-free, but `ctx`/`input` can echo the rejected value; the mapper
    must use `msg` (the design says so) and must not fall back to `str(error)` or include `error["input"]`.
- **Remediation:**
  1. Keep the design as specified — translate from the consumer's already-scrubbed `FieldError` rows;
     never construct a `PromptRenderError` from a raw `KernelError` `detail`.
  2. Make the binding-authored `PromptingPressError.__str__`/`__repr__` derive **only** from the
     scrubbed rows (`code` + fixed `message` + `field`), never interpolating a raw value or `detail`.
  3. Extend T008/T009 so the seeded-secret assertion runs against the **Python** exception object's
     `str(e)`, `repr(e)`, and `e.errors` — the externally-observable surfaces — closing the loop from
     "the Rust rows are clean" to "nothing a Python app can print/log carries the secret."
  4. For the Pydantic mapper, assert (a test) that a `ValidationError` whose rejected input is a
     sentinel secret yields a `PromptValidationError` whose `message` is Pydantic's `msg` only and does
     **not** contain the sentinel via `input`/`ctx`.

### SEC-101 — New Python dependencies fall outside the Rust-only advisory gate; no Python CVE scan is specified (LOW)

> **RESOLVED (user decision 2026-06-27): in scope for spec 004.** Added FR-025 + SC-011 + task **T028** —
> a `ci:check-advisories-py` gate (`pip-audit` over `packages/python/uv.lock`) mirroring the Rust
> `ci:check-advisories`. The coverage gap below is now closed by a dedicated CI gate rather than deferred.

- **OWASP 2025:** A06:2021 Vulnerable and Outdated Components.
- **CWE:** CWE-1104 (Use of Unmaintained Third-Party Components) — forward-looking; no vulnerable
  component identified at review time.
- **Severity:** LOW.
- **Location:** `packages/python/pyproject.toml` (pydantic, maturin, datamodel-code-generator);
  `crates/prompting-press-py/Cargo.toml` (pyo3 0.29, pythonize 0.29); `deny.toml`; the CI gate set.
- **Evidence:** 004 adds two new Rust deps and three Python deps. plan.md "Primary Dependencies":
  `pyo3 = "0.29"`, `pythonize = "0.29"`, `maturin >=1.14,<2.0`, `Pydantic v2` (latest 2.13.4),
  `datamodel-code-generator 0.65.1` (uv-locked). The two Rust deps are correctly placed: tasks.md T001
  adds `pythonize = "0.29"` to the binding crate (the one crate allowed an FFI toolkit), and T025
  re-runs `ci:check-ffi` to keep `pyo3`/`pythonize` out of the kernel/consumer (verified COVERED_CRATES).
  The Rust deps fall under the existing `cargo-deny` advisory gate automatically (it scans the whole
  `Cargo.lock`). **The gap is the Python side.** I confirmed `scripts/ci/check-advisories.sh` runs only
  `cargo deny --manifest-path .../Cargo.toml check advisories` — it is **Rust-only** and reads only
  `Cargo.lock` + `deny.toml`; it does not scan `uv.lock` / the Python dependency tree. T025's gate list
  (*"`ci:check-floating-versions`; `ci:check-advisories`"*) therefore gives **no CVE coverage** for
  pydantic / maturin / datamodel-code-generator. Floating-version discipline *is* covered: I confirmed
  `check-floating-versions.sh` scans `packages/*/pyproject.toml`, and T001/T002 pin exact patches /
  bounded ranges (the spec notes `maturin>=1.14,<2.0` is a *bounded* range, acceptable).
  - This is hygiene, not a live exposure: no advisory is known to affect the pinned versions at this
    review date, and the inputs are dev/build-time (codegen) or pure-Rust (pythonice/pyo3). But the
    spec-003 review's reassurance that "the new deps fall under the advisory gate automatically" does
    **not** transfer to the Python deps, and no artifact says it does — so the coverage gap should be
    recorded rather than assumed-closed.
- **Remediation:**
  1. Pin the Python deps to exact/bounded versions in `pyproject.toml` (T001/T002 already do; the
     floating-version gate enforces it). Keep `datamodel-code-generator` uv-locked at `0.65.1` for
     codegen determinism (bump deliberately).
  2. Decide explicitly whether a **Python advisory gate** is in scope for v1 (e.g. `uv pip audit` /
     `pip-audit` over `uv.lock`, or `osv-scanner` over the package). If deferred, record it as a known
     coverage gap (this finding) and a trigger to add it before the spec-007 PyPI publish — shipping a
     wheel with un-scanned transitive deps is the point at which this stops being hygiene.
  3. Confirm `pythonize 0.29` and `pyo3 0.29` carry no open RustSec advisory at implementation time via
     the existing (Rust) gate — already wired.

### SEC-005-PY — `depythonize` of deeply nested / large untrusted Python objects has no asserted depth/size bound (INFORMATIONAL)

- **OWASP 2025:** A03:2021-adjacent / A06 availability — informational; not a classic injection/exposure.
- **CWE:** CWE-400 (Uncontrolled Resource Consumption) / CWE-674 (Uncontrolled Recursion) — assessed;
  out of scope for v1's trust model.
- **Severity:** INFORMATIONAL.
- **Location:** `crates/prompting-press-py/src/marshal.rs` (`to_kernel_value` via `pythonize::depythonize`);
  research D2; FR-003a.
- **Evidence:** FR-003a requires lossless marshaling of *"`None`, int/float, nested structures"* via the
  bridge; research D2 specifies *"`depythonize` into the kernel's `minijinja::Value` … nested dict/list →
  nested value."* A maliciously deep or very large Python object handed to `render` could in principle
  drive unbounded stack/heap during `depythonize` (recursive descent over nested containers). Two
  mitigating facts make this **out of scope for v1**, mirroring the spec-003 SEC-005 reasoning at the
  loader: (1) the trust model — the Vars value is **application-authored** (data-model: *"Authored by the
  application, not the library"*) and the validated output of the caller's own Pydantic model, not an
  external attacker payload; the loader's YAML/JSON arm is handled by the consumer's already-reviewed
  serde path (Q3), so 004 introduces no *new* untrusted-document parser — only the depythonize of a
  caller's own dict; (2) validation runs **first** (Q1/FR-002), so the object is the post-`model_validate`
  `model_dump`, shaped by the caller's declared model. This is the 004 analogue of spec-003 SEC-005
  (untrusted-YAML depth/size) — recorded so the depythonize trust boundary is explicit and revisitable,
  not because v1 should add limits reflexively.
- **Remediation:** None required for v1 — do not add depth/size caps against a vector the v1 trust model
  does not expose (the Vars value is the caller's own validated object). **Trigger to revisit:** if a
  future spec/consumer lets *untrusted/external* parties supply the raw Python object marshaled to
  `render` (rather than a caller-authored Pydantic instance), assert a nesting-depth bound before
  `depythonize` and re-confirm pythonize's recursion posture at that version. Record the trigger so the
  decision stays revisitable.

### SEC-002-PY — Provenance lint is a lint, not a runtime sanitizer; carry the non-enforcement invariant to the Python `check`/`Finding` surface (INFORMATIONAL) — inherited carryover

- **OWASP 2025:** A04:2021 Insecure Design (security-control assumption mismatch).
- **CWE:** CWE-655 (a feature that reads like a control but enforces nothing).
- **Severity:** INFORMATIONAL. (Same as spec-003: the plan does **not** over-promise; `check()` is
  consistently a pure CI lint. The only residual is the connotation of `untrusted`/`guard` at the
  Python surface where a user wires a tag.)
- **Location:** `crates/prompting-press-py/src/check.rs` (`CheckReport`/`Finding`); contract §check;
  data-model §CheckReport/Finding; the T023 README/docstring.
- **Evidence:** FR-018 reports a prompt declaring an `untrusted`/`external` variable with **no guard** as
  a `Finding`. FR-019 keeps it honest: *"The check MUST be pure analysis — pass/fail — and MUST NOT
  mutate any prompt, definition, or input, render anything, or produce side effects."* FR-017: the check
  *"MUST obtain referenced variables and the provenance view from the Rust core's analysis (the binding
  does not re-derive them)"* — so the binding does not even own the analysis, it surfaces the consumer's.
  **Nothing in the four artifacts claims that tagging a field `untrusted`/`external` sanitizes, strips,
  or protects it.** The residual is identical to spec-003 SEC-002: the words connote a sanitizer to a
  casual reader, and the Python `check`/`Finding` surface (plus T023 docs) is exactly where a user wires
  a tag and is most likely to conclude "the untrusted field is now handled."
- **Remediation:** No design change. In `check.rs` docstrings, the `Finding` docs, and the T023 README,
  repeat the kernel/consumer normative invariant: *"Provenance tags and the guard are advisory metadata
  only; `check()` reports their misuse but does NOT sanitize, strip, or neutralize any value. A passing
  `check()` is not evidence that an untrusted field's content is safe."* (T023 already commits to
  documenting the C-06 normalization boundary and the three-sets invariant; add this one sentence.)
  This discharges the spec-003 SEC-002 "carry forward to consumer specs 004/005" item at 004.

### SEC-102 — FFI marshaling fidelity (lossless, no silent coercion) is a correctness AND a type-confusion control; keep it pinned (INFORMATIONAL)

- **OWASP 2025:** A04:2021 Insecure Design (input-handling fidelity at a trust boundary).
- **CWE:** N/A as a vuln — recorded as a control to preserve.
- **Severity:** INFORMATIONAL.
- **Location:** `crates/prompting-press-py/src/marshal.rs`; FR-003a; the "Marshaling edge values" edge case.
- **Evidence:** FR-003a: *"the binding MUST marshal the validated Vars into the kernel's value type
  **losslessly** (no silent coercion of `None`, int/float, nested structures)."* The edge-case list adds
  dates/decimals/null/int-vs-float. research D2 resolves the one fidelity footgun explicitly — *"**Lean
  `mode="json"`** so the marshaled value is JSON-primitive … pin the choice with a marshaling unit test"*
  — and T008 specifies that test (*"a Python dict with None / int / float / nested → the expected
  `minijinja::Value` (lossless — FR-003a)"*). This is well-specified. It is recorded here (not as a gap)
  because *silent coercion at an FFI boundary is a classic type-confusion source* (a value that
  round-trips as a different type than the caller intended can change which template branch renders);
  keeping the lossless property test-pinned is the control. The broad cross-binding marshaling corpus is
  deferred to spec 006 (spec.md Assumptions) — 004 correctly scopes itself to its own render/check paths.
- **Remediation:** None — the property is specified and test-pinned. Keep T008's lossless-marshal test as
  a CI gate; ensure the `mode="json"` decision (date/Decimal stringification) is the one pinned, so the
  binding's value shape matches the kernel's text rendering deterministically. Defer broad fidelity to
  spec 006 as planned.

### SEC-103 — `abi3-py39` → `abi3-py310` bump is an availability/correctness fix, not a security issue, but is the one committed-state divergence (LOW)

- **OWASP 2025:** N/A (correctness/availability — recorded for completeness, not a classic security class).
- **CWE:** N/A.
- **Severity:** LOW. (Borderline INFO; kept LOW because it is a *latent install-then-ImportError* on a
  supported-looking platform — a real, shipped-wheel availability defect, just not a confidentiality/
  integrity one. No exploit path.)
- **Location:** `crates/prompting-press-py/Cargo.toml` (`features = ["extension-module", "abi3-py39"]`).
- **Evidence:** I confirmed the committed crate still declares `abi3-py39` ("`abi3-py39` targets the
  stable ABI (one wheel across CPython >= 3.9)"). The spec resolves this as Q4 and FR-021: *"the crate's
  `abi3-py39` is bumped to `abi3-py310` so the ABI floor, `requires-python >=3.10`, and the codegen's
  3.10 target syntax all agree."* research D1 names the trap: *"the generated `X | None` syntax does not
  import on 3.9, so it was a latent install-then-ImportError trap."* T001 performs the bump. The spec also
  records a real watch-item (plan.md / research D1): *"CPython **3.10 reaches EOL 2026-10-31** … the
  abi3-py310 floor still runs post-EOL; 3.10 just stops receiving upstream patches."*
- **Remediation:** Land T001 (the bump) — already specified. At spec-007 (publish), reconsider whether
  the floor should advance given 3.10 EOL is ~4 months out, so the shipped wheel does not advertise a
  floor that no longer receives upstream security patches. No action needed in 004 beyond the bump.

## Confirmed secure-by-design patterns

Not findings — properties I verified hold in the plan (and, where noted, in live code) that materially
reduce the binding's real risk. Recorded so a later reviewer does not re-litigate them.

- **No validation bypass — no value-bearing path skips `model_validate`.** The binding OWNS validation
  at the render boundary (Q1/FR-002): *"runs validation once, before any templating … If validation
  fails, no render is performed."* `Composition` validates **eagerly at append/from_messages** (D7/T020),
  so `resolve` never emits a partial-as-success (FR-012, US4 sc.2). The two non-validating operations —
  `check()` and `get_source` — take **no** Vars values (contract), so they cannot smuggle unvalidated
  input to the kernel. Only validated values cross FFI (FR-003); the kernel stays validation-blind.
- **Native error types never leak (closed normalized boundary).** `PromptingPressError` and its subtypes
  are the only public error surface; Pydantic `ValidationError` and Rust `ConsumerError`/`KernelError`
  are mapped at the boundary and never exposed (FR-004/014, SC-006). The `code` vocabulary is the
  consumer's closed set. T006 makes the `ConsumerError` match **exhaustive** ("a new variant must be a
  compile error, not a fallthrough"), so a future kernel error cannot silently fall through to an
  unscrubbed default — this both satisfies C-06 and bounds the info-leak surface to one auditable site.
- **The SEC-004 scrub is reused, not re-implemented.** The binding surfaces the consumer's
  **already-scrubbed** rows (T006/research D4); I verified the consumer's scrub is real and tested in
  `crates/prompting-press/src/error.rs`. The binding deliberately does not reach past the consumer to
  raw `KernelError` detail — the correct architecture (fix the leak once, at the deepest layer that owns
  the message).
- **`check()` is pure analysis surfaced from the core — no re-derivation, no mutation, no render.**
  FR-017 (obtain referenced vars + provenance from the core), FR-019 (pure, no mutation/render/side
  effects), data-model (deterministic order preserved from the consumer). T016 pins purity (snapshot the
  registry before/after `check` → unchanged). No analysis/render disagreement surface in the binding.
- **No logic duplication — render/agreement/variant/hash stay in the kernel.** FR-011/C-02: the binding
  *"MUST NOT reimplement rendering, agreement analysis, variant resolution, or hashing."* It marshals to
  the consumer/kernel. SHA-256 hashes are computed once in the kernel and surfaced 1:1 as opaque
  identifiers; the binding adds no crypto. Render byte-parity is structural (Principle I), not re-tested.
- **FFI isolation is CI-enforced and verified.** I confirmed `check-ffi-isolation.sh` `COVERED_CRATES`
  lists `prompting-press-core` + `prompting-press`; `pyo3`/`pythonize` live ONLY in the binding crate
  (T001), and T003/T025 re-run `ci:check-ffi` (`cargo tree -p prompting-press -i pyo3` empty). pythonize
  is pure-Rust. SC-007 is structurally verifiable, not aspirational.
- **No I/O / no LLM / no request-body / no output parsing / no token counter.** FR-023 states all of
  these as MUST-NOTs; the contract repeats them; `output_model` is metadata only (FR-009/023). T025 +
  quickstart include an explicit `rg -ri "count_tokens|token"` check that finds **no** token surface
  (SC-010 / F4). The roadmap's stale "token hook" line is explicitly dropped (spec.md Assumptions). The
  boundary-defense list (roadmap "Never") is respected — no boundary-expanding capability is added.
- **Dual-input parity is a load (not a transform), via the consumer's single loader.** YAML, JSON, and a
  constructed object all normalize to the kernel's single `PromptDefinition` by **reusing the consumer's
  loader via FFI** (Q3/FR-005): no Python-side YAML parser, no second loader to keep in agreement, so
  YAML↔JSON parity + accept/reject are structural (the spec-003 Norway-safe behavior is inherited; T013
  pins the `no`/`off`→string case). Malformed input → `LoadError`, nothing partially loaded (FR-007, T014
  "on error insert NOTHING").
- **Codegen'd shape — no parallel hand-maintained definition.** The Pydantic `PromptDefinition` is
  code-generated from the JSON Schema and freshness-gated (FR-008/024, C-07); T022 forbids hand-editing
  `generated/`; T025/quickstart run `schemas:codegen-check`. No schema↔shape drift surface in the binding.

## Summary table

| ID | Title | Severity | OWASP / CWE | Status in plan |
|---|---|---|---|---|
| SEC-004-PY | SEC-004 scrub must survive Rust→Python translation; assert at Python `str()`/`errors` | MODERATE | A09 / CWE-209 | Specified (FR-015) + test-pinned (T006/T008/quickstart); residual: assert at the Python exception surface + Pydantic-mapper value-free |
| SEC-101 | New Python deps outside the Rust-only advisory gate; no Python CVE scan specified | LOW | A06 / CWE-1104 | Gap — floating-version covered, advisory coverage Rust-only (verified) |
| SEC-103 | `abi3-py39` → `abi3-py310` bump (availability/correctness, committed-state divergence) | LOW | N/A | Resolved in design (Q4/FR-021); T001 performs the bump |
| SEC-005-PY | `depythonize` of deeply nested/large untrusted input has no asserted depth/size bound | INFO | A03-adj / CWE-400, CWE-674 | Out of scope for v1 trust model (app-authored Vars); trigger recorded |
| SEC-002-PY | Provenance lint is a lint, not a runtime sanitizer — carry non-enforcement invariant to Python surface | INFO | A04 / CWE-655 | Plan honest; add one doc sentence at check.rs / T023 README |
| SEC-102 | FFI marshaling lossless / no silent coercion is a type-confusion control — keep pinned | INFO | A04 | Specified (FR-003a) + test-pinned (T008); keep `mode="json"` pinned |

## Carryover from spec-003 — do the 004 mitigations hold?

The spec-003 review raised four items. All four are accounted for in 004, and 004 preserves their
mitigations:

- **SEC-004 (error-detail leakage)** → **carries over as SEC-004-PY, escalated to MODERATE.** The
  mitigation is preserved by *reuse*: 004 surfaces the consumer's already-scrubbed `FieldError` rows and
  must not reach past to raw `KernelError` detail (T006/D4), with a Rust-side seeded-secret test (T008)
  and a Python-side `str()` assertion (quickstart). Escalated because the content now crosses a second
  translation hop and reaches a Python `str(exc)`/logging surface; the residual is to assert the scrub at
  that Python surface and confirm the Pydantic mapper stays value-free.
- **SEC-002 (provenance lint is not a sanitizer)** → **carries over as SEC-002-PY (INFORMATIONAL).** 004
  does not over-promise (FR-017/019 keep `check()` a pure lint over the core's analysis); the only action
  is to repeat the non-enforcement invariant at the Python `check`/`Finding`/README surface (T023).
  Mitigation preserved.
- **SEC-005 (untrusted-doc deserialization depth/size)** → **partially N/A, partially recast as
  SEC-005-PY.** The *loader* arm of SEC-005 is **structurally inherited and not re-opened**: 004 reuses
  the consumer's serde loader via FFI (Q3), so no new untrusted-document parser is introduced in Python.
  The new 004 analogue is `depythonize` of a caller's Python object (SEC-005-PY) — same INFORMATIONAL
  verdict, same app-authored/PR-gated trust model, same revisit trigger. Mitigation posture preserved.
- **SEC-001 (pin new deps; never the archived `serde_yaml`)** → **carries over as SEC-101 (LOW), with a
  NEW gap.** The Rust-dep half is preserved: `pyo3`/`pythonize` are pinned (T001) and covered by the
  verified Rust advisory + floating-version + FFI gates. The new gap is the Python deps (pydantic,
  maturin, datamodel-code-generator): the floating-version gate covers their `pyproject.toml` pins, but
  the advisory gate is Rust-only (verified), so they have **no CVE coverage** — recorded as SEC-101 with
  a trigger to add a Python advisory gate before the spec-007 publish.

**Net:** 004 does not weaken any spec-003 mitigation. It re-uses the deepest-layer scrub rather than
duplicating it, inherits the loader's structural parity/scrub, and keeps the lint honest. The one
genuinely new obligation is asserting the scrub at the new Python exception surface (SEC-004-PY), and
the one genuinely new coverage gap is Python-dependency CVE scanning (SEC-101). Neither blocks
implementation.
