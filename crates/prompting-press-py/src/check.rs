//! The Python lint report types — [`CheckReport`] and [`Finding`] pyclasses
//! (spec 003 US3 / FR-016..020; constitution Principle IV / C-04/C-09).
//!
//! ## Lint is `prompt.check()` now (spec 008 Phase 4)
//!
//! The pre-reshape `check(reg)` free function that walked a `Registry` is removed.
//! Linting is now `prompt.check()` on each [`Prompt`](crate::prompt::Prompt) object.
//! This module retains only the shared report types that `prompt.rs` uses when it
//! converts the consumer's [`prompting_press::CheckReport`] to the Python surface.
//!
//! ## The `kind` discriminant string (FR-020)
//!
//! Python matches on a [`Finding`]'s **`kind`**, exposed as a stable `snake_case` **discriminant
//! string** (not an opaque enum): `"untrusted_without_guard"`. The kind's inner datum (the
//! uncovered `field`) is already echoed in [`Finding::detail`], so `kind` stays a single stable
//! matchable value. The mapping is an **exhaustive** match over the consumer's [`FindingKind`]
//! (no wildcard arm): a new consumer variant becomes a compile error here, forcing the Python
//! surface to be updated deliberately rather than silently mapping to a stale string.

use pyo3::prelude::*;

use prompting_press::FindingKind;

/// The output of [`Prompt::check`](crate::prompt::Prompt::check): an ordered, read-only list of
/// [`Finding`]s. Empty ⇒ the lint passed.
///
/// The Python mirror of the consumer's [`prompting_press::CheckReport`] (data-model §`CheckReport`;
/// FR-020). Surfaced **1:1** — the binding adds nothing and interprets nothing. Read-only
/// (`frozen`): a report is produced by `prompt.check()`, never constructed from Python.
// `skip_from_py_object`: output-only — Python reads `findings` / `passed()`, never passes a
// `CheckReport` *in* — so opt out of the implicit `FromPyObject` derive PyO3 0.29 would pull in.
#[pyclass(
    name = "CheckReport",
    frozen,
    module = "prompting_press",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct CheckReport {
    /// Every lint failure found, in the consumer's deterministic order. Empty ⇒ pass.
    #[pyo3(get)]
    pub findings: Vec<Finding>,
}

#[pymethods]
impl CheckReport {
    /// `report.passed()` — `True` iff there are no findings (the lint passed). Mirrors the
    /// consumer's [`CheckReport::passed`](prompting_press::CheckReport::passed); reads clearly at a
    /// CI gate (`if not prompt.check().passed(): sys.exit(1)`).
    fn passed(&self) -> bool {
        self.findings.is_empty()
    }

    /// `report.is_empty()` — alias for [`passed`](Self::passed): `True` iff there are no findings.
    fn is_empty(&self) -> bool {
        self.findings.is_empty()
    }

    /// `len(report)` — the number of findings, so a `CheckReport` is truthy iff the lint failed
    /// (Python `bool(report)` follows `__len__`: empty ⇒ falsy ⇒ passed).
    fn __len__(&self) -> usize {
        self.findings.len()
    }

    /// `repr(report)` — a compact, fixed-shape rendering. Reports the finding count only; the
    /// per-finding detail is reachable via the `findings` getter.
    fn __repr__(&self) -> String {
        format!("CheckReport(findings={})", self.findings.len())
    }
}

impl From<prompting_press::CheckReport> for CheckReport {
    fn from(r: prompting_press::CheckReport) -> Self {
        Self {
            findings: r.findings.into_iter().map(Finding::from).collect(),
        }
    }
}

/// One actionable lint finding, read-only from Python.
///
/// Names the `prompt`, the `variant` where applicable (`None` for a prompt-level finding), the
/// failure `kind` (a stable `snake_case` discriminant string — see the module docs), and a
/// human-readable `detail`. The `detail` carries no bound-value content (it is built from prompt
/// and field names only). Read-only (`frozen`): a finding is produced by `prompt.check()`, never
/// constructed from Python.
// `skip_from_py_object`: output-only — Python reads the getters, never passes a `Finding` *in*.
#[pyclass(
    name = "Finding",
    frozen,
    module = "prompting_press",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct Finding {
    /// The prompt's name.
    #[pyo3(get)]
    pub prompt: String,
    /// The variant the finding pertains to; `None` for a prompt-level provenance finding.
    #[pyo3(get)]
    pub variant: Option<String>,
    /// The failure kind as a stable `snake_case` **discriminant string** — the value Python matches
    /// on. Currently always `"untrusted_without_guard"`. The kind's inner datum is echoed in
    /// [`detail`](Self::detail).
    #[pyo3(get)]
    pub kind: String,
    /// A human-readable, actionable description (FR-020). Carries no bound-value content.
    #[pyo3(get)]
    pub detail: String,
}

#[pymethods]
impl Finding {
    /// `repr(finding)` — a compact, fixed-shape rendering. All four fields are name/metadata only
    /// (no bound-value content), so they are safe to surface verbatim.
    fn __repr__(&self) -> String {
        format!(
            "Finding(prompt={:?}, variant={:?}, kind={:?}, detail={:?})",
            self.prompt, self.variant, self.kind, self.detail
        )
    }
}

impl From<prompting_press::Finding> for Finding {
    fn from(f: prompting_press::Finding) -> Self {
        Self {
            prompt: f.prompt,
            variant: f.variant,
            kind: kind_discriminant(&f.kind).to_string(),
            detail: f.detail,
        }
    }
}

/// Map a consumer [`FindingKind`] to its stable `snake_case` **discriminant string**.
///
/// **Exhaustive on purpose** (no `_` wildcard): if the consumer adds a `FindingKind` variant, this
/// match fails to compile, forcing the Python-visible string set to be updated deliberately rather
/// than silently mapping a new variant to a stale value. The bound inner data (`field`) is
/// intentionally *not* read here — it is already carried in [`Finding::detail`].
pub(crate) fn kind_discriminant(kind: &FindingKind) -> &'static str {
    match kind {
        FindingKind::UntrustedWithoutGuard { .. } => "untrusted_without_guard",
    }
}
