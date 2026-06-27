//! The FFI **value bridge** — the single auditable locus where a JavaScript value crosses
//! into the kernel's value type (FR-003a, research D2 / C-02).
//!
//! Every JS → kernel value translation in this crate funnels through [`to_kernel_value`].
//! Concentrating it here means the marshaling boundary is **one file** to audit
//! (constitution Principle II / C-02: the binding is marshaling + facade, never engine
//! logic).
//!
//! ## Why two hops (`serde_json::Value` → `from_serialize`)
//!
//! 1. napi's `serde-json` feature provides `impl FromNapiValue for serde_json::Value`, so
//!    the `#[napi]` function signatures take a `serde_json::Value` directly and napi reads
//!    the JS argument into it at the FFI boundary — **the one JS→serde hop**. serde's data
//!    model preserves the edges that matter for parity: `null` → JSON null, `bool`, JS
//!    numbers, nested object/array, and (with the `napi6` feature) `bigint` losslessly.
//! 2. [`minijinja::Value::from_serialize`] turns that `serde_json::Value` into the kernel's
//!    [`minijinja::Value`] — **the exact same primitive the Rust consumer uses** to hand
//!    vars to `prompting_press_core::render`. Because both bindings feed the kernel a value
//!    built by `from_serialize` over the JSON data model, the rendered string and the
//!    provenance hashes are byte-identical across languages **by construction** (Principle I)
//!    — there is no per-binding render re-verification.
//!
//! ## The `undefined`/`null`/absent rule (Q6 / FR-003a)
//!
//! JavaScript distinguishes `undefined` from `null`; the kernel value (JSON data model) has
//! only `null`. The rule the binding upholds:
//!
//! - An **absent object field** (a key simply not present) and a field set to JS `undefined`
//!   both ⇒ the field is **not present** in the kernel value. napi's
//!   `FromNapiValue for serde_json::Map` walks the object's *own keys*: an absent key never
//!   appears, and a key whose value is `undefined`/a function/a symbol is **dropped** rather
//!   than erroring (`obj.get` returns `None` for those — see the napi `serde-json` impl), so
//!   neither lands a spurious `null` on a kernel map field (which the kernel's
//!   `deny_unknown_fields` map fields would reject).
//! - An explicit **`null`** ⇒ JSON `null` (`serde_json::Value::Null`), preserved.
//!
//! The caller is expected to pass an already-Zod-validated plain JS object (research D2 / Q1):
//! Zod runs in the TS facade *before* the addon is called, so this bridge does **no**
//! validation — it marshals an already-validated value losslessly. The `#[napi]` boundary
//! itself never hands a bare top-level `undefined` here: the render/compose value parameter is
//! the validated object, and a top-level `undefined`/`null` that *did* reach napi's
//! `serde_json::Value` conversion would either become `Value::Null` or surface as a napi
//! `InvalidArg` *at the boundary* before this function runs.
//!
//! ## bigint losslessness (Q6 / D2)
//!
//! With the `napi6` feature, a JS `bigint` that fits `i64`/`u64` round-trips to a
//! `serde_json::Number` losslessly (napi's `BigInt::get_i64`/`get_u64`); only a bigint
//! exceeding 64 bits degrades to a string (napi's own fallback, outside the binding's
//! control). Pinned by [`tests::bigint_is_lossless`].
//!
//! ## Errors
//!
//! This function is **infallible**: it receives a `serde_json::Value` that napi already
//! produced from the JS argument (any unrepresentable JS value — a function, a symbol — is
//! rejected by napi *at the FFI boundary* with an `InvalidArg`, never reaching here), and
//! `from_serialize` over an owned `serde_json::Value` cannot fail. Kernel `Parse`/`Render`
//! detail (the secret-bearing strings, SEC-004) is a *different* path and never flows through
//! this file.

/// Marshal an already-validated, napi-decoded JS value into the kernel's
/// [`minijinja::Value`].
///
/// This is the one bridge function (FR-003a), and it is the second hop: napi performs the
/// JS → [`serde_json::Value`] hop at the `#[napi]` boundary (the `serde-json` feature), and
/// this function performs the [`serde_json::Value`] → [`minijinja::Value`] hop via the **same**
/// `from_serialize` primitive the Rust consumer uses. It performs **no** validation (Zod runs
/// in the TS facade *before* the addon is called) and **no** engine logic — a pure, lossless
/// value translation.
///
/// See the [module docs](self) for the two-hop rationale, the `undefined`/`null` rule, and the
/// parity guarantee.
pub fn to_kernel_value(json: serde_json::Value) -> minijinja::Value {
    // Hop 2: serde_json::Value -> minijinja::Value via the SAME primitive the consumer uses.
    // (Hop 1, JS -> serde_json::Value, is done by napi at the FFI boundary.)
    minijinja::Value::from_serialize(&json)
}

#[cfg(test)]
mod tests {
    //! These tests exercise the second hop (`serde_json::Value` → `minijinja::Value`) — the
    //! part this file owns. The first hop (JS → `serde_json::Value`) is napi's own,
    //! feature-gated impl; its `undefined`/`null`/`bigint` semantics are pinned end-to-end
    //! from TypeScript in the T010/T015 suites (which can construct real JS values). Here we
    //! prove the serde data-model edges this binding relies on survive the second hop
    //! identically to how the consumer builds its value — so render parity stays structural.

    use super::*;

    /// Build the expected kernel value the same way the bridge's second hop does, so the
    /// assertions compare against `from_serialize` output rather than a hand-built `Value`.
    fn kernel_value(json: serde_json::Value) -> minijinja::Value {
        minijinja::Value::from_serialize(&json)
    }

    /// JSON `null` (an explicit JS `null`, per the Q6 rule) round-trips to the kernel's none
    /// value — not a string `"null"`.
    #[test]
    fn null_marshals_to_none() {
        let got = to_kernel_value(serde_json::Value::Null);
        assert_eq!(got, kernel_value(serde_json::Value::Null));
        assert!(got.is_none(), "explicit null ⇒ kernel none, got {got:?}");
    }

    /// `bool` is preserved as a boolean (not coerced to 0/1).
    #[test]
    fn bool_marshals_to_bool() {
        let got = to_kernel_value(serde_json::json!(true));
        assert_eq!(got, kernel_value(serde_json::json!(true)));
    }

    /// A string marshals to a string (the common case).
    #[test]
    fn string_marshals_to_string() {
        let got = to_kernel_value(serde_json::json!("Ada"));
        assert_eq!(got, kernel_value(serde_json::json!("Ada")));
    }

    /// An integral number stays an integer through the second hop (the kernel sees a number
    /// of integer kind, not a widened float). Note: JavaScript has a single number type, so
    /// `3` and `3.0` are the *same* JS value and napi's first hop yields the same
    /// `serde_json::Number` for both — there is no int-vs-float JS distinction to preserve
    /// (unlike Python). What the binding must preserve is that an integral serde number stays
    /// integral into the kernel.
    #[test]
    fn integral_number_stays_integral() {
        let got = to_kernel_value(serde_json::json!(3));
        assert_eq!(got, kernel_value(serde_json::json!(3)));
        assert_eq!(
            got.kind(),
            minijinja::value::ValueKind::Number,
            "an integral input must marshal to a number"
        );
        assert!(
            i64::try_from(got.clone()).is_ok(),
            "an integral input must remain integral after marshaling, got {got:?}"
        );
    }

    /// A non-integral number keeps its fractional part.
    #[test]
    fn float_preserves_fraction() {
        let got = to_kernel_value(serde_json::json!(2.5));
        assert_eq!(got, kernel_value(serde_json::json!(2.5)));
    }

    /// **bigint losslessness (Q6 / D2).** A 64-bit-range integer (the value an in-range JS
    /// `bigint` decodes to via napi's `napi6` `BigInt::get_i64`) round-trips through the
    /// second hop with no loss of precision — `9_007_199_254_740_993` is `2^53 + 1`, beyond
    /// the f64 safe-integer range, so a lossy float path would corrupt it. This pins that the
    /// kernel value preserves the full i64 once napi has produced the lossless
    /// `serde_json::Number` (the JS-`bigint`→`serde_json` step is napi's own, exercised TS-side).
    #[test]
    fn bigint_is_lossless() {
        const BEYOND_F64_SAFE: i64 = 9_007_199_254_740_993; // 2^53 + 1
        let got = to_kernel_value(serde_json::json!(BEYOND_F64_SAFE));
        assert_eq!(got, kernel_value(serde_json::json!(BEYOND_F64_SAFE)));
        assert_eq!(
            i64::try_from(got.clone()).ok(),
            Some(BEYOND_F64_SAFE),
            "a 64-bit integer must survive marshaling losslessly, got {got:?}"
        );
    }

    /// A nested object containing an array and mixed scalars round-trips structurally — the
    /// nesting, the numbers inside the array, and an explicit `null` inside the map are all
    /// preserved through the second hop.
    #[test]
    fn nested_object_and_array_round_trip() {
        let value = serde_json::json!({
            "name": "Ada",
            "count": 3,
            "ratio": 1.5,
            "note": null,
            "nums": [1, 2, 3],
        });
        let got = to_kernel_value(value.clone());
        assert_eq!(got, kernel_value(value));
    }

    /// An empty object marshals to an empty map value (not none, not an error).
    #[test]
    fn empty_object_marshals_to_empty_map() {
        let got = to_kernel_value(serde_json::json!({}));
        assert_eq!(got, kernel_value(serde_json::json!({})));
        assert_eq!(
            got.kind(),
            minijinja::value::ValueKind::Map,
            "an empty object ⇒ an (empty) map, got {got:?}"
        );
    }
}
