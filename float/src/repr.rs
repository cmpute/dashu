use crate::{
    round::{Round, Rounded},
    utils::{base_as_ibig, digit_len, split_digits, split_digits_ref},
};
use core::marker::PhantomData;
use dashu_base::{Approximation::*, EstimatedLog2, Sign};
use dashu_int::{IBig, UBig};
pub use dashu_int::Word;

// TODO(next): make Repr public to user, only expose constructors and maybe a selective subset of methods.
//       then all context methods could take repr as input instead of a full FBig object.
//       also add conversion between FBig and repr (FBig::{new(repr, context), repr(), into_repr()}).
//       this is necessary for compositing complex numbers without duplicate context.
//       the existing FBig::new() should be renamed as new_raw(), the new FBig::new() will check whether repr is within the precision limitation (panic if not)
#[derive(PartialEq, Eq)]
pub struct Repr<const BASE: Word> {
    pub(crate) significand: IBig,
    pub(crate) exponent: isize,
}

#[derive(Clone, Copy)]
pub struct Context<RoundingMode: Round> {
    /// The precision of the floating point number. If set to zero, then
    /// the precision is unlimited
    pub(crate) precision: usize,
    _marker: PhantomData<RoundingMode>,
}

impl<const B: Word> Repr<B> {
    pub const BASE: IBig = base_as_ibig::<B>();

    #[inline]
    pub const fn zero() -> Self {
        Self {
            significand: IBig::ZERO,
            exponent: 0,
        }
    }
    #[inline]
    pub const fn one() -> Self {
        Self {
            significand: IBig::ONE,
            exponent: 0,
        }
    }
    #[inline]
    pub const fn neg_one() -> Self {
        Self {
            significand: IBig::NEG_ONE,
            exponent: 0,
        }
    }

    #[inline]
    pub const fn infinity() -> Self {
        Self {
            significand: IBig::ZERO,
            exponent: 1,
        }
    }
    #[inline]
    pub const fn neg_infinity() -> Self {
        Self {
            significand: IBig::ZERO,
            exponent: -1,
        }
    }

    #[inline]
    pub const fn is_zero(&self) -> bool {
        self.significand.is_zero() && self.exponent == 0
    }
    #[inline]
    pub const fn is_one(&self) -> bool {
        self.significand.is_one() && self.exponent == 0
    }
    #[inline]
    pub const fn is_infinite(&self) -> bool {
        self.significand.is_zero() && self.exponent != 0
    }
    #[inline]
    pub const fn is_finite(&self) -> bool {
        !self.is_infinite()
    }

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
    pub fn normalize(self) -> Self {
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

    /// Get the number of digits in the significand.
    #[inline]
    pub fn digits(&self) -> usize {
        digit_len::<B>(&self.significand)
    }

    /// Fast over estimation of [digits][Self::digits]
    #[inline]
    pub fn digits_ub(&self) -> usize {
        let log = if B == 10 {
            self.significand.log2_bounds().1 * core::f32::consts::LOG10_2
        } else {
            self.significand.log2_bounds().1 / Self::BASE.log2_bounds().0
        };
        log as usize + 1
    }

    /// Fast under estimation of [digits][Self::digits]
    #[inline]
    pub fn digits_lb(&self) -> usize {
        let log = if B == 10 {
            self.significand.log2_bounds().0 * core::f32::consts::LOG10_2
        } else {
            self.significand.log2_bounds().0 / Self::BASE.log2_bounds().1
        };
        log as usize
    }

    /// Create a [Repr] from significand and exponent. This
    /// constructor will normalize the representation.
    #[inline]
    pub fn new(significand: IBig, exponent: isize) -> Self {
        Self {
            significand,
            exponent,
        }
        .normalize()
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
    #[inline]
    pub const fn new(precision: usize) -> Self {
        Self {
            precision,
            _marker: PhantomData,
        }
    }

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
    pub(crate) fn limited(&self) -> bool {
        self.precision != 0
    }

    /// Round the repr to the desired precision
    pub(crate) fn repr_round<const B: Word>(
        &self,
        repr: Repr<B>,
    ) -> Rounded<Repr<B>> {
        assert!(repr.is_finite());
        if !self.limited() {
            return Exact(repr);
        }

        let digits = repr.digits();
        if digits > self.precision {
            let shift = digits - self.precision;
            let (signif_hi, signif_lo) = split_digits::<B>(repr.significand, shift);
            let adjust = R::round_fract::<B>(&signif_hi, signif_lo, shift);
            Inexact(
                Repr::new(signif_hi + adjust, repr.exponent + shift as isize),
                adjust,
            )
        } else {
            Exact(repr)
        }
    }

    /// Round the repr to the desired precision
    pub(crate) fn repr_round_ref<const B: Word>(
        &self,
        repr: &Repr<B>,
    ) -> Rounded<Repr<B>> {
        assert!(repr.is_finite());        
        if !self.limited() {
            return Exact(repr.clone());
        }

        let digits = repr.digits();
        if digits > self.precision {
            let shift = digits - self.precision;
            let (signif_hi, signif_lo) = split_digits_ref::<B>(&repr.significand, shift);
            let adjust = R::round_fract::<B>(&signif_hi, signif_lo, shift);
            Inexact(
                Repr::new(signif_hi + adjust, repr.exponent + shift as isize),
                adjust,
            )
        } else {
            Exact(repr.clone())
        }
    }
}
