//! `dashu.rkyv` — zero-copy serialization of the dashu numeric types.
//!
//! rkyv archives values as *borrowed* byte slices — deserialization does not allocate or copy
//! the payload (only the heap parts of the value, like significand buffers, are reconstructed).
//! This is the fastest serialization available, at the cost of a **machine-specific** byte format:
//! bytes produced by `to_bytes` are only guaranteed readable by `from_bytes` on the same
//! architecture. For a portable format use [`crate::serde`].
//!
//! `from_bytes` does not validate its input (that is the price of zero-copy): the bytes must come
//! from [`to_bytes`] on the same type. Passing arbitrary bytes is undefined behavior.

use pyo3::PyTypeInfo;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;

use crate::types::{CPy, DPy, FPy, IPy, RPy, UPy};

/// Dispatch a serialization body over every dashu value type (same pattern as
/// `crate::serde::with_dashu_value!`).
macro_rules! with_dashu_value {
    ($obj:expr, |$inner:ident| $body:expr) => {{
        const MSG: &str = "expected a dashu numeric type (UBig, IBig, FBig, DBig, RBig, or CBig)";
        if let Ok(v) = $obj.extract::<PyRef<'_, UPy>>() {
            let $inner = &v.0;
            $body
        } else if let Ok(v) = $obj.extract::<PyRef<'_, IPy>>() {
            let $inner = &v.0;
            $body
        } else if let Ok(v) = $obj.extract::<PyRef<'_, FPy>>() {
            let $inner = &v.0;
            $body
        } else if let Ok(v) = $obj.extract::<PyRef<'_, DPy>>() {
            let $inner = &v.0;
            $body
        } else if let Ok(v) = $obj.extract::<PyRef<'_, RPy>>() {
            let $inner = &v.0;
            $body
        } else if let Ok(v) = $obj.extract::<PyRef<'_, CPy>>() {
            let $inner = &v.0;
            $body
        } else {
            return Err(PyTypeError::new_err(MSG));
        }
    }};
}

/// Zero-copy serialize a dashu numeric type to bytes (rkyv format; architecture-specific —
/// see the module docs).
#[pyfunction]
pub fn to_bytes(obj: &Bound<'_, PyAny>) -> PyResult<Py<PyBytes>> {
    let bytes = with_dashu_value!(obj, |inner| ::rkyv::to_bytes::<::rkyv::rancor::Error>(inner));
    let bytes = match bytes {
        Ok(b) => b,
        Err(e) => return Err(pyo3::exceptions::PyValueError::new_err(e.to_string())),
    };
    Ok(PyBytes::new(obj.py(), &bytes).unbind())
}

/// Zero-copy deserialize a dashu numeric type from bytes produced by [`to_bytes`].
///
/// The first argument must be the target type *class* (`dashu.UBig`, …, `dashu.CBig`). The
/// bytes are not validated: passing anything other than the output of `to_bytes` for the same
/// type on the same architecture is undefined behavior.
#[pyfunction]
pub fn from_bytes<'py>(
    py: Python<'py>,
    cls: &Bound<'py, PyAny>,
    data: &[u8],
) -> PyResult<Py<PyAny>> {
    use pyo3::IntoPyObjectExt;
    if cls.is(UPy::type_object(py)) {
        // SAFETY: `data` must come from `to_bytes` on the same type (documented above).
        let v = unsafe {
            ::rkyv::from_bytes_unchecked::<dashu::integer::UBig, ::rkyv::rancor::Error>(data)
        }
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        UPy(v).into_py_any(py)
    } else if cls.is(IPy::type_object(py)) {
        // SAFETY: `data` must come from `to_bytes` on the same type (documented above).
        let v = unsafe {
            ::rkyv::from_bytes_unchecked::<dashu::integer::IBig, ::rkyv::rancor::Error>(data)
        }
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        IPy(v).into_py_any(py)
    } else if cls.is(FPy::type_object(py)) {
        // SAFETY: `data` must come from `to_bytes` on the same type (documented above).
        let v = unsafe {
            ::rkyv::from_bytes_unchecked::<
                dashu::float::FBig<dashu::float::round::mode::Zero, 2>,
                ::rkyv::rancor::Error,
            >(data)
        }
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        FPy(v).into_py_any(py)
    } else if cls.is(DPy::type_object(py)) {
        // SAFETY: `data` must come from `to_bytes` on the same type (documented above).
        let v = unsafe {
            ::rkyv::from_bytes_unchecked::<dashu::float::DBig, ::rkyv::rancor::Error>(data)
        }
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        DPy(v).into_py_any(py)
    } else if cls.is(RPy::type_object(py)) {
        // SAFETY: `data` must come from `to_bytes` on the same type (documented above).
        let v = unsafe {
            ::rkyv::from_bytes_unchecked::<dashu::rational::RBig, ::rkyv::rancor::Error>(data)
        }
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        RPy(v).into_py_any(py)
    } else if cls.is(CPy::type_object(py)) {
        // SAFETY: `data` must come from `to_bytes` on the same type (documented above).
        let v = unsafe {
            ::rkyv::from_bytes_unchecked::<
                dashu::complex::CBig<dashu::float::round::mode::Zero, 2>,
                ::rkyv::rancor::Error,
            >(data)
        }
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        CPy(v).into_py_any(py)
    } else {
        Err(PyTypeError::new_err("unexpected target type for deserialization"))
    }
}

/// The `dashu.rkyv` submodule.
#[pymodule]
pub fn rkyv(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(to_bytes, m)?)?;
    m.add_function(wrap_pyfunction!(from_bytes, m)?)?;
    Ok(())
}
