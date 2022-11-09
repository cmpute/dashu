use crate::{
    error::panic_operate_with_inf,
    round::{Round, Rounded},
    utils::{base_as_ibig, digit_len, split_digits, split_digits_ref},
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
/// `|signficand| < base^precision`. However, the precision limit is not always enforced.
/// In rare cases, the significand can have one more digit than the precision limit.
///
/// # Infinity
///
/// This struct supports representing the infinity, but the infinity is only supposed to be used
/// as sentinels. That is, only equality test and comparison are implemented for the infinity.
/// Any other operations on the infinity will lead to panic. If an operation result is too large
/// or too small, the operation will **panic** instead of returning an infinity.
///
#[derive(PartialEq, Eq)]
pub struct Repr<const BASE: Word> {
    /// The significand of the floating point number. If the significand is zero, then the number is:
    /// - Zero, if exponent = 0
    /// - Positive infinity, if exponent > 0
    /// - Negative infinity, if exponent < 0
    pub(crate) significand: IBig,

    /// The exponent of the floating point number.
    pub(crate) exponent: isize,
}

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

impl<const B: Word> Repr<B> {
    /// The base of the representation. It's exposed as an [IBig] constant.
    pub const BASE: IBig = base_as_ibig::<B>();

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
            exponent: 1,
        }
    }
    /// Create a [Repr] instance representing the negative infinity
    #[inline]
    pub const fn neg_infinity() -> Self {
        Self {
            significand: IBig::ZERO,
            exponent: -1,
        }
    }
    
    // TODO: Add support for representing NEG_ZERO, but don't provide method to generate it.
    // neg_zero: exponent -1, infinity: exponent: isize::MAX, neg_infinity: exponent: isize::MIN

    /// Determine if the [Repr] represents zero
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_float::Repr;
    /// assert!(Repr::<2>::zero().is_zero());
    /// assert!(!Repr::<10>::one().is_zero());
    /// ```
    #[inline]
    pub const fn is_zero(&self) -> bool {
        self.significand.is_zero() && self.exponent == 0
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
    /// ```
    #[inline]
    pub const fn is_infinite(&self) -> bool {
        self.significand.is_zero() && self.exponent != 0
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

    /// Get the sign of the number
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::Sign;
    /// # use dashu_float::Repr;
    /// assert_eq!(Repr::<2>::zero().sign(), Sign::Positive);
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

    /// Normalize the float representation so that the significand is not divisible by the base.
    /// Any floats with zero significand will be considered as zero value (instead of an `INFINITY`)
    pub(crate) fn normalize(self) -> Self {
        let Self {
            mut significand,
            mut exponent,
        } = self;
        if significand.is_zero() {
            return Self::zero();
        }

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
        if self.is_infinite() {
            panic_operate_with_inf();
        }

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
        if self.is_infinite() {
            panic_operate_with_inf();
        } else if self.significand.is_zero() {
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
    /// assert_eq!(Repr::<10>::new(1001.into(), 0).digits_lb(), 3);
    /// ```
    #[inline]
    pub fn digits_lb(&self) -> usize {
        if self.is_infinite() {
            panic_operate_with_inf();
        } else if self.significand.is_zero() {
            return 0;
        }

        let log = match B {
            2 => self.significand.log2_bounds().0,
            10 => self.significand.log2_bounds().0 * core::f32::consts::LOG10_2,
            _ => self.significand.log2_bounds().0 / Self::BASE.log2_bounds().1,
        };
        log as usize
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
    pub(crate) fn is_limited(&self) -> bool {
        self.precision != 0
    }

    /// Get the precision limited from the context
    #[inline]
    pub const fn precision(&self) -> usize {
        self.precision
    }

    /// Round the repr to the desired precision
    pub(crate) fn repr_round<const B: Word>(&self, repr: Repr<B>) -> Rounded<Repr<B>> {
        assert!(repr.is_finite());
        if !self.is_limited() {
            return Exact(repr);
        }

        let digits = repr.digits();
        if digits > self.precision {
            let shift = digits - self.precision;
            let (signif_hi, signif_lo) = split_digits::<B>(repr.significand, shift);
            let adjust = R::round_fract::<B>(&signif_hi, signif_lo, shift);
            Inexact(Repr::new(signif_hi + adjust, repr.exponent + shift as isize), adjust)
        } else {
            Exact(repr)
        }
    }

    /// Round the repr to the desired precision
    pub(crate) fn repr_round_ref<const B: Word>(&self, repr: &Repr<B>) -> Rounded<Repr<B>> {
        assert!(repr.is_finite());
        if !self.is_limited() {
            return Exact(repr.clone());
        }

        let digits = repr.digits();
        if digits > self.precision {
            let shift = digits - self.precision;
            let (signif_hi, signif_lo) = split_digits_ref::<B>(&repr.significand, shift);
            let adjust = R::round_fract::<B>(&signif_hi, signif_lo, shift);
            Inexact(Repr::new(signif_hi + adjust, repr.exponent + shift as isize), adjust)
        } else {
            Exact(repr.clone())
        }
    }
}
