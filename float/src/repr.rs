use crate::{
    error::assert_finite,
    round::{Round, Rounded},
    utils::{digit_len, split_digits, split_digits_ref},
};
use core::marker::PhantomData;
use dashu_base::{Approximation::*, EstimatedLog2, Sign};
pub use dashu_int::Word;
use dashu_int::{IBig, UBig};

/// Underlying representation of an arbitrary precision floating number.
///
/// The floating point number is represented as `significand * base^exponent`, where the
/// type of the significand is [IBig], and the type of exponent is [isize]. The representation
/// is always normalized (nonzero signficand is not divisible by the base, or zero signficand
/// with zero exponent).
///
/// When it's used together with a [Context], its precision will be limited so that
/// `|significand| < base^precision`. As an intentional exception, the result of an inexact
/// addition or subtraction may carry one extra guard digit, so `|significand|` can be up to
/// `base^(precision+1)`; the guard digit is what lets a much-smaller operand be reduced to a
/// sign-only sticky bit during alignment without mis-rounding.
///
/// # Infinity and signed zero
///
/// Special values are encoded with a zero significand and a sentinel exponent:
/// - value zero (`+0`): exponent = 0
/// - negative zero (`-0`): exponent = -1
/// - positive infinity (`+inf`): exponent = `isize::MAX`
/// - negative infinity (`-inf`): exponent = `isize::MIN`
///
/// The infinities are only supposed to be consumed as sentinels: only equality test and
/// comparison are implemented for them, and any arithmetic operation that takes an infinity
/// as input will lead to panic (at the `FBig` layer) or return an error (at the `Context`
/// layer). If an operation result is too large or too small, the operation will return an
/// infinity (as a value) at the `Context` layer, or panic at the `FBig` layer.
///
pub struct Repr<const BASE: Word> {
    /// The significand of the floating point number. If the significand is zero, then the
    /// number is a special value identified by the exponent (see the struct-level docs):
    /// `+0`, `-0`, `+inf`, or `-inf`.
    pub(crate) significand: IBig,

    /// The exponent of the floating point number.
    pub(crate) exponent: isize,
}

impl<const B: Word> PartialEq for Repr<B> {
    /// Two representations are equal when they denote the same value. In particular `+0`
    /// and `-0` compare equal, as do two infinities of the same sign.
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        if self.significand.is_zero() && other.significand.is_zero() {
            let (self_inf, other_inf) = (self.is_infinite(), other.is_infinite());
            match (self_inf, other_inf) {
                (true, true) => self.sign() == other.sign(),
                (false, false) => true, // both are ±0
                _ => false,             // one is zero, the other is infinite
            }
        } else {
            self.significand == other.significand && self.exponent == other.exponent
        }
    }
}

impl<const B: Word> Eq for Repr<B> {}

/// The context containing runtime information for the floating point number and its operations.
///
/// The context currently consists of a *precision limit* and a *rounding mode*. All the operation
/// associated with the context will be precise to the **full precision** (`|error| < 1 ulp`).
/// The rounding result returned from the functions tells additional error information, see
/// [the rounding mode module][crate::round::mode] for details.
///
/// # Precision
///
/// The precision limit determine the number of significant digits in the float number.
///
/// For binary operations, the result will have the higher one between the precisions of two
/// operands.
///
/// If the precision is set to 0, then the precision is **unlimited** during operations.
/// Be cautious to use unlimited precision because it can leads to very huge significands.
/// Unlimited precision is forbidden for some operations where the result is always inexact.
///
/// # Rounding Mode
///
/// The rounding mode determines the rounding behavior of the float operations.
///
/// See [the rounding mode module][crate::round::mode] for built-in rounding modes.
/// Users can implement custom rounding mode by implementing the [Round][crate::round::Round]
/// trait, but this is discouraged since in the future we might restrict the rounding
/// modes to be chosen from the the built-in modes.
///
/// For binary operations, the two oprands must have the same rounding mode.
///
#[derive(Clone, Copy)]
pub struct Context<RoundingMode: Round> {
    /// The precision of the floating point number.
    /// If set to zero, then the precision is unlimited.
    pub(crate) precision: usize,
    _marker: PhantomData<RoundingMode>,
}

/// Flip the sign of a special-value exponent: `+0 (0) <-> -0 (-1)`, `+inf (MAX) <-> -inf (MIN)`.
/// For any other (non-canonical) exponent the plain negation is used, which is safe because such
/// values have magnitude strictly less than `isize::MAX`.
#[inline]
const fn negate_special_exponent(exp: isize) -> isize {
    match exp {
        0 => -1,
        -1 => 0,
        isize::MAX => isize::MIN,
        isize::MIN => isize::MAX,
        other => -other,
    }
}

/// Build a `Repr` from a rounded significand, preserving the input sign when rounding
/// produces zero (`significand * B^exponent` where the significand collapsed to `+0`).
fn rounded_to_repr<const B: Word>(
    significand: IBig,
    exponent: isize,
    input_negative: bool,
) -> Repr<B> {
    if significand.is_zero() && input_negative {
        Repr::neg_zero()
    } else {
        Repr::new(significand, exponent)
    }
}

impl<const B: Word> Repr<B> {
    /// The base of the representation. It's exposed as an [IBig] constant.
    pub const BASE: UBig = UBig::from_word(B);

    /// Create a [Repr] instance representing value zero
    #[inline]
    pub const fn zero() -> Self {
        Self {
            significand: IBig::ZERO,
            exponent: 0,
        }
    }
    /// Create a [Repr] instance representing value one
    #[inline]
    pub const fn one() -> Self {
        Self {
            significand: IBig::ONE,
            exponent: 0,
        }
    }
    /// Create a [Repr] instance representing value negative one
    #[inline]
    pub const fn neg_one() -> Self {
        Self {
            significand: IBig::NEG_ONE,
            exponent: 0,
        }
    }
    /// Create a [Repr] instance representing the (positive) infinity
    #[inline]
    pub const fn infinity() -> Self {
        Self {
            significand: IBig::ZERO,
            exponent: isize::MAX,
        }
    }
    /// Create a [Repr] instance representing the negative infinity
    #[inline]
    pub const fn neg_infinity() -> Self {
        Self {
            significand: IBig::ZERO,
            exponent: isize::MIN,
        }
    }
    /// Create a [Repr] instance representing the negative zero (`-0`)
    ///
    /// Negative zero is produced by operations (e.g. `1 / -inf`, `ceil(-0)`, cancellation
    /// under round-toward-negative) and is distinct from `+0` only in operations that are
    /// sensitive to the sign of zero (e.g. `1 / -0 = -inf`). It compares equal to `+0`.
    #[inline]
    pub const fn neg_zero() -> Self {
        Self {
            significand: IBig::ZERO,
            exponent: -1,
        }
    }

    /// Determine if the [Repr] represents zero
    ///
    /// Note that this returns `true` only for `+0`; use [`Self::is_neg_zero`] to detect `-0`,
    /// or check `self.significand.is_zero()` to detect either signed zero.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_float::Repr;
    /// assert!(Repr::<2>::zero().is_zero());
    /// assert!(!Repr::<10>::neg_zero().is_zero());
    /// assert!(!Repr::<10>::one().is_zero());
    /// ```
    #[inline]
    pub const fn is_zero(&self) -> bool {
        self.significand.is_zero() && self.exponent == 0
    }

    /// Determine if the [Repr] represents the negative zero (`-0`)
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_float::Repr;
    /// assert!(Repr::<2>::neg_zero().is_neg_zero());
    /// assert!(!Repr::<10>::zero().is_neg_zero());
    /// assert!(!Repr::<10>::one().is_neg_zero());
    /// ```
    #[inline]
    pub const fn is_neg_zero(&self) -> bool {
        self.significand.is_zero() && self.exponent == -1
    }

    /// Determine if the [Repr] represents one
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_float::Repr;
    /// assert!(Repr::<2>::zero().is_zero());
    /// assert!(!Repr::<10>::one().is_zero());
    /// ```
    #[inline]
    pub const fn is_one(&self) -> bool {
        self.significand.is_one() && self.exponent == 0
    }

    /// Determine if the [Repr] represents the (±)infinity
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_float::Repr;
    /// assert!(Repr::<2>::infinity().is_infinite());
    /// assert!(Repr::<10>::neg_infinity().is_infinite());
    /// assert!(!Repr::<10>::one().is_infinite());
    /// assert!(!Repr::<10>::neg_zero().is_infinite());
    /// ```
    #[inline]
    pub const fn is_infinite(&self) -> bool {
        self.significand.is_zero() && (self.exponent == isize::MAX || self.exponent == isize::MIN)
    }

    /// Determine if the [Repr] represents a finite number
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_float::Repr;
    /// assert!(Repr::<2>::zero().is_finite());
    /// assert!(Repr::<10>::one().is_finite());
    /// assert!(!Repr::<16>::infinity().is_finite());
    /// ```
    #[inline]
    pub const fn is_finite(&self) -> bool {
        !self.is_infinite()
    }

    /// Determine if the number can be regarded as an integer.
    ///
    /// Note that this function returns false when the number is infinite.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_float::Repr;
    /// assert!(Repr::<2>::zero().is_int());
    /// assert!(Repr::<10>::one().is_int());
    /// assert!(!Repr::<16>::new(123.into(), -1).is_int());
    /// ```
    pub fn is_int(&self) -> bool {
        if self.is_infinite() {
            false
        } else {
            self.exponent >= 0
        }
    }

    /// Get the sign of the number
    ///
    /// Note that `-0` has a negative sign (so `1 / -0 = -inf`), while `+0` has a positive sign.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::Sign;
    /// # use dashu_float::Repr;
    /// assert_eq!(Repr::<2>::zero().sign(), Sign::Positive);
    /// assert_eq!(Repr::<2>::neg_zero().sign(), Sign::Negative);
    /// assert_eq!(Repr::<2>::neg_one().sign(), Sign::Negative);
    /// assert_eq!(Repr::<10>::neg_infinity().sign(), Sign::Negative);
    /// ```
    #[inline]
    pub const fn sign(&self) -> Sign {
        if self.significand.is_zero() {
            if self.exponent >= 0 {
                Sign::Positive
            } else {
                Sign::Negative
            }
        } else {
            self.significand.sign()
        }
    }

    /// Negate the number, correctly toggling the sign of `±0` and `±inf` by flipping the
    /// special-value exponent (negating the significand alone is a no-op for zero).
    #[inline]
    pub(crate) fn neg(self) -> Self {
        if self.significand.is_zero() {
            Self {
                significand: self.significand,
                exponent: negate_special_exponent(self.exponent),
            }
        } else {
            Self {
                significand: -self.significand,
                exponent: self.exponent,
            }
        }
    }

    /// Normalize the float representation so that the significand is not divisible by the base.
    ///
    /// A zero significand denotes a canonical special value (`+0`, `-0`, `+inf`, `-inf`) and is
    /// returned unchanged; any other (non-canonical) zero significand is normalized to `+0`.
    pub(crate) fn normalize(self) -> Self {
        if self.significand.is_zero() {
            // Preserve the four canonical special-value encodings; collapse anything else to +0.
            if self.exponent == 0
                || self.exponent == -1
                || self.exponent == isize::MAX
                || self.exponent == isize::MIN
            {
                return self;
            }
            return Self::zero();
        }

        let Self {
            mut significand,
            mut exponent,
        } = self;
        if B == 2 {
            let shift = significand.trailing_zeros().unwrap();
            significand >>= shift;
            exponent += shift as isize;
        } else if B.is_power_of_two() {
            let bits = B.trailing_zeros() as usize;
            let shift = significand.trailing_zeros().unwrap() / bits;
            significand >>= shift * bits;
            exponent += shift as isize;
        } else {
            let (sign, mut mag) = significand.into_parts();
            let shift = mag.remove(&UBig::from_word(B)).unwrap();
            exponent += shift as isize;
            significand = IBig::from_parts(sign, mag);
        }
        Self {
            significand,
            exponent,
        }
    }

    /// Get the number of digits (under base `B`) in the significand.
    ///
    /// If the number is 0, then 0 is returned (instead of 1).
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_float::Repr;
    /// assert_eq!(Repr::<2>::zero().digits(), 0);
    /// assert_eq!(Repr::<2>::one().digits(), 1);
    /// assert_eq!(Repr::<10>::one().digits(), 1);
    ///
    /// assert_eq!(Repr::<10>::new(100.into(), 0).digits(), 1); // 1e2
    /// assert_eq!(Repr::<10>::new(101.into(), 0).digits(), 3);
    /// ```
    #[inline]
    pub fn digits(&self) -> usize {
        assert_finite(self);
        digit_len::<B>(&self.significand)
    }

    /// Fast over-estimation of [digits][Self::digits]
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_float::Repr;
    /// assert_eq!(Repr::<2>::zero().digits_ub(), 0);
    /// assert_eq!(Repr::<2>::one().digits_ub(), 1);
    /// assert_eq!(Repr::<10>::one().digits_ub(), 1);
    /// assert_eq!(Repr::<2>::new(31.into(), 0).digits_ub(), 5);
    /// assert_eq!(Repr::<10>::new(99.into(), 0).digits_ub(), 2);
    /// ```
    #[inline]
    pub fn digits_ub(&self) -> usize {
        assert_finite(self);
        if self.significand.is_zero() {
            return 0;
        }

        let log = match B {
            2 => self.significand.log2_bounds().1,
            10 => self.significand.log2_bounds().1 * core::f32::consts::LOG10_2,
            _ => self.significand.log2_bounds().1 / Self::BASE.log2_bounds().0,
        };
        log as usize + 1
    }

    /// Fast under-estimation of [digits][Self::digits]
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_float::Repr;
    /// assert_eq!(Repr::<2>::zero().digits_lb(), 0);
    /// assert_eq!(Repr::<2>::one().digits_lb(), 0);
    /// assert_eq!(Repr::<10>::one().digits_lb(), 0);
    /// assert!(Repr::<10>::new(1001.into(), 0).digits_lb() <= 3);
    /// ```
    #[inline]
    pub fn digits_lb(&self) -> usize {
        assert_finite(self);
        if self.significand.is_zero() {
            return 0;
        }

        let log = match B {
            2 => self.significand.log2_bounds().0,
            10 => self.significand.log2_bounds().0 * core::f32::consts::LOG10_2,
            _ => self.significand.log2_bounds().0 / Self::BASE.log2_bounds().1,
        };
        log as usize
    }

    /// Quickly test if `|self| < 1`. IT's not always correct,
    /// but there are guaranteed to be no false postives.
    #[inline]
    pub(crate) fn smaller_than_one(&self) -> bool {
        debug_assert!(self.is_finite());
        self.exponent + (self.digits_ub() as isize) < -1
    }

    /// Create a [Repr] from the significand and exponent. This
    /// constructor will normalize the representation.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::IBig;
    /// # use dashu_float::Repr;
    /// let a = Repr::<2>::new(400.into(), -2);
    /// assert_eq!(a.significand(), &IBig::from(25));
    /// assert_eq!(a.exponent(), 2);
    ///
    /// let b = Repr::<10>::new(400.into(), -2);
    /// assert_eq!(b.significand(), &IBig::from(4));
    /// assert_eq!(b.exponent(), 0);
    /// ```
    #[inline]
    pub fn new(significand: IBig, exponent: isize) -> Self {
        Self {
            significand,
            exponent,
        }
        .normalize()
    }

    /// Get the significand of the representation
    #[inline]
    pub fn significand(&self) -> &IBig {
        &self.significand
    }

    /// Get the exponent of the representation
    #[inline]
    pub fn exponent(&self) -> isize {
        self.exponent
    }

    /// Convert the float number into raw `(signficand, exponent)` parts
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_float::Repr;
    /// use dashu_int::IBig;
    ///
    /// let a = Repr::<2>::new(400.into(), -2);
    /// assert_eq!(a.into_parts(), (IBig::from(25), 2));
    ///
    /// let b = Repr::<10>::new(400.into(), -2);
    /// assert_eq!(b.into_parts(), (IBig::from(4), 0));
    /// ```
    #[inline]
    pub fn into_parts(self) -> (IBig, isize) {
        (self.significand, self.exponent)
    }

    /// Create an Repr from a static sequence of [Word][crate::Word]s representing the significand.
    ///
    /// This method is intended for static creation macros.
    #[doc(hidden)]
    #[rustversion::since(1.64)]
    #[inline]
    pub const unsafe fn from_static_words(
        sign: Sign,
        significand: &'static [Word],
        exponent: isize,
    ) -> Self {
        let significand = IBig::from_static_words(sign, significand);
        assert!(!significand.is_multiple_of_const(B as _));

        Self {
            significand,
            exponent,
        }
    }
}

// This custom implementation is necessary due to https://github.com/rust-lang/rust/issues/98374
impl<const B: Word> Clone for Repr<B> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            significand: self.significand.clone(),
            exponent: self.exponent,
        }
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        self.significand.clone_from(&source.significand);
        self.exponent = source.exponent;
    }
}

impl<R: Round> Context<R> {
    /// Create a float operation context with the given precision limit.
    #[inline]
    pub const fn new(precision: usize) -> Self {
        Self {
            precision,
            _marker: PhantomData,
        }
    }

    /// Create a float operation context with the higher precision from the two context inputs.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashu_float::{Context, round::mode::Zero};
    ///
    /// let ctxt1 = Context::<Zero>::new(2);
    /// let ctxt2 = Context::<Zero>::new(5);
    /// assert_eq!(Context::max(ctxt1, ctxt2).precision(), 5);
    /// ```
    #[inline]
    pub const fn max(lhs: Self, rhs: Self) -> Self {
        Self {
            // this comparison also correctly handles ulimited precisions (precision = 0)
            precision: if lhs.precision > rhs.precision {
                lhs.precision
            } else {
                rhs.precision
            },
            _marker: PhantomData,
        }
    }

    /// Check whether the precision is limited (not zero)
    #[inline]
    pub(crate) const fn is_limited(&self) -> bool {
        self.precision != 0
    }

    /// Get the precision limited from the context
    #[inline]
    pub const fn precision(&self) -> usize {
        self.precision
    }

    /// Round the repr to the desired precision
    pub(crate) fn repr_round<const B: Word>(&self, repr: Repr<B>) -> Rounded<Repr<B>> {
        assert_finite(&repr);
        if !self.is_limited() {
            return Exact(repr);
        }

        let digits = repr.digits();
        if digits > self.precision {
            let shift = digits - self.precision;
            let input_neg = repr.sign() == Sign::Negative;
            let (signif_hi, signif_lo) = split_digits::<B>(repr.significand, shift);
            let adjust = R::round_fract::<B>(&signif_hi, signif_lo, shift);
            let sig = signif_hi + adjust;
            let result = rounded_to_repr(sig, repr.exponent + shift as isize, input_neg);
            Inexact(result, adjust)
        } else {
            Exact(repr)
        }
    }

    /// Round the repr to the desired precision
    pub(crate) fn repr_round_ref<const B: Word>(&self, repr: &Repr<B>) -> Rounded<Repr<B>> {
        assert_finite(repr);
        if !self.is_limited() {
            return Exact(repr.clone());
        }

        let digits = repr.digits();
        if digits > self.precision {
            let shift = digits - self.precision;
            let input_neg = repr.sign() == Sign::Negative;
            let (signif_hi, signif_lo) = split_digits_ref::<B>(&repr.significand, shift);
            let adjust = R::round_fract::<B>(&signif_hi, signif_lo, shift);
            let sig = signif_hi + adjust;
            let result = rounded_to_repr(sig, repr.exponent + shift as isize, input_neg);
            Inexact(result, adjust)
        } else {
            Exact(repr.clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashu_base::Sign;

    #[test]
    fn infinity_encoding() {
        assert_eq!(Repr::<2>::infinity().exponent, isize::MAX);
        assert_eq!(Repr::<10>::neg_infinity().exponent, isize::MIN);
        assert!(Repr::<2>::infinity().is_infinite());
        assert!(Repr::<10>::neg_infinity().is_infinite());
        assert!(!Repr::<2>::infinity().is_finite());
        assert_eq!(Repr::<2>::infinity().sign(), Sign::Positive);
        assert_eq!(Repr::<10>::neg_infinity().sign(), Sign::Negative);
    }

    #[test]
    fn neg_zero_encoding() {
        assert_eq!(Repr::<2>::neg_zero().exponent, -1);
        assert!(Repr::<2>::neg_zero().is_neg_zero());
        assert!(!Repr::<2>::neg_zero().is_zero());
        assert!(!Repr::<2>::neg_zero().is_infinite());
        assert_eq!(Repr::<2>::neg_zero().sign(), Sign::Negative);
        assert_eq!(Repr::<2>::zero().sign(), Sign::Positive);
    }

    #[test]
    fn normalize_preserves_specials() {
        // infinities are preserved (the previous clobbering bug)
        assert_eq!(Repr::<2>::infinity(), Repr::<2>::infinity().normalize());
        assert_eq!(Repr::<10>::neg_infinity(), Repr::<10>::neg_infinity().normalize());
        // +0 is preserved
        assert_eq!(Repr::<2>::zero(), Repr::<2>::zero().normalize());
        // a stray zero significand with a non-sentinel exponent collapses to +0
        let stray: Repr<2> = Repr {
            significand: IBig::ZERO,
            exponent: 7,
        };
        assert_eq!(Repr::<2>::zero(), stray.normalize());
        // non-zero significands are still normalized
        let r: Repr<2> = Repr {
            significand: IBig::from(0b10100i32),
            exponent: 0,
        };
        let r = r.normalize();
        assert_eq!(r.significand, IBig::from(0b101i32));
        assert_eq!(r.exponent, 2);
    }
}
