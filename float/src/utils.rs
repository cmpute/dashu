use dashu_base::{DivRem, DivRemAssign, UnsignedAbs};
use dashu_int::{DoubleWord, IBig, Sign, UBig, Word};

#[inline]
pub const fn base_as_ibig<const B: Word>() -> IBig {
    IBig::from_parts_const(Sign::Positive, B as DoubleWord)
}

/// Calculate the number of digits in base `B`.
///
/// Returns the integer `k` such that `radix^(k-1) <= value < radix^k`.
/// If value is `0`, then `k = 0` is returned.
#[inline]
pub fn digit_len<const B: Word>(value: &IBig) -> usize {
    if value.is_zero() {
        return 0;
    };
    value.ilog(&UBig::from_word(B)) + 1
}

/// "Left shifting" in given radix, i.e. multiply by a power of radix
#[inline]
pub fn shl_radix<const B: Word>(value: &IBig, exp: usize) -> IBig {
    if exp == 0 {
        return value.clone();
    }

    match B {
        2 => value << exp,
        10 => value * IBig::from(5).pow(exp) << exp,
        16 => value << 4 * exp,
        _ => value * base_as_ibig::<B>().pow(exp),
    }
}

#[inline]
pub fn shl_radix_in_place<const B: Word>(value: &mut IBig, exp: usize) {
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
pub fn shr_radix<const B: Word>(value: &IBig, exp: usize) -> IBig {
    if exp == 0 {
        return value.clone();
    }

    match B {
        2 => value >> exp,
        10 => (value >> exp) / IBig::from(5).pow(exp),
        16 => value >> 4 * exp,
        _ => value / base_as_ibig::<B>().pow(exp),
    }
}

#[inline]
pub fn shr_radix_in_place<const B: Word>(value: &mut IBig, exp: usize) {
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

// TODO(next): deprecate shr_rem_radix and shr_rem_in_place_radix, use split_radix_at. For inplace operation, use mem::take

/// "Right shifting" in given radix, i.e. divide by a power of radix.
/// It returns the "shifted" value and the "remainder" part of integer that got removed
#[inline]
pub fn shr_rem_radix<const B: Word>(value: &IBig, exp: usize) -> (IBig, IBig) {
    if exp != 0 {
        match B {
            // TODO(next): a dedicate method for splitting bits is necessary for IBig to have a correct sign!
            // 2 => {
            //     let rem = value & ((IBig::ONE << exp) - 1);
            //     (value >> exp, rem * value.sign())
            // }
            // 10 => {
            //     let rem1 = value & ((IBig::ONE << exp) - 1);
            //     let (q, rem2) = (value >> exp).div_rem(IBig::from(5).pow(exp));
            //     let rem = (rem2 << exp) + rem1 * value.sign();
            //     (q, rem)
            // }
            // 16 => {
            //     let rem = value & ((IBig::ONE << (4 * exp)) - 1);
            //     (value >> 4 * exp, rem)
            // }
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
            // 2 => {
            //
            //     let rem = &*value & ((IBig::ONE << exp) - 1);
            //     *value >>= exp;
            //     rem * value.sign()
            // }
            // 10 => {
            //     let rem1 = &*value & ((IBig::ONE << exp) - 1);
            //     let (q, rem2) = (&*value >> exp).div_rem(IBig::from(5).pow(exp));
            //     *value = q;
            //     let rem = value.sign() * (rem2 << exp) + rem1;
            //     rem
            // }
            // 16 => {
            //     let rem = &*value & ((IBig::ONE << (4 * exp)) - 1);
            //     *value >>= 4 * exp;
            //     rem
            // }
            _ => value.div_rem_assign(base_as_ibig::<B>().pow(exp)),
        }
    } else {
        IBig::ZERO
    }
}

/// Split the integer at given digit position. Return the low part and high part,
/// and the sign is applied to both parts.
///
/// For example in base 10:
/// * split_digits(123, 1) returns (3, 12)
/// * split_digits(-123, 2) returns (-23, -1)
#[inline]
pub fn split_digits<const B: Word>(mut value: IBig, pos: usize) -> (IBig, IBig) {
    let r = shr_rem_radix_in_place::<B>(&mut value, pos);
    (r, value)
}
