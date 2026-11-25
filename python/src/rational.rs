use std::ops::{Add, Div, Mul, Sub};

use dashu::base::{Abs, UnsignedAbs};
use dashu::float::round::mode;
use dashu::float::{Context, DBig, Repr};
use dashu::integer::UBig;
use dashu::rational::RBig;
use num_order::NumOrd;
use pyo3::{Bound, IntoPyObjectExt, Py, PyAny, PyResult, basic::CompareOp, intern, prelude::*};

use crate::{
    convert::{parse_error_to_py, parse_to_rbig, zero_division_error},
    types::{DPy, FPy, IPy, PySign, RPy, UPy, UniInput},
};

/// Forward/reverse arithmetic dispatchers that coerce the operand to RBig first.
macro_rules! impl_rpy_binops {
    ($method:ident, $rs_method:ident) => {
        fn $method(lhs: &RPy, rhs: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
            let rhs = rhs.into_rpy()?;
            RPy((&lhs.0).$rs_method(&rhs.0)).into_py_any(py)
        }
    };
    ($method:ident, $rev_method:ident, $rs_method:ident) => {
        impl_rpy_binops!($method, $rs_method);
        fn $rev_method(lhs: UniInput<'_>, rhs: &RPy, py: Python<'_>) -> PyResult<Py<PyAny>> {
            let lhs = lhs.into_rpy()?;
            RPy((&lhs.0).$rs_method(&rhs.0)).into_py_any(py)
        }
    };
}

impl_rpy_binops!(rpy_add, add);
impl_rpy_binops!(rpy_sub, rpy_rsub, sub);
impl_rpy_binops!(rpy_mul, mul);

/// Division, where a zero divisor raises `ZeroDivisionError` (as for `fractions.Fraction`).
fn rpy_div(lhs: &RPy, rhs: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
    let rhs = rhs.into_rpy()?;
    if rhs.0.is_zero() {
        return Err(zero_division_error());
    }
    RPy((&lhs.0).div(&rhs.0)).into_py_any(py)
}
fn rpy_rdiv(lhs: UniInput<'_>, rhs: &RPy, py: Python<'_>) -> PyResult<Py<PyAny>> {
    let lhs = lhs.into_rpy()?;
    if rhs.0.is_zero() {
        return Err(zero_division_error());
    }
    RPy((&lhs.0).div(&rhs.0)).into_py_any(py)
}

/// The floored modulo of Python (`a - b * floor(a / b)`, which has the sign of `b`), as for
/// `fractions.Fraction` and for the integers. (The `%` operator of `RBig` gives the remainder
/// of the least magnitude instead: `5 % 3` is `-1` there.)
///
/// A zero divisor raises `ZeroDivisionError`.
fn rbig_mod_floor(a: &RBig, b: &RBig) -> PyResult<RBig> {
    if b.is_zero() {
        return Err(zero_division_error());
    }
    let quotient = RBig::from(a.div(b).floor());
    Ok(a - b * quotient)
}
pub(crate) fn rpy_mod(lhs: &RPy, rhs: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
    let rhs = rhs.into_rpy()?;
    RPy(rbig_mod_floor(&lhs.0, &rhs.0)?).into_py_any(py)
}
fn rpy_rmod(lhs: UniInput<'_>, rhs: &RPy, py: Python<'_>) -> PyResult<Py<PyAny>> {
    let lhs = lhs.into_rpy()?;
    RPy(rbig_mod_floor(&lhs.0, &rhs.0)?).into_py_any(py)
}

/// `10^n` as a rational number
fn pow10(n: usize) -> RBig {
    RBig::from(UBig::from(10u8).pow(n))
}

/// `r * 10^n`
fn scale10(r: &RBig, n: isize) -> RBig {
    if n >= 0 {
        r * pow10(n.unsigned_abs())
    } else {
        r / pow10(n.unsigned_abs())
    }
}

/// The decimal float with `digits` significant digits that is nearest to the rational number
/// (ties away from zero), rounding the exact value once.
fn round_to_digits(r: &RBig, digits: usize) -> DBig {
    let digits = digits.max(1);
    if r.is_zero() {
        return DBig::from_repr(Repr::zero(), Context::new(digits));
    }

    // the decimal exponent of the leading digit is either `estimate` or `estimate - 1`
    let ten = UBig::from(10u8);
    let estimate =
        r.numerator().unsigned_abs().ilog(&ten) as isize - r.denominator().ilog(&ten) as isize;
    let mut exponent = estimate - (digits as isize - 1);
    let mut scaled = scale10(r, -exponent);
    if scaled.clone().abs() < pow10(digits - 1) {
        exponent -= 1;
        scaled = scale10(r, -exponent);
    }
    // (rounding up to `10^digits` is fine: the significand is normalized)
    DBig::from_repr(Repr::new(scaled.round(), exponent), Context::new(digits))
}

fn rpy_richcmp(lhs: &RPy, other: UniInput<'_>, op: CompareOp) -> bool {
    let order = match other {
        UniInput::Uint(x) => lhs.0.num_cmp(&x),
        UniInput::Int(x) => lhs.0.num_cmp(&x),
        UniInput::BUint(x) => lhs.0.num_cmp(&x.0),
        UniInput::BInt(x) => lhs.0.num_cmp(&x.0),
        UniInput::OBInt(x) => lhs.0.num_cmp(&x),
        UniInput::Float(x) => lhs.0.num_cmp(&x),
        UniInput::BFloat(x) => lhs.0.num_cmp(&x.0),
        UniInput::BDecimal(x) => lhs.0.num_cmp(&x.0),
        UniInput::OBDecimal(x) => lhs.0.num_cmp(&x),
        UniInput::BRational(x) => lhs.0.cmp(&x.0),
        UniInput::OBRational(x) => lhs.0.cmp(&x),
    };
    op.matches(order)
}

#[pymethods]
impl RPy {
    #[new]
    fn __new__(ob: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(s) = ob.extract::<String>() {
            let r = RBig::from_str_with_radix_prefix(&s).map(|v| v.0);
            return Ok(RPy(r.map_err(parse_error_to_py)?));
        }
        // fractions.Fraction fast path (preserves the original parse_to_rbig round-trip)
        {
            let py = ob.py();
            let fractions = py.import(intern!(py, "fractions"))?;
            let fraction_type = fractions.getattr(intern!(py, "Fraction"))?;
            if ob.is_instance(&fraction_type)? {
                return Ok(RPy(parse_to_rbig(ob)?));
            }
        }
        if let Ok(obj) = ob.extract::<PyRef<Self>>() {
            return Ok(RPy(obj.0.clone()));
        }
        // any other Python number -> permissive construction
        Ok(RPy(UniInput::extract(ob.as_borrowed())?.construct_rpy()?))
    }
    fn unwrap(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        crate::convert::convert_from_rbig(&self.0, py)?.into_py_any(py)
    }

    fn __repr__(&self) -> String {
        format!("<RBig {:?}>", self.0)
    }
    fn __str__(&self) -> String {
        format!("{}", self.0)
    }
    fn __format__(&self, format_spec: &str) -> PyResult<String> {
        if format_spec.is_empty() {
            return Ok(format!("{}", self.0));
        }
        // non-empty spec: convert to a decimal float and render with it. A rational has no
        // native decimal precision, so the conversion produces exactly the digits that will
        // be printed, rounding the exact value once (half away from zero, like `to_decimal`).
        let parsed = crate::format::parse(format_spec)?;
        let dbig = match parsed.ty {
            // fixed notations: the digit count is relative to the decimal point
            'f' | 'F' | '%' => {
                let scale = if parsed.ty == '%' { 2 } else { 0 };
                let fraction_digits = parsed.prec.unwrap_or(6).saturating_add(scale);
                let scaled = (&self.0 * pow10(fraction_digits)).round();
                let repr = Repr::<10>::new(scaled, -(fraction_digits as isize));
                // (a few spare digits, so that the scaling of '%' stays exact)
                let precision = repr.digits() + 3;
                DBig::from_repr(repr, Context::new(precision))
            }
            // scientific notation: one digit before the point, then the requested ones
            'e' | 'E' => round_to_digits(&self.0, parsed.prec.map_or(53, |p| p.saturating_add(1))),
            // general notation: the precision counts the significant digits
            _ => round_to_digits(&self.0, parsed.prec.map_or(53, |p| p.max(1))),
        };
        crate::format::format_dbig(&dbig, format_spec)
    }
    fn __hash__(&self) -> isize {
        crate::hash::hash_rbig(&self.0)
    }
    fn __richcmp__(&self, other: UniInput<'_>, op: CompareOp) -> bool {
        rpy_richcmp(self, other, op)
    }
    fn __bool__(&self) -> bool {
        !self.0.is_zero()
    }
    fn __float__(&self) -> f64 {
        self.0.to_f64_fast()
    }
    fn __int__(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        crate::convert::convert_from_ibig(&self.0.trunc(), py)?.into_py_any(py)
    }

    /********** arithmetic **********/
    #[inline]
    fn __add__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        rpy_add(self, other, py)
    }
    #[inline]
    fn __radd__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        rpy_add(self, other, py)
    }
    #[inline]
    fn __sub__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        rpy_sub(self, other, py)
    }
    #[inline]
    fn __rsub__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        rpy_rsub(other, self, py)
    }
    #[inline]
    fn __mul__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        rpy_mul(self, other, py)
    }
    #[inline]
    fn __rmul__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        rpy_mul(self, other, py)
    }
    #[inline]
    fn __truediv__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        rpy_div(self, other, py)
    }
    #[inline]
    fn __rtruediv__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        rpy_rdiv(other, self, py)
    }
    #[inline]
    fn __mod__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        rpy_mod(self, other, py)
    }
    #[inline]
    fn __rmod__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        rpy_rmod(other, self, py)
    }
    #[inline]
    fn __neg__(&self) -> Self {
        RPy(-&self.0)
    }
    #[inline]
    fn __pos__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }
    #[inline]
    fn __abs__(&self) -> Self {
        RPy(self.0.clone().abs())
    }

    /********** properties & predicates **********/
    #[getter]
    fn numerator(&self) -> IPy {
        IPy(self.0.numerator().clone())
    }
    #[getter]
    fn denominator(&self) -> UPy {
        UPy(self.0.denominator().clone())
    }
    fn is_int(&self) -> bool {
        self.0.is_int()
    }
    fn is_one(&self) -> bool {
        self.0.is_one()
    }
    fn sign(&self) -> PySign {
        self.0.sign().into()
    }
    fn signum(&self) -> Self {
        RPy(self.0.signum())
    }

    /********** rounding (return IBig / RPy) **********/
    fn trunc(&self) -> IPy {
        IPy(self.0.trunc())
    }
    fn floor(&self) -> IPy {
        IPy(self.0.floor())
    }
    fn ceil(&self) -> IPy {
        IPy(self.0.ceil())
    }
    fn round(&self) -> IPy {
        IPy(self.0.round())
    }
    fn fract(&self) -> Self {
        RPy(self.0.fract())
    }

    /********** Python numeric protocol (math.floor/ceil/trunc and round(), all exact) **********/
    fn __trunc__(&self) -> IPy {
        IPy(self.0.trunc())
    }
    fn __floor__(&self) -> IPy {
        IPy(self.0.floor())
    }
    fn __ceil__(&self) -> IPy {
        IPy(self.0.ceil())
    }
    /// `round(x)` is the nearest integer, and `round(x, n)` the nearest multiple of `10^-n`
    /// (as a rational). Ties round away from zero, like `RBig.round()` and the `round()` of
    /// the float types.
    #[pyo3(signature = (ndigits=None))]
    fn __round__(&self, ndigits: Option<isize>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        match ndigits {
            None => IPy(self.0.round()).into_py_any(py),
            Some(n) => {
                let scale = pow10(n.unsigned_abs());
                let rounded = if n >= 0 {
                    RBig::from((&self.0 * &scale).round()) / scale
                } else {
                    RBig::from((&self.0 / &scale).round()) * scale
                };
                RPy(rounded).into_py_any(py)
            }
        }
    }

    fn split_at_point(&self) -> (IPy, Self) {
        let (int_part, frac_part) = self.0.clone().split_at_point();
        (IPy(int_part), RPy(frac_part))
    }

    /********** powers **********/
    fn sqr(&self) -> Self {
        RPy(self.0.sqr())
    }
    fn cubic(&self) -> Self {
        RPy(self.0.cubic())
    }
    fn pow(&self, n: isize) -> PyResult<Self> {
        if n < 0 && self.0.is_zero() {
            return Err(zero_division_error());
        }
        Ok(RPy(self.0.pow(n)))
    }

    /********** construction & conversion **********/
    fn to_int(&self) -> IPy {
        IPy(self.0.trunc())
    }
    /// Lossy conversion to a binary float with the given precision.
    fn to_float(&self, precision: usize) -> FPy {
        FPy(self.0.to_float::<mode::Zero, 2>(precision).value())
    }
    /// Lossy conversion to a decimal float with the given precision.
    fn to_decimal(&self, precision: usize) -> DPy {
        DPy(self.0.to_float::<mode::HalfAway, 10>(precision).value())
    }
    #[staticmethod]
    fn from_parts(numerator: UniInput<'_>, denominator: UniInput<'_>) -> PyResult<Self> {
        let num = numerator.into_ibig()?;
        let den = denominator.into_ubig()?;
        if den.is_zero() {
            return Err(zero_division_error());
        }
        Ok(RPy(RBig::from_parts(num, den)))
    }
    /// Find the simplest rational within the error bounds of the given float.
    #[staticmethod]
    fn simplest_from_float(f: UniInput<'_>) -> PyResult<Option<Self>> {
        let f = f.into_fpy()?;
        Ok(RBig::simplest_from_float(&f.0).map(RPy))
    }
}

#[cfg(feature = "zeroize")]
#[pymethods]
impl RPy {
    /// Zeroize the internal buffers, clearing the memory used by this rational. The value
    /// becomes zero.
    fn zeroize(&mut self) {
        use zeroize::Zeroize;
        self.0.zeroize();
    }
}
