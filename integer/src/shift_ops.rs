//! Bit shift operators.

use crate::{
    arch::word::{Word, DoubleWord},
    buffer::{Buffer, TypedRepr::*, TypedReprRef::*},
    ibig::IBig,
    primitive::{double_word, extend_word, split_dword, WORD_BITS_USIZE, DWORD_BITS_USIZE},
    shift,
    sign::Sign::*,
    ubig::UBig,
};
use core::{
    mem,
    ops::{Shl, ShlAssign, Shr, ShrAssign},
};

macro_rules! impl_shifts {
    ($t:ty) => {
        impl Shl<&usize> for $t {
            type Output = $t;

            #[inline]
            fn shl(self, rhs: &usize) -> $t {
                self.shl(*rhs)
            }
        }

        impl Shl<&usize> for &$t {
            type Output = $t;

            #[inline]
            fn shl(self, rhs: &usize) -> $t {
                self.shl(*rhs)
            }
        }

        impl ShlAssign<usize> for $t {
            #[inline]
            fn shl_assign(&mut self, rhs: usize) {
                *self = mem::take(self) << rhs;
            }
        }

        impl ShlAssign<&usize> for $t {
            #[inline]
            fn shl_assign(&mut self, rhs: &usize) {
                *self = mem::take(self) << rhs;
            }
        }

        impl Shr<&usize> for $t {
            type Output = $t;

            #[inline]
            fn shr(self, rhs: &usize) -> $t {
                self.shr(*rhs)
            }
        }

        impl Shr<&usize> for &$t {
            type Output = $t;

            #[inline]
            fn shr(self, rhs: &usize) -> $t {
                self.shr(*rhs)
            }
        }

        impl ShrAssign<usize> for $t {
            #[inline]
            fn shr_assign(&mut self, rhs: usize) {
                *self = mem::take(self).shr(rhs);
            }
        }

        impl ShrAssign<&usize> for $t {
            #[inline]
            fn shr_assign(&mut self, rhs: &usize) {
                *self = mem::take(self).shr(rhs);
            }
        }
    };
}

impl_shifts!(UBig);
impl_shifts!(IBig);

impl Shl<usize> for UBig {
    type Output = UBig;

    #[inline]
    fn shl(self, rhs: usize) -> UBig {
        ubig::shl_repr_val(self.into_repr(), rhs)
    }
}

impl Shl<usize> for &UBig {
    type Output = UBig;

    #[inline]
    fn shl(self, rhs: usize) -> UBig {
        ubig::shl_repr_ref(self.repr(), rhs)
    }
}

impl Shr<usize> for UBig {
    type Output = UBig;

    #[inline]
    fn shr(self, rhs: usize) -> UBig {
        ubig::shr_repr_val(self.into_repr(), rhs)
    }
}

impl Shr<usize> for &UBig {
    type Output = UBig;

    #[inline]
    fn shr(self, rhs: usize) -> UBig {
        ubig::shr_repr_ref(self.repr(), rhs)
    }
}

impl Shl<usize> for IBig {
    type Output = IBig;

    #[inline]
    fn shl(self, rhs: usize) -> IBig {
        let (sign, mag) = self.into_sign_repr();
        IBig::from_sign_magnitude(sign, ubig::shl_repr_val(mag, rhs))
    }
}

impl Shl<usize> for &IBig {
    type Output = IBig;

    #[inline]
    fn shl(self, rhs: usize) -> IBig {
        let (sign, mag) = self.as_sign_repr();
        IBig::from_sign_magnitude(sign, ubig::shl_repr_ref(mag, rhs))
    }
}

impl Shr<usize> for IBig {
    type Output = IBig;

    #[inline]
    fn shr(self, rhs: usize) -> IBig {
        let (sign, mag) = self.into_sign_repr();
        match sign {
            Positive => IBig::from(ubig::shr_repr_val(mag, rhs)),
            Negative => {
                let b = mag.are_low_bits_nonzero(rhs);
                -IBig::from(ubig::shr_repr_val(mag, rhs)) - IBig::from(b)
            }
        }
    }
}

impl Shr<usize> for &IBig {
    type Output = IBig;

    #[inline]
    fn shr(self, rhs: usize) -> IBig {
        let (sign, mag) = self.as_sign_repr();
        match sign {
            Positive => IBig::from(ubig::shr_repr_ref(mag, rhs)),
            Negative => {
                let b = mag.are_low_bits_nonzero(rhs);
                -IBig::from(ubig::shr_repr_ref(mag, rhs)) - IBig::from(b)
            }
        }
    }
}

mod ubig {
    use crate::buffer::{TypedReprRef, TypedRepr};
    use super::*;

    #[inline]
    pub(crate) fn shl_repr_val(repr: TypedRepr, rhs: usize) -> UBig {
        match repr {
            Small(0) => UBig::zero(),
            Small(dword) => shl_dword(dword, rhs),
            Large(buffer) => shl_large(buffer, rhs),
        }
    }

    #[inline]
    pub(crate) fn shl_repr_ref(repr: TypedReprRef, rhs: usize) -> UBig {
        match repr {
            RefSmall(0) => UBig::zero(),
            RefSmall(dword) => shl_dword(dword, rhs),
            RefLarge(buffer) => shl_ref_large(buffer, rhs),
        }
    }

    #[inline]
    pub(crate) fn shr_repr_val(repr: TypedRepr, rhs: usize) -> UBig {
        match repr {
            Small(dword) => shr_dword(dword, rhs),
            Large(buffer) => shr_large(buffer, rhs),
        }
    }

    #[inline]
    pub(crate) fn shr_repr_ref(repr: TypedReprRef, rhs: usize) -> UBig {
        match repr {
            RefSmall(dword) => shr_dword(dword, rhs),
            RefLarge(buffer) => shr_large_ref(buffer, rhs),
        }
    }

    /// Shift left a non-zero `DoubleWord` by `rhs` bits.
    // TODO: specialize the case where word == 1 using set_bit?
    #[inline]
    pub fn shl_dword(dword: DoubleWord, rhs: usize) -> UBig {
        debug_assert!(dword != 0);

        if rhs <= dword.leading_zeros() as usize {
            UBig::from(dword << rhs)
        } else {
            shl_dword_slow(dword, rhs)
        }
    }

    /// Shift left a non-zero `DoubleWord` by `rhs` bits.
    pub fn shl_dword_slow(dword: DoubleWord, rhs: usize) -> UBig {
        let shift_words = rhs / WORD_BITS_USIZE;
        let shift_bits = (rhs % WORD_BITS_USIZE) as u32;

        let (lo, hi) = split_dword(dword);
        let (n0, carry) = split_dword(extend_word(lo) << shift_bits);
        let (n1, n2) = split_dword((extend_word(hi) << shift_bits) | carry as u128);
        let mut buffer = Buffer::allocate(shift_words + 3);
        buffer.push_zeros(shift_words);
        buffer.push(n0);
        buffer.push(n1);
        buffer.push(n2);
        buffer.into()
    }

    /// Shift left `buffer` by `rhs` bits.
    fn shl_large(mut buffer: Buffer, rhs: usize) -> UBig {
        let shift_words = rhs / WORD_BITS_USIZE;

        if buffer.capacity() < buffer.len() + shift_words + 1 {
            return shl_ref_large(&buffer, rhs);
        }

        let shift_bits = (rhs % WORD_BITS_USIZE) as u32;
        let carry = shift::shl_in_place(&mut buffer, shift_bits);
        buffer.push(carry);
        buffer.push_zeros_front(shift_words);
        buffer.into()
    }

    /// Shift left large number of words by `rhs` bits.
    fn shl_ref_large(words: &[Word], rhs: usize) -> UBig {
        let shift_words = rhs / WORD_BITS_USIZE;
        let shift_bits = (rhs % WORD_BITS_USIZE) as u32;

        let mut buffer = Buffer::allocate(shift_words + words.len() + 1);
        buffer.push_zeros(shift_words);
        buffer.push_slice(words);
        let carry = shift::shl_in_place(&mut buffer[shift_words..], shift_bits);
        buffer.push(carry);
        buffer.into()
    }

    /// Shift right one `DoubleWord` by `rhs` bits.
    #[inline]
    fn shr_dword(dword: DoubleWord, rhs: usize) -> UBig {
        let dword = if rhs < DWORD_BITS_USIZE {
            dword >> rhs
        } else {
            0
        };
        dword.into()
    }

    /// Shift right `buffer` by `rhs` bits.
    fn shr_large(mut buffer: Buffer, rhs: usize) -> UBig {
        let shift_words = rhs / WORD_BITS_USIZE;
        if shift_words >= buffer.len() {
            return UBig::zero();
        }
        let shift_bits = (rhs % WORD_BITS_USIZE) as u32;
        buffer.erase_front(shift_words);
        shift::shr_in_place(&mut buffer, shift_bits);
        buffer.into()
    }

    /// Shift right large number of words by `rhs` bits.
    fn shr_large_ref(words: &[Word], rhs: usize) -> UBig {
        let shift_words = rhs / WORD_BITS_USIZE;
        let shift_bits = (rhs % WORD_BITS_USIZE) as u32;

        let words = &words[shift_words.min(words.len())..];

        match words {
            [] => UBig::zero(),
            &[w] => UBig::from(w >> shift_bits),
            &[lo, hi] => UBig::from(double_word(lo, hi) >> shift_bits),
            _ => {
                let mut buffer = Buffer::allocate(words.len());
                buffer.push_slice(words);
                shift::shr_in_place(&mut buffer, shift_bits);
                buffer.into()
            }
        }
    }
}
