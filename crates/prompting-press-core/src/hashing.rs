//! Content-addressed hashing helpers (spec 002, T018; FR-012/FR-013; research D8).
//!
//! The kernel emits two provenance hashes per render, each a lowercase-hex SHA-256 over
//! the UTF-8 bytes of a **string**:
//!
//! - `template_hash = SHA256(resolved variant source)` (FR-012), and
//! - `render_hash   = SHA256(rendered output)` (FR-013).
//!
//! Hashing over the string — never over structured input — is exactly constitution C-05:
//! there is **no** `vars_hash` (FR-014), which sidesteps the JSON-canonicalization problem
//! entirely. `sha2` is pure-Rust and deterministic, so these hashes are byte-identical
//! across languages by construction (constitution Principle I).

use sha2::{Digest, Sha256};

/// Compute the lowercase-hex SHA-256 of a string's UTF-8 bytes.
///
/// Deterministic and pure: the same input always yields the same 64-character
/// lowercase-hex digest. Used for both `template_hash` and `render_hash`.
pub(crate) fn sha256_hex(s: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::sha256_hex;

    #[test]
    fn known_vector_empty_string() {
        // SHA-256 of the empty input (NIST vector), lowercase hex.
        assert_eq!(
            sha256_hex(""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn known_vector_abc() {
        // SHA-256("abc") (NIST vector), lowercase hex.
        assert_eq!(
            sha256_hex("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn deterministic_and_64_hex_chars() {
        let a = sha256_hex("Hello Ada");
        let b = sha256_hex("Hello Ada");
        assert_eq!(a, b, "same input ⇒ same digest");
        assert_eq!(a.len(), 64, "lowercase-hex SHA-256 is 64 chars");
        assert!(
            a.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "digest is lowercase hex"
        );
    }
}
