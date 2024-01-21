use dashu_base::ParseError;
use pyo3::PyErr;
use pyo3::exceptions::PySyntaxError;

pub fn parse_error_to_py(error: ParseError) -> PyErr {
    let expl = match error {
        ParseError::NoDigits => "no valid digits in the string.",
        ParseError::InvalidDigit => "invalid digit for the given radix.",
        ParseError::UnsupportedRadix => "the radix is not supported.",
        ParseError::InconsistentRadix => "the radices of different components of the number are different.",
    };

    PySyntaxError::new_err(expl).into()
}

// TODO: split_dword, double_word, etc.
