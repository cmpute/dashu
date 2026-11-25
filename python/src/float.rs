use std::ops::{Add, Div, Mul, Rem, Sub};
use std::str::FromStr;

use dashu::base::{Abs, Sign};
use dashu::float::{Context, DBig, FBig, Word, round::Round};
use dashu::integer::IBig;
use num_order::NumOrd;
use pyo3::{
    Bound, IntoPyObjectExt, Py, PyAny, PyResult,
    basic::CompareOp,
    exceptions::{PyOverflowError, PyValueError, PyZeroDivisionError},
    intern,
    prelude::*,
    types::PyFloat,
};

use crate::{
    cache::{limited, unwrap_float, with_cache},
    convert::{conversion_error_to_py, parse_error_to_py, parse_to_dbig},
    types::{DPy, FPy, IPy, PySign, RPy, UniInput},
};

const ERRMSG_INFINITY: &str = "arithmetic with infinity";

/// Arithmetic with an infinite operand raises `ValueError`, as the transcendentals and `%` do
/// (the Rust operators panic on it). An infinity is a terminal value here, the result of an
/// overflow or of a division by zero: it can be compared, negated and printed, but there is no
/// NaN to stand for `inf - inf` or `inf * 0`, so no arithmetic is defined on it.
fn check_finite<R: Round, const B: Word>(lhs: &FBig<R, B>, rhs: &FBig<R, B>) -> PyResult<()> {
    if lhs.repr().is_infinite() || rhs.repr().is_infinite() {
        Err(PyValueError::new_err(ERRMSG_INFINITY))
    } else {
        Ok(())
    }
}

pub(crate) fn float_add<R: Round, const B: Word>(
    lhs: &FBig<R, B>,
    rhs: &FBig<R, B>,
) -> PyResult<FBig<R, B>> {
    check_finite(lhs, rhs)?;
    Ok(lhs.add(rhs))
}
pub(crate) fn float_sub<R: Round, const B: Word>(
    lhs: &FBig<R, B>,
    rhs: &FBig<R, B>,
) -> PyResult<FBig<R, B>> {
    check_finite(lhs, rhs)?;
    Ok(lhs.sub(rhs))
}
pub(crate) fn float_mul<R: Round, const B: Word>(
    lhs: &FBig<R, B>,
    rhs: &FBig<R, B>,
) -> PyResult<FBig<R, B>> {
    check_finite(lhs, rhs)?;
    Ok(lhs.mul(rhs))
}
/// The quotient of two floats. A non-zero number divided by zero is an infinity, `0 / 0`
/// raises `ZeroDivisionError` (it is indeterminate), and two operands of unlimited precision
/// raise `ValueError` (the quotient has to be rounded to some precision).
pub(crate) fn float_div<R: Round, const B: Word>(
    lhs: &FBig<R, B>,
    rhs: &FBig<R, B>,
) -> PyResult<FBig<R, B>> {
    check_finite(lhs, rhs)?;
    let is_zero = |f: &FBig<R, B>| f.repr().significand().is_zero();
    if is_zero(rhs) && is_zero(lhs) {
        return Err(PyZeroDivisionError::new_err("indeterminate form (0/0)"));
    }
    if !is_zero(rhs) {
        limited(Context::max(lhs.context(), rhs.context()))?;
    }
    Ok(lhs.div(rhs))
}

/// Generate forward (and optional reverse) arithmetic dispatchers that first coerce the
/// operand to the wrapper's own type via the `into_*` helper, then apply the operator
/// (one of the `float_*` functions above).
macro_rules! impl_float_binops {
    ($ty:ident, $into:ident, $method:ident, $op:ident) => {
        pub(crate) fn $method(lhs: &$ty, rhs: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
            let rhs = rhs.$into()?;
            $ty($op(&lhs.0, &rhs.0)?).into_py_any(py)
        }
    };
    ($ty:ident, $into:ident, $method:ident, $rev_method:ident, $op:ident) => {
        impl_float_binops!($ty, $into, $method, $op);
        pub(crate) fn $rev_method(
            lhs: UniInput<'_>,
            rhs: &$ty,
            py: Python<'_>,
        ) -> PyResult<Py<PyAny>> {
            let lhs = lhs.$into()?;
            $ty($op(&lhs.0, &rhs.0)?).into_py_any(py)
        }
    };
}

/// The floored modulo of Python (`a - b * floor(a / b)`, which has the sign of `b`), with
/// Python exceptions for the operands that the `%` operator of the float panics on: a zero
/// divisor raises `ZeroDivisionError` (as `float` does), and an infinite operand raises
/// `ValueError`.
///
/// The `%` operator of the float gives a remainder whose sign depends on the rounding mode of
/// the type (the sign of the dividend for `FBig`, the remainder of the least magnitude for
/// `DBig`); it is exact and smaller than the divisor in magnitude, so adding the divisor to a
/// remainder of the other sign gives the floored one (rounded to the precision of the operands
/// if it has more digits, as for `float`).
fn float_rem<R: Round, const B: Word>(lhs: &FBig<R, B>, rhs: &FBig<R, B>) -> PyResult<FBig<R, B>> {
    if rhs.repr().significand().is_zero() && rhs.repr().is_finite() {
        return Err(PyZeroDivisionError::new_err("float modulo by zero"));
    }
    if lhs.repr().is_infinite() || rhs.repr().is_infinite() {
        return Err(PyValueError::new_err(ERRMSG_INFINITY));
    }
    let rem = lhs.rem(rhs);
    if !rem.repr().significand().is_zero() && rem.repr().sign() != rhs.repr().sign() {
        Ok(rem + rhs)
    } else {
        Ok(rem)
    }
}

/// Generate the forward and reverse modulo dispatchers (see [`float_rem`]).
macro_rules! impl_float_mod {
    ($ty:ident, $into:ident, $method:ident, $rev_method:ident) => {
        pub(crate) fn $method(lhs: &$ty, rhs: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
            let rhs = rhs.$into()?;
            $ty(float_rem(&lhs.0, &rhs.0)?).into_py_any(py)
        }
        fn $rev_method(lhs: UniInput<'_>, rhs: &$ty, py: Python<'_>) -> PyResult<Py<PyAny>> {
            let lhs = lhs.$into()?;
            $ty(float_rem(&lhs.0, &rhs.0)?).into_py_any(py)
        }
    };
}

/// Round a float to an integral value (with `trunc`, `floor`, `ceil` or `round`) and convert
/// it to an integer. An infinity raises `OverflowError`, as for Python's `float`.
fn integral_to_ibig<R: Round, const B: Word>(
    f: &FBig<R, B>,
    round: impl FnOnce(&FBig<R, B>) -> FBig<R, B>,
) -> PyResult<IBig> {
    if f.repr().is_infinite() {
        return Err(PyOverflowError::new_err("cannot convert infinity to integer"));
    }
    Ok(round(f).to_int().value())
}

/// Give the result of a rounding method (`trunc`, `floor`, `ceil`, `round`, `fract`) the
/// precision of the number it was computed from, where the Rust method returns it with an
/// unlimited precision (which it does for a zero, and wherever no digit of the number is left).
/// The result has at most as many digits as the source, so this is exact, and the result can be
/// used like the source in the operations that need a limited precision
/// (`FBig(0.5).floor().cos()`).
fn keep_precision<R: Round, const B: Word>(result: FBig<R, B>, source: &FBig<R, B>) -> FBig<R, B> {
    if result.precision() == 0 && source.precision() != 0 && result.repr().is_finite() {
        result.with_precision(source.precision()).value()
    } else {
        result
    }
}

impl_float_binops!(FPy, into_fpy, fpy_add, float_add);
impl_float_binops!(FPy, into_fpy, fpy_sub, fpy_rsub, float_sub);
impl_float_binops!(FPy, into_fpy, fpy_mul, float_mul);
impl_float_binops!(FPy, into_fpy, fpy_div, fpy_rdiv, float_div);
impl_float_mod!(FPy, into_fpy, fpy_mod, fpy_rmod);

impl_float_binops!(DPy, into_dpy, dpy_add, float_add);
impl_float_binops!(DPy, into_dpy, dpy_sub, dpy_rsub, float_sub);
impl_float_binops!(DPy, into_dpy, dpy_mul, float_mul);
impl_float_binops!(DPy, into_dpy, dpy_div, dpy_rdiv, float_div);
impl_float_mod!(DPy, into_dpy, dpy_mod, dpy_rmod);

/// Generate a `__richcmp__` dispatcher over every `UniInput` variant using `num_cmp`.
/// `FPy` and `DPy` order against all numeric inputs identically, so the bodies match.
macro_rules! impl_float_richcmp {
    ($name:ident, $ty:ident) => {
        fn $name(lhs: &$ty, other: UniInput<'_>, op: CompareOp) -> bool {
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
                UniInput::BRational(x) => lhs.0.num_cmp(&x.0),
                UniInput::OBRational(x) => lhs.0.num_cmp(&x),
            };
            op.matches(order)
        }
    };
}

impl_float_richcmp!(fpy_richcmp, FPy);
impl_float_richcmp!(dpy_richcmp, DPy);

/// Generate the unary cache-routed transcendentals as a dedicated `#[pymethods]` block.
///
/// `$($name),+` are the transcendentals whose Python name matches the `Context` method name,
/// so a single token serves as both the method name and the dispatched call. Shared by `FPy`
/// (base-2) and `DPy` (base-10): both expose the same `Context` API, and [`unwrap_float`] is
/// generic over `<R, B>`.
///
/// The macro emits the *entire* `#[pymethods] impl` — a `macro_rules!` invocation placed
/// inside a `#[pymethods]` block is not registered by PyO3, so the attribute must be wrapped
/// by the macro. Every other math method (roots, `powi`, `powf`, `atan2`) has a distinct
/// shape and stays as plain code in each type's impl.
macro_rules! impl_float_math {
    ($ty:ident, $($name:ident),+ $(,)?) => {
        #[pymethods]
        impl $ty {
            $(
                fn $name(&self) -> PyResult<Self> {
                    let ctx = limited(self.0.context())?;
                    let res = with_cache(|c| ctx.$name(self.0.repr(), Some(c)));
                    Ok(Self(unwrap_float(res, ctx)?))
                }
            )+
        }
    };
}

#[pymethods]
impl FPy {
    #[new]
    fn __new__(ob: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(s) = ob.extract::<String>() {
            return Ok(FPy(FBig::from_str(&s).map_err(parse_error_to_py)?));
        }
        if ob.is_instance_of::<PyFloat>() {
            // Represent the float at the current default precision (configurable via
            // `dashu.set_precision`), so that subsequent operations (transcendentals in
            // particular, which require precision > 0) are well-defined.
            let f = crate::utils::fbig_from_f64(ob.extract::<f64>()?)
                .map_err(conversion_error_to_py)?;
            return Ok(FPy(f));
        }
        if let Ok(obj) = ob.extract::<PyRef<Self>>() {
            return Ok(FPy(obj.0.clone()));
        }
        // any other Python number -> permissive construction (Decimal/Fraction accepted)
        Ok(FPy(UniInput::extract(ob.as_borrowed())?.construct_fpy()?))
    }
    fn unwrap(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let (signif, exp) = self.0.repr().clone().into_parts();
        (IPy(signif), exp).into_py_any(py)
    }

    fn __repr__(&self) -> String {
        format!("<FBig {:?}>", self.0)
    }
    fn __str__(&self) -> PyResult<String> {
        // an FBig is base 2: print in hexadecimal (lossless — no base conversion rounding)
        crate::format::format_fbig_hex(&self.0, "")
    }
    fn __format__(&self, format_spec: &str) -> PyResult<String> {
        // default / 'a' / 'A' -> hexadecimal (lossless); decimal presentations convert to base 10
        match crate::format::parse(format_spec)?.ty {
            '\0' | 'a' | 'A' => crate::format::format_fbig_hex(&self.0, format_spec),
            _ => crate::format::format_fbig_decimal(&self.0, format_spec),
        }
    }
    fn __hash__(&self) -> isize {
        crate::hash::hash_fbig(&self.0)
    }
    fn __richcmp__(&self, other: UniInput<'_>, op: CompareOp) -> bool {
        fpy_richcmp(self, other, op)
    }
    fn __bool__(&self) -> bool {
        !(self.0.repr().is_pos_zero() || self.0.repr().is_neg_zero())
    }
    fn __float__(&self) -> f64 {
        self.0.to_f64().value()
    }
    /// `int(x)` truncates toward zero, as for Python's own numbers (whatever the rounding
    /// mode of the type, which `to_int()` follows).
    fn __int__(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let int = integral_to_ibig(&self.0, FBig::trunc)?;
        crate::convert::convert_from_ibig(&int, py)?.into_py_any(py)
    }

    /********** arithmetic **********/
    #[inline]
    fn __add__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        fpy_add(self, other, py)
    }
    #[inline]
    fn __radd__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        fpy_add(self, other, py)
    }
    #[inline]
    fn __sub__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        fpy_sub(self, other, py)
    }
    #[inline]
    fn __rsub__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        fpy_rsub(other, self, py)
    }
    #[inline]
    fn __mul__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        fpy_mul(self, other, py)
    }
    #[inline]
    fn __rmul__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        fpy_mul(self, other, py)
    }
    #[inline]
    fn __truediv__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        fpy_div(self, other, py)
    }
    #[inline]
    fn __rtruediv__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        fpy_rdiv(other, self, py)
    }
    #[inline]
    fn __mod__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        fpy_mod(self, other, py)
    }
    #[inline]
    fn __rmod__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        fpy_rmod(other, self, py)
    }
    #[inline]
    fn __neg__(&self) -> Self {
        FPy(-&self.0)
    }
    #[inline]
    fn __pos__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }
    #[inline]
    fn __abs__(&self) -> Self {
        FPy(self.0.clone().abs())
    }

    /********** predicates & sign **********/
    fn is_zero(&self) -> bool {
        self.0.repr().is_pos_zero() || self.0.repr().is_neg_zero()
    }
    fn is_finite(&self) -> bool {
        self.0.repr().is_finite()
    }
    fn is_infinite(&self) -> bool {
        self.0.repr().is_infinite()
    }
    fn sign(&self) -> PySign {
        self.0.repr().sign().into()
    }

    /********** rounding (algebraic, no cache) **********/
    fn trunc(&self) -> Self {
        FPy(keep_precision(self.0.trunc(), &self.0))
    }
    fn floor(&self) -> Self {
        FPy(keep_precision(self.0.floor(), &self.0))
    }
    fn ceil(&self) -> Self {
        FPy(keep_precision(self.0.ceil(), &self.0))
    }
    fn round(&self) -> Self {
        FPy(keep_precision(self.0.round(), &self.0))
    }
    fn fract(&self) -> Self {
        FPy(keep_precision(self.0.fract(), &self.0))
    }

    /********** Python numeric protocol (dunders; math.floor/ceil/trunc and round() return int) **********/
    fn __trunc__(&self) -> PyResult<IPy> {
        integral_to_ibig(&self.0, FBig::trunc).map(IPy)
    }
    fn __floor__(&self) -> PyResult<IPy> {
        integral_to_ibig(&self.0, FBig::floor).map(IPy)
    }
    fn __ceil__(&self) -> PyResult<IPy> {
        integral_to_ibig(&self.0, FBig::ceil).map(IPy)
    }
    #[pyo3(signature = (ndigits=None))]
    fn __round__(&self, ndigits: Option<i32>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        use pyo3::IntoPyObjectExt;
        match ndigits {
            // round(x) with no digits returns a Python int (CPython contract).
            None => IPy(integral_to_ibig(&self.0, FBig::round)?).into_py_any(py),
            // round(x, n) rounds to n *decimal* places, which is ill-defined for a base-2 float;
            // direct the user to the decimal type instead.
            Some(_) => Err(PyValueError::new_err(
                "round(x, ndigits) rounds to decimal places, which is not supported for the \
                 base-2 FBig; convert to a decimal first (e.g. round(x.to_decimal(), ndigits))",
            )),
        }
    }

    /********** precision & parts **********/
    fn precision(&self) -> usize {
        self.0.precision()
    }
    fn digits(&self) -> usize {
        self.0.digits()
    }
    fn with_precision(&self, precision: usize) -> Self {
        FPy(self.0.clone().with_precision(precision).value())
    }
    #[staticmethod]
    fn from_parts(significand: UniInput<'_>, exponent: isize) -> PyResult<Self> {
        Ok(FPy(FBig::from_parts(significand.into_ibig()?, exponent)))
    }

    /********** conversions **********/
    fn to_int(&self) -> IPy {
        IPy(self.0.to_int().value())
    }
    fn to_decimal(&self) -> DPy {
        DPy(self.0.to_decimal().value())
    }
    fn to_binary(&self) -> Self {
        FPy(self.0.to_binary().value())
    }
    fn to_rational(&self) -> PyResult<RPy> {
        dashu::rational::RBig::try_from(self.0.clone())
            .map(RPy)
            .map_err(conversion_error_to_py)
    }

    /********** roots, powers & atan2 **********/
    fn sqrt(&self) -> PyResult<Self> {
        let ctx = limited(self.0.context())?;
        Ok(Self(unwrap_float(ctx.sqrt(self.0.repr()), ctx)?))
    }
    fn cbrt(&self) -> PyResult<Self> {
        let ctx = limited(self.0.context())?;
        Ok(Self(unwrap_float(ctx.cbrt(self.0.repr()), ctx)?))
    }
    fn nth_root(&self, n: usize) -> PyResult<Self> {
        let ctx = limited(self.0.context())?;
        Ok(Self(unwrap_float(ctx.nth_root(n, self.0.repr()), ctx)?))
    }
    fn powi(&self, n: UniInput<'_>) -> PyResult<Self> {
        let n = n.into_ibig()?;
        // (a non-negative power is exact with an unlimited precision)
        let ctx = if n.sign() == Sign::Negative {
            limited(self.0.context())?
        } else {
            self.0.context()
        };
        Ok(Self(unwrap_float(ctx.powi(self.0.repr(), n), ctx)?))
    }
    fn powf(&self, w: UniInput<'_>) -> PyResult<Self> {
        let w = w.into_fpy()?;
        // at the larger precision of the two operands, as for the arithmetic operators
        let ctx = limited(Context::max(self.0.context(), w.0.context()))?;
        let res = with_cache(|c| ctx.powf(self.0.repr(), w.0.repr(), Some(c)));
        Ok(Self(unwrap_float(res, ctx)?))
    }
    fn atan2(&self, x: UniInput<'_>) -> PyResult<Self> {
        let x = x.into_fpy()?;
        let ctx = limited(Context::max(self.0.context(), x.0.context()))?;
        let res = with_cache(|c| ctx.atan2(self.0.repr(), x.0.repr(), Some(c)));
        Ok(Self(unwrap_float(res, ctx)?))
    }
    fn sin_cos(&self) -> PyResult<(Self, Self)> {
        let ctx = limited(self.0.context())?;
        let (s, c) = with_cache(|cache| ctx.sin_cos(self.0.repr(), Some(cache)));
        Ok((Self(unwrap_float(s, ctx)?), Self(unwrap_float(c, ctx)?)))
    }
    fn sinh_cosh(&self) -> PyResult<(Self, Self)> {
        let ctx = limited(self.0.context())?;
        let (s, c) = with_cache(|cache| ctx.sinh_cosh(self.0.repr(), Some(cache)));
        Ok((Self(unwrap_float(s, ctx)?), Self(unwrap_float(c, ctx)?)))
    }
}

impl_float_math!(
    FPy, sin, cos, tan, asin, acos, atan, sinh, cosh, tanh, asinh, acosh, atanh, exp, exp_m1, ln,
    ln_1p, log2, log10,
);

#[pymethods]
impl DPy {
    #[new]
    fn __new__(ob: &Bound<'_, PyAny>) -> PyResult<Self> {
        if let Ok(s) = ob.extract::<String>() {
            return Ok(DPy(DBig::from_str(&s).map_err(parse_error_to_py)?));
        }
        // decimal.Decimal fast path (preserves the original parse_to_dbig round-trip)
        {
            let py = ob.py();
            let decimal = py.import(intern!(py, "decimal"))?;
            let decimal_type = decimal.getattr(intern!(py, "Decimal"))?;
            if ob.is_instance(&decimal_type)? {
                return Ok(DPy(parse_to_dbig(ob)?));
            }
        }
        if let Ok(obj) = ob.extract::<PyRef<Self>>() {
            return Ok(DPy(obj.0.clone()));
        }
        // any other Python number -> permissive construction (float/FBig accepted)
        Ok(DPy(UniInput::extract(ob.as_borrowed())?.construct_dpy()?))
    }
    fn unwrap(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let decimal = py.import(intern!(py, "decimal"))?;
        let decimal_type = decimal.getattr(intern!(py, "Decimal"))?;
        let decimal_str = format!("{:e}", self.0);
        decimal_type.call1((decimal_str,))?.into_py_any(py)
    }

    fn __repr__(&self) -> String {
        format!("<DBig {:?}>", self.0)
    }
    fn __str__(&self) -> PyResult<String> {
        crate::format::format_dbig(&self.0, "")
    }
    fn __format__(&self, format_spec: &str) -> PyResult<String> {
        crate::format::format_dbig(&self.0, format_spec)
    }
    fn __hash__(&self) -> isize {
        crate::hash::hash_fbig(&self.0)
    }
    fn __richcmp__(&self, other: UniInput<'_>, op: CompareOp) -> bool {
        dpy_richcmp(self, other, op)
    }
    fn __bool__(&self) -> bool {
        !(self.0.repr().is_pos_zero() || self.0.repr().is_neg_zero())
    }
    fn __float__(&self) -> f64 {
        self.0.to_f64().value()
    }
    /// `int(x)` truncates toward zero, as for Python's own numbers (whatever the rounding
    /// mode of the type, which `to_int()` follows).
    fn __int__(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let int = integral_to_ibig(&self.0, FBig::trunc)?;
        crate::convert::convert_from_ibig(&int, py)?.into_py_any(py)
    }

    /********** arithmetic **********/
    #[inline]
    fn __add__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        dpy_add(self, other, py)
    }
    #[inline]
    fn __radd__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        dpy_add(self, other, py)
    }
    #[inline]
    fn __sub__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        dpy_sub(self, other, py)
    }
    #[inline]
    fn __rsub__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        dpy_rsub(other, self, py)
    }
    #[inline]
    fn __mul__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        dpy_mul(self, other, py)
    }
    #[inline]
    fn __rmul__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        dpy_mul(self, other, py)
    }
    #[inline]
    fn __truediv__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        dpy_div(self, other, py)
    }
    #[inline]
    fn __rtruediv__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        dpy_rdiv(other, self, py)
    }
    #[inline]
    fn __mod__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        dpy_mod(self, other, py)
    }
    #[inline]
    fn __rmod__(&self, other: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        dpy_rmod(other, self, py)
    }
    #[inline]
    fn __neg__(&self) -> Self {
        DPy(-&self.0)
    }
    #[inline]
    fn __pos__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }
    #[inline]
    fn __abs__(&self) -> Self {
        DPy(self.0.clone().abs())
    }

    /********** predicates & sign **********/
    fn is_zero(&self) -> bool {
        self.0.repr().is_pos_zero() || self.0.repr().is_neg_zero()
    }
    fn is_finite(&self) -> bool {
        self.0.repr().is_finite()
    }
    fn is_infinite(&self) -> bool {
        self.0.repr().is_infinite()
    }
    fn sign(&self) -> PySign {
        self.0.repr().sign().into()
    }

    /********** rounding (algebraic, no cache) **********/
    fn trunc(&self) -> Self {
        DPy(keep_precision(self.0.trunc(), &self.0))
    }
    fn floor(&self) -> Self {
        DPy(keep_precision(self.0.floor(), &self.0))
    }
    fn ceil(&self) -> Self {
        DPy(keep_precision(self.0.ceil(), &self.0))
    }
    fn round(&self) -> Self {
        DPy(keep_precision(self.0.round(), &self.0))
    }
    fn fract(&self) -> Self {
        DPy(keep_precision(self.0.fract(), &self.0))
    }

    /********** Python numeric protocol (dunders; math.floor/ceil/trunc and round() return int) **********/
    fn __trunc__(&self) -> PyResult<IPy> {
        integral_to_ibig(&self.0, FBig::trunc).map(IPy)
    }
    fn __floor__(&self) -> PyResult<IPy> {
        integral_to_ibig(&self.0, FBig::floor).map(IPy)
    }
    fn __ceil__(&self) -> PyResult<IPy> {
        integral_to_ibig(&self.0, FBig::ceil).map(IPy)
    }
    #[pyo3(signature = (ndigits=None))]
    fn __round__(&self, ndigits: Option<i32>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        use pyo3::IntoPyObjectExt;
        match ndigits {
            // round(x) with no digits returns a Python int (CPython contract).
            None => IPy(integral_to_ibig(&self.0, FBig::round)?).into_py_any(py),
            // round(x, n): quantize to the 10^(-n) digit place (Python's Decimal.quantize).
            Some(n) => {
                let rounded = self.0.quantize(-(n as isize)).value();
                DPy(keep_precision(rounded, &self.0)).into_py_any(py)
            }
        }
    }

    /********** precision & parts **********/
    fn precision(&self) -> usize {
        self.0.precision()
    }
    fn digits(&self) -> usize {
        self.0.digits()
    }
    fn with_precision(&self, precision: usize) -> Self {
        DPy(self.0.clone().with_precision(precision).value())
    }
    #[staticmethod]
    fn from_parts(significand: UniInput<'_>, exponent: isize) -> PyResult<Self> {
        Ok(DPy(DBig::from_parts(significand.into_ibig()?, exponent)))
    }

    /********** conversions **********/
    fn to_int(&self) -> IPy {
        IPy(self.0.to_int().value())
    }
    fn to_decimal(&self) -> Self {
        DPy(self.0.to_decimal().value())
    }
    fn to_binary(&self) -> FPy {
        FPy(self.0.to_binary().value())
    }
    fn to_rational(&self) -> PyResult<RPy> {
        dashu::rational::RBig::try_from(self.0.clone())
            .map(RPy)
            .map_err(conversion_error_to_py)
    }

    /********** roots, powers & atan2 **********/
    fn sqrt(&self) -> PyResult<Self> {
        let ctx = limited(self.0.context())?;
        Ok(Self(unwrap_float(ctx.sqrt(self.0.repr()), ctx)?))
    }
    fn cbrt(&self) -> PyResult<Self> {
        let ctx = limited(self.0.context())?;
        Ok(Self(unwrap_float(ctx.cbrt(self.0.repr()), ctx)?))
    }
    fn nth_root(&self, n: usize) -> PyResult<Self> {
        let ctx = limited(self.0.context())?;
        Ok(Self(unwrap_float(ctx.nth_root(n, self.0.repr()), ctx)?))
    }
    fn powi(&self, n: UniInput<'_>) -> PyResult<Self> {
        let n = n.into_ibig()?;
        // (a non-negative power is exact with an unlimited precision)
        let ctx = if n.sign() == Sign::Negative {
            limited(self.0.context())?
        } else {
            self.0.context()
        };
        Ok(Self(unwrap_float(ctx.powi(self.0.repr(), n), ctx)?))
    }
    fn powf(&self, w: UniInput<'_>) -> PyResult<Self> {
        let w = w.into_dpy()?;
        // at the larger precision of the two operands, as for the arithmetic operators
        let ctx = limited(Context::max(self.0.context(), w.0.context()))?;
        let res = with_cache(|c| ctx.powf(self.0.repr(), w.0.repr(), Some(c)));
        Ok(Self(unwrap_float(res, ctx)?))
    }
    fn atan2(&self, x: UniInput<'_>) -> PyResult<Self> {
        let x = x.into_dpy()?;
        let ctx = limited(Context::max(self.0.context(), x.0.context()))?;
        let res = with_cache(|c| ctx.atan2(self.0.repr(), x.0.repr(), Some(c)));
        Ok(Self(unwrap_float(res, ctx)?))
    }
    fn sin_cos(&self) -> PyResult<(Self, Self)> {
        let ctx = limited(self.0.context())?;
        let (s, c) = with_cache(|cache| ctx.sin_cos(self.0.repr(), Some(cache)));
        Ok((Self(unwrap_float(s, ctx)?), Self(unwrap_float(c, ctx)?)))
    }
    fn sinh_cosh(&self) -> PyResult<(Self, Self)> {
        let ctx = limited(self.0.context())?;
        let (s, c) = with_cache(|cache| ctx.sinh_cosh(self.0.repr(), Some(cache)));
        Ok((Self(unwrap_float(s, ctx)?), Self(unwrap_float(c, ctx)?)))
    }
}

impl_float_math!(
    DPy, sin, cos, tan, asin, acos, atan, sinh, cosh, tanh, asinh, acosh, atanh, exp, exp_m1, ln,
    ln_1p, log2, log10,
);

#[cfg(feature = "zeroize")]
#[pymethods]
impl FPy {
    /// Zeroize the internal buffers, clearing the memory used by this float. The value
    /// becomes zero, and keeps its precision.
    fn zeroize(&mut self) {
        use zeroize::Zeroize;
        // (the precision is zeroized with the buffers, which would leave a zero of unlimited
        // precision behind)
        let context = self.0.context();
        self.0.zeroize();
        self.0 = FBig::from_repr(dashu::float::Repr::zero(), context);
    }
}

#[cfg(feature = "zeroize")]
#[pymethods]
impl DPy {
    /// Zeroize the internal buffers, clearing the memory used by this float. The value
    /// becomes zero, and keeps its precision.
    fn zeroize(&mut self) {
        use zeroize::Zeroize;
        // (the precision is zeroized with the buffers, which would leave a zero of unlimited
        // precision behind)
        let context = self.0.context();
        self.0.zeroize();
        self.0 = FBig::from_repr(dashu::float::Repr::zero(), context);
    }
}
