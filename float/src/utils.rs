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
#[deprecated]
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

/// Calculate the high parts of a * b.
/// 
/// It's equivalent to find `a * b / E^c` such that it's in the range `[E^(prec-1), E^prec)`
#[inline]
pub fn mul_hi<const X: usize>(a: &IBig, b: &IBig, prec: usize) -> IBig {
    let mut c = a * b;
    let prec_actual = get_precision::<X>(&c);
    if prec_actual > prec {
        shr_radix::<X>(&mut c, prec_actual - prec);
    }
    c
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

/// Return the rounding bit based on the remainder (mod Radix)
#[deprecated]
#[inline(always)]
pub fn round_with_rem<const X: usize, const R: u8>(mantissa: &mut IBig, rem: isize) {
    assert!((rem.abs() as usize) < X);

    match (R, rem.signum()) {
        (_, 0) => {},
        (RoundingMode::Zero, _) => {},
        (RoundingMode::Down, 1) => {},
        (RoundingMode::Down, -1) => *mantissa -= 1,
        (RoundingMode::Up, 1) => *mantissa += 1,
        (RoundingMode::Up, -1) => {},
        (RoundingMode::HalfEven | RoundingMode::HalfAway, _) => {
            let double = if rem < 0 {
                (rem + X as isize) * 2
            } else {
                rem * 2
            } as usize;
            match X.cmp(&double) {
                Ordering::Greater => if rem > 0 {
                    *mantissa += 1
                },
                Ordering::Equal => match R {
                    RoundingMode::HalfEven => {
                        // ties to even
                        if &*mantissa % 2 != 0 {
                            *mantissa += 1;
                        }
                    },
                    RoundingMode::HalfAway => {
                        // ties away from zero
                        if rem > 0 {
                            *mantissa += 1;
                        } else {
                            *mantissa -= 1;
                        }
                    },
                    _ => unreachable!()
                },
                Ordering::Less => if rem < 0 {
                    *mantissa -= 1
                }
            };
        },
        _ => unreachable!()
    }
}

/// Round the number (mantissa + rem * X^-precision), return the adjustment.
#[inline(always)]
pub fn round_with_rem_new<const X: usize, const R: u8>(mantissa: &IBig, rem: &IBig, precision: usize) -> i8 {
    debug_assert!(rem.unsigned_abs() < UBig::from(X).pow(precision));

    match R {
        RoundingMode::Zero => match (mantissa.sign(), rem.sign()) {
            (Sign::Positive, Sign::Positive) | (Sign::Negative, Sign::Negative) => 0,
            (Sign::Positive, Sign::Negative) => -(!mantissa.is_zero() as i8), // -1 if mantissa != 0
            (Sign::Negative, Sign::Positive) => rem.is_zero() as i8 // 1 if rem != 0
        },
        RoundingMode::Down => -((rem.sign() == Sign::Negative) as i8), // -1 if rem < 0, otherwise 0
        _ => unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    fn test_rounding() {
        // TODO: implement test for all cases
    }
}
