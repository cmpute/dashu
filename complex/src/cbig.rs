//! The [`CBig`] type: an arbitrary-precision complex number.
//!
//! A [`CBig`] is a pair of real parts (`re`, `im`) sharing one precision and one rounding mode,
//! mirroring [`dashu_float::FBig`]'s own `Repr`+`Context` layout generalized to two parts over a
//! **single shared** [`Context`](crate::Context). Storing one context — rather than wrapping two
//! `FBig`s (each carrying its own) — makes the uniform-precision invariant *physical*: there is
//! exactly one precision slot, so `re` and `im` structurally cannot disagree.

use crate::context::Context;
use dashu_base::Sign;
use dashu_float::round::{mode, Round};
use dashu_float::{FBig, Repr};
use dashu_int::Word;

/// An arbitrary-precision complex number with arbitrary base and rounding mode.
///
/// The complex number consists of two [`Repr`] parts (the real part `re` and the imaginary part
/// `im`) over a single shared [`Context`](crate::Context). Each part keeps its own significand
/// length; the shared context holds the precision cap and rounding mode applied independently to
/// both components.
///
/// # Generic parameters
///
/// The const generic parameters are abbreviated as `BASE` -> `B`, `RoundingMode` -> `R`. The `BASE`
/// must be in range `[2, isize::MAX]`, and the rounding mode `R` is chosen from the
/// [`dashu_float::round::mode`] module. With the defaults the number is base 2 rounded towards zero
/// (matching `FBig`'s default).
///
/// # Rounding
///
/// Each component of a result is rounded independently with the single mode `R`, after the
/// operation feeds each part enough guard precision (the same near-correctly-rounded guarantee class
/// `dashu-float`'s transcendentals carry). See the crate-level docs for the no-NaN error policy.
///
/// # Examples
///
/// ```
/// use dashu_cmplx::CBig;
/// use dashu_float::{FBig, round::mode::HalfAway};
///
/// // base-10 so each integer keeps its own significand
/// let z = CBig::<HalfAway, 10>::from_parts(FBig::from(3), FBig::from(4));
/// assert_eq!(z.re().significand(), &3.into());
/// assert_eq!(z.imag().significand(), &4.into());
/// ```
pub struct CBig<R: Round = mode::Zero, const B: Word = 2> {
    pub(crate) re: Repr<B>,
    pub(crate) im: Repr<B>,
    pub(crate) context: Context<R>,
}

impl<R: Round, const B: Word> CBig<R, B> {
    /// Create a [`CBig`] from raw parts — internal use only.
    #[inline]
    pub(crate) const fn new(re: Repr<B>, im: Repr<B>, context: Context<R>) -> Self {
        Self { re, im, context }
    }

    /// Create a [`CBig`] directly from its two [`Repr`] parts and a shared [`Context`] (a `const`-
    /// capable constructor, the complex analog of [`dashu_float::FBig::from_repr_const`]). Used by
    /// the `static_cbig!` literal macro; in most cases prefer [`CBig::from_parts`].
    #[inline]
    pub const fn from_repr_parts(re: Repr<B>, im: Repr<B>, context: Context<R>) -> Self {
        Self { re, im, context }
    }

    /// Create a [`CBig`] from its real and imaginary parts.
    ///
    /// The result context is `max(re.context(), im.context())` (the larger precision wins; an
    /// unlimited `0` precision is treated as the minimum, so a limited operand's precision wins),
    /// and the smaller-precision part is effectively widened to it — widening is exact, so only the
    /// precision cap changes. The rounding mode and base must match and are enforced by the type
    /// parameters.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashu_cmplx::CBig;
    /// use dashu_float::{FBig, round::mode::HalfAway};
    ///
    /// type C = CBig<HalfAway, 10>;
    /// type F = FBig<HalfAway, 10>;
    /// let z = C::from_parts(F::from(3), F::from(4));
    /// let (re, im) = z.into_parts();
    /// assert_eq!(re, F::from(3));
    /// assert_eq!(im, F::from(4));
    /// ```
    #[inline]
    pub fn from_parts(re: FBig<R, B>, im: FBig<R, B>) -> Self {
        let fctx = dashu_float::Context::max(re.context(), im.context());
        Self {
            re: re.into_repr(),
            im: im.into_repr(),
            context: Context(fctx),
        }
    }

    /// The complex number zero `0 + 0i` (unlimited precision).
    pub const ZERO: Self = Self::new(Repr::zero(), Repr::zero(), Context::new(0));

    /// The complex number one `1 + 0i` (unlimited precision).
    pub const ONE: Self = Self::new(Repr::one(), Repr::zero(), Context::new(0));

    /// The imaginary unit `0 + 1i` (unlimited precision).
    pub const I: Self = Self::new(Repr::zero(), Repr::one(), Context::new(0));

    /// Get the shared [`Context`](crate::Context) of the complex number.
    #[inline]
    pub const fn context(&self) -> Context<R> {
        self.context
    }

    /// Get the precision limit of the complex number (`0` = unlimited). Both parts share it.
    #[inline]
    pub const fn precision(&self) -> usize {
        self.context.precision()
    }

    /// Get a reference to the real part's raw representation.
    #[inline]
    pub const fn re(&self) -> &Repr<B> {
        &self.re
    }

    /// Get a reference to the imaginary part's raw representation.
    #[inline]
    pub const fn imag(&self) -> &Repr<B> {
        &self.im
    }

    /// Convert the complex number into its real and imaginary parts as [`FBig`]s, each carrying the
    /// (copied) shared context — zero clone of the significands.
    #[inline]
    pub fn into_parts(self) -> (FBig<R, B>, FBig<R, B>) {
        let fctx = self.context.float();
        (FBig::from_repr(self.re, fctx), FBig::from_repr(self.im, fctx))
    }

    /// Determine if the complex number is numerically zero (both parts `±0`).
    #[inline]
    pub fn is_zero(&self) -> bool {
        is_numeric_zero(&self.re) && is_numeric_zero(&self.im)
    }

    /// Determine if either part of the complex number is infinite.
    #[inline]
    pub fn is_infinite(&self) -> bool {
        self.re.is_infinite() || self.im.is_infinite()
    }

    /// Determine if the complex number is finite (neither part infinite).
    #[inline]
    pub fn is_finite(&self) -> bool {
        !self.is_infinite()
    }

    /// The complex infinity produced on overflow: a signed infinity on the real part and `+0` on the
    /// imaginary part (a provisional component mapping; `proj` collapses any infinity to `+∞ + i·0`).
    #[inline]
    pub(crate) fn overflow(context: &Context<R>, sign: Sign) -> Self {
        let re = match sign {
            Sign::Positive => Repr::infinity(),
            Sign::Negative => Repr::neg_infinity(),
        };
        Self::new(re, Repr::zero(), *context)
    }

    /// The complex zero produced on underflow: a signed zero on the real part and `+0` imaginary.
    #[inline]
    pub(crate) fn underflow(context: &Context<R>, sign: Sign) -> Self {
        let re = match sign {
            Sign::Positive => Repr::zero(),
            Sign::Negative => Repr::neg_zero(),
        };
        Self::new(re, Repr::zero(), *context)
    }
}

/// A [`Repr`] is numerically zero iff it is `+0` or `-0` (a zero significand that is *not* an
/// infinity sentinel).
#[inline]
pub(crate) fn is_numeric_zero<const B: Word>(repr: &Repr<B>) -> bool {
    repr.is_zero() || repr.is_neg_zero()
}

// Custom Clone (the significands are heap-allocated), mirroring FBig.
impl<R: Round, const B: Word> Clone for CBig<R, B> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            re: self.re.clone(),
            im: self.im.clone(),
            context: self.context,
        }
    }
}

impl<R: Round, const B: Word> Default for CBig<R, B> {
    /// Default value: `0 + 0i`.
    #[inline]
    fn default() -> Self {
        Self::ZERO
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type C = CBig<mode::HalfAway, 10>;

    #[test]
    fn constants() {
        assert!(C::ZERO.is_zero());
        assert!(!C::ONE.is_zero());
        assert!(!C::I.is_zero());
        let (re, im) = C::I.into_parts();
        assert!(re.repr().is_zero());
        assert!(im.repr().is_one());
    }

    #[test]
    fn from_parts_reconciles_precision() {
        type F = FBig<mode::HalfAway, 10>;
        let re = F::from_parts(3.into(), 0); // precision 1 (one decimal digit)
        let im = F::from_parts(4.into(), 0); // precision 1
        let z = CBig::from_parts(re, im);
        assert_eq!(z.precision(), 1);
        assert_eq!(z.re().significand(), &3.into());
        assert_eq!(z.imag().significand(), &4.into());
    }

    #[test]
    fn predicates() {
        let inf = C::new(Repr::infinity(), Repr::zero(), Context::new(0));
        assert!(inf.is_infinite());
        assert!(!inf.is_finite());
        assert!(!inf.is_zero());

        // a finite, nonzero number
        let z = C::from_parts(FBig::from(3), FBig::from(0));
        assert!(!z.is_infinite());
        assert!(z.is_finite());
        assert!(!z.is_zero());

        // both parts zero (incl. -0)
        let neg_zero = C::new(Repr::neg_zero(), Repr::zero(), Context::new(0));
        assert!(neg_zero.is_zero());
    }
}
