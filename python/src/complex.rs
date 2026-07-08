//! Python bindings for arbitrary-precision complex numbers ([`dashu_cmplx::CBig`]).
//!
//! `CPy` wraps a *bare* `CBig<Zero, 2>`. Transcendentals route through the module-global
//! [`ConstCache`](dashu_float::ConstCache) + the panic-free complex `Context` layer (see
//! [`crate::cache::unwrap_complex`]); `abs`/`arg`/`norm` route through the float layer.
//! Arithmetic accepts any real number (FPy/int/float/Decimal/Python complex) by promoting it
//! to a complex with zero imaginary part, so `CBig(1, 0) + 2.0` works as expected.

use std::ops::{Add, Div, Mul, Sub};
use std::str::FromStr;

use dashu_cmplx::CBig;
use dashu_float::FBig;
use pyo3::{
    Bound, IntoPyObjectExt, Py, PyAny, PyResult,
    basic::CompareOp,
    exceptions::PyTypeError,
    intern,
    prelude::*,
    types::{PyComplex, PyFloat, PyInt},
};

use crate::{
    cache::{unwrap_complex, unwrap_float, with_cache},
    convert::{conversion_error_to_py, parse_error_to_py, parse_to_ibig, parse_to_long},
    types::{CPy, DPy, FPy, UniInput},
};

/// Promote any real Python number (or a `(re, im)` pair / Python `complex`) to a bare `CBig`.
fn to_cbig(ob: &Bound<'_, PyAny>) -> PyResult<CBig> {
    if let Ok(c) = ob.extract::<PyRef<CPy>>() {
        return Ok(c.0.clone());
    }
    if let Ok(f) = ob.extract::<PyRef<FPy>>() {
        return Ok(CBig::from(f.0.clone()));
    }
    if let Ok(d) = ob.extract::<PyRef<DPy>>() {
        // base 10 -> base 2 via the correctly-rounded conversion, then embed as z + 0i
        return Ok(CBig::from(d.0.to_binary().value()));
    }
    if ob.is_instance_of::<PyComplex>() {
        let re: f64 = ob.getattr(intern!(ob.py(), "real"))?.extract()?;
        let im: f64 = ob.getattr(intern!(ob.py(), "imag"))?.extract()?;
        let re = FBig::try_from(re).map_err(conversion_error_to_py)?;
        let im = FBig::try_from(im).map_err(conversion_error_to_py)?;
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
        let f = FBig::try_from(f).map_err(conversion_error_to_py)?;
        return Ok(CBig::from(f));
    }
    if let Ok(s) = ob.extract::<String>() {
        return CBig::from_str(&s).map_err(parse_error_to_py);
    }
    Err(PyTypeError::new_err("expected a CBig, float, int, complex, or string"))
}

macro_rules! impl_cpy_binops {
    ($method:ident, $rs_method:ident) => {
        fn $method(lhs: &CPy, rhs: &Bound<'_, PyAny>, py: Python<'_>) -> PyResult<Py<PyAny>> {
            let rhs = to_cbig(rhs)?;
            CPy((&lhs.0).$rs_method(&rhs)).into_py_any(py)
        }
    };
    ($method:ident, $rev_method:ident, $rs_method:ident) => {
        impl_cpy_binops!($method, $rs_method);
        fn $rev_method(lhs: &Bound<'_, PyAny>, rhs: &CPy, py: Python<'_>) -> PyResult<Py<PyAny>> {
            let lhs = to_cbig(lhs)?;
            CPy(lhs.$rs_method(&rhs.0)).into_py_any(py)
        }
    };
}

impl_cpy_binops!(cpy_add, add);
impl_cpy_binops!(cpy_sub, cpy_rsub, sub);
impl_cpy_binops!(cpy_mul, mul);
impl_cpy_binops!(cpy_div, cpy_rdiv, div);

#[pymethods]
impl CPy {
    #[new]
    #[pyo3(signature = (re, im=None))]
    fn __new__(re: &Bound<'_, PyAny>, im: Option<&Bound<'_, PyAny>>) -> PyResult<Self> {
        // CBig(re, im): build from two real parts
        if let Some(im_ob) = im {
            let re_f = to_cbig(re)?.into_parts().0;
            let im_f = to_cbig(im_ob)?.into_parts().0;
            return Ok(CPy(CBig::from_parts(re_f, im_f)));
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
    fn __format__(&self, _format_spec: &str) -> String {
        format!("{}", self.0)
    }
    fn __hash__(&self) -> u64 {
        // mirror Python's complex hash convention loosely: combine real/imag float hashes
        let (re, im) = self.0.clone().into_parts();
        let re_hash = re.to_f64().value().to_bits();
        let im_hash = im.to_f64().value().to_bits();
        re_hash.wrapping_add(im_hash.wrapping_mul(3))
    }
    fn __bool__(&self) -> bool {
        !self.0.is_zero()
    }
    /// Complex numbers have no ordering — only `==` and `!=` are defined.
    fn __richcmp__(&self, other: &Bound<'_, PyAny>, op: CompareOp) -> PyResult<bool> {
        let other = to_cbig(other)?;
        match op {
            CompareOp::Eq => Ok(self.0 == other),
            CompareOp::Ne => Ok(self.0 != other),
            _ => Err(PyTypeError::new_err("no ordering relation is defined for complex numbers")),
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
        let ctx = self.0.context();
        let res = ctx.abs(&self.0);
        Ok(FPy(unwrap_float(res, ctx_to_float(&ctx))?))
    }
    /// Argument (phase) of z.
    fn arg(&self) -> PyResult<FPy> {
        let ctx = self.0.context();
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

    /********** transcendentals (global cache + complex Context layer) **********/
    fn sin(&self) -> PyResult<Self> {
        let ctx = self.0.context();
        let res = with_cache(|c| ctx.sin(&self.0, Some(c)));
        Ok(Self(unwrap_complex(res, ctx)?))
    }
    fn cos(&self) -> PyResult<Self> {
        let ctx = self.0.context();
        let res = with_cache(|c| ctx.cos(&self.0, Some(c)));
        Ok(Self(unwrap_complex(res, ctx)?))
    }
    fn tan(&self) -> PyResult<Self> {
        let ctx = self.0.context();
        let res = with_cache(|c| ctx.tan(&self.0, Some(c)));
        Ok(Self(unwrap_complex(res, ctx)?))
    }
    fn exp(&self) -> PyResult<Self> {
        let ctx = self.0.context();
        let res = with_cache(|c| ctx.exp(&self.0, Some(c)));
        Ok(Self(unwrap_complex(res, ctx)?))
    }
    fn ln(&self) -> PyResult<Self> {
        let ctx = self.0.context();
        let res = with_cache(|c| ctx.log(&self.0, Some(c)));
        Ok(Self(unwrap_complex(res, ctx)?))
    }
    fn sqrt(&self) -> PyResult<Self> {
        let ctx = self.0.context();
        Ok(Self(unwrap_complex(ctx.sqrt(&self.0), ctx)?))
    }
    fn powi(&self, n: UniInput<'_>) -> PyResult<Self> {
        let n = n.into_ibig()?;
        let ctx = self.0.context();
        Ok(Self(unwrap_complex(ctx.powi(&self.0, n), ctx)?))
    }
    fn powf(&self, w: &Self) -> PyResult<Self> {
        let ctx = self.0.context();
        let res = with_cache(|c| ctx.powf(&self.0, &w.0, Some(c)));
        Ok(Self(unwrap_complex(res, ctx)?))
    }
}

/// The complex `Context<R>` wraps a float `Context<R>`; recover it for the real-valued
/// `abs`/`arg`/`norm` results that go through [`crate::cache::unwrap_float`].
fn ctx_to_float<R: dashu_float::round::Round>(
    ctx: &dashu_cmplx::Context<R>,
) -> dashu_float::Context<R> {
    dashu_float::Context::new(ctx.precision())
}
