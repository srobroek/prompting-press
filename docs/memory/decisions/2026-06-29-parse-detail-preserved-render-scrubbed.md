### 2026-06-29 — SEC-004 refinement: Parse-error detail is preserved; only Render-error detail is scrubbed

**ID**: D2

**Status**
Active

**Why this is durable**
SEC-004 ("never leak bound-value content into an error message/stack/trace") governs every error path the
library exposes, in every binding. The exact line — *which* `KernelError` detail is safe to surface — recurs
whenever a new error path is added or a binding's error mapper is touched. This decision is the durable,
reusable answer: it pins the safe/unsafe boundary to *when values are bound*, not to error kind uniformly.

**Decision**
The consumer's `From<KernelError> for ConsumerError` mapper (`crates/prompting-press/src/error.rs`):
- **`Render` detail → SCRUBBED** (fixed templated message, raw detail discarded). Rendering is the only
  stage where bound values flow into the engine, so render-error detail can contain untrusted input / PII /
  secrets.
- **`Parse` detail → PRESERVED** (surfaced in the message). The engine parses the template source EAGERLY,
  *before* any value is bound (`engine.rs`: `add_template_owned(...)` then `template.render(values)`), so a
  parse error carries only template-syntax context (line/column, the offending construct) — never a bound
  value. Preserving it is what makes a syntax error debuggable.
- `ExcludedFeature` detail stays templated (defense in depth — names a construct, not a value);
  `UnknownVariant`/`UndefinedVariable` surface only the caller-supplied name (not value content).

This is a **refinement, not a redefinition**, of SEC-004: the PII guarantee is fully preserved (Render still
scrubbed), and debuggability is *added* for the one path that provably has no bound values. Therefore it is
NOT a constitution amendment (no numbered principle changed) and lives here, not in `DECISIONS.md`.

**Evidence**
- `engine.rs` `render(...)`: `add_template_owned("kernel", source)` (parse) runs and can return a
  `SyntaxError`→`KernelError::Parse` BEFORE the subsequent `template.render(values)` — so no value is bound
  at parse time. Verified by reading the render pipeline this session.
- Tests flipped to assert preservation: `error::tests::parse_detail_is_preserved_for_debuggability`
  (consumer) and `parse_kernel_detail_is_preserved` (node binding); `render_detail_secret_is_scrubbed` and
  the `fuzz_scrub` corpus tests remain green (Render still scrubbed). All gates passed
  (cargo --workspace, Python 132+32, Node 133+45).
- Bindings inherit this automatically: `prompting-press-py` and `-node` route kernel errors through the
  consumer's `From` scrubber first, so neither reads raw `KernelError::detail`.

**Tradeoffs**
- Gained: actionable template-syntax errors (line/column/construct) at construction time, instead of an
  opaque "template parse error" — removes a real debugging pain point.
- Cost: a contrived secret literally embedded in a *template body* that fails to parse would surface — but
  that is the developer's own authored source file, not runtime/bound data, so it is not a PII-leak class.
- Reconsider: an opt-in "unsafe Render detail" mode (caller explicitly accepts responsibility to receive
  full Render detail) is a SEPARATE future spec — it adds a new seam and DOES touch the Render scrub, so it
  requires its own design + a constitution check, unlike this refinement.

**Where to look next**
`crates/prompting-press/src/error.rs` (the `From<KernelError>` mapper + the module doc-comment explaining
the parse-vs-render boundary), `crates/prompting-press-node/src/error.rs` (the binding test),
`docs/site/src/content/docs/reference/{rust,python,typescript}.mdx` (the "Parse detail preserved / Render
scrubbed" Asides). The opt-in unsafe-Render-detail spec, once written, is the place that may revisit the
Render half.
