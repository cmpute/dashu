use dashu_base::ParseError;
use pyo3::exceptions::PySyntaxError;
use pyo3::ffi::PyObject;
use pyo3::{PyAny, PyErr};

pub fn parse_signed_index(index: isize, length: usize, unlimited: bool) -> Option<usize> {
    if index >= 0 {
        let i = index as usize;
        if unlimited || i <= length {
            Some(i)
        } else {
            None
        }
    } else {
        let i = index.unsigned_abs();
        if i <= length {
            Some(length - i)
        } else {
            None
        }
    }
}

pub fn parse_error_to_py(error: ParseError) -> PyErr {
    let expl = match error {
        ParseError::NoDigits => "no valid digits in the string.",
        ParseError::InvalidDigit => "invalid digit for the given radix.",
        ParseError::UnsupportedRadix => "the radix is not supported.",
        ParseError::InconsistentRadix => {
            "the radices of different components of the number are different."
        }
    };

    PySyntaxError::new_err(expl).into()
}

pub fn auto(ob: PyAny) -> PyObject {
    // convert input automatically to corresponding type (int -> IBig, float -> FBig, decimal -> DBig, fraction -> RBig)
    todo!()
}

// TODO: split_dword, double_word, etc.
