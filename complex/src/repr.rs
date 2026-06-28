//! The complex [`Context`] and the result/inexactness types.
//!
//! Mirroring `dashu-float`'s `repr.rs` (which hosts both `Repr` and `Context`), the complex
//! [`Context`] lives here. `dashu-cmplx` reuses `dashu-float`'s [`Repr`] unchanged, so unlike float's
//! module this one holds only the complex-side pieces: the [`Context`] newtype and the
//! [`CfpResult`]/[`CRounded`] result types.
//!
//! [`Context`] is a thin newtype around [`dashu_float::Context`] that hosts the context-layer
//! CBig operations (it can't be added to `FBig`'s own `Context` from this crate — coherence). The
//! wrapped value *is* the shared precision/rounding config, so the config API
//! ([`Context::new`] / [`Context::max`] / [`Context::precision`]) just delegates to the inner float
//! context.
//!
//! The complex analog of `FpResult`/`Rounded` is [`CfpResult`]/[`CRounded`]: a complex result
//! carries **two** inexactness flags (one per axis), modeled as
//! `Approximation<CBig, (Rounding, Rounding)>`.

use dashu_base::Approximation;
use dashu_base::Approximation::*;
use dashu_float::round::{Round, Rounding};
use dashu_float::{ConstCache, Context as FloatCtxt, FBig, FpError, Repr};
use dashu_int::Word;

use crate::cbig::CBig;

/// CBig operation context — a newtype wrapper around [`dashu_float::Context`], and also the type
/// stored on each [`CBig`] as its shared precision/rounding config (so [`CBig::context`] returns it
/// directly, with no wrapping).
///
/// It is a separate type because inherent methods cannot be added to `FBig`'s `Context` from this
/// crate; it exists to host the context-layer CBig operations ([`Context::mul`], [`Context::exp`], …).
/// The config API just delegates inward to the wrapped float context.
#[derive(Clone, Copy)]
pub struct Context<R: Round>(pub(crate) FloatCtxt<R>);

/// Correctly-rounded complex result with per-axis inexactness.
///
/// `Exact(v)` ⟺ both parts are exact; `Inexact(v, (re, im))` carries each part's rounding
/// direction. This is the complex twin of [`dashu_float::Rounded`] (`Approximation<T, Rounding>`),
/// reusing the same [`Rounding`] flag type for each axis.
pub type CRounded<R, const B: Word> = Approximation<CBig<R, B>, (Rounding, Rounding)>;

/// The result of a context-layer CBig operation: a correctly-rounded [`CBig`] (with per-axis
/// inexactness) or an [`FpError`]. The complex analog of [`dashu_float::FpResult`].
pub type CfpResult<R, const B: Word> = Result<CRounded<R, B>, FpError>;

impl<R: Round> Context<R> {
    /// Create a CBig operation context with the given precision limit (`0` = unlimited).
    #[inline]
    pub const fn new(precision: usize) -> Self {
        Self(FloatCtxt::new(precision))
    }

    /// Create a context with the higher precision from the two inputs (unlimited `0` dominates).
    #[inline]
    pub const fn max(lhs: Self, rhs: Self) -> Self {
        Self(FloatCtxt::max(lhs.0, rhs.0))
    }

    /// The precision limit stored in the context (`0` = unlimited). Both parts of a [`CBig`] always
    /// share this single precision.
    #[inline]
    pub const fn precision(&self) -> usize {
        self.0.precision()
    }

    /// The inner float context used to drive the real-part math (copied, since it is `Copy`).
    #[inline]
    pub(crate) const fn float(&self) -> FloatCtxt<R> {
        self.0
    }

    /// Build a transient float working context at `p + g` guard digits — the guard-digit recipe
    /// (§6.1 of the design doc) evaluates each component at extra precision and re-rounds to `p`.
    #[inline]
    pub(crate) fn guard(&self, g: usize) -> FloatCtxt<R> {
        FloatCtxt::new(self.precision() + g)
    }

    /// Unwrap a [`CfpResult`], returning the [`CBig`] value directly.
    ///
    /// The complex analog of [`dashu_float::Context::unwrap_fp`]. It drops the per-axis
    /// `(Rounding, Rounding)` flags, and applies the same error policy: [`FpError::Overflow`]
    /// saturates to a signed infinity, [`FpError::Underflow`] to a signed zero, and the remaining
    /// variants panic.
    #[inline]
    pub fn unwrap_cfp<const B: Word>(&self, result: CfpResult<R, B>) -> CBig<R, B> {
        match result {
            Ok(rounded) => rounded.value(),
            Err(FpError::Overflow(sign)) => CBig::overflow(self, sign),
            Err(FpError::Underflow(sign)) => CBig::underflow(self, sign),
            Err(FpError::InfiniteInput) => {
                panic!("arithmetic operations with the infinity are not allowed!")
            }
            Err(FpError::OutOfDomain) => panic!("the operation result is out of domain!"),
            Err(FpError::Indeterminate) => {
                panic!("the result of the operation is an indeterminate form!")
            }
        }
    }
}

/// Combine two per-part float rounding results into a [`CRounded`] complex result, carrying each
/// part's inexactness flag. `Exact` iff both parts are exact.
pub(crate) fn combine_parts<R: Round, const B: Word>(
    re: Approximation<FBig<R, B>, Rounding>,
    im: Approximation<FBig<R, B>, Rounding>,
) -> CRounded<R, B> {
    let (re_val, re_rnd) = match re {
        Approximation::Exact(v) => (v, Rounding::NoOp),
        Approximation::Inexact(v, r) => (v, r),
    };
    let (im_val, im_rnd) = match im {
        Approximation::Exact(v) => (v, Rounding::NoOp),
        Approximation::Inexact(v, r) => (v, r),
    };
    let value = CBig::from_parts(re_val, im_val);
    if re_rnd == Rounding::NoOp && im_rnd == Rounding::NoOp {
        Exact(value)
    } else {
        Inexact(value, (re_rnd, im_rnd))
    }
}

/// Build a [`CRounded`] from two already-unwrapped float results, when the per-part rounding flags
/// are known directly (used by the exact/short-circuit special-value paths).
pub(crate) fn exact<R: Round, const B: Word>(re: FBig<R, B>, im: FBig<R, B>) -> CRounded<R, B> {
    Exact(CBig::from_parts(re, im))
}

/// The Riemann point at infinity `+∞ + i·0` as an exact [`CRounded`] result (dashu's complex
/// infinity — the single point `proj` collapses any infinity to).
pub(crate) fn riemann<R: Round, const B: Word>(context: Context<R>) -> CRounded<R, B> {
    exact(
        FBig::from_repr(Repr::infinity(), context.float()),
        FBig::from_repr(Repr::zero(), context.float()),
    )
}

/// Reborrow an `Option<&mut ConstCache>` for a sequential sub-call (mirrors `dashu-float`'s
/// `reborrow_cache`; `as_deref_mut` is the natural reborrow, allowed here centrally).
#[inline]
#[allow(clippy::needless_option_as_deref)]
pub(crate) fn reborrow_cache<'a>(
    cache: &'a mut Option<&mut ConstCache>,
) -> Option<&'a mut ConstCache> {
    cache.as_deref_mut()
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashu_base::Sign;
    use dashu_float::round::mode;
    use dashu_float::Repr;

    #[test]
    fn context_delegates_to_float() {
        let ctx: Context<mode::HalfEven> = Context::new(53);
        assert_eq!(ctx.precision(), 53);
        let bigger = Context::max(ctx, Context::new(10));
        assert_eq!(bigger.precision(), 53);
        // unlimited (0) is treated as the minimum precision, so a limited operand wins
        let limited_wins = Context::max(ctx, Context::new(0));
        assert_eq!(limited_wins.precision(), 53);
        let both_unlimited = Context::max(Context::<mode::HalfEven>::new(0), Context::new(0));
        assert_eq!(both_unlimited.precision(), 0);
    }

    #[test]
    fn combine_parts_exact_and_inexact() {
        let ctx: Context<mode::HalfEven> = Context::new(10);
        let f = ctx.float();
        let one = FBig::from_repr(Repr::<2>::one(), f);
        // two exact results combine to Exact
        let combined = combine_parts(Exact(one.clone()), Exact(one.clone()));
        assert!(matches!(combined, Exact(_)));
        // an inexact result combines to Inexact
        let add_one = f.add(one.repr(), one.repr()).unwrap(); // 1+1, exact at p=10
        let combined2 = combine_parts(add_one, Inexact(one, Rounding::AddOne));
        assert!(matches!(combined2, Inexact(_, _)));
        let _ = Sign::Positive; // keep Sign referenced
    }
}
