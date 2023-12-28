//! Primitive integral types.

use crate::{
    arch::word::{DoubleWord, SignedDoubleWord, SignedWord, Word},
    Sign::{self, *},
};
use core::{
    convert::{TryFrom, TryInto},
    fmt::Debug,
    hint::unreachable_unchecked,
    mem,
    ops::{Add, Div, Mul, Shl, Shr, Sub},
};
use dashu_base::ConversionError;

/// Cast [Word] to [DoubleWord].
#[inline]
pub const fn extend_word(word: Word) -> DoubleWord {
    word as DoubleWord
}

/// Cast [Word] to [SignedDoubleWord].
#[inline]
pub const fn signed_extend_word(word: Word) -> SignedDoubleWord {
    word as SignedDoubleWord
}

/// Create a [DoubleWord] from two `Word`s.
#[inline]
pub const fn double_word(low: Word, high: Word) -> DoubleWord {
    extend_word(low) | extend_word(high) << WORD_BITS
}

/// Split a [DoubleWord] into (low, high) parts
#[inline]
pub const fn split_dword(dw: DoubleWord) -> (Word, Word) {
    (dw as Word, (dw >> WORD_BITS) as Word)
}

/// Split a [SignedDoubleWord] into (low, high) parts, where the high part is signed
/// and low part is unsigned
#[inline]
pub const fn split_signed_dword(dw: SignedDoubleWord) -> (Word, SignedWord) {
    (dw as Word, (dw >> WORD_BITS) as SignedWord)
}

/// Get the low part of a `DoubleWord` if the high part is zero
#[inline]
pub const fn shrink_dword(dw: DoubleWord) -> Option<Word> {
    let (lo, hi) = split_dword(dw);
    if hi == 0 {
        Some(lo)
    } else {
        None
    }
}

/// Get the lowest double word of a slice of words
///
/// Note that then length is only checked in the debug mode.
#[inline(always)]
pub fn lowest_dword(words: &[Word]) -> DoubleWord {
    debug_assert!(words.len() >= 2);

    // SAFETY: length checked by the assertion above
    unsafe {
        let lo = *words.get_unchecked(0);
        let hi = *words.get_unchecked(1);
        double_word(lo, hi)
    }
}

/// Get the highest double word of a slice of words
///
/// Note that then length is only checked in the debug mode.
#[inline(always)]
pub fn highest_dword(words: &[Word]) -> DoubleWord {
    let len = words.len();
    debug_assert!(len >= 2);

    // SAFETY: length checked by the assertion above
    unsafe {
        let lo = *words.get_unchecked(len - 2);
        let hi = *words.get_unchecked(len - 1);
        double_word(lo, hi)
    }
}

/// Split the the highest word from the word array.
#[inline]
pub const fn split_hi_word(words: &[Word]) -> (Word, &[Word]) {
    debug_assert!(words.len() >= 2);
    match words.split_last() {
        Some((hi, lo)) => (*hi, lo),
        // SAFETY: the words length is checked by the assertion
        None => unsafe { unreachable_unchecked() },
    }
}

/// Locate the top non-zero word in a slice. It returns the position of the
/// word added by one for convenience, if the input is zero, then 0 is returned.
#[inline]
pub fn locate_top_word_plus_one(words: &[Word]) -> usize {
    for pos in (0..words.len()).rev() {
        if words[pos] != 0 {
            return pos + 1;
        }
    }
    0
}

pub trait PrimitiveUnsigned
where
    Self: Copy,
    Self: Debug,
    Self: Default,
    Self: From<u8>,
    Self: TryFrom<Word>,
    Self: TryFrom<DoubleWord>,
    Self: TryInto<Word>,
    Self: TryInto<DoubleWord>,
    Self: TryInto<usize>,
    Self: Eq,
    Self: Add<Output = Self>,
    Self: Div<Output = Self>,
    Self: Mul<Output = Self>,
    Self: Sub<Output = Self>,
    Self: Shl<u32, Output = Self>,
    Self: Shr<u32, Output = Self>,
{
    const BYTE_SIZE: usize = mem::size_of::<Self>();
    const BIT_SIZE: u32 = 8 * Self::BYTE_SIZE as u32;
    const MAX: Self;
    type ByteRepr: AsRef<[u8]> + AsMut<[u8]>;

    fn to_le_bytes(self) -> Self::ByteRepr;
    fn from_le_bytes(repr: Self::ByteRepr) -> Self;
    fn leading_zeros(self) -> u32;
}

pub trait PrimitiveSigned
where
    Self: Copy,
    Self: TryFrom<DoubleWord>,
    Self::Unsigned: PrimitiveUnsigned,
    Self::Unsigned: TryFrom<Self>,
    Self::Unsigned: TryInto<Self>,
{
    type Unsigned;

    fn to_sign_magnitude(self) -> (Sign, Self::Unsigned);
    fn try_from_sign_magnitude(sign: Sign, mag: Self::Unsigned) -> Result<Self, ConversionError>;
}

macro_rules! impl_primitive_unsigned {
    ($t:ty) => {
        impl PrimitiveUnsigned for $t {
            type ByteRepr = [u8; Self::BYTE_SIZE];
            const MAX: Self = Self::MAX;

            #[inline]
            fn to_le_bytes(self) -> Self::ByteRepr {
                self.to_le_bytes()
            }

            #[inline]
            fn from_le_bytes(repr: Self::ByteRepr) -> Self {
                Self::from_le_bytes(repr)
            }

            #[inline]
            fn leading_zeros(self) -> u32 {
                self.leading_zeros()
            }
        }
    };
}

macro_rules! impl_primitive_signed {
    ($t:ty, $u:ty) => {
        impl PrimitiveSigned for $t {
            type Unsigned = $u;

            #[inline]
            fn to_sign_magnitude(self) -> (Sign, Self::Unsigned) {
                if self >= 0 {
                    (Positive, self as Self::Unsigned)
                } else {
                    (Negative, (self as Self::Unsigned).wrapping_neg())
                }
            }

            #[inline]
            fn try_from_sign_magnitude(
                sign: Sign,
                mag: Self::Unsigned,
            ) -> Result<Self, ConversionError> {
                match sign {
                    Positive => mag.try_into().map_err(|_| ConversionError::OutOfBounds),
                    Negative => {
                        let x = mag.wrapping_neg() as Self;
                        if x <= 0 {
                            Ok(x)
                        } else {
                            Err(ConversionError::OutOfBounds)
                        }
                    }
                }
            }
        }
    };
}

impl_primitive_unsigned!(u8);
impl_primitive_unsigned!(u16);
impl_primitive_unsigned!(u32);
impl_primitive_unsigned!(u64);
impl_primitive_unsigned!(u128);
impl_primitive_unsigned!(usize);

impl_primitive_signed!(i8, u8);
impl_primitive_signed!(i16, u16);
impl_primitive_signed!(i32, u32);
impl_primitive_signed!(i64, u64);
impl_primitive_signed!(i128, u128);
impl_primitive_signed!(isize, usize);

pub const WORD_BITS: u32 = Word::BIT_SIZE;
pub const WORD_BITS_USIZE: usize = WORD_BITS as usize;
pub const WORD_BYTES: usize = Word::BYTE_SIZE;
pub const DWORD_BITS: u32 = DoubleWord::BIT_SIZE;
pub const DWORD_BITS_USIZE: usize = DWORD_BITS as usize;
pub const DWORD_BYTES: usize = DoubleWord::BYTE_SIZE;

#[inline]
pub fn word_from_le_bytes_partial(bytes: &[u8]) -> Word {
    let mut word_bytes = [0; WORD_BYTES];
    word_bytes[..bytes.len()].copy_from_slice(bytes);
    Word::from_le_bytes(word_bytes)
}

#[inline]
pub fn dword_from_le_bytes_partial(bytes: &[u8]) -> DoubleWord {
    let mut dword_bytes = [0; DWORD_BYTES];
    dword_bytes[..bytes.len()].copy_from_slice(bytes);
    DoubleWord::from_le_bytes(dword_bytes)
}

#[inline]
pub fn word_from_be_bytes_partial(bytes: &[u8]) -> Word {
    let mut word_bytes = [0; WORD_BYTES];
    word_bytes[WORD_BYTES - bytes.len()..].copy_from_slice(bytes);
    Word::from_be_bytes(word_bytes)
}

#[inline]
pub fn dword_from_be_bytes_partial(bytes: &[u8]) -> DoubleWord {
    let mut dword_bytes = [0; DWORD_BYTES];
    dword_bytes[DWORD_BYTES - bytes.len()..].copy_from_slice(bytes);
    DoubleWord::from_be_bytes(dword_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bits_bytes() {
        assert_eq!(u8::BIT_SIZE, 8);
        assert_eq!(u64::BIT_SIZE, 64);
        assert_eq!(u8::BYTE_SIZE, 1);
        assert_eq!(u64::BYTE_SIZE, 8);
    }

    #[test]
    fn test_word_from_le_bytes_partial() {
        assert_eq!(word_from_le_bytes_partial(&[1, 2]), 0x0201);
    }

    #[test]
    fn test_word_from_be_bytes_partial() {
        assert_eq!(word_from_be_bytes_partial(&[1, 2]), 0x0102);
    }

    #[test]
    fn test_double_word() {
        assert_eq!(DoubleWord::BIT_SIZE, 2 * WORD_BITS);
        assert_eq!(split_dword(double_word(3, 4)), (3, 4));
    }

    #[test]
    fn test_to_sign_magnitude() {
        assert_eq!(0.to_sign_magnitude(), (Positive, 0u32));
        assert_eq!(5.to_sign_magnitude(), (Positive, 5u32));
        assert_eq!(0x7fffffff.to_sign_magnitude(), (Positive, 0x7fffffffu32));
        assert_eq!((-0x80000000).to_sign_magnitude(), (Negative, 0x80000000u32));
    }

    #[test]
    fn test_try_from_sign_magnitude() {
        assert_eq!(i32::try_from_sign_magnitude(Positive, 0), Ok(0));
        assert_eq!(i32::try_from_sign_magnitude(Positive, 5), Ok(5));
        assert_eq!(i32::try_from_sign_magnitude(Positive, 0x7fffffff), Ok(0x7fffffff));
        assert!(i32::try_from_sign_magnitude(Positive, 0x80000000).is_err());
        assert_eq!(i32::try_from_sign_magnitude(Negative, 0), Ok(0));
        assert_eq!(i32::try_from_sign_magnitude(Negative, 5), Ok(-5));
        assert_eq!(i32::try_from_sign_magnitude(Negative, 0x7fffffff), Ok(-0x7fffffff));
        assert_eq!(i32::try_from_sign_magnitude(Negative, 0x80000000), Ok(-0x80000000));
        assert!(i32::try_from_sign_magnitude(Negative, 0x80000001).is_err());
        assert!(i32::try_from_sign_magnitude(Negative, 0xffffffff).is_err());
    }
}
