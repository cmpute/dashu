use core::convert::TryInto;
use dashu_base::{DivRem, UnsignedAbs};
use dashu_int::{IBig, Word, Sign, DoubleWord, UBig};

#[inline]
pub const fn base_as_ibig<const B: Word>() -> IBig {
    IBig::from_parts_const(Sign::Positive, B as DoubleWord)
}

// TODO: deprecate this function?
/// Get the integer k such that `radix^(k-1) <= value < radix^k`.
/// If value is 0, then `k = 0` is returned.
pub fn get_precision<const B: Word>(value: &IBig) -> usize {
    if value.is_zero() {
        return 0;
    };

    let e = value.ilog(&UBig::from_word(B));
    let e: usize = e.try_into().unwrap();
    e + 1
}

/// "Left shifting" in given radix, i.e. multiply by a power of radix
#[inline]
pub fn shl_radix<const B: Word>(value: &mut IBig, exp: usize) {
    if exp != 0 {
        match B {
            2 => *value <<= exp,
            10 => {
                *value *= IBig::from(5).pow(exp);
                *value <<= exp;
            }
            16 => *value <<= 4 * exp,
            _ => *value *= base_as_ibig::<B>().pow(exp),
        }
    }
}

/// "Right shifting" in given radix, i.e. divide by a power of radix
#[inline]
pub fn shr_radix<const B: Word>(value: &mut IBig, exp: usize) {
    if exp != 0 {
        match B {
            2 => *value >>= exp,
            10 => {
                *value >>= exp;
                *value /= IBig::from(5).pow(exp);
            }
            16 => *value >>= 4 * exp,
            _ => *value /= base_as_ibig::<B>().pow(exp),
        }
    }
}

/// "Right shifting" in given radix, i.e. divide by a power of radix.
/// It returns the "shifted" value and the "remainder" part of integer that got removed
#[inline]
pub fn shr_rem_radix<const B: Word>(value: &IBig, exp: usize) -> (IBig, IBig) {
    if exp != 0 {
        match B {
            2 => {
                // FIXME: a dedicate method to extract low bits for IBig might be helpful here
                let rem = value & ((IBig::ONE << exp) - 1);
                (value >> exp, rem)
            }
            10 => {
                let rem1 = value & ((IBig::ONE << exp) - 1);
                let (q, rem2) = (value >> exp).div_rem(IBig::from(5).pow(exp));
                let rem = (rem2 << exp) + rem1;
                (q, rem)
            }
            16 => {
                let rem = value & ((IBig::ONE << (4 * exp)) - 1);
                (value >> 4 * exp, rem)
            }
            _ => value.div_rem(base_as_ibig::<B>().pow(exp)),
        }
    } else {
        (value.clone(), IBig::ZERO)
    }
}

#[inline]
pub fn shr_rem_radix_in_place<const B: Word>(value: &mut IBig, exp: usize) -> IBig {
    if exp != 0 {
        match B {
            2 => {
                // FIXME: a dedicate method to extract low bits for IBig might be helpful here
                let rem = &*value & ((IBig::ONE << exp) - 1);
                *value >>= exp;
                rem
            }
            10 => {
                let rem1 = &*value & ((IBig::ONE << exp) - 1);
                let (q, rem2) = (&*value >> exp).div_rem(IBig::from(5).pow(exp));
                *value = q;
                let rem = (rem2 << exp) + rem1;
                rem
            }
            16 => {
                let rem = &*value & ((IBig::ONE << (4 * exp)) - 1);
                *value >>= 4 * exp;
                rem
            }
            _ => {
                let (q, r) = (&*value).div_rem(base_as_ibig::<B>().pow(exp));
                *value = q;
                r
            }
        }
    } else {
        IBig::ZERO
    }
}
