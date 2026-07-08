//! Methods related to type conversions.
//!
//! Ideally the implementations in the module should be moved to each `dashu-*` crates,
//! but it should happen when both PyO3 and this crate have a relatively stable API.

use pyo3::{
    Bound, FromPyObject, PyErr, PyResult,
    exceptions::{PySyntaxError, PyTypeError, PyValueError},
    ffi, intern,
    prelude::*,
    types::{PyBytes, PyDict, PyFloat, PyInt},
};
use std::os::raw::{c_double, c_longlong};
use std::str::FromStr;

use crate::types::*;
use dashu_base::{ConversionError, ParseError};
use dashu_float::{DBig, FBig};
use dashu_int::{IBig, UBig};
use dashu_ratio::RBig;

const ERRMSG_NAN_NOT_SUPPORTED: &str = "nan values are not supported by dashu types";
const ERRMSG_UNIINPUT_PARSE_FAILED: &str = "the input is an invalid number or unsupported";
const ERRMSG_INPUT_NOT_UBIG: &str = "the input is not an unsigned integer";
const ERRMSG_DECIMAL_WITH_BINARY: &str = "decimal values cannot be mixed with binary floats; convert explicitly with to_binary()/to_decimal()";
const ERRMSG_BINARY_WITH_DECIMAL: &str =
    "binary floats cannot be mixed with decimals; convert explicitly with to_binary()/to_decimal()";

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
        if i <= length { Some(length - i) } else { None }
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
pub fn parse_to_long(ob: &Bound<'_, PyAny>) -> PyResult<(c_longlong, bool)> {
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
pub fn parse_to_ubig(ob: &Bound<'_, PyAny>) -> PyResult<UBig> {
    let py = ob.py();
    let bit_len: usize = ob.call_method0(intern!(py, "bit_length"))?.extract()?;
    let byte_len = bit_len.div_ceil(8);

    // The most efficient way here is to use ffi::_PyLong_AsByteArray.
    // However, the conversion should not performed frequently, so the stable
    // API `to_bytes` is preferred here.
    let bytes_obj = ob.call_method1(intern!(py, "to_bytes"), (byte_len, intern!(py, "little")))?;
    let bytes = bytes_obj.cast::<PyBytes>()?;
    Ok(UBig::from_le_bytes(bytes.as_bytes()))
}

/// Conversion from UBig instance to python integer object
pub fn convert_from_ubig<'py>(ob: &UBig, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
    let bytes = ob.to_le_bytes();
    let bytes_obj = PyBytes::new(py, &bytes);
    py.get_type::<PyInt>()
        .call_method1(intern!(py, "from_bytes"), (bytes_obj, intern!(py, "little")))
}

/// Conversion from python integer object to IBig instance, without type checking.
pub fn parse_to_ibig(ob: &Bound<'_, PyAny>) -> PyResult<IBig> {
    let py = ob.py();
    let bit_len: usize = ob.call_method0(intern!(py, "bit_length"))?.extract()?;
    let byte_len = bit_len / 8 + 1; // extra byte for sign

    // The stable API `to_bytes` is also chosen over ffi::_PyLong_AsByteArray here.
    let kwargs = PyDict::new(py);
    kwargs.set_item(intern!(py, "signed"), true).unwrap();
    let bytes_obj =
        ob.call_method(intern!(py, "to_bytes"), (byte_len, intern!(py, "little")), Some(&kwargs))?;
    let bytes = bytes_obj.cast::<PyBytes>()?;
    Ok(IBig::from_le_bytes(bytes.as_bytes()))
}

/// Conversion from IBig instance to python integer object
pub fn convert_from_ibig<'py>(ob: &IBig, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
    let bytes = ob.to_le_bytes();
    let bytes_obj = PyBytes::new(py, &bytes);

    let kwargs = PyDict::new(py);
    kwargs.set_item(intern!(py, "signed"), true).unwrap();
    py.get_type::<PyInt>().call_method(
        intern!(py, "from_bytes"),
        (bytes_obj, intern!(py, "little")),
        Some(&kwargs),
    )
}

/// Conversion from decimal.Decimal object to DBig instance, without type checking.
pub fn parse_to_dbig(ob: &Bound<'_, PyAny>) -> PyResult<DBig> {
    // use string to convert Decimal to DBig is okay, because Decimal.__format__ will
    // produce string in scientific notation. It will not produce many zeros when the
    // exponent is large.
    let s = ob.str()?;
    DBig::from_str(s.to_str()?).map_err(parse_error_to_py)
}

/// Conversion from fractions.Fraction object to RBig instance, without type checking.
pub fn parse_to_rbig(ob: &Bound<'_, PyAny>) -> PyResult<RBig> {
    let py = ob.py();
    let num = parse_to_ibig(&ob.getattr(intern!(py, "numerator"))?)?;
    let den = parse_to_ibig(&ob.getattr(intern!(py, "denominator"))?)?;
    let den: UBig = den.try_into().unwrap(); // this should be ensured by the Fraction type.
    Ok(RBig::from_parts(num, den))
}

/// Conversion from RBig instance to fractions.Fraction object
pub fn convert_from_rbig<'py>(ob: &RBig, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
    let fractions = py.import(intern!(py, "fractions"))?;
    let fraction_type = fractions.getattr(intern!(py, "Fraction"))?;

    let num = convert_from_ibig(ob.numerator(), py)?;
    let den = convert_from_ubig(ob.denominator(), py)?;
    fraction_type.call1((num, den))
}

impl<'a, 'py> FromPyObject<'a, 'py> for UniInput<'py> {
    type Error = PyErr;
    fn extract(ob: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        if ob.is_instance_of::<PyInt>() {
            let (v, overflow) = parse_to_long(&ob)?;
            if overflow {
                Ok(Self::OBInt(parse_to_ibig(&ob)?))
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
        } else if let Ok(u) = ob.extract::<PyRef<'py, UPy>>() {
            Ok(Self::BUint(u))
        } else if let Ok(i) = ob.extract::<PyRef<'py, IPy>>() {
            Ok(Self::BInt(i))
        } else if let Ok(f) = ob.extract::<PyRef<'py, FPy>>() {
            Ok(Self::BFloat(f))
        } else if let Ok(d) = ob.extract::<PyRef<'py, DPy>>() {
            Ok(Self::BDecimal(d))
        } else if let Ok(r) = ob.extract::<PyRef<'py, RPy>>() {
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
            if ob.is_instance(&decimal_type)? {
                Ok(Self::OBDecimal(parse_to_dbig(&ob)?))
            } else if ob.is_instance(&fraction_type)? {
                Ok(Self::OBRational(parse_to_rbig(&ob)?))
            } else {
                Err(PyTypeError::new_err(ERRMSG_UNIINPUT_PARSE_FAILED))
            }
        }
    }
}

impl<'a> UniInput<'a> {
    pub fn into_ubig(self) -> PyResult<UBig> {
        let err = PyTypeError::new_err(ERRMSG_INPUT_NOT_UBIG);
        match self {
            Self::Uint(x) => Ok(x.into()),
            Self::BUint(x) => Ok(x.0.clone()),
            Self::OBInt(x) => x.try_into().map_err(|_| err),
            Self::BInt(x) => {
                if let Some(u) = x.0.as_ubig() {
                    Ok(u.clone())
                } else {
                    Err(err)
                }
            }
            _ => Err(err),
        }
    }

    /// Strict conversion of any numeric input to a binary float (`FPy`).
    /// Decimals are rejected (convert explicitly).
    pub fn into_fpy(self) -> PyResult<FPy> {
        match self {
            Self::Uint(x) => Ok(FPy(FBig::from(UBig::from(x)))),
            Self::Int(x) => Ok(FPy(FBig::from(IBig::from(x)))),
            Self::BUint(x) => Ok(FPy(FBig::from(x.0.clone()))),
            Self::BInt(x) => Ok(FPy(FBig::from(x.0.clone()))),
            Self::OBInt(x) => Ok(FPy(FBig::from(x))),
            Self::Float(x) => FBig::try_from(x).map(FPy).map_err(conversion_error_to_py),
            Self::BFloat(x) => Ok(FPy(x.0.clone())),
            Self::BRational(x) => FBig::try_from(x.0.clone())
                .map(FPy)
                .map_err(conversion_error_to_py),
            Self::OBRational(x) => FBig::try_from(x).map(FPy).map_err(conversion_error_to_py),
            Self::BDecimal(_) | Self::OBDecimal(_) => {
                Err(PyTypeError::new_err(ERRMSG_DECIMAL_WITH_BINARY))
            }
        }
    }

    /// Strict conversion of any numeric input to a decimal float (`DPy`).
    /// Binary floats are rejected (convert explicitly).
    pub fn into_dpy(self) -> PyResult<DPy> {
        match self {
            Self::Uint(x) => Ok(DPy(DBig::from(UBig::from(x)))),
            Self::Int(x) => Ok(DPy(DBig::from(IBig::from(x)))),
            Self::BUint(x) => Ok(DPy(DBig::from(x.0.clone()))),
            Self::BInt(x) => Ok(DPy(DBig::from(x.0.clone()))),
            Self::OBInt(x) => Ok(DPy(DBig::from(x))),
            // base-10 floats have no direct TryFrom<f64>; round-trip through the
            // shortest scientific-notation string that f64 produces.
            Self::Float(x) => {
                let s = format!("{:e}", x);
                DBig::from_str(&s).map(DPy).map_err(parse_error_to_py)
            }
            Self::BDecimal(x) => Ok(DPy(x.0.clone())),
            Self::OBDecimal(x) => Ok(DPy(x)),
            Self::BRational(x) => DBig::try_from(x.0.clone())
                .map(DPy)
                .map_err(conversion_error_to_py),
            Self::OBRational(x) => DBig::try_from(x).map(DPy).map_err(conversion_error_to_py),
            Self::BFloat(_) => Err(PyTypeError::new_err(ERRMSG_BINARY_WITH_DECIMAL)),
        }
    }

    /// Strict conversion of any numeric input to a rational (`RPy`). Exact-only for
    /// floats and big floats.
    pub fn into_rpy(self) -> PyResult<RPy> {
        match self {
            Self::Uint(x) => Ok(RPy(RBig::from(x))),
            Self::Int(x) => Ok(RPy(RBig::from(x))),
            Self::BUint(x) => Ok(RPy(RBig::from(x.0.clone()))),
            Self::BInt(x) => Ok(RPy(RBig::from(x.0.clone()))),
            Self::OBInt(x) => Ok(RPy(RBig::from(x))),
            Self::Float(x) => RBig::try_from(x).map(RPy).map_err(conversion_error_to_py),
            Self::BFloat(x) => RBig::try_from(x.0.clone())
                .map(RPy)
                .map_err(conversion_error_to_py),
            Self::BDecimal(x) => RBig::try_from(x.0.clone())
                .map(RPy)
                .map_err(conversion_error_to_py),
            Self::OBDecimal(x) => RBig::try_from(x).map(RPy).map_err(conversion_error_to_py),
            Self::BRational(x) => Ok(RPy(x.0.clone())),
            Self::OBRational(x) => Ok(RPy(x)),
        }
    }

    /// Permissive construction of a binary float from any Python number (used by
    /// `FBig.__new__`). Unlike [`UniInput::into_fpy`], this ACCEPTS `Decimal`/`Fraction`
    /// by routing them through the correctly-rounded base conversion / exact rational path.
    pub fn construct_fpy(self) -> PyResult<FBig> {
        match self {
            Self::Uint(x) => Ok(FBig::from(UBig::from(x))),
            Self::Int(x) => Ok(FBig::from(IBig::from(x))),
            Self::BUint(x) => Ok(FBig::from(x.0.clone())),
            Self::BInt(x) => Ok(FBig::from(x.0.clone())),
            Self::OBInt(x) => Ok(FBig::from(x)),
            Self::Float(x) => FBig::try_from(x)
                .map(|f| f.with_precision(f64::MANTISSA_DIGITS as usize).value())
                .map_err(conversion_error_to_py),
            Self::BFloat(x) => Ok(x.0.clone()),
            // Decimal -> base-2 via the correctly-rounded to_binary conversion
            Self::BDecimal(x) => Ok(x.0.to_binary().value()),
            Self::OBDecimal(x) => Ok(x.to_binary().value()),
            Self::BRational(x) => FBig::try_from(x.0.clone()).map_err(conversion_error_to_py),
            Self::OBRational(x) => FBig::try_from(x).map_err(conversion_error_to_py),
        }
    }

    /// Permissive construction of a decimal float from any Python number (used by
    /// `DBig.__new__`). Binary floats are routed through the correctly-rounded
    /// `to_decimal` conversion rather than being rejected.
    pub fn construct_dpy(self) -> PyResult<DBig> {
        match self {
            Self::Uint(x) => Ok(DBig::from(UBig::from(x))),
            Self::Int(x) => Ok(DBig::from(IBig::from(x))),
            Self::BUint(x) => Ok(DBig::from(x.0.clone())),
            Self::BInt(x) => Ok(DBig::from(x.0.clone())),
            Self::OBInt(x) => Ok(DBig::from(x)),
            Self::Float(x) => {
                let s = format!("{:e}", x);
                DBig::from_str(&s).map_err(parse_error_to_py)
            }
            Self::BFloat(x) => Ok(x.0.to_decimal().value()),
            Self::BDecimal(x) => Ok(x.0.clone()),
            Self::OBDecimal(x) => Ok(x),
            Self::BRational(x) => DBig::try_from(x.0.clone()).map_err(conversion_error_to_py),
            Self::OBRational(x) => DBig::try_from(x).map_err(conversion_error_to_py),
        }
    }

    /// Permissive construction of a rational from any Python number (used by
    /// `RBig.__new__`). Same rules as [`UniInput::into_rpy`].
    pub fn construct_rpy(self) -> PyResult<RBig> {
        Ok(self.into_rpy()?.0)
    }
}
