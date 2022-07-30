use core::cmp::Ordering;
use core::ops::{Add, AddAssign};
use dashu_base::UnsignedAbs;
use dashu_int::{IBig, Sign, UBig};

// TODO: refactor RoundingMode to structs, implement a `Round` trait, requiring a `from_fract` and `from_ratio` method.
// And the RoundingMode enum implements this trait, user can also provide their own rounding function

// FIXME: this should be a enum when enum const is supported in generic argument
/// Defines rounding modes of the floating numbers.
pub mod mode {
    /// Round toward 0 (default mode for binary float)
    pub struct Zero;

    /// Round toward +infinity
    pub struct Up;

    /// Round toward -infinity
    pub struct Down;

    /// Round to the nearest value, ties are rounded to an even value. (default mode for decimal float)
    pub struct HalfEven;

    /// Round to the nearest value, ties away from zero
    pub struct HalfAway;
}

/// The adjustment of a rounding operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rounding {
    /// No adjustment
    NoOp,

    /// Add one
    AddOne,

    /// Subtract one
    SubOne,
}

pub trait Round {
    /// Calculate the rounding of the number (mantissa + rem), assuming rem != 0 and |rem| < 1.
    /// `rem_half_test` should tell |rem|.cmp(0.5)
    fn round_rem<F: FnOnce() -> Ordering>(
        mantissa: &IBig,
        rem_sign: Sign,
        rem_half_test: F,
    ) -> Rounding;

    /// Calculate the rounding of the number (mantissa + fract / X^precision), assuming |fract| / X^precision < 1. Return the adjustment.
    #[inline]
    fn round_fract<const X: usize>(mantissa: &IBig, fract: IBig, precision: usize) -> Rounding {
        // this assertion is costly, so only check in debug mode
        debug_assert!(fract.clone().unsigned_abs() < UBig::from(X).pow(precision));

        if fract.is_zero() {
            return Rounding::NoOp;
        }
        let (fsign, fmag) = fract.into_parts();
        // TODO: here we can use logarithm to compare, instead of calculating the power?
        Self::round_rem::<_>(mantissa, fsign, || {
            (fmag << 1).cmp(&UBig::from(X).pow(precision))
        })
    }

    /// Calculate the rounding of the number (mantissa + numerator / denominator), assuming |numerator / denominator| < 1. Return the adjustment.
    #[inline]
    fn round_ratio(mantissa: &IBig, num: IBig, den: &IBig) -> Rounding {
        assert!(!den.is_zero());
        // this assertion can be costly, so only check in debug mode
        debug_assert!(num.clone().unsigned_abs() < den.clone().unsigned_abs());

        if num.is_zero() {
            return Rounding::NoOp;
        }
        let (nsign, nmag) = num.into_parts();
        Self::round_rem::<_>(mantissa, nsign * den.sign(), || {
            if den.sign() == Sign::Positive {
                IBig::from((nmag) << 1).cmp(&den)
            } else {
                den.cmp(&IBig::from_parts(Sign::Negative, nmag << 1))
            }
        })
    }
}

impl Round for mode::Zero {
    #[inline]
    fn round_rem<F: FnOnce() -> Ordering>(
        mantissa: &IBig,
        rem_sign: Sign,
        _rem_half_test: F,
    ) -> Rounding {
        if mantissa.is_zero() {
            return Rounding::NoOp;
        }
        match (mantissa.sign(), rem_sign) {
            (Sign::Positive, Sign::Positive) | (Sign::Negative, Sign::Negative) => Rounding::NoOp,
            (Sign::Positive, Sign::Negative) => Rounding::SubOne,
            (Sign::Negative, Sign::Positive) => Rounding::AddOne,
        }
    }
}

impl Round for mode::Down {
    #[inline]
    fn round_rem<F: FnOnce() -> Ordering>(
        _mantissa: &IBig,
        rem_sign: Sign,
        _rem_half_test: F,
    ) -> Rounding {
        // -1 if fract < 0, otherwise 0
        if rem_sign == Sign::Negative {
            Rounding::SubOne
        } else {
            Rounding::NoOp
        }
    }
}

impl Round for mode::Up {
    #[inline]
    fn round_rem<F: FnOnce() -> Ordering>(
        _mantissa: &IBig,
        rem_sign: Sign,
        _rem_half_test: F,
    ) -> Rounding {
        // +1 if fract > 0, otherwise 0
        if rem_sign == Sign::Positive {
            Rounding::AddOne
        } else {
            Rounding::NoOp
        }
    }
}

impl Round for mode::HalfAway {
    #[inline]
    fn round_rem<F: FnOnce() -> Ordering>(
        mantissa: &IBig,
        rem_sign: Sign,
        rem_half_test: F,
    ) -> Rounding {
        match rem_half_test() {
            // |rem| < 1/2
            Ordering::Less => Rounding::NoOp,
            // |rem| = 1/2
            Ordering::Equal => {
                // +1 if mantissa and rem >= 0, -1 if mantissa and rem <= 0
                if mantissa >= &IBig::ZERO && rem_sign == Sign::Positive {
                    Rounding::AddOne
                } else if mantissa <= &IBig::ZERO && rem_sign == Sign::Negative {
                    Rounding::SubOne
                } else {
                    Rounding::NoOp
                }
            }
            // |rem| > 1/2
            Ordering::Greater => {
                // +1 if rem > 0, -1 if rem < 0
                match rem_sign {
                    Sign::Positive => Rounding::AddOne,
                    Sign::Negative => Rounding::SubOne,
                }
            }
        }
    }
}

impl Round for mode::HalfEven {
    #[inline]
    fn round_rem<F: FnOnce() -> Ordering>(
        mantissa: &IBig,
        rem_sign: Sign,
        rem_half_test: F,
    ) -> Rounding {
        match rem_half_test() {
            // |rem| < 1/2
            Ordering::Less => Rounding::NoOp,
            // |rem| = 1/2
            Ordering::Equal => {
                // if mantissa is odd, +1 if rem > 0, -1 if rem < 0
                if mantissa & 1 == 1 {
                    match rem_sign {
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
                match rem_sign {
                    Sign::Positive => Rounding::AddOne,
                    Sign::Negative => Rounding::SubOne,
                }
            }
        }
    }
}

impl Add<Rounding> for IBig {
    type Output = IBig;

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

    fn add(self, rhs: Rounding) -> Self::Output {
        match rhs {
            Rounding::NoOp => self.clone(),
            Rounding::AddOne => self + IBig::ONE,
            Rounding::SubOne => self - IBig::ONE,
        }
    }
}

impl AddAssign<Rounding> for IBig {
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
        fn test_all_rounding<const X: usize, const D: usize>(
            input: &(i32, i32, Rounding, Rounding, Rounding, Rounding, Rounding),
        ) {
            let (value, fract, rnd_zero, rnd_up, rnd_down, rnd_halfeven, rnd_halfaway) = *input;
            let (value, fract) = (IBig::from(value), IBig::from(fract));
            assert_eq!(Zero::round_fract::<X>(&value, fract.clone(), D), rnd_zero);
            assert_eq!(Up::round_fract::<X>(&value, fract.clone(), D), rnd_up);
            assert_eq!(Down::round_fract::<X>(&value, fract.clone(), D), rnd_down);
            assert_eq!(HalfEven::round_fract::<X>(&value, fract.clone(), D), rnd_halfeven);
            assert_eq!(HalfAway::round_fract::<X>(&value, fract.clone(), D), rnd_halfaway);
        }

        // cases for radix = 2, 2 digit fraction
        #[rustfmt::skip]
        let binary_cases = [
            // (mantissa value, fraction part, roundings...)
            // Mode: Zero,   Up,     Down,   HEven,  HAway
            (0,  3,  NoOp,   AddOne, NoOp,   AddOne, AddOne),
            (0,  2,  NoOp,   AddOne, NoOp,   NoOp,   AddOne),
            (0,  1,  NoOp,   AddOne, NoOp,   NoOp,   NoOp),
            (0,  0,  NoOp,   NoOp,   NoOp,   NoOp,   NoOp),
            (0,  -1, NoOp,   NoOp,   SubOne, NoOp,   NoOp),
            (0,  -2, NoOp,   NoOp,   SubOne, NoOp,   SubOne),
            (0,  -3, NoOp,   NoOp,   SubOne, SubOne, SubOne),
            (1,  3,  NoOp,   AddOne, NoOp,   AddOne, AddOne),
            (1,  2,  NoOp,   AddOne, NoOp,   AddOne, AddOne),
            (1,  1,  NoOp,   AddOne, NoOp,   NoOp,   NoOp),
            (1,  0,  NoOp,   NoOp,   NoOp,   NoOp,   NoOp),
            (1,  -1, SubOne, NoOp,   SubOne, NoOp,   NoOp),
            (1,  -2, SubOne, NoOp,   SubOne, SubOne, NoOp),
            (1,  -3, SubOne, NoOp,   SubOne, SubOne, SubOne),
            (-1, 3,  AddOne, AddOne, NoOp,   AddOne, AddOne),
            (-1, 2,  AddOne, AddOne, NoOp,   AddOne, NoOp),
            (-1, 1,  AddOne, AddOne, NoOp,   NoOp,   NoOp),
            (-1, 0,  NoOp,   NoOp,   NoOp,   NoOp,   NoOp),
            (-1, -1, NoOp,   NoOp,   SubOne, NoOp,   NoOp),
            (-1, -2, NoOp,   NoOp,   SubOne, SubOne, SubOne),
            (-1, -3, NoOp,   NoOp,   SubOne, SubOne, SubOne),
        ];
        binary_cases.iter().for_each(test_all_rounding::<2, 2>);

        // cases for radix = 3, 1 digit fraction
        #[rustfmt::skip]
        let tenary_cases = [
            // (mantissa value, fraction part, roundings...)
            // Mode: Zero,   Up,     Down,   HEven,  HAway
            (0,  2,  NoOp,   AddOne, NoOp,   AddOne, AddOne),
            (0,  1,  NoOp,   AddOne, NoOp,   NoOp,   NoOp),
            (0,  0,  NoOp,   NoOp,   NoOp,   NoOp,   NoOp),
            (0,  -1, NoOp,   NoOp,   SubOne, NoOp,   NoOp),
            (0,  -2, NoOp,   NoOp,   SubOne, SubOne, SubOne),
            (1,  2,  NoOp,   AddOne, NoOp,   AddOne, AddOne),
            (1,  1,  NoOp,   AddOne, NoOp,   NoOp,   NoOp),
            (1,  0,  NoOp,   NoOp,   NoOp,   NoOp,   NoOp),
            (1,  -1, SubOne, NoOp,   SubOne, NoOp,   NoOp),
            (1,  -2, SubOne, NoOp,   SubOne, SubOne, SubOne),
            (-1, 2,  AddOne, AddOne, NoOp,   AddOne, AddOne),
            (-1, 1,  AddOne, AddOne, NoOp,   NoOp,   NoOp),
            (-1, 0,  NoOp,   NoOp,   NoOp,   NoOp,   NoOp),
            (-1, -1, NoOp,   NoOp,   SubOne, NoOp,   NoOp),
            (-1, -2, NoOp,   NoOp,   SubOne, SubOne, SubOne),
        ];
        tenary_cases.iter().for_each(test_all_rounding::<3, 1>);

        // cases for radix = 10, 1 digit fraction
        #[rustfmt::skip]
        let decimal_cases = [
            // (mantissa value, fraction part, roundings...)
            // Mode: Zero  , Up    , Down  , HEven , HAway
            ( 0,  7, NoOp  , AddOne, NoOp  , AddOne, AddOne),
            ( 0,  5, NoOp  , AddOne, NoOp  , NoOp  , AddOne),
            ( 0,  2, NoOp  , AddOne, NoOp  , NoOp  , NoOp  ),
            ( 0,  0, NoOp  , NoOp  , NoOp  , NoOp  , NoOp  ),
            ( 0, -2, NoOp  , NoOp  , SubOne, NoOp  , NoOp  ),
            ( 0, -5, NoOp  , NoOp  , SubOne, NoOp  , SubOne),
            ( 0, -7, NoOp  , NoOp  , SubOne, SubOne, SubOne),
            ( 1,  7, NoOp  , AddOne, NoOp  , AddOne, AddOne),
            ( 1,  5, NoOp  , AddOne, NoOp  , AddOne, AddOne),
            ( 1,  2, NoOp  , AddOne, NoOp  , NoOp  , NoOp  ),
            ( 1,  0, NoOp  , NoOp  , NoOp  , NoOp  , NoOp  ),
            ( 1, -2, SubOne, NoOp  , SubOne, NoOp  , NoOp  ),
            ( 1, -5, SubOne, NoOp  , SubOne, SubOne, NoOp  ),
            ( 1, -7, SubOne, NoOp  , SubOne, SubOne, SubOne),
            (-1,  7, AddOne, AddOne, NoOp  , AddOne, AddOne),
            (-1,  5, AddOne, AddOne, NoOp  , AddOne, NoOp  ),
            (-1,  2, AddOne, AddOne, NoOp  , NoOp  , NoOp  ),
            (-1,  0, NoOp  , NoOp  , NoOp  , NoOp  , NoOp  ),
            (-1, -2, NoOp  , NoOp  , SubOne, NoOp  , NoOp  ),
            (-1, -5, NoOp  , NoOp  , SubOne, SubOne, SubOne),
            (-1, -7, NoOp  , NoOp  , SubOne, SubOne, SubOne),
        ];
        decimal_cases.iter().for_each(test_all_rounding::<10, 1>);
    }

    #[test]
    fn test_from_ratio() {
        #[rustfmt::skip]
        fn test_all_rounding(
            input: &(i32, i32, i32, Rounding, Rounding, Rounding, Rounding, Rounding),
        ) {
            let (value, num, den, rnd_zero, rnd_up, rnd_down, rnd_halfeven, rnd_halfaway) = *input;
            let (value, num, den) = (IBig::from(value), IBig::from(num), IBig::from(den));
            assert_eq!(Zero::round_ratio(&value, num.clone(), &den), rnd_zero);
            assert_eq!(Up::round_ratio(&value, num.clone(), &den), rnd_up);
            assert_eq!(Down::round_ratio(&value, num.clone(), &den), rnd_down);
            assert_eq!(HalfEven::round_ratio(&value, num.clone(), &den), rnd_halfeven);
            assert_eq!(HalfAway::round_ratio(&value, num.clone(), &den), rnd_halfaway);
        }

        // cases for radix = 2, 2 digit fraction
        #[rustfmt::skip]
        let test_cases = [
            // (mantissa value, mumerator, denominator, roundings...)
            // Mode:     Zero  , Up    , Down  , HEven , HAway
            ( 0,  0,  2, NoOp  , NoOp  , NoOp  , NoOp  , NoOp  ),
            ( 0,  1,  2, NoOp  , AddOne, NoOp  , NoOp  , AddOne),
            ( 0, -1,  2, NoOp  , NoOp  , SubOne, NoOp  , SubOne),
            ( 0,  0, -2, NoOp  , NoOp  , NoOp  , NoOp  , NoOp  ),
            ( 0,  1, -2, NoOp  , NoOp  , SubOne, NoOp  , SubOne),
            ( 0, -1, -2, NoOp  , AddOne, NoOp  , NoOp  , AddOne),
            ( 1,  0,  2, NoOp  , NoOp  , NoOp  , NoOp  , NoOp  ),
            ( 1,  1,  2, NoOp  , AddOne, NoOp  , AddOne, AddOne),
            ( 1, -1,  2, SubOne, NoOp  , SubOne, SubOne, NoOp  ),
            ( 1,  0, -2, NoOp  , NoOp  , NoOp  , NoOp  , NoOp  ),
            ( 1,  1, -2, SubOne, NoOp  , SubOne, SubOne, NoOp  ),
            ( 1, -1, -2, NoOp  , AddOne, NoOp  , AddOne, AddOne),
            (-1,  0,  2, NoOp  , NoOp  , NoOp  , NoOp  , NoOp  ),
            (-1,  1,  2, AddOne, AddOne, NoOp  , AddOne, NoOp  ),
            (-1, -1,  2, NoOp  , NoOp  , SubOne, SubOne, SubOne),
            (-1,  0, -2, NoOp  , NoOp  , NoOp  , NoOp  , NoOp  ),
            (-1,  1, -2, NoOp  , NoOp  , SubOne, SubOne, SubOne),
            (-1, -1, -2, AddOne, AddOne, NoOp  , AddOne, NoOp  ),

            ( 0, -2,  3, NoOp  , NoOp  , SubOne, SubOne, SubOne),
            ( 0, -1,  3, NoOp  , NoOp  , SubOne, NoOp  , NoOp  ),
            ( 0,  0,  3, NoOp  , NoOp  , NoOp  , NoOp  , NoOp  ),
            ( 0,  1,  3, NoOp  , AddOne, NoOp  , NoOp  , NoOp  ),
            ( 0,  2,  3, NoOp  , AddOne, NoOp  , AddOne, AddOne),
            ( 0, -2, -3, NoOp  , AddOne, NoOp  , AddOne, AddOne),
            ( 0, -1, -3, NoOp  , AddOne, NoOp  , NoOp  , NoOp  ),
            ( 0,  0, -3, NoOp  , NoOp  , NoOp  , NoOp  , NoOp  ),
            ( 0,  1, -3, NoOp  , NoOp  , SubOne, NoOp  , NoOp  ),
            ( 0,  2, -3, NoOp  , NoOp  , SubOne, SubOne, SubOne),
            ( 1, -2,  3, SubOne, NoOp  , SubOne, SubOne, SubOne),
            ( 1, -1,  3, SubOne, NoOp  , SubOne, NoOp  , NoOp  ),
            ( 1,  0,  3, NoOp  , NoOp  , NoOp  , NoOp  , NoOp  ),
            ( 1,  1,  3, NoOp  , AddOne, NoOp  , NoOp  , NoOp  ),
            ( 1,  2,  3, NoOp  , AddOne, NoOp  , AddOne, AddOne),
            ( 1, -2, -3, NoOp  , AddOne, NoOp  , AddOne, AddOne),
            ( 1, -1, -3, NoOp  , AddOne, NoOp  , NoOp  , NoOp  ),
            ( 1,  0, -3, NoOp  , NoOp  , NoOp  , NoOp  , NoOp  ),
            ( 1,  1, -3, SubOne, NoOp  , SubOne, NoOp  , NoOp  ),
            ( 1,  2, -3, SubOne, NoOp  , SubOne, SubOne, SubOne),
            (-1, -2,  3, NoOp  , NoOp  , SubOne, SubOne, SubOne),
            (-1, -1,  3, NoOp  , NoOp  , SubOne, NoOp  , NoOp  ),
            (-1,  0,  3, NoOp  , NoOp  , NoOp  , NoOp  , NoOp  ),
            (-1,  1,  3, AddOne, AddOne, NoOp  , NoOp  , NoOp  ),
            (-1,  2,  3, AddOne, AddOne, NoOp  , AddOne, AddOne),
            (-1, -2, -3, AddOne, AddOne, NoOp  , AddOne, AddOne),
            (-1, -1, -3, AddOne, AddOne, NoOp  , NoOp  , NoOp  ),
            (-1,  0, -3, NoOp  , NoOp  , NoOp  , NoOp  , NoOp  ),
            (-1,  1, -3, NoOp  , NoOp  , SubOne, NoOp  , NoOp  ),
            (-1,  2, -3, NoOp  , NoOp  , SubOne, SubOne, SubOne),
        ];
        test_cases.iter().for_each(test_all_rounding);
    }
}
