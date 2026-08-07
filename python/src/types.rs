use pyo3::prelude::*;
use std::os::raw::{c_double, c_longlong, c_ulonglong};

use dashu::base::Sign;
use dashu::float::DBig;
use dashu::integer::{IBig, UBig};
use dashu::rational::RBig;
type FBig = dashu::float::FBig;

#[pyclass(eq, eq_int)]
#[derive(PartialEq, Clone, Copy)]
pub enum PySign {
    Positive,
    Negative,
}

impl From<Sign> for PySign {
    #[inline]
    fn from(value: Sign) -> Self {
        match value {
            Sign::Positive => Self::Positive,
            Sign::Negative => Self::Negative,
        }
    }
}

impl From<PySign> for Sign {
    #[inline]
    fn from(value: PySign) -> Self {
        match value {
            PySign::Positive => Sign::Positive,
            PySign::Negative => Sign::Negative,
        }
    }
}

/// This struct is used for representing [UBig] in Python
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
#[pyclass(name = "UBig")]
pub struct UPy(pub UBig);

impl From<UBig> for UPy {
    fn from(n: UBig) -> Self {
        UPy(n)
    }
}

/// This struct is used for representing [IBig] in Python
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
#[pyclass(name = "IBig")]
pub struct IPy(pub IBig);

impl From<IBig> for IPy {
    fn from(n: IBig) -> Self {
        IPy(n)
    }
}

/// This struct is used for representing [FBig] in Python
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
#[pyclass(name = "FBig")]
pub struct FPy(pub FBig);

impl From<FBig> for FPy {
    fn from(n: FBig) -> Self {
        FPy(n)
    }
}

/// This struct is used for representing [DBig] in Python
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
#[pyclass(name = "DBig")]
pub struct DPy(pub DBig);

impl From<DBig> for DPy {
    fn from(n: DBig) -> Self {
        DPy(n)
    }
}

/// This struct is used for representing [RBig] in Python
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
#[pyclass(name = "RBig")]
pub struct RPy(pub RBig);

impl From<RBig> for RPy {
    fn from(n: RBig) -> Self {
        RPy(n)
    }
}

/// This struct is used for representing [CBig] in Python. It wraps a *bare* complex number
/// (base 2, rounding `Zero`); the constant cache is shared module-wide (see [`crate::cache`]).
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
#[pyclass(name = "CBig")]
pub struct CPy(pub dashu::complex::CBig<dashu::float::round::mode::Zero, 2>);

impl From<dashu::complex::CBig<dashu::float::round::mode::Zero, 2>> for CPy {
    fn from(n: dashu::complex::CBig<dashu::float::round::mode::Zero, 2>) -> Self {
        CPy(n)
    }
}

#[pyclass(name = "Words")]
pub struct PyWords(pub std::vec::Vec<dashu::integer::Word>);

/// An input type that accepts all possible numeric types from Python
///
/// Notes:
/// - Variants starting with 'B': big numbers
/// - Variants starting with 'OB': owned big numbers
pub enum UniInput<'a> {
    Uint(c_ulonglong), // from int
    Int(c_longlong),   // from int
    BUint(PyRef<'a, UPy>),
    BInt(PyRef<'a, IPy>),
    OBInt(IBig),     // from int
    Float(c_double), // from float
    BFloat(PyRef<'a, FPy>),
    BDecimal(PyRef<'a, DPy>),
    OBDecimal(DBig), // from decimal.Decimal
    BRational(PyRef<'a, RPy>),
    OBRational(RBig), // from fractions.Fraction
}
