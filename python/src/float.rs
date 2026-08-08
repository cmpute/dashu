use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::ops::{Add, Div, Mul, Rem, Sub};
use std::str::FromStr;

use dashu::base::Abs;
use dashu::float::{DBig, FBig};
use num_order::{NumHash, NumOrd};
use pyo3::{
    Bound, IntoPyObjectExt, Py, PyAny, PyResult, basic::CompareOp, intern, prelude::*,
    types::PyFloat,
};

use crate::{
    cache::{unwrap_float, with_cache},
    convert::{conversion_error_to_py, parse_error_to_py, parse_to_dbig},
    types::{DPy, FPy, IPy, PySign, RPy, UniInput},
};

/// Generate forward (and optional reverse) arithmetic dispatchers that first coerce the
/// operand to the wrapper's own type via the `into_*` helper, then apply the Rust operator.
macro_rules! impl_float_binops {
    ($ty:ident, $into:ident, $method:ident, $rs_method:ident) => {
        fn $method(lhs: &$ty, rhs: UniInput<'_>, py: Python<'_>) -> PyResult<Py<PyAny>> {
            let rhs = rhs.$into()?;
            $ty((&lhs.0).$rs_method(&rhs.0)).into_py_any(py)
        }
    };
    ($ty:ident, $into:ident, $method:ident, $rev_method:ident, $rs_method:ident) => {
        impl_float_binops!($ty, $into, $method, $rs_method);
        fn $rev_method(lhs: UniInput<'_>, rhs: &$ty, py: Python<'_>) -> PyResult<Py<PyAny>> {
            let lhs = lhs.$into()?;
            $ty((&lhs.0).$rs_method(&rhs.0)).into_py_any(py)
        }
    };
}

impl_float_binops!(FPy, into_fpy, fpy_add, add);
impl_float_binops!(FPy, into_fpy, fpy_sub, fpy_rsub, sub);
impl_float_binops!(FPy, into_fpy, fpy_mul, mul);
impl_float_binops!(FPy, into_fpy, fpy_div, fpy_rdiv, div);
impl_float_binops!(FPy, into_fpy, fpy_mod, fpy_rmod, rem);

impl_float_binops!(DPy, into_dpy, dpy_add, add);
impl_float_binops!(DPy, into_dpy, dpy_sub, dpy_rsub, sub);
impl_float_binops!(DPy, into_dpy, dpy_mul, mul);
impl_float_binops!(DPy, into_dpy, dpy_div, dpy_rdiv, div);
impl_float_binops!(DPy, into_dpy, dpy_mod, dpy_rmod, rem);

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
                    let ctx = self.0.context();
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
            _ => crate::format::format_dbig(&self.0.to_decimal().value(), format_spec),
        }
    }
    fn __hash__(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.0.num_hash(&mut hasher);
        hasher.finish()
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
    fn __int__(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        crate::convert::convert_from_ibig(&self.0.to_int().value(), py)?.into_py_any(py)
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
        FPy(self.0.trunc())
    }
    fn floor(&self) -> Self {
        FPy(self.0.floor())
    }
    fn ceil(&self) -> Self {
        FPy(self.0.ceil())
    }
    fn round(&self) -> Self {
        FPy(self.0.round())
    }
    fn fract(&self) -> Self {
        FPy(self.0.fract())
    }

    /********** Python numeric protocol (dunders; math.floor/ceil/trunc and round() return int) **********/
    fn __trunc__(&self) -> IPy {
        IPy(self.0.trunc().to_int().value())
    }
    fn __floor__(&self) -> IPy {
        IPy(self.0.floor().to_int().value())
    }
    fn __ceil__(&self) -> IPy {
        IPy(self.0.ceil().to_int().value())
    }
    #[pyo3(signature = (ndigits=None))]
    fn __round__(&self, ndigits: Option<i32>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        use pyo3::IntoPyObjectExt;
        match ndigits {
            // round(x) with no digits returns a Python int (CPython contract).
            None => IPy(self.0.round().to_int().value()).into_py_any(py),
            // round(x, n) rounds to n *decimal* places, which is ill-defined for a base-2 float;
            // direct the user to the decimal type instead.
            Some(_) => Err(pyo3::exceptions::PyValueError::new_err(
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
        let ctx = self.0.context();
        Ok(Self(unwrap_float(ctx.sqrt(self.0.repr()), ctx)?))
    }
    fn cbrt(&self) -> PyResult<Self> {
        let ctx = self.0.context();
        Ok(Self(unwrap_float(ctx.cbrt(self.0.repr()), ctx)?))
    }
    fn nth_root(&self, n: usize) -> PyResult<Self> {
        let ctx = self.0.context();
        Ok(Self(unwrap_float(ctx.nth_root(n, self.0.repr()), ctx)?))
    }
    fn powi(&self, n: UniInput<'_>) -> PyResult<Self> {
        let n = n.into_ibig()?;
        let ctx = self.0.context();
        Ok(Self(unwrap_float(ctx.powi(self.0.repr(), n), ctx)?))
    }
    fn powf(&self, w: UniInput<'_>) -> PyResult<Self> {
        let w = w.into_fpy()?;
        let ctx = self.0.context();
        let res = with_cache(|c| ctx.powf(self.0.repr(), w.0.repr(), Some(c)));
        Ok(Self(unwrap_float(res, ctx)?))
    }
    fn atan2(&self, x: UniInput<'_>) -> PyResult<Self> {
        let x = x.into_fpy()?;
        let ctx = self.0.context();
        let res = with_cache(|c| ctx.atan2(self.0.repr(), x.0.repr(), Some(c)));
        Ok(Self(unwrap_float(res, ctx)?))
    }
    fn sin_cos(&self) -> PyResult<(Self, Self)> {
        let ctx = self.0.context();
        let (s, c) = with_cache(|cache| ctx.sin_cos(self.0.repr(), Some(cache)));
        Ok((Self(unwrap_float(s, ctx)?), Self(unwrap_float(c, ctx)?)))
    }
    fn sinh_cosh(&self) -> PyResult<(Self, Self)> {
        let ctx = self.0.context();
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
    fn __hash__(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.0.num_hash(&mut hasher);
        hasher.finish()
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
    fn __int__(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        crate::convert::convert_from_ibig(&self.0.to_int().value(), py)?.into_py_any(py)
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
        DPy(self.0.trunc())
    }
    fn floor(&self) -> Self {
        DPy(self.0.floor())
    }
    fn ceil(&self) -> Self {
        DPy(self.0.ceil())
    }
    fn round(&self) -> Self {
        DPy(self.0.round())
    }
    fn fract(&self) -> Self {
        DPy(self.0.fract())
    }

    /********** Python numeric protocol (dunders; math.floor/ceil/trunc and round() return int) **********/
    fn __trunc__(&self) -> IPy {
        IPy(self.0.trunc().to_int().value())
    }
    fn __floor__(&self) -> IPy {
        IPy(self.0.floor().to_int().value())
    }
    fn __ceil__(&self) -> IPy {
        IPy(self.0.ceil().to_int().value())
    }
    #[pyo3(signature = (ndigits=None))]
    fn __round__(&self, ndigits: Option<i32>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        use pyo3::IntoPyObjectExt;
        match ndigits {
            // round(x) with no digits returns a Python int (CPython contract).
            None => IPy(self.0.round().to_int().value()).into_py_any(py),
            // round(x, n): quantize to the 10^(-n) digit place (Python's Decimal.quantize).
            Some(n) => DPy(self.0.quantize(-(n as isize)).value()).into_py_any(py),
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
        let ctx = self.0.context();
        Ok(Self(unwrap_float(ctx.sqrt(self.0.repr()), ctx)?))
    }
    fn cbrt(&self) -> PyResult<Self> {
        let ctx = self.0.context();
        Ok(Self(unwrap_float(ctx.cbrt(self.0.repr()), ctx)?))
    }
    fn nth_root(&self, n: usize) -> PyResult<Self> {
        let ctx = self.0.context();
        Ok(Self(unwrap_float(ctx.nth_root(n, self.0.repr()), ctx)?))
    }
    fn powi(&self, n: UniInput<'_>) -> PyResult<Self> {
        let n = n.into_ibig()?;
        let ctx = self.0.context();
        Ok(Self(unwrap_float(ctx.powi(self.0.repr(), n), ctx)?))
    }
    fn powf(&self, w: UniInput<'_>) -> PyResult<Self> {
        let w = w.into_dpy()?;
        let ctx = self.0.context();
        let res = with_cache(|c| ctx.powf(self.0.repr(), w.0.repr(), Some(c)));
        Ok(Self(unwrap_float(res, ctx)?))
    }
    fn atan2(&self, x: UniInput<'_>) -> PyResult<Self> {
        let x = x.into_dpy()?;
        let ctx = self.0.context();
        let res = with_cache(|c| ctx.atan2(self.0.repr(), x.0.repr(), Some(c)));
        Ok(Self(unwrap_float(res, ctx)?))
    }
    fn sin_cos(&self) -> PyResult<(Self, Self)> {
        let ctx = self.0.context();
        let (s, c) = with_cache(|cache| ctx.sin_cos(self.0.repr(), Some(cache)));
        Ok((Self(unwrap_float(s, ctx)?), Self(unwrap_float(c, ctx)?)))
    }
    fn sinh_cosh(&self) -> PyResult<(Self, Self)> {
        let ctx = self.0.context();
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
    /// becomes zero.
    fn zeroize(&mut self) {
        use zeroize::Zeroize;
        self.0.zeroize();
    }
}

#[cfg(feature = "zeroize")]
#[pymethods]
impl DPy {
    /// Zeroize the internal buffers, clearing the memory used by this float. The value
    /// becomes zero.
    fn zeroize(&mut self) {
        use zeroize::Zeroize;
        self.0.zeroize();
    }
}
