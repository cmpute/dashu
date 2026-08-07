use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::ops::*;
use std::vec::Vec;

use pyo3::basic::CompareOp;
use pyo3::exceptions::{
    PyIndexError, PyNotImplementedError, PyOverflowError, PyTypeError, PyValueError,
};
use pyo3::{
    Bound, IntoPyObjectExt, Py, PyAny, PyResult,
    prelude::*,
    types::{PyBytes, PyInt, PyIterator, PyList, PySlice, PyTuple},
};

use crate::{
    convert::{
        conversion_error_to_py, convert_from_ibig, convert_from_ubig, parse_error_to_py,
        parse_signed_index, parse_to_ibig, parse_to_long, parse_to_ubig,
    },
    types::{DPy, FPy, IPy, PySign, PyWords, RPy, UPy, UniInput},
};

use dashu::base::{
    Abs, BitTest, CubicRoot, PowerOfTwo, Sign, Signed, SquareRoot, UnsignedAbs,
    ring::{DivRemEuclid, ExtendedGcd, Gcd},
};
use dashu::float::FBig;
use dashu::integer::{IBig, UBig, Word, fast_div};
use num_order::{NumHash, NumOrd};

// error messages
const ERRMSG_LENGTH_TOO_LARGE: &str = "the integer has too many bits for indexing";
const ERRMSG_STEPSIZE_TOO_LARGE: &str =
    "bit slicing with step size larger than 1 is not supported yet";
const ERRMSG_UBIG_WRONG_SRC_TYPE: &str =
    "only integers or strings can be used to construct a UBig instance";
const ERRMSG_IBIG_WRONG_SRC_TYPE: &str =
    "only integers or strings can be used to construct an IBig instance";
const ERRMSG_FROM_WORDS_WRONG_TYPE: &str =
    "only list of integers or Words instance can be used in UBig.from_words()";
const ERRMSG_WRONG_ENDIANNESS: &str = "byteorder must be either 'little' or 'big'";
const ERRMSG_NEGATIVE_TO_UNSIGNED: &str = "can't convert negative int to unsigned";
const ERRMSG_INT_WITH_RADIX: &str = "can't convert non-string with explicit base";
const ERRMSG_WRONG_INDEX_TYPE: &str = "indices must be integers or slices";
const ERRMSG_UBIG_BITS_OOR: &str = "bits index out of range";
const ERRMSG_BITOPS_TYPE: &str = "bit operations are only defined between integers";
const ERRMSG_WRONG_CHUNKS_TYPE: &str = "the chunks input is not a recognized iterable";
const ERRMSG_ZERO_CHUNK_SIZE: &str = "the chunk size must not be zero";
const ERRMSG_FLOORDIV_INT_ONLY: &str = "floor division requires an integer divisor";
const ERRMSG_MODULUS_INT_ONLY: &str = "modulus must be an integer";
const ERRMSG_POW_INT_EXP: &str = "the exponent must be a non-negative integer";
const ERRMSG_POW_MOD_INT_EXP: &str =
    "modular exponentiation requires a non-negative integer exponent";

macro_rules! impl_binops {
    ($ty_variant:ident, $py_method:ident, $rs_method:ident) => {
        fn $py_method(lhs: &$ty_variant, rhs: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
            use pyo3::IntoPyObjectExt;
            let obj = match rhs {
                UniInput::Uint(x) => $ty_variant((&lhs.0).$rs_method(x)).into_py_any(py)?,
                UniInput::Int(x) => IPy((&lhs.0).$rs_method(IBig::from(x))).into_py_any(py)?,
                UniInput::BUint(x) => $ty_variant((&lhs.0).$rs_method(&x.0)).into_py_any(py)?,
                UniInput::BInt(x) => IPy((&lhs.0).$rs_method(&x.0)).into_py_any(py)?,
                UniInput::OBInt(x) => IPy((&lhs.0).$rs_method(x)).into_py_any(py)?,
                UniInput::Float(x) => {
                    let f = crate::utils::fbig_from_f64(x).map_err(conversion_error_to_py)?;
                    FPy((&lhs.0).$rs_method(f)).into_py_any(py)?
                }
                UniInput::BFloat(x) => FPy((&lhs.0).$rs_method(&x.0)).into_py_any(py)?,
                UniInput::BDecimal(x) => DPy((&lhs.0).$rs_method(&x.0)).into_py_any(py)?,
                UniInput::OBDecimal(x) => DPy((&lhs.0).$rs_method(x)).into_py_any(py)?,
                UniInput::BRational(x) => RPy((&lhs.0).$rs_method(&x.0)).into_py_any(py)?,
                UniInput::OBRational(x) => RPy((&lhs.0).$rs_method(x)).into_py_any(py)?,
            };
            Ok(obj)
        }
    };
    ($ty_variant:ident, $py_method:ident, $py_method_rev:ident, $rs_method:ident) => {
        impl_binops!($ty_variant, $py_method, $rs_method);

        fn $py_method_rev(
            lhs: UniInput<'_>,
            rhs: &$ty_variant,
            py: Python<'_>,
        ) -> PyResult<Py<PyAny>> {
            use pyo3::IntoPyObjectExt;
            let obj = match lhs {
                UniInput::Uint(x) => $ty_variant(x.$rs_method(&rhs.0).into()).into_py_any(py)?,
                UniInput::Int(x) => IPy(IBig::from(x).$rs_method(&rhs.0).into()).into_py_any(py)?,
                UniInput::BUint(x) => $ty_variant((&x.0).$rs_method(&rhs.0)).into_py_any(py)?,
                UniInput::BInt(x) => IPy((&x.0).$rs_method(&rhs.0)).into_py_any(py)?,
                UniInput::OBInt(x) => IPy(x.$rs_method(&rhs.0)).into_py_any(py)?,
                UniInput::Float(x) => {
                    let f = crate::utils::fbig_from_f64(x).map_err(conversion_error_to_py)?;
                    FPy(f.$rs_method(&rhs.0)).into_py_any(py)?
                }
                UniInput::BFloat(x) => FPy((&x.0).$rs_method(&rhs.0)).into_py_any(py)?,
                UniInput::BDecimal(x) => DPy((&x.0).$rs_method(&rhs.0)).into_py_any(py)?,
                UniInput::OBDecimal(x) => DPy(x.$rs_method(&rhs.0)).into_py_any(py)?,
                UniInput::BRational(x) => RPy((&x.0).$rs_method(&rhs.0)).into_py_any(py)?,
                UniInput::OBRational(x) => RPy(x.$rs_method(&rhs.0)).into_py_any(py)?,
            };
            Ok(obj)
        }
    };
}

impl_binops!(UPy, upy_add, add);
impl_binops!(UPy, upy_sub, upy_rsub, sub);
impl_binops!(UPy, upy_mul, mul);
impl_binops!(UPy, upy_div, upy_rdiv, div);
impl_binops!(IPy, ipy_add, add);
impl_binops!(IPy, ipy_sub, ipy_rsub, sub);
impl_binops!(IPy, ipy_mul, mul);
impl_binops!(IPy, ipy_div, ipy_rdiv, div);

fn upy_bitand(lhs: &UPy, rhs: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
    use pyo3::IntoPyObjectExt;
    let obj = match rhs {
        UniInput::Uint(x) => UPy((&lhs.0).bitand(x).into()).into_py_any(py)?,
        UniInput::BUint(x) => UPy((&lhs.0).bitand(&x.0)).into_py_any(py)?,
        UniInput::Int(x) => UPy((&lhs.0).bitand(IBig::from(x))).into_py_any(py)?,
        UniInput::BInt(x) => UPy((&lhs.0).bitand(&x.0)).into_py_any(py)?,
        UniInput::OBInt(x) => UPy((&lhs.0).bitand(x)).into_py_any(py)?,
        _ => return Err(PyTypeError::new_err(ERRMSG_BITOPS_TYPE)),
    };
    Ok(obj)
}
fn ipy_bitand(lhs: &IPy, rhs: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
    use pyo3::IntoPyObjectExt;
    let obj = match rhs {
        UniInput::Uint(x) => UPy((&lhs.0).bitand(x).into()).into_py_any(py)?,
        UniInput::BUint(x) => UPy((&lhs.0).bitand(&x.0)).into_py_any(py)?,
        UniInput::Int(x) => IPy((&lhs.0).bitand(x)).into_py_any(py)?,
        UniInput::BInt(x) => IPy((&lhs.0).bitand(&x.0)).into_py_any(py)?,
        UniInput::OBInt(x) => IPy((&lhs.0).bitand(x)).into_py_any(py)?,
        _ => return Err(PyTypeError::new_err(ERRMSG_BITOPS_TYPE)),
    };
    Ok(obj)
}

macro_rules! impl_ubig_bit_binops {
    ($ty_variant:ident, $py_method:ident, $rs_method:ident) => {
        fn $py_method(lhs: &$ty_variant, rhs: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
            use pyo3::IntoPyObjectExt;
            let obj = match rhs {
                UniInput::Uint(x) => $ty_variant((&lhs.0).$rs_method(x)).into_py_any(py)?,
                UniInput::BUint(x) => $ty_variant((&lhs.0).$rs_method(&x.0)).into_py_any(py)?,
                UniInput::Int(x) => IPy((&lhs.0).$rs_method(IBig::from(x))).into_py_any(py)?,
                UniInput::BInt(x) => IPy((&lhs.0).$rs_method(&x.0)).into_py_any(py)?,
                UniInput::OBInt(x) => IPy((&lhs.0).$rs_method(x)).into_py_any(py)?,
                _ => return Err(PyTypeError::new_err(ERRMSG_BITOPS_TYPE)),
            };
            Ok(obj)
        }
    };
}
impl_ubig_bit_binops!(UPy, upy_bitor, bitor);
impl_ubig_bit_binops!(UPy, upy_bitxor, bitxor);
impl_ubig_bit_binops!(IPy, ipy_bitor, bitor);
impl_ubig_bit_binops!(IPy, ipy_bitxor, bitxor);

/// Python floored integer division/remainder derived from Euclidean division: `q = floor(a/b)`
/// and `r = a - b*q`, so `r` carries the sign of `b`. This differs from truncating division
/// (`.rem()`, whose remainder has the sign of `a`) and from Euclidean division (`r >= 0`) whenever
/// `b < 0`, and matches CPython's `//`, `%`, and `divmod` for integers.
fn ibig_div_mod_floor(a: &IBig, b: &IBig) -> (IBig, IBig) {
    let (q, r) = a.div_rem_euclid(b);
    let r: IBig = r.into();
    if b.sign() == Sign::Negative && !r.is_zero() {
        (q - IBig::ONE, r + b)
    } else {
        (q, r)
    }
}

fn upy_mod(lhs: &UPy, rhs: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
    use pyo3::IntoPyObjectExt;
    let a = lhs.0.as_ibig();
    let obj = match rhs {
        UniInput::Uint(x) => IPy(ibig_div_mod_floor(a, &IBig::from(x)).1).into_py_any(py)?,
        UniInput::Int(x) => IPy(ibig_div_mod_floor(a, &IBig::from(x)).1).into_py_any(py)?,
        UniInput::BUint(x) => IPy(ibig_div_mod_floor(a, x.0.as_ibig()).1).into_py_any(py)?,
        UniInput::BInt(x) => IPy(ibig_div_mod_floor(a, &x.0).1).into_py_any(py)?,
        UniInput::OBInt(x) => IPy(ibig_div_mod_floor(a, &x).1).into_py_any(py)?,
        UniInput::Float(x) => {
            let f = crate::utils::fbig_from_f64(x).map_err(conversion_error_to_py)?;
            FPy(FBig::from(lhs.0.clone()).rem(f)).into_py_any(py)?
        }
        UniInput::BFloat(x) => FPy(FBig::from(lhs.0.clone()).rem(&x.0)).into_py_any(py)?,
        UniInput::BDecimal(x) => {
            DPy(dashu::float::DBig::from(lhs.0.clone()).rem(&x.0)).into_py_any(py)?
        }
        UniInput::OBDecimal(x) => {
            DPy(dashu::float::DBig::from(lhs.0.clone()).rem(x)).into_py_any(py)?
        }
        UniInput::BRational(x) => {
            RPy(dashu::rational::RBig::from(lhs.0.clone()).rem(&x.0)).into_py_any(py)?
        }
        UniInput::OBRational(x) => {
            RPy(dashu::rational::RBig::from(lhs.0.clone()).rem(x)).into_py_any(py)?
        }
    };
    Ok(obj)
}
fn ipy_mod(lhs: &IPy, rhs: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
    use pyo3::IntoPyObjectExt;
    let a = &lhs.0;
    let obj = match rhs {
        UniInput::Uint(x) => IPy(ibig_div_mod_floor(a, &IBig::from(x)).1).into_py_any(py)?,
        UniInput::Int(x) => IPy(ibig_div_mod_floor(a, &IBig::from(x)).1).into_py_any(py)?,
        UniInput::BUint(x) => IPy(ibig_div_mod_floor(a, x.0.as_ibig()).1).into_py_any(py)?,
        UniInput::BInt(x) => IPy(ibig_div_mod_floor(a, &x.0).1).into_py_any(py)?,
        UniInput::OBInt(x) => IPy(ibig_div_mod_floor(a, &x).1).into_py_any(py)?,
        UniInput::Float(x) => {
            let f = crate::utils::fbig_from_f64(x).map_err(conversion_error_to_py)?;
            FPy(FBig::from(lhs.0.clone()).rem(f)).into_py_any(py)?
        }
        UniInput::BFloat(x) => FPy(FBig::from(lhs.0.clone()).rem(&x.0)).into_py_any(py)?,
        UniInput::BDecimal(x) => {
            DPy(dashu::float::DBig::from(lhs.0.clone()).rem(&x.0)).into_py_any(py)?
        }
        UniInput::OBDecimal(x) => {
            DPy(dashu::float::DBig::from(lhs.0.clone()).rem(x)).into_py_any(py)?
        }
        UniInput::BRational(x) => {
            RPy(dashu::rational::RBig::from(lhs.0.clone()).rem(&x.0)).into_py_any(py)?
        }
        UniInput::OBRational(x) => {
            RPy(dashu::rational::RBig::from(lhs.0.clone()).rem(x)).into_py_any(py)?
        }
    };
    Ok(obj)
}

fn ipy_pow(base: &IBig, exp: UniInput, modulus: Option<UniInput>) -> PyResult<IBig> {
    use fast_div::ConstDivisor;

    if let Some(m) = modulus {
        // parse the modulus, keeping both its sign and magnitude
        let (sign, m_mag) = match m {
            UniInput::Uint(x) => (Sign::Positive, UBig::from(x)),
            UniInput::BUint(x) => (Sign::Positive, x.0.clone()),
            UniInput::Int(x) => (x.sign(), UBig::from(x.unsigned_abs())),
            UniInput::BInt(x) => (x.0.sign(), (&x.0).unsigned_abs()),
            UniInput::OBInt(x) => x.into_parts(),
            _ => return Err(PyTypeError::new_err(ERRMSG_MODULUS_INT_ONLY)),
        };
        let ring = ConstDivisor::new(m_mag.clone());

        match exp {
            UniInput::Uint(x) => {
                let (bsign, u) = base.clone().into_parts();
                let mut r = ring.reduce(u);
                if bsign == Sign::Negative {
                    r = r.neg();
                }
                let residue: IBig = r.pow(&x.into()).residue().into();
                // CPython: the result carries the sign of the modulus (like `%`), so a negative
                // modulus shifts the non-negative residue into `(modulus, 0]`.
                let result = if sign == Sign::Negative && !residue.is_zero() {
                    residue - IBig::from(m_mag)
                } else {
                    residue
                };
                Ok(result)
            }
            _ => Err(PyTypeError::new_err(ERRMSG_POW_MOD_INT_EXP)),
        }
    } else {
        match exp {
            UniInput::Uint(x) => Ok(base.pow(x as _)),
            _ => Err(PyTypeError::new_err(ERRMSG_POW_INT_EXP)),
        }
    }
}

/// Floor division `self // other` for integer operands.
fn upy_floordiv(lhs: &UPy, rhs: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
    use pyo3::IntoPyObjectExt;
    let a = lhs.0.as_ibig();
    let obj = match rhs {
        UniInput::Uint(x) => IPy(ibig_div_mod_floor(a, &IBig::from(x)).0).into_py_any(py)?,
        UniInput::BUint(x) => IPy(ibig_div_mod_floor(a, x.0.as_ibig()).0).into_py_any(py)?,
        UniInput::Int(x) => IPy(ibig_div_mod_floor(a, &IBig::from(x)).0).into_py_any(py)?,
        UniInput::BInt(x) => IPy(ibig_div_mod_floor(a, &x.0).0).into_py_any(py)?,
        UniInput::OBInt(x) => IPy(ibig_div_mod_floor(a, &x).0).into_py_any(py)?,
        _ => return Err(PyTypeError::new_err(ERRMSG_FLOORDIV_INT_ONLY)),
    };
    Ok(obj)
}
fn upy_rfloordiv(lhs: UniInput<'_>, rhs: &UPy, py: Python<'_>) -> PyResult<Py<PyAny>> {
    use pyo3::IntoPyObjectExt;
    let b = rhs.0.as_ibig();
    let obj = match lhs {
        UniInput::Uint(x) => IPy(ibig_div_mod_floor(&IBig::from(x), b).0).into_py_any(py)?,
        UniInput::BUint(x) => IPy(ibig_div_mod_floor(x.0.as_ibig(), b).0).into_py_any(py)?,
        UniInput::Int(x) => IPy(ibig_div_mod_floor(&IBig::from(x), b).0).into_py_any(py)?,
        UniInput::BInt(x) => IPy(ibig_div_mod_floor(&x.0, b).0).into_py_any(py)?,
        UniInput::OBInt(x) => IPy(ibig_div_mod_floor(&x, b).0).into_py_any(py)?,
        _ => return Err(PyTypeError::new_err(ERRMSG_FLOORDIV_INT_ONLY)),
    };
    Ok(obj)
}
fn upy_divmod(lhs: &UPy, rhs: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
    use pyo3::IntoPyObjectExt;
    let a = lhs.0.as_ibig();
    let obj = match rhs {
        UniInput::Uint(x) => {
            let (q, r) = ibig_div_mod_floor(a, &IBig::from(x));
            (IPy(q), IPy(r)).into_py_any(py)?
        }
        UniInput::BUint(x) => {
            let (q, r) = ibig_div_mod_floor(a, x.0.as_ibig());
            (IPy(q), IPy(r)).into_py_any(py)?
        }
        UniInput::Int(x) => {
            let (q, r) = ibig_div_mod_floor(a, &IBig::from(x));
            (IPy(q), IPy(r)).into_py_any(py)?
        }
        UniInput::BInt(x) => {
            let (q, r) = ibig_div_mod_floor(a, &x.0);
            (IPy(q), IPy(r)).into_py_any(py)?
        }
        UniInput::OBInt(x) => {
            let (q, r) = ibig_div_mod_floor(a, &x);
            (IPy(q), IPy(r)).into_py_any(py)?
        }
        _ => return Err(PyTypeError::new_err(ERRMSG_FLOORDIV_INT_ONLY)),
    };
    Ok(obj)
}

fn ipy_floordiv(lhs: &IPy, rhs: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
    use pyo3::IntoPyObjectExt;
    let a = &lhs.0;
    let obj = match rhs {
        UniInput::Uint(x) => IPy(ibig_div_mod_floor(a, &IBig::from(x)).0).into_py_any(py)?,
        UniInput::BUint(x) => IPy(ibig_div_mod_floor(a, x.0.as_ibig()).0).into_py_any(py)?,
        UniInput::Int(x) => IPy(ibig_div_mod_floor(a, &IBig::from(x)).0).into_py_any(py)?,
        UniInput::BInt(x) => IPy(ibig_div_mod_floor(a, &x.0).0).into_py_any(py)?,
        UniInput::OBInt(x) => IPy(ibig_div_mod_floor(a, &x).0).into_py_any(py)?,
        _ => return Err(PyTypeError::new_err(ERRMSG_FLOORDIV_INT_ONLY)),
    };
    Ok(obj)
}
fn ipy_rfloordiv(lhs: UniInput<'_>, rhs: &IPy, py: Python<'_>) -> PyResult<Py<PyAny>> {
    use pyo3::IntoPyObjectExt;
    let b = &rhs.0;
    let obj = match lhs {
        UniInput::Uint(x) => IPy(ibig_div_mod_floor(&IBig::from(x), b).0).into_py_any(py)?,
        UniInput::BUint(x) => IPy(ibig_div_mod_floor(x.0.as_ibig(), b).0).into_py_any(py)?,
        UniInput::Int(x) => IPy(ibig_div_mod_floor(&IBig::from(x), b).0).into_py_any(py)?,
        UniInput::BInt(x) => IPy(ibig_div_mod_floor(&x.0, b).0).into_py_any(py)?,
        UniInput::OBInt(x) => IPy(ibig_div_mod_floor(&x, b).0).into_py_any(py)?,
        _ => return Err(PyTypeError::new_err(ERRMSG_FLOORDIV_INT_ONLY)),
    };
    Ok(obj)
}
fn ipy_divmod(lhs: &IPy, rhs: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
    use pyo3::IntoPyObjectExt;
    let a = &lhs.0;
    let obj = match rhs {
        UniInput::Uint(x) => {
            let (q, r) = ibig_div_mod_floor(a, &IBig::from(x));
            (IPy(q), IPy(r)).into_py_any(py)?
        }
        UniInput::BUint(x) => {
            let (q, r) = ibig_div_mod_floor(a, x.0.as_ibig());
            (IPy(q), IPy(r)).into_py_any(py)?
        }
        UniInput::Int(x) => {
            let (q, r) = ibig_div_mod_floor(a, &IBig::from(x));
            (IPy(q), IPy(r)).into_py_any(py)?
        }
        UniInput::BInt(x) => {
            let (q, r) = ibig_div_mod_floor(a, &x.0);
            (IPy(q), IPy(r)).into_py_any(py)?
        }
        UniInput::OBInt(x) => {
            let (q, r) = ibig_div_mod_floor(a, &x);
            (IPy(q), IPy(r)).into_py_any(py)?
        }
        _ => return Err(PyTypeError::new_err(ERRMSG_FLOORDIV_INT_ONLY)),
    };
    Ok(obj)
}

#[pymethods]
impl UPy {
    #[new]
    #[pyo3(signature = (ob, radix=None))]
    fn __new__(ob: &Bound<'_, PyAny>, radix: Option<u32>) -> PyResult<Self> {
        if ob.is_instance_of::<PyInt>() {
            // create from int
            if radix.is_some() {
                return Err(PyTypeError::new_err(ERRMSG_INT_WITH_RADIX));
            }

            let (v, overflow) = parse_to_long(ob)?;
            if !overflow {
                if let Ok(n) = u64::try_from(v) {
                    Ok(UPy(UBig::from(n)))
                } else {
                    Err(PyOverflowError::new_err(ERRMSG_NEGATIVE_TO_UNSIGNED))
                }
            } else {
                Ok(UPy(parse_to_ubig(ob)?))
            }
        } else if let Ok(s) = ob.extract::<String>() {
            // create from string
            let n = if let Some(r) = radix {
                UBig::from_str_radix(&s, r)
            } else {
                UBig::from_str_with_radix_prefix(&s).map(|v| v.0)
            };
            Ok(UPy(n.map_err(parse_error_to_py)?))
        } else if let Ok(obj) = ob.extract::<PyRef<Self>>() {
            Ok(UPy(obj.0.clone()))
        } else {
            Err(PyTypeError::new_err(ERRMSG_UBIG_WRONG_SRC_TYPE))
        }
    }
    fn unwrap(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        convert_from_ubig(&self.0, py)?.into_py_any(py)
    }

    fn __repr__(&self) -> String {
        format!("<UBig {:#?}>", self.0)
    }
    fn __str__(&self) -> String {
        format!("{}", self.0)
    }
    fn __format__(&self, format_spec: &str, py: Python<'_>) -> PyResult<String> {
        // delegate to Python int (arbitrary precision — no loss)
        convert_from_ubig(&self.0, py)?
            .call_method1("__format__", (format_spec,))?
            .extract::<String>()
    }
    fn __hash__(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.0.num_hash(&mut hasher);
        hasher.finish()
    }
    fn __richcmp__(&self, other: UniInput<'_>, op: CompareOp) -> bool {
        let order = match other {
            UniInput::Uint(x) => self.0.num_cmp(&x),
            UniInput::Int(x) => self.0.num_cmp(&x),
            UniInput::BUint(x) => self.0.cmp(&x.0),
            UniInput::BInt(x) => self.0.num_cmp(&x.0),
            UniInput::OBInt(x) => self.0.num_cmp(&x),
            UniInput::Float(x) => self.0.num_cmp(&x),
            UniInput::BFloat(x) => self.0.num_cmp(&x.0),
            UniInput::BDecimal(x) => self.0.num_cmp(&x.0),
            UniInput::OBDecimal(x) => self.0.num_cmp(&x),
            UniInput::BRational(x) => self.0.num_cmp(&x.0),
            UniInput::OBRational(x) => self.0.num_cmp(&x),
        };
        op.matches(order)
    }

    /********** use as a bit vector **********/
    fn __len__(&self) -> usize {
        self.0.bit_len()
    }
    fn __getitem__(&self, index: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        use pyo3::IntoPyObjectExt;
        let py = index.py();
        if let Ok(i) = index.extract::<isize>() {
            let i = parse_signed_index(i, self.0.bit_len(), true)
                .ok_or(PyIndexError::new_err(ERRMSG_UBIG_BITS_OOR))?;
            self.0.bit(i).into_py_any(py)
        } else if let Ok(range) = index.cast::<PySlice>() {
            let len: isize = self
                .0
                .bit_len()
                .try_into()
                .map_err(|_| PyNotImplementedError::new_err(ERRMSG_LENGTH_TOO_LARGE))?;
            let indices = range.indices(len)?;
            if indices.step != 1 {
                return Err(PyNotImplementedError::new_err(ERRMSG_STEPSIZE_TOO_LARGE));
            }

            let mut data = self.0.clone();
            data.clear_high_bits(indices.stop as _);
            let split = Self(data.split_bits(indices.start as _).1);
            split.into_py_any(py)
        } else {
            Err(PyTypeError::new_err(ERRMSG_WRONG_INDEX_TYPE))
        }
    }
    fn __setitem__(&mut self, index: &Bound<'_, PyAny>, set: bool) -> PyResult<()> {
        if let Ok(i) = index.extract::<isize>() {
            let i = parse_signed_index(i, self.0.bit_len(), true)
                .ok_or(PyIndexError::new_err(ERRMSG_UBIG_BITS_OOR))?;
            if set {
                self.0.set_bit(i)
            } else {
                self.0.clear_bit(i)
            }
            Ok(())
        } else if let Ok(range) = index.cast::<PySlice>() {
            let len: isize = self
                .0
                .bit_len()
                .try_into()
                .map_err(|_| PyNotImplementedError::new_err(ERRMSG_LENGTH_TOO_LARGE))?;
            let indices = range.indices(len)?;
            if indices.step != 1 {
                return Err(PyNotImplementedError::new_err(ERRMSG_STEPSIZE_TOO_LARGE));
            }

            // shortcut for clearing high bits
            if indices.stop == len && !set {
                self.0.clear_high_bits(indices.start as _);
            }

            // here the operations rely on the And and Or ops.
            // they can be optimized if UBig implements more bit operations.
            if set {
                let ones = indices.stop - indices.start;
                let mask = UBig::ones(ones as _) << (indices.start as usize);
                self.0 |= mask;
            } else {
                let mask_lo = UBig::ones(indices.stop as _) - UBig::ones(indices.start as _);
                let mask = UBig::ones(len as _) - mask_lo;
                self.0 &= mask;
            }
            Ok(())
        } else {
            Err(PyTypeError::new_err(ERRMSG_WRONG_INDEX_TYPE))
        }
    }
    fn __delitem__(&mut self, index: &Bound<'_, PyAny>) -> PyResult<()> {
        fn remove_bits_in_middle(u: &mut UBig, start: usize, end: usize) {
            let (mut left, right) = core::mem::take(u).split_bits(end);
            left.clear_high_bits(end - start);
            *u = (right << start) | left;
        }

        if let Ok(i) = index.extract::<isize>() {
            let i = parse_signed_index(i, self.0.bit_len(), true)
                .ok_or(PyIndexError::new_err(ERRMSG_UBIG_BITS_OOR))?;
            remove_bits_in_middle(&mut self.0, i, i + 1);
            Ok(())
        } else if let Ok(range) = index.cast::<PySlice>() {
            let len: isize = self
                .0
                .bit_len()
                .try_into()
                .map_err(|_| PyNotImplementedError::new_err(ERRMSG_LENGTH_TOO_LARGE))?;
            let indices = range.indices(len)?;
            if indices.step != 1 {
                return Err(PyNotImplementedError::new_err(ERRMSG_STEPSIZE_TOO_LARGE));
            }

            // shortcut for clearing high bits
            if indices.stop == len {
                self.0.clear_high_bits(indices.start as _);
            } else if indices.start == 0 {
                self.0 >>= indices.stop as usize;
            } else {
                remove_bits_in_middle(&mut self.0, indices.start as _, indices.stop as _);
            }
            Ok(())
        } else {
            Err(PyTypeError::new_err(ERRMSG_WRONG_INDEX_TYPE))
        }
    }

    /********** number theory & roots **********/
    fn sqrt(&self) -> Self {
        UPy(self.0.sqrt())
    }
    fn cbrt(&self) -> Self {
        UPy(self.0.cbrt())
    }
    fn nth_root(&self, n: usize) -> Self {
        UPy(self.0.nth_root(n))
    }
    fn sqr(&self) -> Self {
        UPy(self.0.sqr())
    }
    fn cubic(&self) -> Self {
        UPy(self.0.cubic())
    }
    fn ilog(&self, base: UniInput<'_>) -> PyResult<usize> {
        let base = base.into_ubig()?;
        if base <= UBig::ONE {
            return Err(PyValueError::new_err("base must be greater than 1"));
        }
        Ok(self.0.ilog(&base))
    }
    fn is_multiple_of(&self, divisor: UniInput<'_>) -> PyResult<bool> {
        Ok(self.0.is_multiple_of(&divisor.into_ubig()?))
    }
    fn remove(&mut self, factor: UniInput<'_>) -> PyResult<usize> {
        self.0
            .remove(&factor.into_ubig()?)
            .ok_or_else(|| PyValueError::new_err("the factor does not divide this number"))
    }
    fn gcd(&self, other: UniInput<'_>) -> PyResult<Self> {
        Ok(UPy(Gcd::gcd(&self.0, &other.into_ubig()?)))
    }
    fn gcd_ext(&self, other: UniInput<'_>) -> PyResult<(Self, IPy, IPy)> {
        let (g, s, t) = ExtendedGcd::gcd_ext(&self.0, &other.into_ubig()?);
        Ok((UPy(g), IPy(s), IPy(t)))
    }

    /********** bit operations **********/
    fn count_ones(&self) -> usize {
        self.0.count_ones()
    }
    fn count_zeros(&self) -> Option<usize> {
        self.0.count_zeros()
    }
    fn trailing_zeros(&self) -> Option<usize> {
        self.0.trailing_zeros()
    }
    fn trailing_ones(&self) -> Option<usize> {
        self.0.trailing_ones()
    }
    fn is_power_of_two(&self) -> bool {
        self.0.is_power_of_two()
    }
    fn next_power_of_two(&self) -> Self {
        UPy(self.0.clone().next_power_of_two())
    }

    /********** accessors **********/
    fn is_one(&self) -> bool {
        self.0.is_one()
    }
    #[staticmethod]
    fn ones(n: usize) -> Self {
        UPy(UBig::ones(n))
    }

    /********** interop **********/

    fn __int__(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        convert_from_ubig(&self.0, py)?.into_py_any(py)
    }
    /// Get the underlying words representing this integer
    fn to_words(&self) -> PyWords {
        PyWords(self.0.as_words().to_vec())
    }
    /// Create an integer from a list of words
    #[staticmethod]
    fn from_words(ob: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(vec) = ob.extract::<Vec<Word>>() {
            Ok(UPy(UBig::from_words(&vec)))
        } else if let Ok(words) = ob.extract::<PyRef<PyWords>>() {
            Ok(UPy(UBig::from_words(&words.0)))
        } else {
            Err(PyTypeError::new_err(ERRMSG_FROM_WORDS_WRONG_TYPE))
        }
    }
    /// Convert the integer to bytes, like int.to_bytes().
    fn to_bytes(&self, byteorder: Option<&str>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let byteorder = byteorder.unwrap_or("little");
        let bytes = match byteorder {
            "little" => PyBytes::new(py, &self.0.to_le_bytes()),
            "big" => PyBytes::new(py, &self.0.to_be_bytes()),
            _ => return Err(PyValueError::new_err(ERRMSG_WRONG_ENDIANNESS)),
        };
        bytes.into_py_any(py)
    }
    /// Create UBig from bytes, like int.from_bytes().
    #[staticmethod]
    fn from_bytes(bytes: &Bound<'_, PyBytes>, byteorder: Option<&str>) -> PyResult<Self> {
        let byteorder = byteorder.unwrap_or("little");
        let uint = match byteorder {
            "little" => UBig::from_le_bytes(bytes.as_bytes()),
            "big" => UBig::from_be_bytes(bytes.as_bytes()),
            _ => return Err(PyValueError::new_err(ERRMSG_WRONG_ENDIANNESS)),
        };
        Ok(Self(uint))
    }
    fn to_chunks(&self, chunk_bits: usize, py: Python<'_>) -> PyResult<Py<PyAny>> {
        if chunk_bits == 0 {
            return Err(PyValueError::new_err(ERRMSG_ZERO_CHUNK_SIZE));
        }
        let chunks: Vec<Py<PyAny>> = self
            .0
            .to_chunks(chunk_bits)
            .into_vec()
            .into_iter()
            .map(|u| UPy(u).into_py_any(py))
            .collect::<PyResult<_>>()?;
        PyTuple::new(py, chunks)?.into_py_any(py)
    }
    #[staticmethod]
    fn from_chunks(chunks: &Bound<'_, PyAny>, chunk_bits: usize) -> PyResult<Self> {
        if chunk_bits == 0 {
            return Err(PyValueError::new_err(ERRMSG_ZERO_CHUNK_SIZE));
        }

        let mut input = Vec::new();
        if let Ok(list) = chunks.cast::<PyList>() {
            input.reserve_exact(list.len());
            for item in list {
                input.push(UniInput::extract(item.as_borrowed())?.into_ubig()?);
            }
        } else if let Ok(tuple) = chunks.cast::<PyTuple>() {
            input.reserve_exact(tuple.len());
            for item in tuple {
                input.push(UniInput::extract(item.as_borrowed())?.into_ubig()?);
            }
        } else if let Ok(iter) = chunks.cast::<PyIterator>() {
            for item in iter {
                input.push(UniInput::extract(item?.as_borrowed())?.into_ubig()?);
            }
        } else {
            return Err(PyTypeError::new_err(ERRMSG_WRONG_CHUNKS_TYPE));
        }

        Ok(UPy(UBig::from_chunks(input.iter(), chunk_bits)))
    }

    /********** operators **********/
    #[inline]
    fn __add__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        upy_add(self, other, py)
    }
    #[inline]
    fn __radd__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        upy_add(self, other, py)
    }
    #[inline]
    fn __sub__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        upy_sub(self, other, py)
    }
    #[inline]
    fn __rsub__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        upy_rsub(other, self, py)
    }
    #[inline]
    fn __mul__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        upy_mul(self, other, py)
    }
    #[inline]
    fn __rmul__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        upy_mul(self, other, py)
    }
    #[inline]
    fn __truediv__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        upy_div(self, other, py)
    }
    #[inline]
    fn __rtruediv__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        upy_rdiv(other, self, py)
    }
    #[inline]
    fn __floordiv__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        upy_floordiv(self, other, py)
    }
    #[inline]
    fn __rfloordiv__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        upy_rfloordiv(other, self, py)
    }
    #[inline]
    fn __mod__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        upy_mod(self, other, py)
    }
    #[inline]
    fn __divmod__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        upy_divmod(self, other, py)
    }
    #[inline]
    fn __pow__(
        &self,
        other: UniInput,
        modulus: Option<UniInput>,
        py: Python<'_>,
    ) -> PyResult<Py<PyAny>> {
        use pyo3::IntoPyObjectExt;
        ipy_pow(self.0.as_ibig(), other, modulus)
            .and_then(|n| {
                n.try_into()
                    .map_err(|_| PyValueError::new_err(ERRMSG_NEGATIVE_TO_UNSIGNED))
            })
            .and_then(|u: UBig| UPy(u).into_py_any(py))
    }

    #[inline]
    fn __iadd__(&mut self, other: UniInput<'_>) -> PyResult<()> {
        self.0 += &other.into_ubig()?;
        Ok(())
    }
    #[inline]
    fn __isub__(&mut self, other: UniInput<'_>) -> PyResult<()> {
        self.0 -= &other.into_ubig()?;
        Ok(())
    }
    #[inline]
    fn __imul__(&mut self, other: UniInput<'_>) -> PyResult<()> {
        self.0 *= &other.into_ubig()?;
        Ok(())
    }
    #[inline]
    fn __iand__(&mut self, other: UniInput<'_>) -> PyResult<()> {
        self.0 &= &other.into_ubig()?;
        Ok(())
    }
    #[inline]
    fn __ior__(&mut self, other: UniInput<'_>) -> PyResult<()> {
        self.0 |= &other.into_ubig()?;
        Ok(())
    }
    #[inline]
    fn __ixor__(&mut self, other: UniInput<'_>) -> PyResult<()> {
        self.0 ^= &other.into_ubig()?;
        Ok(())
    }
    #[inline]
    fn __ilshift__(&mut self, other: usize) {
        self.0 <<= other;
    }
    #[inline]
    fn __irshift__(&mut self, other: usize) {
        self.0 >>= other;
    }

    #[inline]
    fn __pos__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }
    #[inline]
    fn __neg__(&self) -> IPy {
        IPy((&self.0).neg())
    }
    #[inline]
    fn __abs__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }
    #[inline]
    fn __bool__(&self) -> bool {
        !self.0.is_zero()
    }

    #[inline]
    fn __lshift__(&self, other: usize) -> UPy {
        UPy((&self.0) << other)
    }
    #[inline]
    fn __rshift__(&self, other: usize) -> UPy {
        UPy((&self.0) >> other)
    }
    #[inline]
    fn __and__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        upy_bitand(self, other, py)
    }
    #[inline]
    fn __or__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        upy_bitor(self, other, py)
    }
    #[inline]
    fn __xor__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        upy_bitxor(self, other, py)
    }
}

#[pymethods]
impl IPy {
    #[new]
    #[inline]
    #[pyo3(signature = (ob, radix=None))]
    fn __new__(ob: &Bound<'_, PyAny>, radix: Option<u32>) -> PyResult<Self> {
        if ob.is_instance_of::<PyInt>() {
            // create from int
            if radix.is_some() {
                return Err(PyTypeError::new_err(ERRMSG_INT_WITH_RADIX));
            }

            let (v, overflow) = parse_to_long(ob)?;
            if !overflow {
                Ok(IPy(IBig::from(v)))
            } else {
                Ok(IPy(parse_to_ibig(ob)?))
            }
        } else if let Ok(s) = ob.extract::<String>() {
            // create from string
            let n = if let Some(r) = radix {
                IBig::from_str_radix(&s, r)
            } else {
                IBig::from_str_with_radix_prefix(&s).map(|v| v.0)
            };
            Ok(IPy(n.map_err(parse_error_to_py)?))
        } else if let Ok(obj) = ob.extract::<PyRef<UPy>>() {
            Ok(IPy(obj.0.clone().into()))
        } else if let Ok(obj) = ob.extract::<PyRef<Self>>() {
            Ok(IPy(obj.0.clone()))
        } else {
            Err(PyTypeError::new_err(ERRMSG_IBIG_WRONG_SRC_TYPE))
        }
    }
    fn unwrap(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        convert_from_ibig(&self.0, py)?.into_py_any(py)
    }

    fn __repr__(&self) -> String {
        format!("<IBig {:#?}>", self.0)
    }
    fn __str__(&self) -> String {
        format!("{}", self.0)
    }
    fn __format__(&self, format_spec: &str, py: Python<'_>) -> PyResult<String> {
        // delegate to Python int (arbitrary precision — no loss)
        convert_from_ibig(&self.0, py)?
            .call_method1("__format__", (format_spec,))?
            .extract::<String>()
    }
    fn __hash__(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.0.num_hash(&mut hasher);
        hasher.finish()
    }
    fn __richcmp__(&self, other: UniInput<'_>, op: CompareOp) -> bool {
        let order = match other {
            UniInput::Uint(x) => self.0.num_cmp(&x),
            UniInput::Int(x) => self.0.num_cmp(&x),
            UniInput::BUint(x) => self.0.num_cmp(&x.0),
            UniInput::BInt(x) => self.0.cmp(&x.0),
            UniInput::OBInt(x) => self.0.cmp(&x),
            UniInput::Float(x) => self.0.num_cmp(&x),
            UniInput::BFloat(x) => self.0.num_cmp(&x.0),
            UniInput::BDecimal(x) => self.0.num_cmp(&x.0),
            UniInput::OBDecimal(x) => self.0.num_cmp(&x),
            UniInput::BRational(x) => self.0.num_cmp(&x.0),
            UniInput::OBRational(x) => self.0.num_cmp(&x),
        };
        op.matches(order)
    }

    /********** use as a bit vector with very limited capabilities **********/
    fn __len__(&self) -> usize {
        self.0.bit_len()
    }
    fn __getitem__(&self, index: &Bound<'_, PyAny>) -> PyResult<bool> {
        if let Ok(i) = index.extract::<isize>() {
            let i = parse_signed_index(i, self.0.bit_len(), true)
                .ok_or(PyIndexError::new_err(ERRMSG_UBIG_BITS_OOR))?;
            Ok(self.0.bit(i))
        } else {
            Err(PyTypeError::new_err(ERRMSG_WRONG_INDEX_TYPE))
        }
    }

    /********** number theory & roots (note return types!) **********/
    fn sqrt(&self) -> PyResult<UPy> {
        if self.0.is_negative() {
            return Err(PyValueError::new_err(
                "cannot compute the square root of a negative number",
            ));
        }
        Ok(UPy(self.0.sqrt()))
    }
    fn cbrt(&self) -> Self {
        // IBig's CubicRoot trait impl panics on negatives; nth_root(3) is sign-preserving.
        IPy(self.0.nth_root(3))
    }
    fn nth_root(&self, n: usize) -> PyResult<Self> {
        if n % 2 == 0 && self.0.sign() == Sign::Negative {
            return Err(PyValueError::new_err("cannot compute an even root of a negative number"));
        }
        Ok(IPy(self.0.nth_root(n)))
    }
    fn sqr(&self) -> UPy {
        UPy(self.0.sqr())
    }
    fn cubic(&self) -> Self {
        IPy(self.0.cubic())
    }
    fn ilog(&self, base: UniInput<'_>) -> PyResult<usize> {
        let base = base.into_ubig()?;
        if base <= UBig::ONE {
            return Err(PyValueError::new_err("base must be greater than 1"));
        }
        Ok(self.0.ilog(&base))
    }
    fn trailing_zeros(&self) -> Option<usize> {
        self.0.trailing_zeros()
    }
    fn trailing_ones(&self) -> Option<usize> {
        self.0.trailing_ones()
    }

    /********** accessors **********/
    fn is_one(&self) -> bool {
        self.0.is_one()
    }
    fn sign(&self) -> PySign {
        self.0.sign().into()
    }
    fn signum(&self) -> Self {
        IPy(self.0.signum())
    }
    fn is_negative(&self) -> bool {
        self.0.sign() == Sign::Negative
    }
    fn is_positive(&self) -> bool {
        self.0.sign() == Sign::Positive
    }
    fn to_parts(&self) -> (PySign, UPy) {
        let (sign, mag) = self.0.clone().into_parts();
        (sign.into(), UPy(mag))
    }
    #[staticmethod]
    fn from_parts(sign: PySign, magnitude: UniInput<'_>) -> PyResult<Self> {
        Ok(IPy(IBig::from_parts(sign.into(), magnitude.into_ubig()?)))
    }
    fn as_ubig(&self) -> Option<UPy> {
        self.0.as_ubig().cloned().map(UPy)
    }
    #[staticmethod]
    fn ones(n: usize) -> Self {
        IPy(IBig::from(UBig::ones(n)))
    }

    /********** interop **********/
    fn __int__(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        convert_from_ibig(&self.0, py)?.into_py_any(py)
    }
    /// Get the underlying (sign, words) representing this integer
    fn to_words(&self) -> PyWords {
        let (_, words) = self.0.as_sign_words();
        PyWords(words.to_vec())
    }
    /// Create an integer from a list of words (interpreted as non-negative magnitude)
    #[staticmethod]
    fn from_words(ob: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(vec) = ob.extract::<Vec<Word>>() {
            Ok(IPy(IBig::from(UBig::from_words(&vec))))
        } else if let Ok(words) = ob.extract::<PyRef<PyWords>>() {
            Ok(IPy(IBig::from(UBig::from_words(&words.0))))
        } else {
            Err(PyTypeError::new_err(ERRMSG_FROM_WORDS_WRONG_TYPE))
        }
    }
    fn to_chunks(&self, chunk_bits: usize, py: Python<'_>) -> PyResult<Py<PyAny>> {
        if chunk_bits == 0 {
            return Err(PyValueError::new_err(ERRMSG_ZERO_CHUNK_SIZE));
        }
        let mag = (&self.0).unsigned_abs();
        let chunks: Vec<Py<PyAny>> = mag
            .to_chunks(chunk_bits)
            .into_vec()
            .into_iter()
            .map(|u| UPy(u).into_py_any(py))
            .collect::<PyResult<_>>()?;
        PyTuple::new(py, chunks)?.into_py_any(py)
    }
    #[staticmethod]
    fn from_chunks(chunks: &Bound<'_, PyAny>, chunk_bits: usize) -> PyResult<Self> {
        if chunk_bits == 0 {
            return Err(PyValueError::new_err(ERRMSG_ZERO_CHUNK_SIZE));
        }

        let mut input = Vec::new();
        if let Ok(list) = chunks.cast::<PyList>() {
            input.reserve_exact(list.len());
            for item in list {
                input.push(UniInput::extract(item.as_borrowed())?.into_ubig()?);
            }
        } else if let Ok(tuple) = chunks.cast::<PyTuple>() {
            input.reserve_exact(tuple.len());
            for item in tuple {
                input.push(UniInput::extract(item.as_borrowed())?.into_ubig()?);
            }
        } else if let Ok(iter) = chunks.cast::<PyIterator>() {
            for item in iter {
                input.push(UniInput::extract(item?.as_borrowed())?.into_ubig()?);
            }
        } else {
            return Err(PyTypeError::new_err(ERRMSG_WRONG_CHUNKS_TYPE));
        }

        let mag = UBig::from_chunks(input.iter(), chunk_bits);
        Ok(IPy(IBig::from(mag)))
    }
    /// Convert the integer to bytes, like int.to_bytes().
    fn to_bytes(
        &self,
        byteorder: Option<&str>,
        signed: Option<bool>,
        py: Python<'_>,
    ) -> PyResult<Py<PyAny>> {
        use pyo3::IntoPyObjectExt;
        let signed = signed.unwrap_or(false);
        if !signed && self.0.is_negative() {
            return Err(PyOverflowError::new_err(ERRMSG_NEGATIVE_TO_UNSIGNED));
        }

        let byteorder = byteorder.unwrap_or("little");
        let bytes = match byteorder {
            "little" => PyBytes::new(py, &self.0.to_le_bytes()),
            "big" => PyBytes::new(py, &self.0.to_be_bytes()),
            _ => return Err(PyValueError::new_err(ERRMSG_WRONG_ENDIANNESS)),
        };
        bytes.into_py_any(py)
    }
    /// Create IBig from bytes, like int.from_bytes().
    #[staticmethod]
    fn from_bytes(
        bytes: &Bound<'_, PyBytes>,
        byteorder: Option<&str>,
        signed: Option<bool>,
    ) -> PyResult<Self> {
        let byteorder = byteorder.unwrap_or("little");
        let signed = signed.unwrap_or(false);
        let int = match byteorder {
            "little" => match signed {
                false => UBig::from_le_bytes(bytes.as_bytes()).into(),
                true => IBig::from_le_bytes(bytes.as_bytes()),
            },
            "big" => match signed {
                false => UBig::from_be_bytes(bytes.as_bytes()).into(),
                true => IBig::from_be_bytes(bytes.as_bytes()),
            },
            _ => return Err(PyValueError::new_err(ERRMSG_WRONG_ENDIANNESS)),
        };
        Ok(Self(int))
    }

    /********** operators **********/
    #[inline]
    fn __add__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        ipy_add(self, other, py)
    }
    #[inline]
    fn __radd__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        ipy_add(self, other, py)
    }
    #[inline]
    fn __sub__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        ipy_sub(self, other, py)
    }
    #[inline]
    fn __rsub__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        ipy_rsub(other, self, py)
    }
    #[inline]
    fn __mul__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        ipy_mul(self, other, py)
    }
    #[inline]
    fn __rmul__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        ipy_mul(self, other, py)
    }
    #[inline]
    fn __truediv__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        ipy_div(self, other, py)
    }
    #[inline]
    fn __rtruediv__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        ipy_rdiv(other, self, py)
    }
    #[inline]
    fn __floordiv__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        ipy_floordiv(self, other, py)
    }
    #[inline]
    fn __rfloordiv__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        ipy_rfloordiv(other, self, py)
    }
    #[inline]
    fn __mod__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        ipy_mod(self, other, py)
    }
    #[inline]
    fn __divmod__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        ipy_divmod(self, other, py)
    }
    #[inline]
    fn __pow__(
        &self,
        other: UniInput,
        modulus: Option<UniInput>,
        py: Python<'_>,
    ) -> PyResult<Py<PyAny>> {
        use pyo3::IntoPyObjectExt;
        ipy_pow(&self.0, other, modulus).and_then(|n| IPy(n).into_py_any(py))
    }

    #[inline]
    fn __iadd__(&mut self, other: UniInput<'_>) -> PyResult<()> {
        self.0 += &other.into_ibig()?;
        Ok(())
    }
    #[inline]
    fn __isub__(&mut self, other: UniInput<'_>) -> PyResult<()> {
        self.0 -= &other.into_ibig()?;
        Ok(())
    }
    #[inline]
    fn __imul__(&mut self, other: UniInput<'_>) -> PyResult<()> {
        self.0 *= &other.into_ibig()?;
        Ok(())
    }
    #[inline]
    fn __iand__(&mut self, other: UniInput<'_>) -> PyResult<()> {
        self.0 &= &other.into_ibig()?;
        Ok(())
    }
    #[inline]
    fn __ior__(&mut self, other: UniInput<'_>) -> PyResult<()> {
        self.0 |= &other.into_ibig()?;
        Ok(())
    }
    #[inline]
    fn __ixor__(&mut self, other: UniInput<'_>) -> PyResult<()> {
        self.0 ^= &other.into_ibig()?;
        Ok(())
    }
    #[inline]
    fn __ilshift__(&mut self, other: usize) {
        self.0 <<= other;
    }
    #[inline]
    fn __irshift__(&mut self, other: usize) {
        self.0 >>= other;
    }

    #[inline]
    fn __pos__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }
    #[inline]
    fn __neg__(&self) -> IPy {
        IPy((&self.0).neg())
    }
    #[inline]
    fn __abs__(&self) -> IPy {
        IPy((&self.0).abs())
    }
    #[inline]
    fn __invert__(&self) -> Self {
        IPy(!&self.0)
    }
    #[inline]
    fn __bool__(&self) -> bool {
        !self.0.is_zero()
    }

    #[inline]
    fn __lshift__(&self, other: usize) -> IPy {
        IPy((&self.0) << other)
    }
    #[inline]
    fn __rshift__(&self, other: usize) -> IPy {
        IPy((&self.0) >> other)
    }
    #[inline]
    fn __and__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        ipy_bitand(self, other, py)
    }
    #[inline]
    fn __or__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        ipy_bitor(self, other, py)
    }
    #[inline]
    fn __xor__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        ipy_bitxor(self, other, py)
    }
}

#[cfg(feature = "zeroize")]
#[pymethods]
impl UPy {
    /// Zeroize the internal buffer, clearing the memory used by this integer. The value
    /// becomes zero.
    fn zeroize(&mut self) {
        use zeroize::Zeroize;
        self.0.zeroize();
    }
}

#[cfg(feature = "zeroize")]
#[pymethods]
impl IPy {
    /// Zeroize the internal buffer, clearing the memory used by this integer. The value
    /// becomes zero.
    fn zeroize(&mut self) {
        use zeroize::Zeroize;
        self.0.zeroize();
    }
}
