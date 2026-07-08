//! Module-level math functions. Each routes through the global `ConstCache` + the
//! panic-free `Context` layer (via [`crate::cache::unwrap_float`]), so domain errors raise
//! Python exceptions instead of aborting the session.

use crate::cache::{unwrap_float, with_cache};
use crate::types::{FPy, IPy, UPy};
use dashu_base::ring::{ExtendedGcd, Gcd};
use pyo3::prelude::*;

// Trigonometric
#[pyfunction]
pub fn sin(x: &FPy) -> PyResult<FPy> {
    let ctx = x.0.context();
    let res = with_cache(|c| ctx.sin(x.0.repr(), Some(c)));
    Ok(FPy(unwrap_float(res, ctx)?))
}
#[pyfunction]
pub fn cos(x: &FPy) -> PyResult<FPy> {
    let ctx = x.0.context();
    let res = with_cache(|c| ctx.cos(x.0.repr(), Some(c)));
    Ok(FPy(unwrap_float(res, ctx)?))
}
#[pyfunction]
pub fn tan(x: &FPy) -> PyResult<FPy> {
    let ctx = x.0.context();
    let res = with_cache(|c| ctx.tan(x.0.repr(), Some(c)));
    Ok(FPy(unwrap_float(res, ctx)?))
}
#[pyfunction]
pub fn asin(x: &FPy) -> PyResult<FPy> {
    let ctx = x.0.context();
    let res = with_cache(|c| ctx.asin(x.0.repr(), Some(c)));
    Ok(FPy(unwrap_float(res, ctx)?))
}
#[pyfunction]
pub fn acos(x: &FPy) -> PyResult<FPy> {
    let ctx = x.0.context();
    let res = with_cache(|c| ctx.acos(x.0.repr(), Some(c)));
    Ok(FPy(unwrap_float(res, ctx)?))
}
#[pyfunction]
pub fn atan(x: &FPy) -> PyResult<FPy> {
    let ctx = x.0.context();
    let res = with_cache(|c| ctx.atan(x.0.repr(), Some(c)));
    Ok(FPy(unwrap_float(res, ctx)?))
}
#[pyfunction]
pub fn atan2(y: &FPy, x: &FPy) -> PyResult<FPy> {
    let ctx = y.0.context();
    let res = with_cache(|c| ctx.atan2(y.0.repr(), x.0.repr(), Some(c)));
    Ok(FPy(unwrap_float(res, ctx)?))
}

// Hyperbolic
#[pyfunction]
pub fn sinh(x: &FPy) -> PyResult<FPy> {
    let ctx = x.0.context();
    let res = with_cache(|c| ctx.sinh(x.0.repr(), Some(c)));
    Ok(FPy(unwrap_float(res, ctx)?))
}
#[pyfunction]
pub fn cosh(x: &FPy) -> PyResult<FPy> {
    let ctx = x.0.context();
    let res = with_cache(|c| ctx.cosh(x.0.repr(), Some(c)));
    Ok(FPy(unwrap_float(res, ctx)?))
}
#[pyfunction]
pub fn tanh(x: &FPy) -> PyResult<FPy> {
    let ctx = x.0.context();
    let res = with_cache(|c| ctx.tanh(x.0.repr(), Some(c)));
    Ok(FPy(unwrap_float(res, ctx)?))
}
#[pyfunction]
pub fn asinh(x: &FPy) -> PyResult<FPy> {
    let ctx = x.0.context();
    let res = with_cache(|c| ctx.asinh(x.0.repr(), Some(c)));
    Ok(FPy(unwrap_float(res, ctx)?))
}
#[pyfunction]
pub fn acosh(x: &FPy) -> PyResult<FPy> {
    let ctx = x.0.context();
    let res = with_cache(|c| ctx.acosh(x.0.repr(), Some(c)));
    Ok(FPy(unwrap_float(res, ctx)?))
}
#[pyfunction]
pub fn atanh(x: &FPy) -> PyResult<FPy> {
    let ctx = x.0.context();
    let res = with_cache(|c| ctx.atanh(x.0.repr(), Some(c)));
    Ok(FPy(unwrap_float(res, ctx)?))
}

// Exponential and logarithm
#[pyfunction]
pub fn exp(x: &FPy) -> PyResult<FPy> {
    let ctx = x.0.context();
    let res = with_cache(|c| ctx.exp(x.0.repr(), Some(c)));
    Ok(FPy(unwrap_float(res, ctx)?))
}
#[pyfunction]
pub fn expm1(x: &FPy) -> PyResult<FPy> {
    let ctx = x.0.context();
    let res = with_cache(|c| ctx.exp_m1(x.0.repr(), Some(c)));
    Ok(FPy(unwrap_float(res, ctx)?))
}
#[pyfunction]
pub fn log(x: &FPy) -> PyResult<FPy> {
    let ctx = x.0.context();
    let res = with_cache(|c| ctx.ln(x.0.repr(), Some(c)));
    Ok(FPy(unwrap_float(res, ctx)?))
}
#[pyfunction]
pub fn log1p(x: &FPy) -> PyResult<FPy> {
    let ctx = x.0.context();
    let res = with_cache(|c| ctx.ln_1p(x.0.repr(), Some(c)));
    Ok(FPy(unwrap_float(res, ctx)?))
}
#[pyfunction]
pub fn ln(x: &FPy) -> PyResult<FPy> {
    let ctx = x.0.context();
    let res = with_cache(|c| ctx.ln(x.0.repr(), Some(c)));
    Ok(FPy(unwrap_float(res, ctx)?))
}
#[pyfunction]
pub fn ln_1p(x: &FPy) -> PyResult<FPy> {
    let ctx = x.0.context();
    let res = with_cache(|c| ctx.ln_1p(x.0.repr(), Some(c)));
    Ok(FPy(unwrap_float(res, ctx)?))
}

// Roots and power (sqrt/cbrt/nth_root/hypot are algebraic — no cache, but still via Context)
#[pyfunction]
pub fn sqrt(x: &FPy) -> PyResult<FPy> {
    let ctx = x.0.context();
    Ok(FPy(unwrap_float(ctx.sqrt(x.0.repr()), ctx)?))
}
#[pyfunction]
pub fn cbrt(x: &FPy) -> PyResult<FPy> {
    let ctx = x.0.context();
    Ok(FPy(unwrap_float(ctx.cbrt(x.0.repr()), ctx)?))
}
#[pyfunction]
pub fn nth_root(x: &FPy, n: usize) -> PyResult<FPy> {
    let ctx = x.0.context();
    Ok(FPy(unwrap_float(ctx.nth_root(n, x.0.repr()), ctx)?))
}
#[pyfunction]
pub fn powf(x: &FPy, y: &FPy) -> PyResult<FPy> {
    let ctx = x.0.context();
    let res = with_cache(|c| ctx.powf(x.0.repr(), y.0.repr(), Some(c)));
    Ok(FPy(unwrap_float(res, ctx)?))
}
#[pyfunction]
pub fn powi(x: &FPy, n: crate::types::UniInput<'_>) -> PyResult<FPy> {
    let n = n.into_ibig()?;
    let ctx = x.0.context();
    Ok(FPy(unwrap_float(ctx.powi(x.0.repr(), n), ctx)?))
}
#[pyfunction]
pub fn hypot(x: &FPy, y: &FPy) -> PyResult<FPy> {
    let ctx = x.0.context();
    Ok(FPy(unwrap_float(ctx.hypot(x.0.repr(), y.0.repr()), ctx)?))
}

// Integer number theory
#[pyfunction]
pub fn gcd(a: &UPy, b: &UPy) -> UPy {
    UPy(Gcd::gcd(&a.0, &b.0))
}
#[pyfunction]
pub fn gcd_ext(a: &UPy, b: &UPy) -> (UPy, IPy, IPy) {
    let (g, s, t) = ExtendedGcd::gcd_ext(&a.0, &b.0);
    (UPy(g), IPy(s), IPy(t))
}
#[pyfunction]
pub fn lcm(a: &UPy, b: &UPy) -> UPy {
    let g = Gcd::gcd(&a.0, &b.0);
    UPy((a.0.clone() / g) * b.0.clone())
}
