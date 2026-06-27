//! The FFI **value bridge** — the single auditable locus where a Python value crosses
//! into the kernel's value type (FR-003a, research D2 / C-02).
//!
//! Every Python → kernel value translation in this crate funnels through
//! [`to_kernel_value`]. Concentrating it here means the marshaling boundary is **one
//! file** to audit (constitution Principle II / C-02: the binding is marshaling + facade,
//! never engine logic).
//!
//! ## Why two hops (`depythonize` → `from_serialize`)
//!
//! 1. [`pythonize::depythonize`] reads the Python object into a [`serde_json::Value`].
//!    serde's data model preserves the edges that matter for parity: `None` → JSON null,
//!    `bool`, **`int` vs `float`** (an integral Python value stays a JSON integer, a float
//!    stays a JSON float), and nested `dict`/`list` → nested object/array.
//! 2. [`minijinja::Value::from_serialize`] turns that `serde_json::Value` into the kernel's
//!    [`minijinja::Value`] — **the exact same primitive the Rust consumer uses** to hand
//!    vars to `prompting_press_core::render`. Because both bindings feed the kernel a value
//!    built by `from_serialize` over the JSON data model, the rendered string and the
//!    provenance hashes are byte-identical across languages **by construction** (Principle I)
//!    — there is no per-binding render re-verification.
//!
//! The caller is expected to pass an already-`model_dump(mode="json")`'d Pydantic payload
//! (research D2): `mode="json"` stringifies `datetime`/`Decimal` deterministically, so they
//! arrive as JSON-primitive strings and parity with the other bindings is trivial. This file
//! does not depend on that — it marshals whatever Python object it is given losslessly — but
//! that is the documented call convention the render path uses.
//!
//! ## Errors
//!
//! A value that `depythonize` cannot represent in the serde data model (e.g. an arbitrary
//! Python object with no serde mapping) surfaces as a [`ConsumerError::Load`] carrying a
//! short, **shape-level** description. `depythonize`'s error text describes the *type* it
//! could not convert (e.g. "unsupported type"), not bound-value content, so it is safe to
//! surface here — the same class of message as the loader's serde errors (which are likewise
//! `Load`). Kernel `Parse`/`Render` detail (the secret-bearing strings, SEC-004) is a
//! *different* path and never flows through this file.

use pyo3::types::PyAny;
use pyo3::Bound;

use prompting_press::ConsumerError;

/// Marshal an already-validated Python value into the kernel's [`minijinja::Value`].
///
/// This is the one bridge function (FR-003a). It performs **no validation** (the render
/// path validates against the Pydantic Vars model *before* calling this) and **no engine
/// logic** — it is a pure, lossless value translation.
///
/// See the [module docs](self) for the two-hop rationale and the parity guarantee.
///
/// # Errors
///
/// Returns [`ConsumerError::Load`] if the Python object cannot be represented in the serde
/// data model. The message is shape-level (a type description), never bound-value content.
pub fn to_kernel_value(obj: &Bound<'_, PyAny>) -> Result<minijinja::Value, ConsumerError> {
    // Hop 1: Python object -> serde_json::Value (lossless for null/bool/int/float/nested).
    let json: serde_json::Value = pythonize::depythonize(obj)
        .map_err(|e| ConsumerError::Load(format!("could not marshal Python value: {e}")))?;

    // Hop 2: serde_json::Value -> minijinja::Value via the SAME primitive the consumer uses.
    Ok(minijinja::Value::from_serialize(&json))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pyo3::prelude::*;
    use pyo3::types::{PyDict, PyList};

    /// Build the expected kernel value the same way the bridge's second hop does, so the
    /// assertions compare against `from_serialize` output rather than a hand-built `Value`
    /// (the comparison is then exactly "did hop 1 preserve the serde shape").
    fn kernel_value(json: serde_json::Value) -> minijinja::Value {
        minijinja::Value::from_serialize(&json)
    }

    /// `None` must round-trip to a JSON null, i.e. the kernel's `Value` for `null`.
    #[test]
    fn none_marshals_to_null() {
        Python::attach(|py| {
            let obj = py.None();
            let got = to_kernel_value(obj.bind(py)).expect("None marshals");
            assert_eq!(got, kernel_value(serde_json::Value::Null));
            // And it is the kernel's notion of "undefined-ish" none, not a string "None".
            assert!(
                got.is_none(),
                "expected a none/undefined value, got {got:?}"
            );
        });
    }

    /// `bool` is preserved as a boolean (not coerced to 0/1).
    #[test]
    fn bool_marshals_to_bool() {
        Python::attach(|py| {
            let t = true.into_pyobject(py).expect("bool obj");
            let got = to_kernel_value(&t).expect("bool marshals");
            assert_eq!(got, kernel_value(serde_json::json!(true)));
        });
    }

    /// **The int-vs-float edge (FR-003a):** an integral Python value stays an integer; a
    /// Python float stays a float. They must NOT collapse to the same kernel value.
    #[test]
    fn int_and_float_are_distinct() {
        Python::attach(|py| {
            let int_obj = 3i64.into_pyobject(py).expect("int obj");
            let float_obj = 3.0f64.into_pyobject(py).expect("float obj");

            let int_val = to_kernel_value(&int_obj).expect("int marshals");
            let float_val = to_kernel_value(&float_obj).expect("float marshals");

            assert_eq!(int_val, kernel_value(serde_json::json!(3)));
            assert_eq!(float_val, kernel_value(serde_json::json!(3.0)));

            // The integer kind is preserved (not silently widened to a float).
            assert_eq!(
                int_val.kind(),
                minijinja::value::ValueKind::Number,
                "int should marshal to a number"
            );
            assert!(
                i64::try_from(int_val.clone()).is_ok(),
                "an integral input must remain integral after marshaling, got {int_val:?}"
            );
        });
    }

    /// A non-integral float keeps its fractional part.
    #[test]
    fn float_preserves_fraction() {
        Python::attach(|py| {
            let f = 2.5f64.into_pyobject(py).expect("float obj");
            let got = to_kernel_value(&f).expect("float marshals");
            assert_eq!(got, kernel_value(serde_json::json!(2.5)));
        });
    }

    /// A string marshals to a string (the common case).
    #[test]
    fn string_marshals_to_string() {
        Python::attach(|py| {
            let s = "Ada".into_pyobject(py).expect("str obj");
            let got = to_kernel_value(&s).expect("str marshals");
            assert_eq!(got, kernel_value(serde_json::json!("Ada")));
        });
    }

    /// A nested `dict` containing a `list` and mixed scalars round-trips structurally — the
    /// nesting, the int/float distinction inside the list, and `None` inside the map are all
    /// preserved through the bridge.
    #[test]
    fn nested_dict_and_list_round_trip() {
        Python::attach(|py| {
            let inner = PyList::new(py, [1i64, 2i64, 3i64]).expect("list");
            let dict = PyDict::new(py);
            dict.set_item("name", "Ada").expect("set name");
            dict.set_item("count", 3i64).expect("set count");
            dict.set_item("ratio", 1.5f64).expect("set ratio");
            dict.set_item("note", py.None()).expect("set note");
            dict.set_item("nums", inner).expect("set nums");

            let got = to_kernel_value(dict.as_any()).expect("nested marshals");

            let expected = kernel_value(serde_json::json!({
                "name": "Ada",
                "count": 3,
                "ratio": 1.5,
                "note": null,
                "nums": [1, 2, 3],
            }));
            assert_eq!(got, expected);
        });
    }

    /// An empty mapping marshals to an empty map value (not null, not an error).
    #[test]
    fn empty_dict_marshals_to_empty_map() {
        Python::attach(|py| {
            let dict = PyDict::new(py);
            let got = to_kernel_value(dict.as_any()).expect("empty dict marshals");
            assert_eq!(got, kernel_value(serde_json::json!({})));
        });
    }
}
