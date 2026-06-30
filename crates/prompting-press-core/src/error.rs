//! Structured kernel error type (spec 002, data-model §KernelError; FR-028).
//!
//! The kernel returns its **native** [`KernelError`]. It deliberately does NOT
//! normalize errors to the common `[{field, code, message}]` shape — that is the
//! consumer's job at each binding boundary (roadmap decision C-06 (Principle VI)).
//! Native error types MUST NOT leak across FFI, but that normalization lives in the
//! consumer (spec 003), never here.
//!
//! > **Info-leakage note (security SEC-004):** the `detail` strings on
//! > [`KernelError::Parse`] / [`KernelError::Render`] may embed bound-value content
//! > (which can be untrusted / PII). Holding it in-process is fine; the spec-003
//! > normalization layer is responsible for scrubbing it before logging.

/// Errors surfaced by the engine kernel.
///
/// Each variant maps to a functional requirement in the spec-002 contract; see the
/// per-variant docs. The consumer normalizes these into the common structured shape.
#[derive(Debug, PartialEq, Eq)]
pub enum KernelError {
    /// A render / source / analysis call named a variant that does not exist on the
    /// definition (and is not the reserved `"default"`). [FR-009]
    UnknownVariant {
        /// The variant name the caller requested.
        requested: String,
    },

    /// The template used an **excluded** feature — `{% include %}`, `{% import %}`,
    /// `{% from … import %}`, `{% extends %}`, `{% macro %}`, or `{% block %}`. With
    /// the `macros` / `multi_template` engine features disabled (research D1/D4) these
    /// tags are unrecognised and fail at parse time; the kernel surfaces that as this
    /// variant when it is distinguishable, otherwise as [`KernelError::Parse`]. [FR-002]
    ExcludedFeature {
        /// Human-readable detail describing the rejected construct.
        detail: String,
    },

    /// The template failed to parse (syntax error). [FR-028]
    Parse {
        /// Human-readable detail from the underlying parse failure.
        detail: String,
    },

    /// A strict-undefined reference was hit at render time: the template used or
    /// printed a variable that was not supplied. Under `UndefinedBehavior::Strict`
    /// this is a loud error rather than a silent empty render. [FR-001a]
    UndefinedVariable {
        /// The undefined root variable name (best-effort, from the render error).
        name: String,
    },

    /// Any other render-time failure (e.g. iterating a non-iterable in a loop). [FR-028]
    Render {
        /// Human-readable detail from the underlying render failure.
        detail: String,
    },

    /// A caller-supplied guard advisory override (`GuardConfig::advisory`) does not
    /// reference the delimiter contract. To prevent shipping a guard whose advisory
    /// fails to explain the `<untrusted>` markers (a silently half-broken defense),
    /// an override MUST contain the opening tag `<untrusted>`, the closing tag
    /// `</untrusted>`, and an indication that the markers are escaped inside values
    /// (one of `&amp;`, `&lt;`, `&gt;`, or the word "escap"). The fixed default
    /// satisfies this by construction; only an override can trip it. [spec 015]
    GuardAdvisoryInvalid {
        /// Human-readable detail naming which required element(s) are missing.
        detail: String,
    },
}

impl std::fmt::Display for KernelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownVariant { requested } => {
                write!(f, "unknown variant: `{requested}`")
            }
            Self::ExcludedFeature { detail } => {
                write!(f, "template uses an excluded feature: {detail}")
            }
            Self::Parse { detail } => {
                write!(f, "template parse error: {detail}")
            }
            Self::UndefinedVariable { name } => {
                write!(f, "undefined variable at render: `{name}`")
            }
            Self::Render { detail } => {
                write!(f, "render error: {detail}")
            }
            Self::GuardAdvisoryInvalid { detail } => {
                write!(f, "guard advisory override is invalid: {detail}")
            }
        }
    }
}

impl std::error::Error for KernelError {}

#[cfg(test)]
mod tests {
    use super::KernelError;

    #[test]
    fn display_is_human_readable() {
        let e = KernelError::UnknownVariant {
            requested: "concise".to_string(),
        };
        assert_eq!(e.to_string(), "unknown variant: `concise`");

        let e = KernelError::UndefinedVariable {
            name: "article".to_string(),
        };
        assert_eq!(e.to_string(), "undefined variable at render: `article`");

        // S-3: Display for the remaining three variants — each must be non-empty and
        // carry its key detail string.
        let e = KernelError::ExcludedFeature {
            detail: "unknown statement include".to_string(),
        };
        let s = e.to_string();
        assert!(!s.is_empty());
        assert!(
            s.contains("unknown statement include"),
            "ExcludedFeature Display must surface its detail, got: {s:?}"
        );

        let e = KernelError::Parse {
            detail: "unexpected end of input".to_string(),
        };
        let s = e.to_string();
        assert!(!s.is_empty());
        assert!(
            s.contains("unexpected end of input"),
            "Parse Display must surface its detail, got: {s:?}"
        );

        let e = KernelError::Render {
            detail: "number is not iterable".to_string(),
        };
        let s = e.to_string();
        assert!(!s.is_empty());
        assert!(
            s.contains("number is not iterable"),
            "Render Display must surface its detail, got: {s:?}"
        );
    }

    /// The new `PartialEq, Eq` derive (TY-3) lets tests assert variants structurally.
    #[test]
    fn equality_is_structural() {
        assert_eq!(
            KernelError::UnknownVariant {
                requested: "x".to_string(),
            },
            KernelError::UnknownVariant {
                requested: "x".to_string(),
            },
        );
        assert_ne!(
            KernelError::UnknownVariant {
                requested: "x".to_string(),
            },
            KernelError::UnknownVariant {
                requested: "y".to_string(),
            },
        );
    }

    #[test]
    fn is_std_error() {
        fn assert_error<E: std::error::Error>(_: &E) {}
        let e = KernelError::Parse {
            detail: "unexpected end of input".to_string(),
        };
        assert_error(&e);
    }
}
