use std::str::FromStr;

use crate::{
    convert::{conversion_error_to_py, parse_error_to_py},
    types::*,
};
use dashu_base::{Signed, UnsignedAbs};
use dashu_float::{DBig, FBig};
use dashu_int::{IBig, UBig};
use dashu_ratio::RBig;
use pyo3::prelude::*;

/// Convert input automatically to corresponding dashu type:
/// (int -> UBig/IBig, float -> FBig, decimal -> DBig, fraction -> RBig)
#[pyfunction]
pub fn auto(ob: UniInput, py: Python<'_>) -> PyResult<PyObject> {
    use UniInput::*;

    // shrink IBig to UBig if necessary
    let fit_ibig = |i: IBig| {
        if i.is_negative() {
            IPy(i).into_py(py)
        } else {
            UPy(i.unsigned_abs()).into_py(py)
        }
    };

    let obj = match ob {
        SmallInt(v) => fit_ibig(v.into()),
        BigUint(v) => v.clone().into_py(py),
        BigInt(v) => fit_ibig(v.0.clone()),
        BigIntOwned(v) => fit_ibig(v),
        SmallFloat(v) => match v.try_into() {
            Ok(big) => FPy(big).into_py(py),
            Err(e) => {
                return Err(conversion_error_to_py(e));
            }
        },
        BigFloat(v) => v.clone().into_py(py),
        BigDecimal(v) => v.clone().into_py(py),
        BigDecimalOwned(v) => DPy(v).into_py(py),
        BigRational(v) => v.clone().into_py(py),
        BigRationalOwned(v) => RPy(v).into_py(py),
    };
    Ok(obj)
}

/// Convert input string to corresponding dashu type.
/// The type is heuristically determined
#[pyfunction]
pub fn autos(s: &str, py: Python<'_>) -> PyResult<PyObject> {
    let obj = if s.contains('/') {
        RPy(RBig::from_str_with_radix_prefix(s)
            .map_err(parse_error_to_py)?
            .0)
        .into_py(py)
    } else if s.contains(&['p', 'P']) {
        FPy(FBig::from_str(s).map_err(parse_error_to_py)?).into_py(py)
    } else if s.contains('.') || (!s.contains("0x") && s.contains(&['e', 'E'])) {
        DPy(DBig::from_str(s).map_err(parse_error_to_py)?).into_py(py)
    } else if s.contains('-') {
        IPy(IBig::from_str_with_radix_prefix(s)
            .map_err(parse_error_to_py)?
            .0)
        .into_py(py)
    } else {
        UPy(UBig::from_str_with_radix_prefix(s)
            .map_err(parse_error_to_py)?
            .0)
        .into_py(py)
    };
    Ok(obj)
}

// TODO: split_dword, double_word, etc.
