//! `dashu.serde` — serde (de)serialization of the dashu numeric types.
//!
//! Two formats:
//! * **JSON** ([`to_json`] / [`from_json`]) — human-readable; the `dashu` types serialize to
//!   their string forms (via `serde_json`).
//! * **Binary** ([`serialize`] / [`deserialize`]) — compact, via `postcard`.
//!
//! The target type for deserialization is passed as the first argument (the class object,
//! e.g. `dashu.UBig` or `dashu.FBig`); serialization infers the type from the value itself.

use pyo3::PyTypeInfo;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyBytes;

use crate::types::{CPy, DPy, FPy, IPy, RPy, UPy};

/// Dispatch a serialization body over every dashu value type. `$obj` is the Python object,
/// `$inner` binds to a `&<inner Rust value>` in each branch, `$body` produces the result.
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

fn serde_json_error_to_py(e: serde_json::Error) -> PyErr {
    PyValueError::new_err(e.to_string())
}

/// Serialize a dashu numeric type to a JSON string.
#[pyfunction]
pub fn to_json(obj: &Bound<'_, PyAny>) -> PyResult<String> {
    with_dashu_value!(obj, |inner| serde_json::to_string(inner).map_err(serde_json_error_to_py))
}

/// Deserialize a dashu numeric type from a JSON string.
///
/// The first argument must be the target type *class* (`dashu.UBig`, `dashu.IBig`,
/// `dashu.FBig`, `dashu.DBig`, `dashu.RBig`, or `dashu.CBig`).
#[pyfunction]
pub fn from_json<'py>(py: Python<'py>, cls: &Bound<'py, PyAny>, s: &str) -> PyResult<Py<PyAny>> {
    use pyo3::IntoPyObjectExt;
    if cls.is(UPy::type_object(py)) {
        let v = serde_json::from_str::<dashu::integer::UBig>(s).map_err(serde_json_error_to_py)?;
        UPy(v).into_py_any(py)
    } else if cls.is(IPy::type_object(py)) {
        let v = serde_json::from_str::<dashu::integer::IBig>(s).map_err(serde_json_error_to_py)?;
        IPy(v).into_py_any(py)
    } else if cls.is(FPy::type_object(py)) {
        let v = serde_json::from_str::<dashu::float::FBig<dashu::float::round::mode::Zero, 2>>(s)
            .map_err(serde_json_error_to_py)?;
        FPy(v).into_py_any(py)
    } else if cls.is(DPy::type_object(py)) {
        let v = serde_json::from_str::<dashu::float::DBig>(s).map_err(serde_json_error_to_py)?;
        DPy(v).into_py_any(py)
    } else if cls.is(RPy::type_object(py)) {
        let v = serde_json::from_str::<dashu::rational::RBig>(s).map_err(serde_json_error_to_py)?;
        RPy(v).into_py_any(py)
    } else if cls.is(CPy::type_object(py)) {
        let v = serde_json::from_str::<dashu::complex::CBig<dashu::float::round::mode::Zero, 2>>(s)
            .map_err(serde_json_error_to_py)?;
        CPy(v).into_py_any(py)
    } else {
        Err(PyTypeError::new_err("unexpected target type for deserialization"))
    }
}

fn postcard_error_to_py(e: postcard::Error) -> PyErr {
    PyValueError::new_err(e.to_string())
}

/// Serialize a dashu numeric type to compact binary bytes (postcard format).
#[pyfunction]
pub fn serialize(obj: &Bound<'_, PyAny>) -> PyResult<Py<PyBytes>> {
    let bytes =
        with_dashu_value!(obj, |inner| postcard::to_allocvec(inner).map_err(postcard_error_to_py))?;
    Ok(PyBytes::new(obj.py(), &bytes).unbind())
}

/// Deserialize a dashu numeric type from compact binary bytes (postcard format).
///
/// The first argument must be the target type *class* (see [`from_json`]).
#[pyfunction]
pub fn deserialize<'py>(
    py: Python<'py>,
    cls: &Bound<'py, PyAny>,
    data: &[u8],
) -> PyResult<Py<PyAny>> {
    use pyo3::IntoPyObjectExt;
    if cls.is(UPy::type_object(py)) {
        let v = postcard::from_bytes::<dashu::integer::UBig>(data).map_err(postcard_error_to_py)?;
        UPy(v).into_py_any(py)
    } else if cls.is(IPy::type_object(py)) {
        let v = postcard::from_bytes::<dashu::integer::IBig>(data).map_err(postcard_error_to_py)?;
        IPy(v).into_py_any(py)
    } else if cls.is(FPy::type_object(py)) {
        let v =
            postcard::from_bytes::<dashu::float::FBig<dashu::float::round::mode::Zero, 2>>(data)
                .map_err(postcard_error_to_py)?;
        FPy(v).into_py_any(py)
    } else if cls.is(DPy::type_object(py)) {
        let v = postcard::from_bytes::<dashu::float::DBig>(data).map_err(postcard_error_to_py)?;
        DPy(v).into_py_any(py)
    } else if cls.is(RPy::type_object(py)) {
        let v =
            postcard::from_bytes::<dashu::rational::RBig>(data).map_err(postcard_error_to_py)?;
        RPy(v).into_py_any(py)
    } else if cls.is(CPy::type_object(py)) {
        let v =
            postcard::from_bytes::<dashu::complex::CBig<dashu::float::round::mode::Zero, 2>>(data)
                .map_err(postcard_error_to_py)?;
        CPy(v).into_py_any(py)
    } else {
        Err(PyTypeError::new_err("unexpected target type for deserialization"))
    }
}

/// The `dashu.serde` submodule.
#[pymodule]
pub fn serde(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(to_json, m)?)?;
    m.add_function(wrap_pyfunction!(from_json, m)?)?;
    m.add_function(wrap_pyfunction!(serialize, m)?)?;
    m.add_function(wrap_pyfunction!(deserialize, m)?)?;
    Ok(())
}
