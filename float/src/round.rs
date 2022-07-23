use core::cmp::Ordering;
use core::ops::Add;
use dashu_base::UnsignedAbs;
use dashu_int::{IBig, Sign, UBig};
use std::ops::AddAssign;

// FIXME: this should be a enum when enum const is supported in generic argument
/// Defines rounding modes of the floating numbers.
#[allow(non_snake_case)]
#[allow(non_upper_case_globals)]
pub mod RoundingMode {
    /// Round toward 0 (default mode for binary float)
    pub const Zero: u8 = 0;

    /// Round toward +infinity
    pub const Up: u8 = 1;

    /// Round toward -infinity
    pub const Down: u8 = 2;

    /// Round to the nearest value, ties are rounded to an even value. (default mode for decimal float)
    pub const HalfEven: u8 = 3;

    /// Round to the nearest value, ties away from zero
    pub const HalfAway: u8 = 4;
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

impl Rounding {
    #[inline(always)]
    pub fn from_int(round: i8) -> Self {
        match round.signum() {
            1 => Self::AddOne,
            -1 => Self::SubOne,
            0 => Self::NoOp,
            _ => unreachable!(),
        }
    }

    /// Calculate the rounding of the number (mantissa + rem), assuming rem != 0 and |rem| < 1.
    /// `rem_half_test` should tell |rem|.cmp(0.5)
    #[inline]
    fn from_rem<const R: u8, F: FnOnce() -> Ordering>(
        mantissa: &IBig,
        rem_sign: Sign,
        rem_half_test: F,
    ) -> Self {
        let adjust = match R {
            RoundingMode::Zero => match (mantissa.sign(), rem_sign) {
                (Sign::Positive, Sign::Positive) | (Sign::Negative, Sign::Negative) => 0,
                (Sign::Positive, Sign::Negative) => -(!mantissa.is_zero() as i8), // -1 if mantissa != 0
                (Sign::Negative, Sign::Positive) => 1, // +1 if mantissa < 0 and fract > 0
            },
            RoundingMode::Down => -((rem_sign == Sign::Negative) as i8), // -1 if fract < 0, otherwise 0
            RoundingMode::Up => (rem_sign == Sign::Positive) as i8, // +1 if fract > 0, otherwise 0
            RoundingMode::HalfEven | RoundingMode::HalfAway => {
                match rem_half_test() {
                    // |fract| < 1/2
                    Ordering::Less => 0,
                    // |fract| = 1/2
                    Ordering::Equal => match R {
                        // ties to even
                        RoundingMode::HalfEven => {
                            // if mantissa if odd, +1 if frac > 0, -1 if frac < 0
                            ((rem_sign == Sign::Positive) as i8 - (rem_sign == Sign::Negative) as i8)
                                * (&*mantissa & 1 == 1) as i8
                        }
                        RoundingMode::HalfAway => {
                            // +1 if mantissa and fract >= 0, -1 if mantissa and fract <= 0 (given fract != 0)
                            (mantissa >= &0 && rem_sign == Sign::Positive) as i8
                                - (mantissa <= &0 && rem_sign == Sign::Negative) as i8
                        }
                        _ => unreachable!(),
                    },
                    // |fract| > 1/2
                    Ordering::Greater => {
                        // +1 if fract > 0, -1 if fract < 0
                        (rem_sign == Sign::Positive) as i8 - ((rem_sign == Sign::Negative) as i8)
                    }
                }
            }
            _ => unreachable!(),
        };
        Self::from_int(adjust)
    }

    /// Calculate the rounding of the number (mantissa + fract / X^precision), assuming |fract| / X^precision < 1. Return the adjustment.
    pub fn from_fract<const X: usize, const R: u8>(
        mantissa: &IBig,
        fract: IBig,
        precision: usize,
    ) -> Self {
        debug_assert!(fract.clone().unsigned_abs() < UBig::from(X).pow(precision));

        if fract.is_zero() {
            return Self::NoOp;
        }
        let (fsign, fmag) = fract.to_sign_magnitude();
        // TODO: here we can use logarithm to compare, instead of calculating the power?
        Self::from_rem::<R, _>(mantissa, fsign, || (fmag << 1).cmp(&UBig::from(X).pow(precision)))
    }

    /// Calculate the rounding of the number (mantissa + numerator / denominator), assuming |numerator / denominator| < 1. Return the adjustment.
    pub fn from_ratio<const R: u8>(mantissa: &IBig, num: IBig, den: &IBig) -> Self {
        debug_assert!(num.clone().unsigned_abs() < den.clone().unsigned_abs());

        if num.is_zero() {
            return Self::NoOp;
        }
        let (nsign, nmag) = num.to_sign_magnitude();
        Self::from_rem::<R, _>(mantissa, nsign * den.sign(), || {
            if den.sign() == Sign::Positive {
                IBig::from((nmag) << 1).cmp(&den)
            } else {
                den.cmp(&IBig::from_sign_magnitude(Sign::Negative, nmag << 1))
            }
        })
    }
}

impl Add<Rounding> for IBig {
    type Output = IBig;

    fn add(self, rhs: Rounding) -> Self::Output {
        match rhs {
            Rounding::NoOp => self,
            Rounding::AddOne => self + IBig::one(),
            Rounding::SubOne => self - IBig::one(),
        }
    }
}

impl Add<Rounding> for &IBig {
    type Output = IBig;

    fn add(self, rhs: Rounding) -> Self::Output {
        match rhs {
            Rounding::NoOp => self.clone(),
            Rounding::AddOne => self + IBig::one(),
            Rounding::SubOne => self - IBig::one(),
        }
    }
}

impl AddAssign<Rounding> for IBig {
    fn add_assign(&mut self, rhs: Rounding) {
        match rhs {
            Rounding::NoOp => {}
            Rounding::AddOne => *self += IBig::one(),
            Rounding::SubOne => *self -= IBig::one(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::{Rounding::*, RoundingMode::*};

    #[test]
    fn test_from_fract() {
        #[rustfmt::skip]
        fn test_all_rounding<const X: usize, const D: usize>(
            input: &(i32, i32, Rounding, Rounding, Rounding, Rounding, Rounding),
        ) {
            let (value, fract, rnd_zero, rnd_up, rnd_down, rnd_halfeven, rnd_halfaway) = *input;
            let (value, fract) = (IBig::from(value), IBig::from(fract));
            assert_eq!(Rounding::from_fract::<X, Zero>(&value, fract.clone(), D), rnd_zero);
            assert_eq!(Rounding::from_fract::<X, Up>(&value, fract.clone(), D), rnd_up);
            assert_eq!(Rounding::from_fract::<X, Down>(&value, fract.clone(), D), rnd_down);
            assert_eq!(Rounding::from_fract::<X, HalfEven>(&value, fract.clone(), D), rnd_halfeven);
            assert_eq!(Rounding::from_fract::<X, HalfAway>(&value, fract.clone(), D), rnd_halfaway);
        }

        // cases for Radix = 2, 2 digit fraction
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

        // cases for Radix = 3, 1 digit fraction
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

        // cases for Radix = 10, 1 digit fraction
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
            assert_eq!(Rounding::from_ratio::<Zero>(&value, num.clone(), &den), rnd_zero);
            assert_eq!(Rounding::from_ratio::<Up>(&value, num.clone(), &den), rnd_up);
            assert_eq!(Rounding::from_ratio::<Down>(&value, num.clone(), &den), rnd_down);
            assert_eq!(Rounding::from_ratio::<HalfEven>(&value, num.clone(), &den), rnd_halfeven);
            assert_eq!(Rounding::from_ratio::<HalfAway>(&value, num.clone(), &den), rnd_halfaway);
        }

        // cases for Radix = 2, 2 digit fraction
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
