//! Global constant cache shared by all float/decimal/complex transcendentals.
//!
//! The cache memoizes the big-integer constants (π, ln2, ln10, …) that the
//! transcendental kernels recompute from scratch on every call unless a
//! [`ConstCache`] is threaded in. It is base-free and rounding-mode-free, so a
//! single instance serves base-2 floats (`FPy`), base-10 decimals (`DPy`) and
//! complex numbers (`CPy`) simultaneously, accumulating precision across calls.
//!
//! Keeping the cache here (in the module) rather than inside each value means the
//! values stay plain, `Send` + `Sync` `FBig`/`CBig` — no `#[pyclass(unsendable)]`,
//! and the binding is free-threaded-Python compatible. Each thread gets its own
//! thread-local cache (zero locking), recomputing the constants once.

use std::cell::RefCell;

use dashu::base::Sign;
use dashu::float::{ConstCache, Context, FBig, FpError, FpResult, Repr, Word, round::Round};
use pyo3::exceptions::{PyRuntimeError, PyValueError, PyZeroDivisionError};
use pyo3::prelude::*;

thread_local! {
    /// One cache per thread — zero locking, zero contention. Accumulates precision
    /// across calls and never needs explicit invalidation.
    static CONST_CACHE: RefCell<ConstCache> = const { RefCell::new(ConstCache::new()) };
}

/// Borrow the thread-local cache mutably for one transcendental call.
pub fn with_cache<R>(f: impl FnOnce(&mut ConstCache) -> R) -> R {
    CONST_CACHE.with(|c| f(&mut c.borrow_mut()))
}

/// Map a float `FpResult<Rounded<FBig>>` to a `PyResult<FBig>`:
/// - `Overflow(sign)` / `Underflow(sign)` → signed ∞ / signed 0 (graceful, like Python float);
/// - `InfiniteInput` / `OutOfDomain` → `ValueError`;
/// - `Indeterminate` (e.g. `0/0`) → `ZeroDivisionError`.
///
/// This replaces the panicking `Context::unwrap_fp`, which aborts the session on the
/// last three cases. `ctx` is the value's own context, used to rebuild ∞ / 0 results.
pub fn unwrap_float<R: Round, const B: Word>(
    res: FpResult<FBig<R, B>>,
    ctx: Context<R>,
) -> PyResult<FBig<R, B>> {
    match res {
        Ok(rounded) => Ok(rounded.value()),
        Err(FpError::Overflow(sign)) => {
            let repr = match sign {
                Sign::Positive => Repr::<B>::infinity(),
                Sign::Negative => Repr::<B>::neg_infinity(),
            };
            Ok(FBig::from_repr(repr, ctx))
        }
        Err(FpError::Underflow(sign)) => {
            let repr = match sign {
                Sign::Positive => Repr::<B>::zero(),
                Sign::Negative => Repr::<B>::neg_zero(),
            };
            Ok(FBig::from_repr(repr, ctx))
        }
        Err(FpError::InfiniteInput) => Err(PyValueError::new_err("arithmetic with infinity")),
        Err(FpError::OutOfDomain) => Err(PyValueError::new_err("math domain error")),
        Err(FpError::Indeterminate) => {
            Err(PyZeroDivisionError::new_err("indeterminate form (0/0)"))
        }
        Err(FpError::ZivRetryLimitExceeded) => Err(PyRuntimeError::new_err(
            "the Ziv retry limit was exceeded; the result is not correctly rounded (please report this case to the maintainer)",
        )),
        Err(FpError::ZivRetryLimitExceeded) => Err(PyRuntimeError::new_err(
            "the Ziv retry limit was exceeded; the result is not correctly rounded (please report this case to the maintainer)",
        )),
    }
}

/// Map a complex `CfpResult<CRounded<CBig>>` to a `PyResult<CBig>`, mirroring
/// [`unwrap_float`]. Complex overflow/underflow rebuilds a CBig whose real part carries the
/// signed ∞ / 0 (a conventional choice, matching the real-valued overflow direction).
pub fn unwrap_complex<R: Round, const B: Word>(
    res: dashu::complex::CfpResult<R, B>,
    ctx: dashu::complex::Context<R>,
) -> PyResult<dashu::complex::CBig<R, B>> {
    match res {
        Ok(rounded) => Ok(rounded.value()),
        Err(FpError::Overflow(sign)) => {
            let re = match sign {
                Sign::Positive => Repr::<B>::infinity(),
                Sign::Negative => Repr::<B>::neg_infinity(),
            };
            Ok(dashu::complex::CBig::new(re, Repr::<B>::zero(), ctx))
        }
        Err(FpError::Underflow(_sign)) => {
            Ok(dashu::complex::CBig::new(Repr::<B>::zero(), Repr::<B>::zero(), ctx))
        }
        Err(FpError::InfiniteInput) => Err(PyValueError::new_err("arithmetic with infinity")),
        Err(FpError::OutOfDomain) => Err(PyValueError::new_err("math domain error")),
        Err(FpError::Indeterminate) => {
            Err(PyZeroDivisionError::new_err("indeterminate form (0/0)"))
        }
        Err(FpError::ZivRetryLimitExceeded) => Err(PyRuntimeError::new_err(
            "the Ziv retry limit was exceeded; the result is not correctly rounded (please report this case to the maintainer)",
        )),
        Err(FpError::ZivRetryLimitExceeded) => Err(PyRuntimeError::new_err(
            "the Ziv retry limit was exceeded; the result is not correctly rounded (please report this case to the maintainer)",
        )),
    }
}

/// Optional Python-facing handle for cache inspection / clearing.
#[pyclass(name = "Cache")]
pub struct PyCache;

#[pymethods]
impl PyCache {
    /// Drop all memoized constants (rarely needed — the cache only grows usefully).
    #[staticmethod]
    fn clear() {
        with_cache(|c| c.clear());
    }
    /// Total number of terms accumulated across all cached constants.
    #[staticmethod]
    fn total_terms() -> usize {
        with_cache(|c| c.total_terms())
    }
    /// Total number of machine words accumulated across all cached constants.
    #[staticmethod]
    fn total_words() -> usize {
        with_cache(|c| c.total_words())
    }
}
