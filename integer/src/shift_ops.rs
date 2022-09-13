//! Bit shift operators.

use crate::{ibig::IBig, ubig::UBig, Sign::*};
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
        UBig(self.into_repr().shl(rhs))
    }
}

impl Shl<usize> for &UBig {
    type Output = UBig;

    #[inline]
    fn shl(self, rhs: usize) -> UBig {
        UBig(self.repr().shl(rhs))
    }
}

impl Shr<usize> for UBig {
    type Output = UBig;

    #[inline]
    fn shr(self, rhs: usize) -> UBig {
        UBig(self.into_repr().shr(rhs))
    }
}

impl Shr<usize> for &UBig {
    type Output = UBig;

    #[inline]
    fn shr(self, rhs: usize) -> UBig {
        UBig(self.repr().shr(rhs))
    }
}

impl Shl<usize> for IBig {
    type Output = IBig;

    #[inline]
    fn shl(self, rhs: usize) -> IBig {
        let (sign, mag) = self.into_sign_repr();
        let repr = mag << rhs;
        IBig(repr.with_sign(sign))
    }
}

impl Shl<usize> for &IBig {
    type Output = IBig;

    #[inline]
    fn shl(self, rhs: usize) -> IBig {
        let (sign, mag) = self.as_sign_repr();
        let repr = mag << rhs;
        IBig(repr.with_sign(sign))
    }
}

impl Shr<usize> for IBig {
    type Output = IBig;

    #[inline]
    fn shr(self, rhs: usize) -> IBig {
        let (sign, mag) = self.into_sign_repr();
        match sign {
            Positive => IBig(mag >> rhs),
            Negative => {
                let b = mag.as_ref().are_low_bits_nonzero(rhs);
                -IBig(mag >> rhs) - IBig::from(b)
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
            Positive => IBig(mag >> rhs),
            Negative => {
                let b = mag.are_low_bits_nonzero(rhs);
                -IBig(mag >> rhs) - IBig::from(b)
            }
        }
    }
}

pub(crate) mod repr {
    use super::*;
    use crate::{
        arch::word::{DoubleWord, Word},
        buffer::Buffer,
        math,
        primitive::{double_word, DWORD_BITS_USIZE, WORD_BITS_USIZE},
        repr::{
            Repr,
            TypedRepr::{self, *},
            TypedReprRef::{self, *},
        },
        shift,
    };

    impl Shl<usize> for TypedRepr {
        type Output = Repr;
        #[inline]
        fn shl(self, rhs: usize) -> Repr {
            match self {
                Small(0) => Repr::zero(),
                Small(dword) => shl_dword(dword, rhs),
                Large(buffer) => shl_large(buffer, rhs),
            }
        }
    }

    impl Shr<usize> for TypedRepr {
        type Output = Repr;
        #[inline]
        fn shr(self, rhs: usize) -> Repr {
            match self {
                Small(dword) => shr_dword(dword, rhs),
                Large(buffer) => shr_large(buffer, rhs),
            }
        }
    }

    impl<'a> Shl<usize> for TypedReprRef<'a> {
        type Output = Repr;
        #[inline]
        fn shl(self, rhs: usize) -> Repr {
            match self {
                RefSmall(0) => Repr::zero(),
                RefSmall(dword) => shl_dword(dword, rhs),
                RefLarge(words) => shl_ref_large(words, rhs),
            }
        }
    }

    impl<'a> Shr<usize> for TypedReprRef<'a> {
        type Output = Repr;
        #[inline]
        fn shr(self, rhs: usize) -> Repr {
            match self {
                RefSmall(dword) => shr_dword(dword, rhs),
                RefLarge(words) => shr_large_ref(words, rhs),
            }
        }
    }

    /// Shift left a non-zero `DoubleWord` by `rhs` bits.
    #[inline]
    fn shl_dword(dword: DoubleWord, rhs: usize) -> Repr {
        debug_assert!(dword != 0);

        if rhs <= dword.leading_zeros() as usize {
            Repr::from_dword(dword << rhs)
        } else if dword == 1 {
            shl_one_spilled(rhs)
        } else {
            shl_dword_spilled(dword, rhs)
        }
    }

    /// Shift left 1 by `rhs` bits
    fn shl_one_spilled(rhs: usize) -> Repr {
        debug_assert!(rhs >= DWORD_BITS_USIZE);
        let idx = rhs / WORD_BITS_USIZE;
        let mut buffer = Buffer::allocate(idx + 1);
        buffer.push_zeros(idx);
        buffer.push(1 << (rhs % WORD_BITS_USIZE));
        Repr::from_buffer(buffer)
    }

    /// Shift left a non-zero `DoubleWord` by `rhs` bits.
    fn shl_dword_spilled(dword: DoubleWord, rhs: usize) -> Repr {
        let shift_words = rhs / WORD_BITS_USIZE;
        let shift_bits = (rhs % WORD_BITS_USIZE) as u32;

        let (n0, n1, n2) = math::shl_dword(dword, shift_bits);
        let mut buffer = Buffer::allocate(shift_words + 3);
        buffer.push_zeros(shift_words);
        buffer.push(n0);
        buffer.push(n1);
        buffer.push(n2);
        Repr::from_buffer(buffer)
    }

    /// Shift left `buffer` by `rhs` bits.
    fn shl_large(mut buffer: Buffer, rhs: usize) -> Repr {
        let shift_words = rhs / WORD_BITS_USIZE;

        if buffer.capacity() < buffer.len() + shift_words + 1 {
            return shl_ref_large(&buffer, rhs);
        }

        let shift_bits = (rhs % WORD_BITS_USIZE) as u32;
        let carry = shift::shl_in_place(&mut buffer, shift_bits);
        buffer.push(carry);
        buffer.push_zeros_front(shift_words);
        Repr::from_buffer(buffer)
    }

    /// Shift left large number of words by `rhs` bits.
    fn shl_ref_large(words: &[Word], rhs: usize) -> Repr {
        let shift_words = rhs / WORD_BITS_USIZE;
        let shift_bits = (rhs % WORD_BITS_USIZE) as u32;

        let mut buffer = Buffer::allocate(shift_words + words.len() + 1);
        buffer.push_zeros(shift_words);
        buffer.push_slice(words);
        let carry = shift::shl_in_place(&mut buffer[shift_words..], shift_bits);
        buffer.push(carry);
        Repr::from_buffer(buffer)
    }

    /// Shift right one `DoubleWord` by `rhs` bits.
    #[inline]
    fn shr_dword(dword: DoubleWord, rhs: usize) -> Repr {
        if rhs < DWORD_BITS_USIZE {
            Repr::from_dword(dword >> rhs)
        } else {
            Repr::zero()
        }
    }

    /// Shift right `buffer` by `rhs` bits.
    fn shr_large(mut buffer: Buffer, rhs: usize) -> Repr {
        let shift_words = rhs / WORD_BITS_USIZE;
        if shift_words >= buffer.len() {
            return Repr::zero();
        }
        let shift_bits = (rhs % WORD_BITS_USIZE) as u32;
        buffer.erase_front(shift_words);
        shift::shr_in_place(&mut buffer, shift_bits);
        Repr::from_buffer(buffer)
    }

    /// Shift right large number of words by `rhs` bits.
    pub(crate) fn shr_large_ref(words: &[Word], rhs: usize) -> Repr {
        let shift_words = rhs / WORD_BITS_USIZE;
        let shift_bits = (rhs % WORD_BITS_USIZE) as u32;

        let words = &words[shift_words.min(words.len())..];

        match words {
            [] => Repr::zero(),
            &[w] => Repr::from_word(w >> shift_bits),
            &[lo, hi] => Repr::from_dword(double_word(lo, hi) >> shift_bits),
            _ => {
                let mut buffer = Buffer::allocate(words.len());
                buffer.push_slice(words);
                shift::shr_in_place(&mut buffer, shift_bits);
                Repr::from_buffer(buffer)
            }
        }
    }
}
