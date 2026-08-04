use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{
    convert::{conversion_error_to_py, parse_error_to_py},
    types::*,
};
use dashu_base::{ConversionError, Signed, UnsignedAbs};
use dashu_float::{DBig, FBig};
use dashu_int::{IBig, UBig};
use dashu_ratio::RBig;
use pyo3::{IntoPyObjectExt, Py, PyAny, PyResult, exceptions::PyValueError, prelude::*};

/// Default precision (in bits) for `FBig`/`CBig` constructed from `float`/`complex`.
/// The initial value matches `f64`'s native precision; configure it with [`set_precision`].
static DEFAULT_PRECISION: AtomicUsize = AtomicUsize::new(f64::MANTISSA_DIGITS as usize);

/// The current default precision (bits) used when constructing `FBig`/`CBig` from
/// `float`/`complex` inputs.
pub fn default_precision() -> usize {
    DEFAULT_PRECISION.load(Ordering::Relaxed)
}

/// Build a binary `FBig` from an `f64` at the current default precision.
pub fn fbig_from_f64(f: f64) -> Result<FBig, ConversionError> {
    FBig::try_from(f).map(|x| x.with_precision(default_precision()).value())
}

/// Get the default precision (in bits) used when constructing `FBig`/`CBig` from
/// `float`/`complex` inputs.
#[pyfunction]
pub fn get_precision() -> usize {
    default_precision()
}

/// Set the default precision (in bits) used when constructing `FBig`/`CBig` from
/// `float`/`complex` inputs. Returns the previous value. Affects only construction from
/// `float`/`complex` (and arithmetic mixing them in); integer/string/`Decimal` inputs keep
/// their natural precision — call `.with_precision()` to override per value.
#[pyfunction]
pub fn set_precision(precision: usize) -> PyResult<usize> {
    if precision == 0 {
        return Err(PyValueError::new_err("precision must be a positive integer"));
    }
    Ok(DEFAULT_PRECISION.swap(precision, Ordering::Relaxed))
}

/// Number of *extra* Ziv attempts beyond the first in the most recent transcendental call
/// (0 = first-attempt success). A profiling aid: measure how tight each function's error-radius
/// bound is at a given target precision.
#[pyfunction]
pub fn ziv_retries() -> usize {
    dashu_float::ziv_retries()
}

/// Reset the Ziv retry counter to 0 before a measurement.
#[pyfunction]
pub fn ziv_retries_reset() {
    dashu_float::ziv_retries_reset();
}

/// Convert input automatically to corresponding dashu type:
/// (int -> UBig/IBig, float -> FBig, decimal -> DBig, fraction -> RBig)
#[pyfunction]
pub fn auto(ob: UniInput, py: Python<'_>) -> PyResult<Py<PyAny>> {
    use UniInput::*;

    // shrink IBig to UBig if necessary
    let fit_ibig = |i: IBig| -> PyResult<Py<PyAny>> {
        if i.is_negative() {
            IPy(i).into_py_any(py)
        } else {
            UPy(i.unsigned_abs()).into_py_any(py)
        }
    };

    // TODO: shrink each type to the minimal representation (FBig/RBig -> IBig -> UBig)
    let obj = match ob {
        Uint(v) => UPy(v.into()).into_py_any(py)?,
        Int(v) => fit_ibig(v.into())?,
        BUint(v) => v.clone().into_py_any(py)?,
        BInt(v) => fit_ibig(v.0.clone())?,
        OBInt(v) => fit_ibig(v)?,
        Float(v) => match v.try_into() {
            Ok(big) => FPy(big).into_py_any(py)?,
            Err(e) => return Err(conversion_error_to_py(e)),
        },
        BFloat(v) => v.clone().into_py_any(py)?,
        BDecimal(v) => v.clone().into_py_any(py)?,
        OBDecimal(v) => DPy(v).into_py_any(py)?,
        BRational(v) => v.clone().into_py_any(py)?,
        OBRational(v) => RPy(v).into_py_any(py)?,
    };
    Ok(obj)
}

/// Convert input string to corresponding dashu type.
/// The type is heuristically determined
#[pyfunction]
pub fn autos(s: &str, py: Python<'_>) -> PyResult<Py<PyAny>> {
    let obj = if s.contains('/') {
        RPy(RBig::from_str_with_radix_prefix(s)
            .map_err(parse_error_to_py)?
            .0)
        .into_py_any(py)?
    } else if s.contains(['p', 'P']) {
        FPy(FBig::from_str(s).map_err(parse_error_to_py)?).into_py_any(py)?
    } else if s.contains('.') || (!s.contains("0x") && s.contains(['e', 'E'])) {
        DPy(DBig::from_str(s).map_err(parse_error_to_py)?).into_py_any(py)?
    } else if s.contains('-') {
        IPy(IBig::from_str_with_radix_prefix(s)
            .map_err(parse_error_to_py)?
            .0)
        .into_py_any(py)?
    } else {
        UPy(UBig::from_str_with_radix_prefix(s)
            .map_err(parse_error_to_py)?
            .0)
        .into_py_any(py)?
    };
    Ok(obj)
}

// TODO: split_dword, double_word, etc.
