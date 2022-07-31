use crate::{
    round::{mode, Round},
    utils::get_precision,
};
use core::marker::PhantomData;
use dashu_int::{IBig, Sign, Word};

// TODO(next): change type of radix from usize to Word
// TODO(next): rename mantissa to significand, radix to base

/// An arbitrary precision floating number represented as `mantissa * radix^exponent`, with a precision
/// such that `|mantissa| < radix^precision`. The mantissa is also called significant. The representation
/// is always normalized (nonzero mantissa is not divisible by radix, or zero mantissa with zero exponent).
///
/// The rounding mode of operations between the float numbers is defined by `Rounding`, its value has to
/// be one of [RoundingMode]. Operations are permitted only between float numbers with the same radix and
/// rounding mode. Note that the rounding is only for operations, it's not "associated" with the value.
/// For example, for correct subtraction, the two operands should have reverse rounding direction.
///
/// # Generic Parameters
/// The const generic parameters will be abbreviated as RADIX -> X, Rounding -> R.
/// RADIX should be in range \[2, isize::MAX\], and Rounding value has to be one of [RoundingMode]
///
/// # Infinity
///
/// This struct supports representation the infinity, but the infinity is only supposed to be used as sentinels.
/// That is, only equality test and comparison are implemented for infinity. Any other operations performed
/// with infinity will lead to panic.
///
/// The infinities are represented as:
/// * [Positive infinity][FloatRepr::INFINITY] (+∞): mantissa = 0, exponent > 0
/// * [Negative infinity][FloatRepr::NEG_INFINITY] (-∞): mantissa = 0, exponenet < 0
///
pub struct FloatRepr<const RADIX: usize, RoundingMode: Round = mode::Zero> {
    pub(crate) mantissa: IBig,
    pub(crate) exponent: isize,
    pub(crate) precision: usize, // TODO: let precision = 0 implies no precision bound, but when no-precision number operates with another has-precision number, the precision will be set as the other one's. This will requires us to make sure 0 value also has non-zero precision (1 will be ideal)
    pub(crate) _marker: PhantomData<RoundingMode>,
}

// this implementation is necessary due to the limitation of `#[derive(Clone)]`
impl<const X: usize, R: Round> Clone for FloatRepr<X, R> {
    fn clone(&self) -> Self {
        Self {
            mantissa: self.mantissa.clone(),
            exponent: self.exponent,
            precision: self.precision,
            _marker: PhantomData,
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.mantissa.clone_from(&source.mantissa);
        self.exponent = source.exponent;
        self.precision = source.precision;
    }
}

impl<const X: usize, R: Round> FloatRepr<X, R> {
    /// Get the maximum precision set for the float number.
    #[inline]
    pub const fn precision(&self) -> usize {
        self.precision
    }

    /// Get the actual precision needed for the float number.
    ///
    /// Shrink to this value using [Self::with_precision] will not cause loss of float precision.
    #[inline]
    pub fn actual_precision(&self) -> usize {
        get_precision::<X>(&self.mantissa)
    }

    /// Convert raw parts into a float number, the precision will be inferred from mantissa
    /// (the lowest k such that `mantissa < radix^k`)
    ///
    /// # Panics
    ///
    /// Panics if the mantissa is larger than `radix^usize::MAX`
    #[inline]
    pub fn from_parts(mantissa: IBig, exponent: isize) -> Self {
        // TODO: check we are not using this function internally because we enforce normalized representation
        let (mantissa, exponent) = Self::normalize(mantissa, exponent);
        let precision = get_precision::<X>(&mantissa).max(1); // set precision to 1 if mantissa is zero
        Self {
            mantissa,
            exponent,
            precision,
            _marker: PhantomData,
        }
    }

    pub const fn from_parts_const(
        sign: Sign,
        man_lo: Word,
        man_hi: Word,
        mut exponent: isize,
    ) -> Self {
        let mut mantissa = (man_hi as u128) << Word::BITS | (man_lo as u128);
        if mantissa == 0 {
            return Self::ZERO;
        }

        let mut precision = 0;

        // normalize
        if X.is_power_of_two() {
            let xbits = X.trailing_zeros();
            let shift = mantissa.trailing_zeros() / xbits;
            mantissa >>= shift * xbits;
            exponent += shift as isize;
            precision = ((u128::BITS - mantissa.leading_zeros() + xbits - 1) / xbits) as usize;
        } else {
            let mut pow: u128 = 1;
            while mantissa % (X as u128) == 0 {
                mantissa /= X as u128;
                exponent += 1;
            }
            while let Some(next) = pow.checked_mul(X as u128) {
                precision += 1;
                if next > mantissa {
                    break;
                }
                pow = next;
            }
        }

        let low = (mantissa & Word::MAX as u128) as Word;
        let high = (mantissa >> Word::BITS) as Word;
        Self {
            mantissa: IBig::from_parts_const(sign, low, high),
            exponent,
            precision,
            _marker: PhantomData,
        }
    }

    /// Convert raw parts into a float number, with given precision.
    #[inline]
    pub fn from_parts_with_precision(mantissa: IBig, exponent: isize, precision: usize) -> Self {
        Self::from_parts(mantissa, exponent).with_precision(precision)
    }

    /// Convert the float number into raw (mantissa, exponent) parts
    #[inline]
    pub fn into_parts(self) -> (IBig, isize) {
        (self.mantissa, self.exponent)
    }

    #[inline]
    const fn zero() -> Self {
        Self {
            mantissa: IBig::ZERO,
            exponent: 0,
            precision: 1,
            _marker: PhantomData,
        }
    }
    /// [FloatRepr] with value 0
    pub const ZERO: Self = Self::zero();

    #[inline]
    const fn one() -> Self {
        Self {
            mantissa: IBig::ONE,
            exponent: 0,
            precision: 1,
            _marker: PhantomData,
        }
    }
    /// [FloatRepr] with value 1
    pub const ONE: Self = Self::one();

    #[inline]
    const fn neg_one() -> Self {
        Self {
            mantissa: IBig::NEG_ONE,
            exponent: 0,
            precision: 1,
            _marker: PhantomData,
        }
    }
    /// [FloatRepr] with value -1
    pub const NEG_ONE: Self = Self::neg_one();

    #[inline]
    const fn infinity() -> Self {
        Self {
            mantissa: IBig::ZERO,
            exponent: 1,
            precision: 0,
            _marker: PhantomData,
        }
    }
    /// [FloatRepr] representing positive infinity (+∞)
    pub const INFINITY: Self = Self::infinity();

    #[inline]
    const fn neg_infinity() -> Self {
        Self {
            mantissa: IBig::ZERO,
            exponent: -1,
            precision: 0,
            _marker: PhantomData,
        }
    }
    /// [FloatRepr] representing negative infinity (-∞)
    pub const NEG_INFINITY: Self = Self::neg_infinity();

    pub const fn is_zero(&self) -> bool {
        self.mantissa.is_zero() && self.exponent == 0
    }

    pub const fn is_one(&self) -> bool {
        self.mantissa.is_one() && self.exponent == 0
    }

    pub const fn is_infinite(&self) -> bool {
        self.mantissa.is_zero() && self.exponent != 0
    }

    pub const fn is_finite(&self) -> bool {
        !self.is_infinite()
    }

    fn ulp(&self) -> Self {
        // reference: https://docs.python.org/3/library/math.html#math.ulp
        unimplemented!()
    }

    fn ceil(&self) -> IBig {
        unimplemented!()
    }

    fn floor(&self) -> IBig {
        unimplemented!()
    }

    fn trunc(&self) -> Self {
        unimplemented!()
    }

    fn fract(&self) -> Self {
        unimplemented!()
    }
}

/// A wrapper over the float number. All the operations on the number will return the rounding
/// error along with the result.
pub struct ApprRepr<const RADIX: usize, RoundingMode: Round = mode::Zero>(FloatRepr<RADIX, RoundingMode>);
