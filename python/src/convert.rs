//! Methods related to type conversions.
//!
//! Ideally the implementations in the module should be moved to each `dashu-*` crates,
//! but it should happen when both PyO3 and this crate have a relatively stable API.

use pyo3::{
    exceptions::{PySyntaxError, PyTypeError, PyValueError},
    ffi, intern,
    prelude::*,
    types::{PyBytes, PyDict, PyFloat, PyLong},
    FromPyObject, PyAny, PyErr, PyObject,
};
use std::os::raw::{c_double, c_longlong};
use std::str::FromStr;

use crate::types::*;
use dashu_base::{ConversionError, ParseError};
use dashu_float::DBig;
use dashu_int::{IBig, UBig};
use dashu_ratio::RBig;

const ERRMSG_NAN_NOT_SUPPORTED: &str = "nan values are not supported by dashu types";
const ERRMSG_UNIINPUT_PARSE_FAILED: &str = "the input is an invalid number or unsupported";
const ERRMSG_INPUT_NOT_UBIG: &str = "the input is not an unsigned integer";

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

pub fn conversion_error_to_py(error: ConversionError) -> PyErr {
    let expl = match error {
        ConversionError::OutOfBounds => "the input is out of the representable range",
        ConversionError::LossOfPrecision => "precision loss happened during converison",
    };

    PyValueError::new_err(expl)
}

pub fn parse_error_to_py(error: ParseError) -> PyErr {
    let expl = match error {
        ParseError::NoDigits => "no valid digits in the string",
        ParseError::InvalidDigit => "invalid digit for the given radix",
        ParseError::UnsupportedRadix => "the radix is not supported",
        ParseError::InconsistentRadix => {
            "the radices of different components of the number are different"
        }
    };

    PySyntaxError::new_err(expl)
}

/// Conversion from python integer object to rust int, without type checking.
/// Returns the parsed number (when success) and the overflow flag.
pub fn parse_to_long(ob: &PyAny) -> PyResult<(c_longlong, bool)> {
    let py = ob.py();

    unsafe {
        let ptr = ob.as_ptr();
        let mut overflow: i32 = 0;
        let v = ffi::PyLong_AsLongLongAndOverflow(ptr, &mut overflow);

        if v == -1 && PyErr::occurred(py) {
            Err(PyErr::fetch(py))
        } else {
            Ok((v, overflow != 0))
        }
    }
}

/// Conversion from python integer object to UBig instance, without type checking.
pub fn parse_to_ubig(ob: &PyAny) -> PyResult<UBig> {
    let py = ob.py();
    let bit_len: usize = ob.call_method0(intern!(py, "bit_length"))?.extract()?;
    let byte_len = (bit_len + 7) / 8;

    // The most efficient way here is to use ffi::_PyLong_AsByteArray.
    // However, the conversion should not performed frequently, so the stable
    // API `to_bytes` is preferred here.
    let bytes: &PyBytes = ob
        .call_method1(intern!(py, "to_bytes"), (byte_len, intern!(py, "little")))?
        .downcast()?;
    Ok(UBig::from_le_bytes(bytes.as_bytes()))
}

/// Conversion from UBig instance to python integer object
pub fn convert_from_ubig(ob: &UBig, py: Python) -> PyResult<PyObject> {
    let bytes = ob.to_le_bytes();
    let bytes_obj = PyBytes::new(py, &bytes);
    py.get_type::<PyLong>()
        .call_method1(intern!(py, "from_bytes"), (bytes_obj, intern!(py, "little")))
        .map(PyObject::from)
}

/// Conversion from python integer object to IBig instance, without type checking.
pub fn parse_to_ibig(ob: &PyAny) -> PyResult<IBig> {
    let py = ob.py();
    let bit_len: usize = ob.call_method0(intern!(py, "bit_length"))?.extract()?;
    let byte_len = bit_len / 8 + 1; // extra byte for sign

    // The stable API `to_bytes` is also chosen over ffi::_PyLong_AsByteArray here.
    let kwargs = PyDict::new(py);
    kwargs.set_item(intern!(py, "signed"), true).unwrap();
    let bytes: &PyBytes = ob
        .call_method(intern!(py, "to_bytes"), (byte_len, intern!(py, "little")), Some(kwargs))?
        .downcast()?;
    Ok(IBig::from_le_bytes(bytes.as_bytes()))
}

/// Conversion from IBig instance to python integer object
pub fn convert_from_ibig(ob: &IBig, py: Python) -> PyResult<PyObject> {
    let bytes = ob.to_le_bytes();
    let bytes_obj = PyBytes::new(py, &bytes);

    let kwargs = PyDict::new(py);
    kwargs.set_item(intern!(py, "signed"), true).unwrap();
    py.get_type::<PyLong>()
        .call_method(intern!(py, "from_bytes"), (bytes_obj, intern!(py, "little")), Some(kwargs))
        .map(PyObject::from)
}

/// Conversion from decimal.Decimal object to DBig instance, without type checking.
pub fn parse_to_dbig(ob: &PyAny) -> PyResult<DBig> {
    // use string to convert Decimal to DBig is okay, because Decimal.__format__ will
    // produce string in scientific notation. It will not produce many zeros when the
    // exponent is large.
    let s = ob.str()?;
    DBig::from_str(s.to_str()?).map_err(parse_error_to_py)
}

/// Conversion from fractions.Fraction object to RBig instance, without type checking.
pub fn parse_to_rbig(ob: &PyAny) -> PyResult<RBig> {
    let py = ob.py();
    let num = parse_to_ibig(ob.getattr(intern!(py, "numerator"))?)?;
    let den = parse_to_ibig(ob.getattr(intern!(py, "denominator"))?)?;
    let den: UBig = den.try_into().unwrap(); // this should be ensured by the Fraction type.
    Ok(RBig::from_parts(num, den))
}

/// Conversion from RBig instance to fractions.Fraction object
pub fn convert_from_rbig(ob: &RBig, py: Python<'_>) -> PyResult<PyObject> {
    let fractions = py.import(intern!(py, "fractions"))?;
    let fraction_type = fractions.getattr(intern!(py, "Fraction"))?;

    let num = convert_from_ibig(ob.numerator(), py)?;
    let den = convert_from_ubig(ob.denominator(), py)?;
    fraction_type.call1((num, den)).map(PyObject::from)
}

impl<'source> FromPyObject<'source> for UniInput<'source> {
    fn extract(ob: &'source PyAny) -> PyResult<Self> {
        if ob.is_instance_of::<PyLong>() {
            let (v, overflow) = parse_to_long(ob)?;
            if overflow {
                Ok(Self::OBInt(parse_to_ibig(ob)?))
            } else if v < 0 {
                Ok(Self::Int(v))
            } else {
                Ok(Self::Uint(v as _))
            }
        } else if ob.is_instance_of::<PyFloat>() {
            let f: c_double = ob.extract()?;
            if f.is_nan() {
                Err(PyValueError::new_err(ERRMSG_NAN_NOT_SUPPORTED))
            } else {
                Ok(Self::Float(f))
            }
        } else if let Ok(u) = <PyRef<'source, UPy> as FromPyObject>::extract(ob) {
            Ok(Self::BUint(u))
        } else if let Ok(i) = <PyRef<'source, IPy> as FromPyObject>::extract(ob) {
            Ok(Self::BInt(i))
        } else if let Ok(f) = <PyRef<'source, FPy> as FromPyObject>::extract(ob) {
            Ok(Self::BFloat(f))
        } else if let Ok(d) = <PyRef<'source, DPy> as FromPyObject>::extract(ob) {
            Ok(Self::BDecimal(d))
        } else if let Ok(r) = <PyRef<'source, RPy> as FromPyObject>::extract(ob) {
            Ok(Self::BRational(r))
        } else {
            // slow path:
            // get relevant Python types
            let py = ob.py();
            let decimal = py.import(intern!(py, "decimal"))?;
            let decimal_type = decimal.getattr(intern!(py, "Decimal"))?;
            let fractions = py.import(intern!(py, "fractions"))?;
            let fraction_type = fractions.getattr(intern!(py, "Fraction"))?;

            // and check whether the input is an instance of them
            if ob.is_instance(decimal_type)? {
                Ok(Self::OBDecimal(parse_to_dbig(ob)?))
            } else if ob.is_instance(fraction_type)? {
                Ok(Self::OBRational(parse_to_rbig(ob)?))
            } else {
                Err(PyTypeError::new_err(ERRMSG_UNIINPUT_PARSE_FAILED))
            }
        }
    }
}

impl<'a> UniInput<'a> {
    pub fn to_ubig(self) -> PyResult<UBig> {
        let err = PyTypeError::new_err(ERRMSG_INPUT_NOT_UBIG);
        match self {
            Self::Uint(x) => Ok(x.into()),
            Self::BUint(x) => Ok(x.0.clone()),
            Self::OBInt(x) => x.try_into().map_err(|_| err),
            Self::BInt(x) => if let Some(u) = x.0.as_ubig() {
                Ok(u.clone())
            } else {
                Err(err)
            }
            _ => Err(err)
        }
    }
}