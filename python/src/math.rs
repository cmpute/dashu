//! Module-level math functions. Each accepts any Python number (via [`UniInput`]) and
//! routes through the global `ConstCache` + the panic-free `Context` layer (via
//! [`crate::cache::unwrap_float`]), so domain errors raise Python exceptions instead of
//! aborting the session.

use crate::cache::{unwrap_float, with_cache};
use crate::types::{FPy, UPy, UniInput};
use dashu_base::ring::{ExtendedGcd, Gcd};
use pyo3::prelude::*;

/// Transcendental taking one float operand and a cache (`(repr, Option<&mut ConstCache>)`).
macro_rules! math_trans {
    ($name:ident, $ctx_method:ident) => {
        #[pyfunction]
        pub fn $name(x: UniInput<'_>) -> PyResult<FPy> {
            let x = x.into_fpy()?;
            let ctx = x.0.context();
            let res = with_cache(|c| ctx.$ctx_method(x.0.repr(), Some(c)));
            Ok(FPy(unwrap_float(res, ctx)?))
        }
    };
}

/// Transcendental pair returning `(f, g)` (e.g. `sin_cos`), sharing one Ziv loop.
macro_rules! math_trans_pair {
    ($name:ident, $ctx_method:ident) => {
        #[pyfunction]
        pub fn $name(x: UniInput<'_>) -> PyResult<(FPy, FPy)> {
            let x = x.into_fpy()?;
            let ctx = x.0.context();
            let res = with_cache(|c| ctx.$ctx_method(x.0.repr(), Some(c)));
            let (a, b) = res;
            Ok((FPy(unwrap_float(a, ctx)?), FPy(unwrap_float(b, ctx)?)))
        }
    };
}

math_trans_pair!(sin_cos, sin_cos);
math_trans_pair!(sinh_cosh, sinh_cosh);

/// Algebraic root taking one float operand, no cache.
macro_rules! math_root {
    ($name:ident, $ctx_method:ident) => {
        #[pyfunction]
        pub fn $name(x: UniInput<'_>) -> PyResult<FPy> {
            let x = x.into_fpy()?;
            let ctx = x.0.context();
            Ok(FPy(unwrap_float(ctx.$ctx_method(x.0.repr()), ctx)?))
        }
    };
}

// Trigonometric
math_trans!(sin, sin);
math_trans!(cos, cos);
math_trans!(tan, tan);
math_trans!(asin, asin);
math_trans!(acos, acos);
math_trans!(atan, atan);

// Hyperbolic
math_trans!(sinh, sinh);
math_trans!(cosh, cosh);
math_trans!(tanh, tanh);
math_trans!(asinh, asinh);
math_trans!(acosh, acosh);
math_trans!(atanh, atanh);

// Exponential and logarithm
math_trans!(exp, exp);
math_trans!(expm1, exp_m1);
math_trans!(ln, ln);
math_trans!(ln_1p, ln_1p);
// `log`/`log1p` are aliases for `ln`/`ln_1p`.
math_trans!(log, ln);
math_trans!(log1p, ln_1p);
math_trans!(log2, log2);

// Roots (algebraic — no cache)
math_root!(sqrt, sqrt);
math_root!(cbrt, cbrt);

#[pyfunction]
pub fn nth_root(x: UniInput<'_>, n: usize) -> PyResult<FPy> {
    let x = x.into_fpy()?;
    let ctx = x.0.context();
    Ok(FPy(unwrap_float(ctx.nth_root(n, x.0.repr()), ctx)?))
}

#[pyfunction]
pub fn atan2(y: UniInput<'_>, x: UniInput<'_>) -> PyResult<FPy> {
    let y = y.into_fpy()?;
    let x = x.into_fpy()?;
    let ctx = y.0.context();
    let res = with_cache(|c| ctx.atan2(y.0.repr(), x.0.repr(), Some(c)));
    Ok(FPy(unwrap_float(res, ctx)?))
}

#[pyfunction]
pub fn powf(x: UniInput<'_>, y: UniInput<'_>) -> PyResult<FPy> {
    let x = x.into_fpy()?;
    let y = y.into_fpy()?;
    let ctx = x.0.context();
    let res = with_cache(|c| ctx.powf(x.0.repr(), y.0.repr(), Some(c)));
    Ok(FPy(unwrap_float(res, ctx)?))
}

#[pyfunction]
pub fn powi(x: UniInput<'_>, n: UniInput<'_>) -> PyResult<FPy> {
    let x = x.into_fpy()?;
    let n = n.into_ibig()?;
    let ctx = x.0.context();
    Ok(FPy(unwrap_float(ctx.powi(x.0.repr(), n), ctx)?))
}

#[pyfunction]
pub fn hypot(x: UniInput<'_>, y: UniInput<'_>) -> PyResult<FPy> {
    let x = x.into_fpy()?;
    let y = y.into_fpy()?;
    let ctx = x.0.context();
    Ok(FPy(unwrap_float(ctx.hypot(x.0.repr(), y.0.repr()), ctx)?))
}

// Integer number theory
#[pyfunction]
pub fn gcd(a: UniInput<'_>, b: UniInput<'_>) -> PyResult<UPy> {
    let a = a.into_ubig()?;
    let b = b.into_ubig()?;
    Ok(UPy(Gcd::gcd(&a, &b)))
}

#[pyfunction]
pub fn gcd_ext(
    a: UniInput<'_>,
    b: UniInput<'_>,
) -> PyResult<(UPy, crate::types::IPy, crate::types::IPy)> {
    let a = a.into_ubig()?;
    let b = b.into_ubig()?;
    let (g, s, t) = ExtendedGcd::gcd_ext(&a, &b);
    Ok((UPy(g), crate::types::IPy(s), crate::types::IPy(t)))
}

#[pyfunction]
pub fn lcm(a: UniInput<'_>, b: UniInput<'_>) -> PyResult<UPy> {
    let a = a.into_ubig()?;
    let b = b.into_ubig()?;
    let g = Gcd::gcd(&a, &b);
    Ok(UPy((a / g) * b))
}
