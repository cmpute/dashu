use crate::{convert::conversion_error_to_py, types::*};
use pyo3::prelude::*;

/// Convert input automatically to corresponding dashu type:
/// (int -> IBig, float -> FBig, decimal -> DBig, fraction -> RBig)
#[pyfunction]
pub fn auto(ob: UniInput, py: Python<'_>) -> PyResult<PyObject> {
    use UniInput::*;
    let obj = match ob {
        SmallInt(v) => IPy(v.into()).into_py(py),
        BigUint(v) => IPy(v.0.clone().into()).into_py(py),
        BigInt(v) => v.clone().into_py(py),
        BigIntOwned(v) => IPy(v).into_py(py),
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

#[pyfunction]
pub fn autos(s: &str, py: Python<'_>) -> PyResult<PyObject> {
    // TODO: accept str input and detect the best representation
    todo!()
}

// TODO: split_dword, double_word, etc.
