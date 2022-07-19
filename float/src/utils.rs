use core::cmp::Ordering;
use core::convert::TryInto;

use dashu_base::{DivRem, UnsignedAbs};
use dashu_int::{IBig, UBig, ibig, ubig, Sign};
use crate::{repr::RoundingMode, ibig_ext::{log, magnitude}};

/// Get the integer k such that `radix^(k-1) <= value < radix^k`.
/// If value is 0, then `k = 0` is returned.
pub fn get_precision<const X: usize>(value: &IBig) -> usize{
    if value == &ibig!(0) {
        return 0
    };

    let e = log(&magnitude(value), X);
    let e: usize = e.try_into().unwrap();
    e + 1
}

/// "Left shifting" in given radix, i.e. multiply by a power of radix
#[inline]
pub fn shl_radix<const X: usize>(value: &mut IBig, exp: usize) {
    if exp != 0 {
        match X {
            2 => *value <<= exp,
            10 => {
                *value *= IBig::from(5).pow(exp);
                *value <<= exp;
            }
            16 => *value <<= 4 * exp,
            _ => *value *= IBig::from(X).pow(exp)
        }
    }
}

/// "Right shifting" in given radix, i.e. divide by a power of radix
#[inline]
pub fn shr_radix<const X: usize>(value: &mut IBig, exp: usize) {
    if exp != 0 {
        match X {
            2 => *value >>= exp,
            10 => {
                *value >>= exp;
                *value /= ibig!(5).pow(exp);
            }
            16 => *value >>= 4 * exp,
            _ => *value /= IBig::from(X).pow(exp)
        }
    }
}

/// "Right shifting" in given radix, i.e. divide by a power of radix.
/// It returns the "shifted" value and the "remainder" part of integer that got removed
#[inline]
pub fn shr_rem_radix<const X: usize>(value: &IBig, exp: usize) -> (IBig, IBig) {
    if exp != 0 {
        match X {
            2 => {
                // FIXME: a dedicate method to extract low bits for IBig might be helpful here
                let rem = value & ((ibig!(1) << exp) - 1);
                (value >> exp, rem)
            },
            10 => {
                let rem1 = value & ((ibig!(1) << exp) - 1);
                let (q, rem2) = (value >> exp).div_rem(ibig!(5).pow(exp));
                let rem = (rem2 << exp) + rem1;
                (q, rem)
            },
            16 => {
                let rem = value & ((ibig!(1) << (4 * exp)) - 1);
                (value >> 4 * exp, rem)
            },
            _ => value.div_rem(IBig::from(X).pow(exp))
        }
    } else {
        (value.clone(), ibig!(0))
    }
}

#[inline]
pub fn shr_rem_radix_in_place<const X: usize>(value: &mut IBig, exp: usize) -> IBig {
    if exp != 0 {
        match X {
            2 => {
                // FIXME: a dedicate method to extract low bits for IBig might be helpful here
                let rem = &*value & ((ibig!(1) << exp) - 1);
                *value >>= exp;
                rem
            },
            10 => {
                let rem1 = &*value & ((ibig!(1) << exp) - 1);
                let (q, rem2) = (&*value >> exp).div_rem(ibig!(5).pow(exp));
                *value = q;
                let rem = (rem2 << exp) + rem1;
                rem
            },
            16 => {
                let rem = &*value & ((ibig!(1) << (4 * exp)) - 1);
                *value >>= 4 * exp;
                rem
            },
            _ => {
                let (q, r) = (&*value).div_rem(IBig::from(X).pow(exp));
                *value = q;
                r
            }
        }
    } else {
        ibig!(0)
    }
}

// TODO: store the tenary value in an enum, and returns Approximation<FloatRepr, tenary> for various operations
// make the round_with_fract and round_with_ratio associated with that enum

/// Round the number (mantissa + fract / X^precision), assuming |fract| / X^precision < 1. Return the adjustment.
#[inline(always)]
pub fn round_with_fract<const X: usize, const R: u8>(mantissa: &IBig, fract: IBig, precision: usize) -> i8 {
    debug_assert!(fract.clone().unsigned_abs() < UBig::from(X).pow(precision));

    if fract.is_zero() {
        return 0;
    }
    let (fsign, fmag) = fract.to_sign_magnitude();

    match R {
        RoundingMode::Zero => match (mantissa.sign(), fsign) {
            (Sign::Positive, Sign::Positive) | (Sign::Negative, Sign::Negative) => 0,
            (Sign::Positive, Sign::Negative) => -(!mantissa.is_zero() as i8), // -1 if mantissa != 0
            (Sign::Negative, Sign::Positive) => 1 // +1 if mantissa < 0 and fract > 0
        },
        RoundingMode::Down => -((fsign == Sign::Negative) as i8), // -1 if fract < 0, otherwise 0
        RoundingMode::Up => (fsign == Sign::Positive) as i8, // +1 if fract > 0, otherwise 0
        RoundingMode::HalfEven | RoundingMode::HalfAway => {
            // TODO: here we can use logarithm to compare, instead of calculating the power?
            let double = fmag << 1;
            match UBig::from(X).pow(precision).cmp(&double) {
                // |fract| < 1/2
                Ordering::Greater => (fsign == Sign::Positive) as i8, // +1 if fract > 0
                // |fract| = 1/2
                Ordering::Equal => match R {
                    // ties to even
                    RoundingMode::HalfEven => (&*mantissa & 1 == 1) as i8, // +1 if mantissa % 2 == 0
                    RoundingMode::HalfAway => (fsign == Sign::Positive) as i8 - (fsign == Sign::Negative) as i8, // +1 if fract > 0, -1 if fract < 0
                    _ => unreachable!()
                },
                // |fract| > 1/2
                Ordering::Less => -((fsign == Sign::Negative) as i8), // -1 if fract < 0
            }
        },
        _ => unreachable!()
    }
}

/// Round the number (mantissa + numerator / denominator), assuming |numerator / denominator| < 1. Return the adjustment.
pub fn round_with_ratio<const R: u8>(mantissa: &IBig, num: IBig, den: &IBig) -> i8 {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    fn test_rounding() {
        // TODO: implement test for all cases
    }
}
