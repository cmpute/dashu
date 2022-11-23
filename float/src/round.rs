//! Traits and implementations for rounding during operations.

use core::cmp::Ordering;
use core::ops::{Add, AddAssign};
use dashu_base::{Approximation, EstimatedLog2, Sign, UnsignedAbs};
use dashu_int::{IBig, UBig, Word};

/// Built-in rounding modes of the floating numbers.
///
/// # Rounding Error
///
/// For different rounding modes, the [Rounding][crate::round::Rounding] error
/// in the output of operations tells the error range, as described in
/// the table below.
///
/// | Mode     | Rounding | Error (truth - estimation) Range |
/// |----------|----------|----------------------------------|
/// | Zero     | NoOp     | `(-1 ulp 0)` or `(0, 1 ulp)`*    |
/// | Away     | AddOne   | `(-1 ulp, 0)`                    |
/// | Away     | SubOne   | `(0, 1 ulp)`                     |
/// | Down     | SubOne   | `(0, 1 ulp)`                     |
/// | Up       | AddOne   | `(-1 ulp, 0)`                    |
/// | HalfAway | AddOne   | `[-1/2 ulp, 0)`                  |
/// | HalfAway | NoOp     | `(-1/2 ulp, 1/2 ulp)`            |
/// | HalfAway | SubOne   | `(0, 1/2 ulp]`                   |
/// | HalfEven | AddOne   | `[-1/2 ulp, 0)`                  |
/// | HalfEven | NoOp     | `[-1/2 ulp, 1/2 ulp]`            |
/// | HalfEven | SubOne   | `(0, 1/2 ulp]`                   |
///
/// *: Dependends on the sign of the result
///
pub mod mode {
    /// Round toward 0 (default mode for binary float)
    #[derive(Clone, Copy)]
    pub struct Zero;

    /// Round away from 0
    #[derive(Clone, Copy)]
    pub struct Away;

    /// Round toward +∞
    #[derive(Clone, Copy)]
    pub struct Up;

    /// Round toward -∞
    #[derive(Clone, Copy)]
    pub struct Down;

    /// Round to the nearest value, ties are rounded to an even value. (default mode for decimal float)
    #[derive(Clone, Copy)]
    pub struct HalfEven;

    /// Round to the nearest value, ties away from zero
    #[derive(Clone, Copy)]
    pub struct HalfAway;
}

/// The adjustment of a rounding operation
///
/// See [the `mode` module][mode] for the corresponding error bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rounding {
    /// No adjustment
    NoOp,

    /// Add one
    AddOne,

    /// Subtract one
    SubOne,
}

/// A type representing float operation result
///
/// If the operation result is inexact, the adjustment from the final rounding
/// will be returned along with the result.
pub type Rounded<T> = Approximation<T, Rounding>;

/// A trait describing the rounding strategy
pub trait Round: Copy {
    /// The rounding operation that rounds to an opposite direction
    type Reverse: Round;

    /// Calculate the rounding of the number (integer + rem), assuming rem != 0 and |rem| < 1.
    /// `low_half_test` should tell |rem|.cmp(0.5)
    fn round_low_part<F: FnOnce() -> Ordering>(
        integer: &IBig,
        low_sign: Sign,
        low_half_test: F,
    ) -> Rounding;

    /// Calculate the rounding of the number (integer + fract / X^precision),
    /// assuming |fract| / X^precision < 1. Return the adjustment.
    #[inline]
    fn round_fract<const B: Word>(integer: &IBig, fract: IBig, precision: usize) -> Rounding {
        // this assertion is costly, so only check in debug mode
        debug_assert!(fract.clone().unsigned_abs() < UBig::from_word(B).pow(precision));

        if fract.is_zero() {
            return Rounding::NoOp;
        }
        let (fsign, fmag) = fract.into_parts();

        let test = || {
            // first use the estimated log2 to do coarse comparison, then do the exact comparison
            let (lb, ub) = fmag.log2_bounds();
            let (b_lb, b_ub) = B.log2_bounds();

            // 0.999 and 1.001 are used here to prevent the influence of the precision loss of the multiplcations
            if lb + 0.999 > b_ub * precision as f32 {
                Ordering::Greater
            } else if ub + 1.001 < b_lb * precision as f32 {
                Ordering::Less
            } else {
                (fmag << 1).cmp(&UBig::from_word(B).pow(precision))
            }
        };
        Self::round_low_part::<_>(integer, fsign, test)
    }

    /// Calculate the rounding of the number (integer + numerator / denominator),
    /// assuming |numerator / denominator| < 1. Return the adjustment.
    #[inline]
    fn round_ratio(integer: &IBig, num: IBig, den: &IBig) -> Rounding {
        assert!(!den.is_zero());
        // this assertion can be costly, so only check in debug mode
        debug_assert!(num.clone().unsigned_abs() < den.clone().unsigned_abs());

        if num.is_zero() {
            return Rounding::NoOp;
        }
        let (nsign, nmag) = num.into_parts();
        Self::round_low_part::<_>(integer, nsign * den.sign(), || {
            if den.sign() == Sign::Positive {
                IBig::from(nmag << 1).cmp(den)
            } else {
                den.cmp(&-(nmag << 1))
            }
        })
    }
}

impl Round for mode::Zero {
    type Reverse = mode::Away;

    #[inline]
    fn round_low_part<F: FnOnce() -> Ordering>(
        integer: &IBig,
        low_sign: Sign,
        _low_half_test: F,
    ) -> Rounding {
        if integer.is_zero() {
            return Rounding::NoOp;
        }
        match (integer.sign(), low_sign) {
            (Sign::Positive, Sign::Positive) | (Sign::Negative, Sign::Negative) => Rounding::NoOp,
            (Sign::Positive, Sign::Negative) => Rounding::SubOne,
            (Sign::Negative, Sign::Positive) => Rounding::AddOne,
        }
    }
}

impl Round for mode::Away {
    type Reverse = mode::Zero;

    #[inline]
    fn round_low_part<F: FnOnce() -> Ordering>(
        integer: &IBig,
        low_sign: Sign,
        _low_half_test: F,
    ) -> Rounding {
        if integer.is_zero() {
            match low_sign {
                Sign::Positive => Rounding::AddOne,
                Sign::Negative => Rounding::SubOne,
            }
        } else {
            match (integer.sign(), low_sign) {
                (Sign::Positive, Sign::Positive) => Rounding::AddOne,
                (Sign::Negative, Sign::Negative) => Rounding::SubOne,
                (Sign::Positive, Sign::Negative) | (Sign::Negative, Sign::Positive) => {
                    Rounding::NoOp
                }
            }
        }
    }
}

impl Round for mode::Down {
    type Reverse = mode::Up;

    #[inline]
    fn round_low_part<F: FnOnce() -> Ordering>(
        _integer: &IBig,
        low_sign: Sign,
        _low_half_test: F,
    ) -> Rounding {
        // -1 if fract < 0, otherwise 0
        if low_sign == Sign::Negative {
            Rounding::SubOne
        } else {
            Rounding::NoOp
        }
    }
}

impl Round for mode::Up {
    type Reverse = mode::Down;

    #[inline]
    fn round_low_part<F: FnOnce() -> Ordering>(
        _integer: &IBig,
        low_sign: Sign,
        _low_half_test: F,
    ) -> Rounding {
        // +1 if fract > 0, otherwise 0
        if low_sign == Sign::Positive {
            Rounding::AddOne
        } else {
            Rounding::NoOp
        }
    }
}

impl Round for mode::HalfAway {
    type Reverse = Self;

    #[inline]
    fn round_low_part<F: FnOnce() -> Ordering>(
        integer: &IBig,
        low_sign: Sign,
        low_half_test: F,
    ) -> Rounding {
        match low_half_test() {
            // |rem| < 1/2
            Ordering::Less => Rounding::NoOp,
            // |rem| = 1/2
            Ordering::Equal => {
                // +1 if integer and rem >= 0, -1 if integer and rem <= 0
                if integer >= &IBig::ZERO && low_sign == Sign::Positive {
                    Rounding::AddOne
                } else if integer <= &IBig::ZERO && low_sign == Sign::Negative {
                    Rounding::SubOne
                } else {
                    Rounding::NoOp
                }
            }
            // |rem| > 1/2
            Ordering::Greater => {
                // +1 if rem > 0, -1 if rem < 0
                match low_sign {
                    Sign::Positive => Rounding::AddOne,
                    Sign::Negative => Rounding::SubOne,
                }
            }
        }
    }
}

impl Round for mode::HalfEven {
    type Reverse = Self;

    #[inline]
    fn round_low_part<F: FnOnce() -> Ordering>(
        integer: &IBig,
        low_sign: Sign,
        low_half_test: F,
    ) -> Rounding {
        match low_half_test() {
            // |rem| < 1/2
            Ordering::Less => Rounding::NoOp,
            // |rem| = 1/2
            Ordering::Equal => {
                // if integer is odd, +1 if rem > 0, -1 if rem < 0
                if integer & 1 == IBig::ONE {
                    match low_sign {
                        Sign::Positive => Rounding::AddOne,
                        Sign::Negative => Rounding::SubOne,
                    }
                } else {
                    Rounding::NoOp
                }
            }
            // |rem| > 1/2
            Ordering::Greater => {
                // +1 if rem > 0, -1 if rem < 0
                match low_sign {
                    Sign::Positive => Rounding::AddOne,
                    Sign::Negative => Rounding::SubOne,
                }
            }
        }
    }
}

impl Add<Rounding> for IBig {
    type Output = IBig;
    #[inline]
    fn add(self, rhs: Rounding) -> Self::Output {
        match rhs {
            Rounding::NoOp => self,
            Rounding::AddOne => self + IBig::ONE,
            Rounding::SubOne => self - IBig::ONE,
        }
    }
}

impl Add<Rounding> for &IBig {
    type Output = IBig;
    #[inline]
    fn add(self, rhs: Rounding) -> Self::Output {
        match rhs {
            Rounding::NoOp => self.clone(),
            Rounding::AddOne => self + IBig::ONE,
            Rounding::SubOne => self - IBig::ONE,
        }
    }
}

impl AddAssign<Rounding> for IBig {
    #[inline]
    fn add_assign(&mut self, rhs: Rounding) {
        match rhs {
            Rounding::NoOp => {}
            Rounding::AddOne => *self += IBig::ONE,
            Rounding::SubOne => *self -= IBig::ONE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::{mode::*, Rounding::*};

    #[test]
    fn test_from_fract() {
        #[rustfmt::skip]
        fn test_all_rounding<const B: Word, const D: usize>(
            input: &(i32, i32, Rounding, Rounding, Rounding, Rounding, Rounding, Rounding),
        ) {
            let (value, fract, rnd_zero, rnd_away, rnd_up, rnd_down, rnd_halfeven, rnd_halfaway) = *input;
            let (value, fract) = (IBig::from(value), IBig::from(fract));
            assert_eq!(Zero::round_fract::<B>(&value, fract.clone(), D), rnd_zero);
            assert_eq!(Away::round_fract::<B>(&value, fract.clone(), D), rnd_away);
            assert_eq!(Up::round_fract::<B>(&value, fract.clone(), D), rnd_up);
            assert_eq!(Down::round_fract::<B>(&value, fract.clone(), D), rnd_down);
            assert_eq!(HalfEven::round_fract::<B>(&value, fract.clone(), D), rnd_halfeven);
            assert_eq!(HalfAway::round_fract::<B>(&value, fract, D), rnd_halfaway);
        }

        // cases for radix = 2, 2 digit fraction
        #[rustfmt::skip]
        let binary_cases = [
            // (integer value, fraction part, roundings...)
            // Mode: Zero  , Away  , Up    , Down  , HEven,  HAway
            ( 0,  3, NoOp  , AddOne, AddOne, NoOp  , AddOne, AddOne),
            ( 0,  2, NoOp  , AddOne, AddOne, NoOp  , NoOp  , AddOne),
            ( 0,  1, NoOp  , AddOne, AddOne, NoOp  , NoOp  , NoOp  ),
            ( 0,  0, NoOp  , NoOp  , NoOp  , NoOp  , NoOp  , NoOp  ),
            ( 0, -1, NoOp  , SubOne, NoOp  , SubOne, NoOp  , NoOp  ),
            ( 0, -2, NoOp  , SubOne, NoOp  , SubOne, NoOp  , SubOne),
            ( 0, -3, NoOp  , SubOne, NoOp  , SubOne, SubOne, SubOne),
            ( 1,  3, NoOp  , AddOne, AddOne, NoOp  , AddOne, AddOne),
            ( 1,  2, NoOp  , AddOne, AddOne, NoOp  , AddOne, AddOne),
            ( 1,  1, NoOp  , AddOne, AddOne, NoOp  , NoOp  , NoOp  ),
            ( 1,  0, NoOp  , NoOp  , NoOp  , NoOp  , NoOp  , NoOp  ),
            ( 1, -1, SubOne, NoOp  , NoOp  , SubOne, NoOp  , NoOp  ),
            ( 1, -2, SubOne, NoOp  , NoOp  , SubOne, SubOne, NoOp  ),
            ( 1, -3, SubOne, NoOp  , NoOp  , SubOne, SubOne, SubOne),
            (-1,  3, AddOne, NoOp  , AddOne, NoOp  , AddOne, AddOne),
            (-1,  2, AddOne, NoOp  , AddOne, NoOp  , AddOne, NoOp  ),
            (-1,  1, AddOne, NoOp  , AddOne, NoOp  , NoOp  , NoOp  ),
            (-1,  0, NoOp  , NoOp  , NoOp  , NoOp  , NoOp  , NoOp  ),
            (-1, -1, NoOp  , SubOne, NoOp  , SubOne, NoOp  , NoOp  ),
            (-1, -2, NoOp  , SubOne, NoOp  , SubOne, SubOne, SubOne),
            (-1, -3, NoOp  , SubOne, NoOp  , SubOne, SubOne, SubOne),
        ];
        binary_cases.iter().for_each(test_all_rounding::<2, 2>);

        // cases for radix = 3, 1 digit fraction
        #[rustfmt::skip]
        let tenary_cases = [
            // (integer value, fraction part, roundings...)
            // Mode: Zero,   Away  , Up    , Down  , HEven , HAway
            ( 0,  2, NoOp,   AddOne, AddOne, NoOp  , AddOne, AddOne),
            ( 0,  1, NoOp,   AddOne, AddOne, NoOp  , NoOp  , NoOp  ),
            ( 0,  0, NoOp,   NoOp  , NoOp  , NoOp  , NoOp  , NoOp  ),
            ( 0, -1, NoOp,   SubOne, NoOp  , SubOne, NoOp  , NoOp  ),
            ( 0, -2, NoOp,   SubOne, NoOp  , SubOne, SubOne, SubOne),
            ( 1,  2, NoOp,   AddOne, AddOne, NoOp  , AddOne, AddOne),
            ( 1,  1, NoOp,   AddOne, AddOne, NoOp  , NoOp  , NoOp  ),
            ( 1,  0, NoOp,   NoOp  , NoOp  , NoOp  , NoOp  , NoOp  ),
            ( 1, -1, SubOne, NoOp  , NoOp  , SubOne, NoOp  , NoOp  ),
            ( 1, -2, SubOne, NoOp  , NoOp  , SubOne, SubOne, SubOne),
            (-1,  2, AddOne, NoOp  , AddOne, NoOp  , AddOne, AddOne),
            (-1,  1, AddOne, NoOp  , AddOne, NoOp  , NoOp  , NoOp  ),
            (-1,  0, NoOp,   NoOp  , NoOp  , NoOp  , NoOp  , NoOp  ),
            (-1, -1, NoOp,   SubOne, NoOp  , SubOne, NoOp  , NoOp  ),
            (-1, -2, NoOp,   SubOne, NoOp  , SubOne, SubOne, SubOne),
        ];
        tenary_cases.iter().for_each(test_all_rounding::<3, 1>);

        // cases for radix = 10, 1 digit fraction
        #[rustfmt::skip]
        let decimal_cases = [
            // (integer value, fraction part, roundings...)
            // Mode: Zero  , Away  , Up    , Down  , HEven , HAway
            ( 0,  7, NoOp  , AddOne, AddOne, NoOp  , AddOne, AddOne),
            ( 0,  5, NoOp  , AddOne, AddOne, NoOp  , NoOp  , AddOne),
            ( 0,  2, NoOp  , AddOne, AddOne, NoOp  , NoOp  , NoOp  ),
            ( 0,  0, NoOp  , NoOp  , NoOp  , NoOp  , NoOp  , NoOp  ),
            ( 0, -2, NoOp  , SubOne, NoOp  , SubOne, NoOp  , NoOp  ),
            ( 0, -5, NoOp  , SubOne, NoOp  , SubOne, NoOp  , SubOne),
            ( 0, -7, NoOp  , SubOne, NoOp  , SubOne, SubOne, SubOne),
            ( 1,  7, NoOp  , AddOne, AddOne, NoOp  , AddOne, AddOne),
            ( 1,  5, NoOp  , AddOne, AddOne, NoOp  , AddOne, AddOne),
            ( 1,  2, NoOp  , AddOne, AddOne, NoOp  , NoOp  , NoOp  ),
            ( 1,  0, NoOp  , NoOp  , NoOp  , NoOp  , NoOp  , NoOp  ),
            ( 1, -2, SubOne, NoOp  , NoOp  , SubOne, NoOp  , NoOp  ),
            ( 1, -5, SubOne, NoOp  , NoOp  , SubOne, SubOne, NoOp  ),
            ( 1, -7, SubOne, NoOp  , NoOp  , SubOne, SubOne, SubOne),
            (-1,  7, AddOne, NoOp  , AddOne, NoOp  , AddOne, AddOne),
            (-1,  5, AddOne, NoOp  , AddOne, NoOp  , AddOne, NoOp  ),
            (-1,  2, AddOne, NoOp  , AddOne, NoOp  , NoOp  , NoOp  ),
            (-1,  0, NoOp  , NoOp  , NoOp  , NoOp  , NoOp  , NoOp  ),
            (-1, -2, NoOp  , SubOne, NoOp  , SubOne, NoOp  , NoOp  ),
            (-1, -5, NoOp  , SubOne, NoOp  , SubOne, SubOne, SubOne),
            (-1, -7, NoOp  , SubOne, NoOp  , SubOne, SubOne, SubOne),
        ];
        decimal_cases.iter().for_each(test_all_rounding::<10, 1>);
    }

    #[test]
    fn test_from_ratio() {
        #[rustfmt::skip]
        fn test_all_rounding(
            input: &(i32, i32, i32, Rounding, Rounding, Rounding, Rounding, Rounding, Rounding),
        ) {
            let (value, num, den, rnd_zero, rnd_away, rnd_up, rnd_down, rnd_halfeven, rnd_halfaway) = *input;
            let (value, num, den) = (IBig::from(value), IBig::from(num), IBig::from(den));
            assert_eq!(Zero::round_ratio(&value, num.clone(), &den), rnd_zero);
            assert_eq!(Away::round_ratio(&value, num.clone(), &den), rnd_away);
            assert_eq!(Up::round_ratio(&value, num.clone(), &den), rnd_up);
            assert_eq!(Down::round_ratio(&value, num.clone(), &den), rnd_down);
            assert_eq!(HalfEven::round_ratio(&value, num.clone(), &den), rnd_halfeven);
            assert_eq!(HalfAway::round_ratio(&value, num, &den), rnd_halfaway);
        }

        // cases for radix = 2, 2 digit fraction
        #[rustfmt::skip]
        let test_cases = [
            // (integer value, mumerator, denominator, roundings...)
            // Mode:     Zero  , Away  , Up    , Down  , HEven , HAway
            ( 0,  0,  2, NoOp  , NoOp  , NoOp  , NoOp  , NoOp  , NoOp  ),
            ( 0,  1,  2, NoOp  , AddOne, AddOne, NoOp  , NoOp  , AddOne),
            ( 0, -1,  2, NoOp  , SubOne, NoOp  , SubOne, NoOp  , SubOne),
            ( 0,  0, -2, NoOp  , NoOp  , NoOp  , NoOp  , NoOp  , NoOp  ),
            ( 0,  1, -2, NoOp  , SubOne, NoOp  , SubOne, NoOp  , SubOne),
            ( 0, -1, -2, NoOp  , AddOne, AddOne, NoOp  , NoOp  , AddOne),
            ( 1,  0,  2, NoOp  , NoOp  , NoOp  , NoOp  , NoOp  , NoOp  ),
            ( 1,  1,  2, NoOp  , AddOne, AddOne, NoOp  , AddOne, AddOne),
            ( 1, -1,  2, SubOne, NoOp  , NoOp  , SubOne, SubOne, NoOp  ),
            ( 1,  0, -2, NoOp  , NoOp  , NoOp  , NoOp  , NoOp  , NoOp  ),
            ( 1,  1, -2, SubOne, NoOp  , NoOp  , SubOne, SubOne, NoOp  ),
            ( 1, -1, -2, NoOp  , AddOne, AddOne, NoOp  , AddOne, AddOne),
            (-1,  0,  2, NoOp  , NoOp  , NoOp  , NoOp  , NoOp  , NoOp  ),
            (-1,  1,  2, AddOne, NoOp  , AddOne, NoOp  , AddOne, NoOp  ),
            (-1, -1,  2, NoOp  , SubOne, NoOp  , SubOne, SubOne, SubOne),
            (-1,  0, -2, NoOp  , NoOp  , NoOp  , NoOp  , NoOp  , NoOp  ),
            (-1,  1, -2, NoOp  , SubOne, NoOp  , SubOne, SubOne, SubOne),
            (-1, -1, -2, AddOne, NoOp  , AddOne, NoOp  , AddOne, NoOp  ),

            ( 0, -2,  3, NoOp  , SubOne, NoOp  , SubOne, SubOne, SubOne),
            ( 0, -1,  3, NoOp  , SubOne, NoOp  , SubOne, NoOp  , NoOp  ),
            ( 0,  0,  3, NoOp  , NoOp  , NoOp  , NoOp  , NoOp  , NoOp  ),
            ( 0,  1,  3, NoOp  , AddOne, AddOne, NoOp  , NoOp  , NoOp  ),
            ( 0,  2,  3, NoOp  , AddOne, AddOne, NoOp  , AddOne, AddOne),
            ( 0, -2, -3, NoOp  , AddOne, AddOne, NoOp  , AddOne, AddOne),
            ( 0, -1, -3, NoOp  , AddOne, AddOne, NoOp  , NoOp  , NoOp  ),
            ( 0,  0, -3, NoOp  , NoOp  , NoOp  , NoOp  , NoOp  , NoOp  ),
            ( 0,  1, -3, NoOp  , SubOne, NoOp  , SubOne, NoOp  , NoOp  ),
            ( 0,  2, -3, NoOp  , SubOne, NoOp  , SubOne, SubOne, SubOne),
            ( 1, -2,  3, SubOne, NoOp  , NoOp  , SubOne, SubOne, SubOne),
            ( 1, -1,  3, SubOne, NoOp  , NoOp  , SubOne, NoOp  , NoOp  ),
            ( 1,  0,  3, NoOp  , NoOp  , NoOp  , NoOp  , NoOp  , NoOp  ),
            ( 1,  1,  3, NoOp  , AddOne, AddOne, NoOp  , NoOp  , NoOp  ),
            ( 1,  2,  3, NoOp  , AddOne, AddOne, NoOp  , AddOne, AddOne),
            ( 1, -2, -3, NoOp  , AddOne, AddOne, NoOp  , AddOne, AddOne),
            ( 1, -1, -3, NoOp  , AddOne, AddOne, NoOp  , NoOp  , NoOp  ),
            ( 1,  0, -3, NoOp  , NoOp  , NoOp  , NoOp  , NoOp  , NoOp  ),
            ( 1,  1, -3, SubOne, NoOp  , NoOp  , SubOne, NoOp  , NoOp  ),
            ( 1,  2, -3, SubOne, NoOp  , NoOp  , SubOne, SubOne, SubOne),
            (-1, -2,  3, NoOp  , SubOne, NoOp  , SubOne, SubOne, SubOne),
            (-1, -1,  3, NoOp  , SubOne, NoOp  , SubOne, NoOp  , NoOp  ),
            (-1,  0,  3, NoOp  , NoOp  , NoOp  , NoOp  , NoOp  , NoOp  ),
            (-1,  1,  3, AddOne, NoOp  , AddOne, NoOp  , NoOp  , NoOp  ),
            (-1,  2,  3, AddOne, NoOp  , AddOne, NoOp  , AddOne, AddOne),
            (-1, -2, -3, AddOne, NoOp  , AddOne, NoOp  , AddOne, AddOne),
            (-1, -1, -3, AddOne, NoOp  , AddOne, NoOp  , NoOp  , NoOp  ),
            (-1,  0, -3, NoOp  , NoOp  , NoOp  , NoOp  , NoOp  , NoOp  ),
            (-1,  1, -3, NoOp  , SubOne, NoOp  , SubOne, NoOp  , NoOp  ),
            (-1,  2, -3, NoOp  , SubOne, NoOp  , SubOne, SubOne, SubOne),
        ];
        test_cases.iter().for_each(test_all_rounding);
    }
}
