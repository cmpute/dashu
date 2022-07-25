//! Information about radixes.

use crate::{
    arch::word::Word,
    fast_divide::{FastDivideNormalized, FastDivideSmall},
    primitive::WORD_BITS,
};
use static_assertions::const_assert;

/// Digit and radix type.
pub type Digit = u32;

/// Maximum supported radix.
pub const MAX_RADIX: Digit = 36;

/// Is a radix in valid range?
#[inline]
pub const fn is_radix_valid(radix: Digit) -> bool {
    2 <= radix && radix <= MAX_RADIX
}

/// Panics if `radix` is not in valid range.
#[inline]
pub fn check_radix_valid(radix: Digit) {
    if !is_radix_valid(radix) {
        panic!("Invalid radix: {}", radix);
    }
}

const_assert!(b'a' > b'0' + 10 && b'A' > b'0' + 10);

/// u8 representation is: how much digits >= 10 should be offset by in ASCII.
#[derive(Clone, Copy, Eq, PartialEq)]
#[repr(u8)]
pub enum DigitCase {
    NoLetters = 0,
    Lower = b'a' - b'0' - 10,
    Upper = b'A' - b'0' - 10,
}

/// Converts a byte (ASCII) representation of a digit to its value.
///
/// The radix has to be in range 2..36 (inclusive), and the digit will
/// be parsed with insensitive case.
#[inline]
pub const fn digit_from_ascii_byte(byte: u8, radix: Digit) -> Option<Digit> {
    assert!(is_radix_valid(radix));

    let res = match byte {
        c @ b'0'..=b'9' => (c - b'0') as Digit,
        c @ b'a'..=b'z' => (c - b'a') as Digit + 10,
        c @ b'A'..=b'Z' => (c - b'A') as Digit + 10,
        _ => return None,
    };
    if res < radix {
        Some(res)
    } else {
        None
    }
}

/// Maximum number of digits that a `Word` can ever have for any non-power-of-2 radix.
pub const MAX_WORD_DIGITS_NON_POW_2: usize = RadixInfo::for_radix(3).digits_per_word + 1;

/// Properties of a given radix.
#[derive(Clone, Copy)]
pub struct RadixInfo {
    /// The number of digits that can always fit in a `Word`.
    pub(crate) digits_per_word: usize,

    /// Radix to the power of `max_digits`.
    /// Only for non-power-of-2 radixes.
    pub(crate) range_per_word: Word,

    /// Faster division by `radix`.
    pub(crate) fast_div_radix: FastDivideSmall,

    /// Faster division by normalized range_per_word.
    /// Only for non-power-of-2 radixes.
    pub(crate) fast_div_range_per_word: FastDivideNormalized,
}

/// RadixInfo for a given radix.
#[inline]
pub fn radix_info(radix: Digit) -> &'static RadixInfo {
    debug_assert!(is_radix_valid(radix));
    &RADIX_INFO_TABLE[radix as usize]
}

impl RadixInfo {
    const fn for_radix(radix: Digit) -> RadixInfo {
        let fast_div_radix = FastDivideSmall::new(radix as Word);
        if radix.is_power_of_two() {
            RadixInfo {
                digits_per_word: (WORD_BITS / radix.trailing_zeros()) as usize,
                range_per_word: 0,
                fast_div_radix,
                fast_div_range_per_word: FastDivideNormalized::dummy(),
            }
        } else {
            let mut digits_per_word = 0;
            let mut range_per_word: Word = 1;
            while let Some(range) = range_per_word.checked_mul(radix as Word) {
                digits_per_word += 1;
                range_per_word = range;
            }
            let shift = range_per_word.leading_zeros();
            let fast_div_range_per_word = FastDivideNormalized::new(range_per_word << shift);
            RadixInfo {
                digits_per_word,
                range_per_word,
                fast_div_radix,
                fast_div_range_per_word,
            }
        }
    }
}

// XXX: we may only store the table for 10 to reduce the binary size
type RadixInfoTable = [RadixInfo; MAX_RADIX as usize + 1];

static RADIX_INFO_TABLE: RadixInfoTable = generate_radix_info_table();

const fn generate_radix_info_table() -> RadixInfoTable {
    let mut table = [RadixInfo {
        digits_per_word: 0,
        range_per_word: 0,
        fast_div_radix: FastDivideSmall::dummy(),
        fast_div_range_per_word: FastDivideNormalized::dummy(),
    }; MAX_RADIX as usize + 1];

    let mut radix = 2;
    while radix <= MAX_RADIX {
        table[radix as usize] = RadixInfo::for_radix(radix);
        radix += 1;
    }
    table
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_radix_info_table() {
        for radix in 2..=MAX_RADIX {
            let info = radix_info(radix);
            // Check vs an approximation that happens to work for all bases.
            assert_eq!(
                info.digits_per_word,
                ((WORD_BITS as f64 + 0.01) / (radix as f64).log2()) as usize
            );
            if !radix.is_power_of_two() {
                assert_eq!(
                    info.range_per_word,
                    (radix as Word).pow(info.digits_per_word as u32)
                );
            }
        }
    }

    #[test]
    fn test_digit_from_utf8_byte() {
        assert_eq!(digit_from_ascii_byte(b'7', 10), Some(7));
        assert_eq!(digit_from_ascii_byte(b'a', 16), Some(10));
        assert_eq!(digit_from_ascii_byte(b'z', 36), Some(35));
        assert_eq!(digit_from_ascii_byte(b'Z', 36), Some(35));
        assert_eq!(digit_from_ascii_byte(b'?', 10), None);
        assert_eq!(digit_from_ascii_byte(b'a', 10), None);
        assert_eq!(digit_from_ascii_byte(b'z', 35), None);
        assert_eq!(digit_from_ascii_byte(255, 35), None);
    }
}
