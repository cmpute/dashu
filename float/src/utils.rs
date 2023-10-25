use dashu_base::{DivRem, Sign};
use dashu_int::{DoubleWord, IBig, UBig, Word};

#[inline]
pub const fn base_as_ibig<const B: Word>() -> IBig {
    IBig::from_parts_const(Sign::Positive, B as DoubleWord)
}

/// Calculate the number of digits in base `B`.
///
/// Returns the integer `k` such that `B^(k-1) <= value < B^k`.
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
pub fn shl_digits<const B: Word>(value: &IBig, exp: usize) -> IBig {
    if exp == 0 {
        return value.clone();
    }

    match B {
        2 => value << exp,
        10 => (value * IBig::from(5).pow(exp)) << exp,
        b if b.is_power_of_two() => value << (exp * b.trailing_zeros() as usize),
        _ => value * base_as_ibig::<B>().pow(exp),
    }
}

#[inline]
pub fn shl_digits_in_place<const B: Word>(value: &mut IBig, exp: usize) {
    if exp != 0 {
        match B {
            2 => *value <<= exp,
            10 => {
                *value *= IBig::from(5).pow(exp);
                *value <<= exp;
            }
            b if b.is_power_of_two() => *value <<= exp * b.trailing_zeros() as usize,
            _ => *value *= base_as_ibig::<B>().pow(exp),
        }
    }
}

// The original Shr operation of IBig follows two's complement representation,
// here we only want to shift the magnitude, so a separate operation is necessary
#[inline]
fn shr_ref(value: &IBig, shift: usize) -> IBig {
    let (sign, words) = value.as_sign_words();
    let n_words = shift / Word::BITS as usize;

    let hi = UBig::from_words(&words[n_words.min(words.len())..]);
    IBig::from_parts(sign, hi >> (shift % Word::BITS as usize))
}

/// "Right shifting" in given radix, i.e. divide by a power of radix
#[inline]
pub fn shr_digits<const B: Word>(value: &IBig, exp: usize) -> IBig {
    if exp == 0 {
        return value.clone();
    }

    match B {
        2 => shr_ref(value, exp),
        10 => shr_ref(value, exp) / IBig::from(5).pow(exp),
        b if b.is_power_of_two() => shr_ref(value, exp * b.trailing_zeros() as usize),
        _ => value / base_as_ibig::<B>().pow(exp),
    }
}

/// Equivalent to value.unsigned_abs().split_bits(n), but returns (hi, lo) and preserving the sign
fn split_bits(value: IBig, n: usize) -> (IBig, IBig) {
    let (sign, mag) = value.into_parts();
    let (lo, hi) = mag.split_bits(n);
    (IBig::from_parts(sign, hi), IBig::from_parts(sign, lo))
}

/// Equivalent to value.unsigned_abs().split_bits(n), but returns (hi, lo) and preserving the sign
fn split_bits_ref(value: &IBig, n: usize) -> (IBig, IBig) {
    debug_assert!(n > 0);
    if value.is_zero() {
        return (IBig::ZERO, IBig::ZERO);
    }

    let (sign, words) = value.as_sign_words();
    let n_words = n / Word::BITS as usize;

    let mut hi = UBig::from_words(&words[n_words..]);
    hi >>= n % Word::BITS as usize;
    let mut lo = UBig::from_words(&words[..n_words + 1]);
    lo.clear_high_bits(n);

    (IBig::from_parts(sign, hi), IBig::from_parts(sign, lo))
}

/// Same as [split_digits] but take reference of input
#[inline]
pub fn split_digits_ref<const B: Word>(value: &IBig, pos: usize) -> (IBig, IBig) {
    if pos != 0 {
        match B {
            10 => {
                let (q, rem1) = split_bits_ref(value, pos);
                let (q, rem2) = q.div_rem(IBig::from(5).pow(pos));
                let rem = (rem2 << pos) + rem1;
                (q, rem)
            }
            i if i.is_power_of_two() => split_bits_ref(value, pos * i.trailing_zeros() as usize),
            _ => value.div_rem(base_as_ibig::<B>().pow(pos)),
        }
    } else {
        (value.clone(), IBig::ZERO)
    }
}

/// Split the integer at given digit position. Return the high part and low part,
/// and the sign is applied to both parts.
///
/// For example in base 10:
/// * split_digits(123, 1) returns (12, 3)
/// * split_digits(-123, 2) returns (-1, -23)
#[inline]
pub fn split_digits<const B: Word>(value: IBig, pos: usize) -> (IBig, IBig) {
    if pos != 0 {
        match B {
            10 => {
                let (q, rem1) = split_bits(value, pos);
                let (q, rem2) = q.div_rem(IBig::from(5).pow(pos));
                let rem = (rem2 << pos) + rem1;
                (q, rem)
            }
            i if i.is_power_of_two() => split_bits(value, pos * i.trailing_zeros() as usize),
            _ => value.div_rem(base_as_ibig::<B>().pow(pos)),
        }
    } else {
        (value, IBig::ZERO)
    }
}

/// If n is a power of base, then return the exponent,
/// otherwise return 0.
///
/// This is a const function replacement of `IBig::ilog`.
pub const fn ilog_exact(n: Word, base: Word) -> u32 {
    if n < base {
        return 0;
    }

    let mut pow = base;
    let mut exp = 1;
    while pow < n {
        pow *= base;
        exp += 1;
    }

    if pow == n {
        exp
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashu_base::UnsignedAbs;

    #[test]
    fn test_shr_ref() {
        let a = IBig::from(0x1234567890abcdefu64).pow(12); // 723 bits
        assert_eq!(shr_ref(&a, 10), (&a).unsigned_abs() >> 10);
        assert_eq!(shr_ref(&a, 100), (&a).unsigned_abs() >> 100);
        assert_eq!(shr_ref(&a, 1000), (&a).unsigned_abs() >> 1000);

        let a = IBig::from(-0x1234567890abcdefi64).pow(7); // 422 bits
        assert_eq!(-shr_ref(&a, 10), (&a).unsigned_abs() >> 10);
        assert_eq!(-shr_ref(&a, 100), (&a).unsigned_abs() >> 100);
        assert_eq!(-shr_ref(&a, 1000), (&a).unsigned_abs() >> 1000);
    }

    #[test]
    fn test_split_bits_ref() {
        let a = IBig::from(0x1234567890abcdefu64).pow(12);
        let (hi, lo) = split_bits_ref(&a, 100);
        let (rlo, rhi) = (&a).unsigned_abs().split_bits(100);
        assert_eq!(lo, rlo);
        assert_eq!(hi, rhi);

        let (hi, lo) = split_bits_ref(&a, 192);
        let (rlo, rhi) = (&a).unsigned_abs().split_bits(192);
        assert_eq!(lo, rlo);
        assert_eq!(hi, rhi);

        let a = IBig::from(-0x1234567890abcdefi64).pow(7);
        let (hi, lo) = split_bits_ref(&a, 100);
        let (rlo, rhi) = (&a).unsigned_abs().split_bits(100);
        assert_eq!(-lo, rlo);
        assert_eq!(-hi, rhi);

        let (hi, lo) = split_bits_ref(&a, 192);
        let (rlo, rhi) = (&a).unsigned_abs().split_bits(192);
        assert_eq!(-lo, rlo);
        assert_eq!(-hi, rhi);
    }
}
