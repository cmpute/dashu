//! Module-level `NumOrd`-backed comparison utilities.
//!
//! [`compare`] / [`min`] / [`max`] compare any two Python numbers (native `int`/`float` or
//! `dashu` types) using the `num-order` crate's cross-type total ordering. Because the
//! comparison stays in the `dashu` / `num-order` layer — never round-tripping through a
//! primitive `f64` — `compare(UBig(2**200), 1e60)` is exact, unlike a lossy float conversion.
//!
//! When at least one operand is a `dashu` type, the other operand is matched against it via
//! `NumOrd::num_cmp`. When both operands are plain Python `int`/`float`, we fall back to
//! Python's own rich comparison (which is exact for `int`/`float`).

use std::cmp::Ordering;

use num_order::NumOrd;
use pyo3::prelude::*;

use crate::types::UniInput;

/// `$self` is a `&` reference to a concrete `dashu` big value (e.g. `&x.0` for a `UBig`);
/// `$other` is a `&[`UniInput`]`. Note that `$other` is a *reference*, so under match ergonomics
/// the variant bindings below are already references (e.g. `x: &u64` for `Uint`) — pass them
/// straight through as the `other: &Other` argument.
macro_rules! cmp_other {
    ($self:expr, $other:expr) => {{
        match $other {
            UniInput::Uint(x) => (*$self).num_cmp(x),
            UniInput::Int(x) => (*$self).num_cmp(x),
            UniInput::BUint(x) => (*$self).num_cmp(&x.0),
            UniInput::BInt(x) => (*$self).num_cmp(&x.0),
            UniInput::OBInt(x) => (*$self).num_cmp(x),
            UniInput::Float(x) => (*$self).num_cmp(x),
            UniInput::BFloat(x) => (*$self).num_cmp(&x.0),
            UniInput::BDecimal(x) => (*$self).num_cmp(&x.0),
            UniInput::OBDecimal(x) => (*$self).num_cmp(x),
            UniInput::BRational(x) => (*$self).num_cmp(&x.0),
            UniInput::OBRational(x) => (*$self).num_cmp(x),
        }
    }};
}

/// `$self` is a `&[`UniInput`]` whose variant is a primitive (`Uint`/`Int`/`Float`);
/// `$big` is a `&` reference to a concrete `dashu` big value used as the `self` side of the
/// comparison (the `NumOrd` impls live on the big types). The big value is `b`'s operand while
/// the primitive is `a`'s, so `$big.num_cmp(primitive)` yields `b vs a` — reverse it to get the
/// `a vs b` ordering [`compare`] promises.
macro_rules! cmp_prim {
    ($self:expr, $big:expr) => {{
        match $self {
            UniInput::Uint(x) => (*$big).num_cmp(x).reverse(),
            UniInput::Int(x) => (*$big).num_cmp(x).reverse(),
            UniInput::Float(x) => (*$big).num_cmp(x).reverse(),
            _ => unreachable!(),
        }
    }};
}

/// Compare a concrete `RBig` against any [`UniInput`]. Rational-vs-rational uses `Ord`
/// (`RBig` has no `NumOrd<RBig>` impl); everything else uses `NumOrd`.
fn cmp_rbig(a: &dashu::rational::RBig, b: &UniInput) -> Option<Ordering> {
    Some(match b {
        UniInput::Uint(x) => a.num_cmp(x),
        UniInput::Int(x) => a.num_cmp(x),
        UniInput::BUint(x) => a.num_cmp(&x.0),
        UniInput::BInt(x) => a.num_cmp(&x.0),
        UniInput::OBInt(x) => a.num_cmp(x),
        UniInput::Float(x) => a.num_cmp(x),
        UniInput::BFloat(x) => a.num_cmp(&x.0),
        UniInput::BDecimal(x) => a.num_cmp(&x.0),
        UniInput::OBDecimal(x) => a.num_cmp(x),
        UniInput::BRational(x) => a.cmp(&x.0),
        UniInput::OBRational(x) => a.cmp(x),
    })
}

/// Compare two `UniInput`s with `NumOrd`. Returns `None` when both operands are primitives
/// (no big type to host the comparison — callers fall back to Python's rich comparison).
fn num_cmp_uni(a: &UniInput, b: &UniInput) -> Option<Ordering> {
    match a {
        UniInput::BUint(x) => Some(cmp_other!(&x.0, b)),
        UniInput::BInt(x) => Some(cmp_other!(&x.0, b)),
        UniInput::OBInt(x) => Some(cmp_other!(&x, b)),
        UniInput::BFloat(x) => Some(cmp_other!(&x.0, b)),
        UniInput::BDecimal(x) => Some(cmp_other!(&x.0, b)),
        UniInput::OBDecimal(x) => Some(cmp_other!(&x, b)),
        UniInput::BRational(x) => cmp_rbig(&x.0, b),
        UniInput::OBRational(x) => cmp_rbig(x, b),
        // `a` is a primitive: try to use `b` as the big side instead.
        _ => match b {
            UniInput::BUint(x) => Some(cmp_prim!(a, &x.0)),
            UniInput::BInt(x) => Some(cmp_prim!(a, &x.0)),
            UniInput::OBInt(x) => Some(cmp_prim!(a, &x)),
            UniInput::BFloat(x) => Some(cmp_prim!(a, &x.0)),
            UniInput::BDecimal(x) => Some(cmp_prim!(a, &x.0)),
            UniInput::OBDecimal(x) => Some(cmp_prim!(a, &x)),
            UniInput::BRational(x) => Some(cmp_prim!(a, &x.0)),
            UniInput::OBRational(x) => Some(cmp_prim!(a, &x)),
            // both primitives: no `NumOrd` impl hosts the comparison.
            _ => None,
        },
    }
}

/// Compare two numbers and return `-1`, `0` or `1`.
///
/// The operands can be any mix of Python `int`/`float` and the `dashu` numeric types. The
/// comparison is exact (never lossy through a primitive float conversion). Complex numbers
/// cannot be ordered and raise `TypeError`.
#[pyfunction]
pub fn compare(a: &Bound<PyAny>, b: &Bound<PyAny>) -> PyResult<i32> {
    let a_un: UniInput = a.extract()?;
    let b_un: UniInput = b.extract()?;
    let ord = match num_cmp_uni(&a_un, &b_un) {
        Some(ord) => ord,
        None => {
            // both plain Python int/float: Python's own rich comparison is exact.
            if a.eq(b)? {
                Ordering::Equal
            } else if a.lt(b)? {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        }
    };
    Ok(match ord {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    })
}

/// Return the smaller of two numbers (cross-type, exact — see [`compare`]).
#[pyfunction]
pub fn min<'py>(a: &Bound<'py, PyAny>, b: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    if compare(a, b)? <= 0 {
        Ok(a.clone())
    } else {
        Ok(b.clone())
    }
}

/// Return the larger of two numbers (cross-type, exact — see [`compare`]).
#[pyfunction]
pub fn max<'py>(a: &Bound<'py, PyAny>, b: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    if compare(a, b)? >= 0 {
        Ok(a.clone())
    } else {
        Ok(b.clone())
    }
}
