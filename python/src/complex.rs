//! Python bindings for arbitrary-precision complex numbers ([`dashu::complex::CBig`]).
//!
//! `CPy` wraps a *bare* `CBig<Zero, 2>`. Transcendentals route through the module-global
//! [`ConstCache`](dashu::float::ConstCache) + the panic-free complex `Context` layer (see
//! [`crate::cache::unwrap_complex`]); `abs`/`arg`/`norm` route through the float layer.
//! Arithmetic accepts any real number (FPy/int/float/Decimal/Python complex) by promoting it
//! to a complex with zero imaginary part, so `CBig(1, 0) + 2.0` works as expected.

use std::ops::{Add, Div, Mul, Sub};
use std::str::FromStr;

use dashu::complex::CBig;
use dashu::float::FBig;
use pyo3::{
    Bound, IntoPyObjectExt, Py, PyAny, PyResult,
    basic::CompareOp,
    exceptions::{PyTypeError, PyValueError, PyZeroDivisionError},
    intern,
    prelude::*,
    types::{PyComplex, PyFloat, PyInt, PyString},
};

use crate::{
    cache::{limited_complex, unwrap_complex, unwrap_float, with_cache},
    convert::{conversion_error_to_py, parse_error_to_py, parse_to_ibig, parse_to_long},
    types::{CPy, DPy, FPy, IPy, UPy, UniInput},
};

/// Promote any real Python number (or a `(re, im)` pair / Python `complex`) to a bare `CBig`.
fn to_cbig(ob: &Bound<'_, PyAny>) -> PyResult<CBig> {
    if let Ok(c) = ob.extract::<PyRef<CPy>>() {
        return Ok(c.0.clone());
    }
    if let Ok(f) = ob.extract::<PyRef<FPy>>() {
        return Ok(CBig::from(f.0.clone()));
    }
    if let Ok(u) = ob.extract::<PyRef<UPy>>() {
        return Ok(CBig::from(dashu::integer::IBig::from(u.0.clone())));
    }
    if let Ok(i) = ob.extract::<PyRef<IPy>>() {
        return Ok(CBig::from(i.0.clone()));
    }
    if let Ok(d) = ob.extract::<PyRef<DPy>>() {
        // base 10 -> base 2 via the correctly-rounded conversion, then embed as z + 0i
        return Ok(CBig::from(d.0.to_binary().value()));
    }
    if ob.is_instance_of::<PyComplex>() {
        let re: f64 = ob.getattr(intern!(ob.py(), "real"))?.extract()?;
        let im: f64 = ob.getattr(intern!(ob.py(), "imag"))?.extract()?;
        let re = crate::utils::fbig_from_f64(re).map_err(conversion_error_to_py)?;
        let im = crate::utils::fbig_from_f64(im).map_err(conversion_error_to_py)?;
        return Ok(CBig::from_parts(re, im));
    }
    if ob.is_instance_of::<PyInt>() {
        let (v, overflow) = parse_to_long(ob)?;
        let i = if overflow {
            parse_to_ibig(ob)?
        } else {
            v.into()
        };
        return Ok(CBig::from(i));
    }
    if ob.is_instance_of::<PyFloat>() {
        let f: f64 = ob.extract()?;
        let f = crate::utils::fbig_from_f64(f).map_err(conversion_error_to_py)?;
        return Ok(CBig::from(f));
    }
    if let Ok(s) = ob.extract::<String>() {
        return CBig::from_str(&s).map_err(parse_error_to_py);
    }
    Err(PyTypeError::new_err("expected a CBig, float, int, complex, or string"))
}

/// Exact equality of a real part with a real number.
fn real_eq(re: &dashu::float::Repr<2>, real: UniInput<'_>) -> bool {
    use core::cmp::Ordering::Equal;
    use num_order::NumOrd;
    let re = dashu::float::FBig::<dashu::float::round::mode::Zero, 2>::from_repr(
        re.clone(),
        dashu::float::Context::new(0),
    );
    match real {
        UniInput::Uint(x) => re.num_cmp(&x) == Equal,
        UniInput::Int(x) => re.num_cmp(&x) == Equal,
        UniInput::BUint(x) => re.num_cmp(&x.0) == Equal,
        UniInput::BInt(x) => re.num_cmp(&x.0) == Equal,
        UniInput::OBInt(x) => re.num_cmp(&x) == Equal,
        UniInput::Float(x) => re.num_cmp(&x) == Equal,
        UniInput::BFloat(x) => re.num_cmp(&x.0) == Equal,
        UniInput::BDecimal(x) => re.num_cmp(&x.0) == Equal,
        UniInput::OBDecimal(x) => re.num_cmp(&x) == Equal,
        UniInput::BRational(x) => re.num_cmp(&x.0) == Equal,
        UniInput::OBRational(x) => re.num_cmp(&x) == Equal,
    }
}

/// Arithmetic with an infinite operand raises `ValueError`, as for the floats (the Rust
/// operators panic on it).
fn check_finite(lhs: &CBig, rhs: &CBig) -> PyResult<()> {
    if lhs.is_infinite() || rhs.is_infinite() {
        Err(PyValueError::new_err("arithmetic with infinity"))
    } else {
        Ok(())
    }
}
fn cbig_add(lhs: &CBig, rhs: &CBig) -> PyResult<CBig> {
    check_finite(lhs, rhs)?;
    Ok(lhs.add(rhs))
}
fn cbig_sub(lhs: &CBig, rhs: &CBig) -> PyResult<CBig> {
    check_finite(lhs, rhs)?;
    Ok(lhs.sub(rhs))
}
fn cbig_mul(lhs: &CBig, rhs: &CBig) -> PyResult<CBig> {
    check_finite(lhs, rhs)?;
    Ok(lhs.mul(rhs))
}
/// The quotient of two complex numbers. A non-zero number divided by zero is an infinity,
/// `0 / 0` raises `ZeroDivisionError` (it is indeterminate), and two operands of unlimited
/// precision raise `ValueError` (the quotient has to be rounded to some precision).
fn cbig_div(lhs: &CBig, rhs: &CBig) -> PyResult<CBig> {
    check_finite(lhs, rhs)?;
    if rhs.is_zero() && lhs.is_zero() {
        return Err(PyZeroDivisionError::new_err("indeterminate form (0/0)"));
    }
    if !rhs.is_zero() {
        limited_complex(dashu::complex::Context::max(lhs.context(), rhs.context()))?;
    }
    Ok(lhs.div(rhs))
}

macro_rules! impl_cpy_binops {
    ($method:ident, $op:ident) => {
        fn $method(lhs: &CPy, rhs: &Bound<'_, PyAny>, py: Python<'_>) -> PyResult<Py<PyAny>> {
            let rhs = to_cbig(rhs)?;
            CPy($op(&lhs.0, &rhs)?).into_py_any(py)
        }
    };
    ($method:ident, $rev_method:ident, $op:ident) => {
        impl_cpy_binops!($method, $op);
        fn $rev_method(lhs: &Bound<'_, PyAny>, rhs: &CPy, py: Python<'_>) -> PyResult<Py<PyAny>> {
            let lhs = to_cbig(lhs)?;
            CPy($op(&lhs, &rhs.0)?).into_py_any(py)
        }
    };
}

impl_cpy_binops!(cpy_add, cbig_add);
impl_cpy_binops!(cpy_sub, cpy_rsub, cbig_sub);
impl_cpy_binops!(cpy_mul, cbig_mul);
impl_cpy_binops!(cpy_div, cpy_rdiv, cbig_div);

/// Generate the complex unary cache-routed transcendentals as a dedicated `#[pymethods]`
/// block.
///
/// Each `(name => ctx)` pair maps the Python method name to the `Context` method — they
/// coincide except for `ln`, which dispatches to `Context::log`. The macro wraps the whole
/// `#[pymethods] impl` (rather than placing a `macro_rules!` call inside one) so PyO3
/// registers the generated methods. The remaining math methods (`sqrt` / `powi` / `powf`)
/// have distinct shapes and are plain code in the `CPy` impl.
macro_rules! impl_cpy_math {
    ($(($name:ident => $ctx:ident)),+ $(,)?) => {
        #[pymethods]
        impl CPy {
            $(
                fn $name(&self) -> PyResult<Self> {
                    let ctx = limited_complex(self.0.context())?;
                    let res = with_cache(|c| ctx.$ctx(&self.0, Some(c)));
                    Ok(Self(unwrap_complex(res, ctx)?))
                }
            )+
        }
    };
}

impl_cpy_math!(
    (sin => sin),
    (cos => cos),
    (tan => tan),
    (exp => exp),
    (ln => log),
    (sinh => sinh),
    (cosh => cosh),
    (tanh => tanh),
    (asinh => asinh),
    (acosh => acosh),
    (atanh => atanh),
);

#[pymethods]
impl CPy {
    #[new]
    #[pyo3(signature = (re, im=None))]
    fn __new__(re: &Bound<'_, PyAny>, im: Option<&Bound<'_, PyAny>>) -> PyResult<Self> {
        // CBig(re, im) is `re + im*j`, as for Python's `complex(re, im)`. Usually both
        // arguments are real and become the two parts; an argument that is itself complex
        // contributes both of its parts.
        if let Some(im_ob) = im {
            let (re_re, re_im) = to_cbig(re)?.into_parts();
            let (im_re, im_im) = to_cbig(im_ob)?.into_parts();
            let is_zero = |f: &FBig| f.repr().significand().is_zero() && f.repr().is_finite();
            if is_zero(&re_im) && is_zero(&im_im) {
                return Ok(CPy(CBig::from_parts(re_re, im_re)));
            }

            // (a + bj) + (c + dj)j = (a - d) + (b + c)j, computed exactly: the precision is
            // that of the most precise part, or more if a sum has more digits than that
            let parts = [&re_re, &re_im, &im_re, &im_im];
            if parts.iter().any(|f| f.repr().is_infinite()) {
                return Err(PyValueError::new_err("arithmetic with infinity"));
            }
            let unlimited = parts.iter().any(|f| f.precision() == 0);
            let precision = parts.iter().map(|f| f.precision()).max().unwrap_or(0);
            let exact = |f: FBig| f.with_precision(0).value();
            let rounded = |f: FBig| {
                let precision = if unlimited {
                    0
                } else {
                    precision.max(f.digits())
                };
                f.with_precision(precision).value()
            };
            let real = rounded(exact(re_re) - exact(im_im));
            let imag = rounded(exact(re_im) + exact(im_re));
            return Ok(CPy(CBig::from_parts(real, imag)));
        }
        // single argument forms
        if let Ok(s) = re.extract::<String>() {
            return Ok(CPy(CBig::from_str(&s).map_err(parse_error_to_py)?));
        }
        // (re, im) pair of binary floats
        if let Ok(t) = re.extract::<(PyRef<FPy>, PyRef<FPy>)>() {
            return Ok(CPy(CBig::from_parts(t.0.0.clone(), t.1.0.clone())));
        }
        if let Ok(c) = re.extract::<PyRef<Self>>() {
            return Ok(CPy(c.0.clone()));
        }
        // any real number / Python complex -> promote
        Ok(CPy(to_cbig(re)?))
    }

    fn __repr__(&self) -> String {
        format!("<CBig {:?}>", self.0)
    }
    fn __str__(&self) -> String {
        format!("{}", self.0)
    }
    fn __format__(&self, format_spec: &str) -> PyResult<String> {
        if format_spec.is_empty() {
            return Ok(format!("{}", self.0));
        }
        let (re, im) = self.0.clone().into_parts();
        let re_s = crate::format::format_fbig_decimal(&re, format_spec)?;
        let mut im_s = crate::format::format_fbig_decimal(&im, format_spec)?;
        if !(im_s.starts_with('-') || im_s.starts_with('+')) {
            im_s = format!("+{im_s}");
        }
        Ok(format!("({re_s}{im_s}j)"))
    }
    fn __hash__(&self) -> isize {
        crate::hash::hash_complex(self.0.re(), self.0.im())
    }
    fn __bool__(&self) -> bool {
        !self.0.is_zero()
    }
    /// Complex numbers have no ordering — only `==` and `!=` are defined.
    ///
    /// Comparing with an object that is not a number (`None`, a string, ...) is not an error:
    /// the comparison returns `NotImplemented`, so that `==` is `False` and `!=` is `True`
    /// (unless the other object knows how to compare itself with a `CBig`).
    fn __richcmp__(
        &self,
        other: &Bound<'_, PyAny>,
        op: CompareOp,
        py: Python<'_>,
    ) -> PyResult<Py<PyAny>> {
        if !matches!(op, CompareOp::Eq | CompareOp::Ne) {
            return Err(PyTypeError::new_err(
                "no ordering relation is defined for complex numbers",
            ));
        }
        // a string is not a number here, although the constructor can parse one
        let other = if other.is_instance_of::<PyString>() {
            None
        } else if let Ok(real) = UniInput::extract(other.as_borrowed()) {
            // a real number of any type: compare exactly (no conversion to a binary float)
            let is_zero = |r: &dashu::float::Repr<2>| r.significand().is_zero() && r.is_finite();
            let equal = is_zero(self.0.im()) && real_eq(self.0.re(), real);
            return (equal == matches!(op, CompareOp::Eq)).into_py_any(py);
        } else {
            // a `CBig` or a Python `complex`
            to_cbig(other).ok()
        };
        match other {
            Some(other) => ((self.0 == other) == matches!(op, CompareOp::Eq)).into_py_any(py),
            None => Ok(py.NotImplemented()),
        }
    }

    fn __neg__(&self) -> Self {
        CPy(-&self.0)
    }
    #[inline]
    fn __pos__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    /********** arithmetic **********/
    #[inline]
    fn __add__(&self, other: &Bound<'_, PyAny>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        cpy_add(self, other, py)
    }
    #[inline]
    fn __radd__(&self, other: &Bound<'_, PyAny>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        cpy_add(self, other, py)
    }
    #[inline]
    fn __sub__(&self, other: &Bound<'_, PyAny>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        cpy_sub(self, other, py)
    }
    #[inline]
    fn __rsub__(&self, other: &Bound<'_, PyAny>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        cpy_rsub(other, self, py)
    }
    #[inline]
    fn __mul__(&self, other: &Bound<'_, PyAny>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        cpy_mul(self, other, py)
    }
    #[inline]
    fn __rmul__(&self, other: &Bound<'_, PyAny>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        cpy_mul(self, other, py)
    }
    #[inline]
    fn __truediv__(&self, other: &Bound<'_, PyAny>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        cpy_div(self, other, py)
    }
    #[inline]
    fn __rtruediv__(&self, other: &Bound<'_, PyAny>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        cpy_rdiv(other, self, py)
    }

    /********** accessors & predicates **********/
    fn real(&self) -> FPy {
        FPy(self.0.clone().into_parts().0)
    }
    fn imag(&self) -> FPy {
        FPy(self.0.clone().into_parts().1)
    }
    fn precision(&self) -> usize {
        self.0.precision()
    }
    fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
    fn is_finite(&self) -> bool {
        self.0.is_finite()
    }
    fn is_infinite(&self) -> bool {
        self.0.is_infinite()
    }

    /********** complex-specific (algebraic) **********/
    fn conj(&self) -> Self {
        CPy(self.0.conj())
    }
    fn proj(&self) -> Self {
        CPy(self.0.proj())
    }
    /// Squared modulus (algebraic, exact): re² + im².
    fn norm(&self) -> PyResult<FPy> {
        let ctx = self.0.context();
        let res = ctx.norm(&self.0);
        Ok(FPy(unwrap_float(res, ctx_to_float(&ctx))?))
    }
    /// Modulus |z| (algebraic — hypot; routed through the float Context layer for clean errors).
    fn abs(&self) -> PyResult<FPy> {
        let ctx = limited_complex(self.0.context())?;
        let res = ctx.abs(&self.0);
        Ok(FPy(unwrap_float(res, ctx_to_float(&ctx))?))
    }
    /// Argument (phase) of z.
    fn arg(&self) -> PyResult<FPy> {
        let ctx = limited_complex(self.0.context())?;
        let res = with_cache(|c| ctx.arg(&self.0, Some(c)));
        Ok(FPy(unwrap_float(res, ctx_to_float(&ctx))?))
    }

    /// Convert to a native Python `complex` (lossy, via f64).
    fn __complex__(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let (re, im) = self.0.clone().into_parts();
        let re = re.to_f64().value();
        let im = im.to_f64().value();
        PyComplex::from_doubles(py, re, im).into_py_any(py)
    }

    #[staticmethod]
    fn from_parts(re: &FPy, im: &FPy) -> Self {
        CPy(CBig::from_parts(re.0.clone(), im.0.clone()))
    }

    /********** roots, powers & atan2 **********/
    fn sqrt(&self) -> PyResult<Self> {
        let ctx = limited_complex(self.0.context())?;
        Ok(Self(unwrap_complex(ctx.sqrt(&self.0), ctx)?))
    }
    fn powi(&self, n: UniInput<'_>) -> PyResult<Self> {
        let n = n.into_ibig()?;
        // (a non-negative power is exact with an unlimited precision)
        let ctx = if n.sign() == dashu::base::Sign::Negative {
            limited_complex(self.0.context())?
        } else {
            self.0.context()
        };
        Ok(Self(unwrap_complex(ctx.powi(&self.0, n), ctx)?))
    }
    fn powf(&self, w: &Self) -> PyResult<Self> {
        let ctx = limited_complex(self.0.context())?;
        let res = with_cache(|c| ctx.powf(&self.0, &w.0, Some(c)));
        Ok(Self(unwrap_complex(res, ctx)?))
    }
    // the unary cache-routed transcendentals (sin/cos/tan/exp/ln) are generated by
    // `impl_cpy_math!` above in a dedicated `#[pymethods]` block.
}

#[cfg(feature = "zeroize")]
#[pymethods]
impl CPy {
    /// Zeroize the internal buffers, clearing the memory used by this complex number. The
    /// value becomes zero, and keeps its precision.
    fn zeroize(&mut self) {
        use zeroize::Zeroize;
        // (the precision is zeroized with the buffers; keep it, as for the floats)
        let context = self.0.context();
        self.0.zeroize();
        let zero = dashu::float::Repr::zero;
        self.0 = CBig::new(zero(), zero(), context);
    }
}

/// The complex `Context<R>` wraps a float `Context<R>`; recover it for the real-valued
/// `abs`/`arg`/`norm` results that go through [`crate::cache::unwrap_float`].
fn ctx_to_float<R: dashu::float::round::Round>(
    ctx: &dashu::complex::Context<R>,
) -> dashu::float::Context<R> {
    dashu::float::Context::new(ctx.precision())
}
