//! Bitwise operators.

use crate::{
    arch::word::{DoubleWord, Word},
    repr::{Buffer, TypedRepr::*, TypedReprRef::*},
    helper_macros,
    ibig::IBig,
    math,
    ops::{AndNot, PowerOfTwo, UnsignedAbs},
    primitive::{double_word, split_dword, DWORD_BITS_USIZE, WORD_BITS_USIZE},
    sign::Sign::*,
    ubig::UBig,
};
use core::{
    mem,
    ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not},
};

pub(crate) fn trailing_zeros_large(words: &[Word]) -> usize {
    debug_assert!(*words.last().unwrap() != 0);

    for (idx, word) in words.iter().enumerate() {
        if *word != 0 {
            return idx * WORD_BITS_USIZE + word.trailing_zeros() as usize;
        }
    }

    // the assertion above ensured that there must be at least one non-zero word
    unreachable!()
}

// TODO: move this to math?
// Panics if the length of words is less than 2
#[inline]
fn front_dword(words: &[Word]) -> DoubleWord {
    debug_assert!(words.len() >= 2);
    unsafe {
        let lo = *words.get_unchecked(0);
        let hi = *words.get_unchecked(1);
        double_word(lo, hi)
    }
}

impl UBig {
    /// Returns true if the `n`-th bit is set, n starts from 0.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::ubig;
    /// assert_eq!(ubig!(0b10010).bit(1), true);
    /// assert_eq!(ubig!(0b10010).bit(3), false);
    /// assert_eq!(ubig!(0b10010).bit(100), false);
    /// ```
    #[inline]
    pub fn bit(&self, n: usize) -> bool {
        match self.repr() {
            RefSmall(dword) => n < DWORD_BITS_USIZE && dword & 1 << n != 0,
            RefLarge(buffer) => {
                let idx = n / WORD_BITS_USIZE;
                idx < buffer.len() && buffer[idx] & 1 << (n % WORD_BITS_USIZE) != 0
            }
        }
    }

    /// Set the `n`-th bit, n starts from 0.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::ubig;
    /// let mut a = ubig!(0b100);
    /// a.set_bit(0);
    /// assert_eq!(a, ubig!(0b101));
    /// a.set_bit(10);
    /// assert_eq!(a, ubig!(0b10000000101));
    /// ```
    #[inline]
    pub fn set_bit(&mut self, n: usize) {
        match mem::take(self).into_repr() {
            Small(dword) => {
                if n < DWORD_BITS_USIZE {
                    *self = UBig::from(dword | 1 << n);
                } else {
                    *self = ubig::with_bit_dword_spilled(dword, n);
                }
            }
            Large(buffer) => {
                *self = ubig::with_bit_large(buffer, n);
            }
        }
    }

    /// Clear the `n`-th bit, n starts from 0.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::ubig;
    /// let mut a = ubig!(0b101);
    /// a.clear_bit(0);
    /// assert_eq!(a, ubig!(0b100));
    /// ```
    #[inline]
    pub fn clear_bit(&mut self, n: usize) {
        match mem::take(self).into_repr() {
            Small(dword) => {
                if n < DWORD_BITS_USIZE {
                    *self = UBig::from(dword & !(1 << n));
                }
            }
            Large(mut buffer) => {
                let idx = n / WORD_BITS_USIZE;
                if idx < buffer.len() {
                    buffer[idx] &= !(1 << (n % WORD_BITS_USIZE));
                }
                *self = buffer.into();
            }
        }
    }

    /// Returns the number of trailing zeros in the binary representation.
    ///
    /// In other words, it is the largest `n` such that 2 to the power of `n` divides the number.
    ///
    /// For 0, it returns `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::ubig;
    /// assert_eq!(ubig!(17).trailing_zeros(), Some(0));
    /// assert_eq!(ubig!(48).trailing_zeros(), Some(4));
    /// assert_eq!(ubig!(0b101000000).trailing_zeros(), Some(6));
    /// assert_eq!(ubig!(0).trailing_zeros(), None);
    /// ```
    #[inline]
    pub fn trailing_zeros(&self) -> Option<usize> {
        match self.repr() {
            RefSmall(0) => None,
            RefSmall(dword) => Some(dword.trailing_zeros() as usize),
            RefLarge(buffer) => Some(trailing_zeros_large(buffer)),
        }
    }

    /// Bit length.
    ///
    /// The length of the binary representation of the number.
    ///
    /// For 0, the length is 0.
    ///
    /// For non-zero numbers it is:
    /// * `in_radix(2).to_string().len()`
    /// * the index of the top 1 bit plus one
    /// * the floor of the logarithm base 2 of the number plus one.
    ///
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::ubig;
    /// assert_eq!(ubig!(17).bit_len(), 5);
    /// assert_eq!(ubig!(0b101000000).bit_len(), 9);
    /// assert_eq!(ubig!(0).bit_len(), 0);
    /// let x = ubig!(_0x90ffff3450897234);
    /// assert_eq!(x.bit_len(), x.in_radix(2).to_string().len());
    /// ```
    #[inline]
    pub fn bit_len(&self) -> usize {
        self.repr().bit_len()
    }
}

mod repr {
    use super::*;
    use crate::repr::{Repr, TypedRepr, TypedReprRef};

    impl<'a> TypedReprRef<'a> {
        #[inline]
        pub fn bit_len(self) -> usize {
            match self {
                RefSmall(dword) => math::bit_len(dword) as usize,
                RefLarge(buffer) => {
                    buffer.len() * WORD_BITS_USIZE - buffer.last().unwrap().leading_zeros() as usize
                }
            }
        }

        /// Get the highest n bits from the Repr
        #[inline]
        pub fn high_bits(self, n: usize) -> Repr {
            let bit_len = self.bit_len();
            self >> (bit_len - n)
        }

        /// Check if low n-bits are not all zeros
        #[inline]
        pub(crate) fn are_low_bits_nonzero(&self, n: usize) -> bool {
            match self {
                Self::RefSmall(dword) => are_dword_low_bits_nonzero(dword, n),
                Self::RefLarge(buffer) => are_slice_low_bits_nonzero(buffer, n),
            }
        }
    }

    impl TypedRepr {
        /// Check if low n-bits are not all zeros
        #[inline]
        pub(crate) fn are_low_bits_nonzero(&self, n: usize) -> bool {
            match self {
                Self::Small(dword) => are_dword_low_bits_nonzero(dword, n),
                Self::Large(buffer) => are_slice_low_bits_nonzero(buffer, n),
            }
        }
    }

    #[inline]
    fn are_dword_low_bits_nonzero(dword: &DoubleWord, n: usize) -> bool {
        let n = n.min(WORD_BITS_USIZE) as u32;
        dword & math::ones_dword(n) != 0
    }

    fn are_slice_low_bits_nonzero(words: &[Word], n: usize) -> bool {
        let n_words = n / WORD_BITS_USIZE;
        if n_words >= words.len() {
            true
        } else {
            let n_top = (n % WORD_BITS_USIZE) as u32;
            words[..n_words].iter().any(|x| *x != 0) || words[n_words] & math::ones_word(n_top) != 0
        }
    }
}

impl IBig {
    /// Returns the number of trailing zeros in the two's complement binary representation.
    ///
    /// In other words, it is the largest `n` such that 2 to the power of `n` divides the number.
    ///
    /// For 0, it returns `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::ibig;
    /// assert_eq!(ibig!(17).trailing_zeros(), Some(0));
    /// assert_eq!(ibig!(-48).trailing_zeros(), Some(4));
    /// assert_eq!(ibig!(-0b101000000).trailing_zeros(), Some(6));
    /// assert_eq!(ibig!(0).trailing_zeros(), None);
    /// ```
    #[inline]
    pub fn trailing_zeros(&self) -> Option<usize> {
        match self.as_sign_repr().1 {
            RefSmall(0) => None,
            RefSmall(dword) => Some(dword.trailing_zeros() as usize),
            RefLarge(buffer) => Some(trailing_zeros_large(buffer)),
        }
    }
}

impl PowerOfTwo for UBig {
    #[inline]
    fn is_power_of_two(&self) -> bool {
        match self.repr() {
            RefSmall(dword) => dword.is_power_of_two(),
            RefLarge(buffer) => {
                buffer[..buffer.len() - 1].iter().all(|x| *x == 0)
                    && buffer.last().unwrap().is_power_of_two()
            }
        }
    }

    #[inline]
    fn next_power_of_two(self) -> UBig {
        match self.into_repr() {
            Small(dword) => match dword.checked_next_power_of_two() {
                Some(p) => UBig::from(p),
                None => {
                    let mut buffer = Buffer::allocate(3);
                    buffer.push_zeros(2);
                    buffer.push(1);
                    buffer.into()
                }
            },
            Large(buffer) => ubig::next_power_of_two_large(buffer),
        }
    }
}

impl BitAnd<UBig> for UBig {
    type Output = UBig;

    #[inline]
    fn bitand(self, rhs: UBig) -> UBig {
        match (self.into_repr(), rhs.into_repr()) {
            (Small(dword0), Small(dword1)) => UBig::from(dword0 & dword1),
            (Small(dword0), Large(buffer1)) => UBig::from(dword0 & buffer1.front_dword()),
            (Large(buffer0), Small(dword1)) => UBig::from(buffer0.front_dword() & dword1),
            (Large(buffer0), Large(buffer1)) => {
                if buffer0.len() <= buffer1.len() {
                    ubig::bitand_large(buffer0, &buffer1)
                } else {
                    ubig::bitand_large(buffer1, &buffer0)
                }
            }
        }
    }
}

impl BitAnd<&UBig> for UBig {
    type Output = UBig;

    #[inline]
    fn bitand(self, rhs: &UBig) -> UBig {
        match (self.into_repr(), rhs.repr()) {
            (Small(dword0), RefSmall(dword1)) => UBig::from(dword0 & dword1),
            (Small(dword0), RefLarge(buffer1)) => UBig::from(dword0 & front_dword(buffer1)),
            (Large(buffer0), RefSmall(dword1)) => UBig::from(buffer0.front_dword() & dword1),
            (Large(buffer0), RefLarge(buffer1)) => ubig::bitand_large(buffer0, buffer1),
        }
    }
}

impl BitAnd<UBig> for &UBig {
    type Output = UBig;

    #[inline]
    fn bitand(self, rhs: UBig) -> UBig {
        rhs.bitand(self)
    }
}

impl BitAnd<&UBig> for &UBig {
    type Output = UBig;

    #[inline]
    fn bitand(self, rhs: &UBig) -> UBig {
        match (self.repr(), rhs.repr()) {
            (RefSmall(dword0), RefSmall(dword1)) => UBig::from(dword0 & dword1),
            (RefSmall(dword0), RefLarge(buffer1)) => UBig::from(dword0 & front_dword(buffer1)),
            (RefLarge(buffer0), RefSmall(dword1)) => UBig::from(front_dword(buffer0) & dword1),
            (RefLarge(buffer0), RefLarge(buffer1)) => {
                if buffer0.len() <= buffer1.len() {
                    ubig::bitand_large(buffer0.into(), buffer1)
                } else {
                    ubig::bitand_large(buffer1.into(), buffer0)
                }
            }
        }
    }
}

impl BitAndAssign<UBig> for UBig {
    #[inline]
    fn bitand_assign(&mut self, rhs: UBig) {
        *self = mem::take(self) & rhs;
    }
}

impl BitAndAssign<&UBig> for UBig {
    #[inline]
    fn bitand_assign(&mut self, rhs: &UBig) {
        *self = mem::take(self) & rhs;
    }
}

impl BitOr<UBig> for UBig {
    type Output = UBig;

    #[inline]
    fn bitor(self, rhs: UBig) -> UBig {
        match (self.into_repr(), rhs.into_repr()) {
            (Small(dword0), Small(dword1)) => UBig::from(dword0 | dword1),
            (Small(dword0), Large(buffer1)) => ubig::bitor_large_dword(buffer1, dword0),
            (Large(buffer0), Small(dword1)) => ubig::bitor_large_dword(buffer0, dword1),
            (Large(buffer0), Large(buffer1)) => {
                if buffer0.len() >= buffer1.len() {
                    ubig::bitor_large(buffer0, &buffer1)
                } else {
                    ubig::bitor_large(buffer1, &buffer0)
                }
            }
        }
    }
}

impl BitOr<&UBig> for UBig {
    type Output = UBig;

    #[inline]
    fn bitor(self, rhs: &UBig) -> UBig {
        match (self.into_repr(), rhs.repr()) {
            (Small(dword0), RefSmall(dword1)) => UBig::from(dword0 | dword1),
            (Small(dword0), RefLarge(buffer1)) => ubig::bitor_large_dword(buffer1.into(), dword0),
            (Large(buffer0), RefSmall(dword1)) => ubig::bitor_large_dword(buffer0, dword1),
            (Large(buffer0), RefLarge(buffer1)) => ubig::bitor_large(buffer0, buffer1),
        }
    }
}

impl BitOr<UBig> for &UBig {
    type Output = UBig;

    #[inline]
    fn bitor(self, rhs: UBig) -> UBig {
        rhs.bitor(self)
    }
}

impl BitOr<&UBig> for &UBig {
    type Output = UBig;

    #[inline]
    fn bitor(self, rhs: &UBig) -> UBig {
        match (self.repr(), rhs.repr()) {
            (RefSmall(dword0), RefSmall(dword1)) => UBig::from(dword0 | dword1),
            (RefSmall(dword0), RefLarge(buffer1)) => {
                ubig::bitor_large_dword(buffer1.into(), dword0)
            }
            (RefLarge(buffer0), RefSmall(dword1)) => {
                ubig::bitor_large_dword(buffer0.into(), dword1)
            }
            (RefLarge(buffer0), RefLarge(buffer1)) => {
                if buffer0.len() >= buffer1.len() {
                    ubig::bitor_large(buffer0.into(), buffer1)
                } else {
                    ubig::bitor_large(buffer1.into(), buffer0)
                }
            }
        }
    }
}

impl BitOrAssign<UBig> for UBig {
    #[inline]
    fn bitor_assign(&mut self, rhs: UBig) {
        *self = mem::take(self) | rhs;
    }
}

impl BitOrAssign<&UBig> for UBig {
    #[inline]
    fn bitor_assign(&mut self, rhs: &UBig) {
        *self = mem::take(self) | rhs;
    }
}

impl BitXor<UBig> for UBig {
    type Output = UBig;

    #[inline]
    fn bitxor(self, rhs: UBig) -> UBig {
        match (self.into_repr(), rhs.into_repr()) {
            (Small(dword0), Small(dword1)) => UBig::from(dword0 ^ dword1),
            (Small(dword0), Large(buffer1)) => ubig::bitxor_large_dword(buffer1, dword0),
            (Large(buffer0), Small(dword1)) => ubig::bitxor_large_dword(buffer0, dword1),
            (Large(buffer0), Large(buffer1)) => {
                if buffer0.len() >= buffer1.len() {
                    ubig::bitxor_large(buffer0, &buffer1)
                } else {
                    ubig::bitxor_large(buffer1, &buffer0)
                }
            }
        }
    }
}

impl BitXor<&UBig> for UBig {
    type Output = UBig;

    #[inline]
    fn bitxor(self, rhs: &UBig) -> UBig {
        match (self.into_repr(), rhs.repr()) {
            (Small(dword0), RefSmall(dword1)) => UBig::from(dword0 ^ dword1),
            (Small(dword0), RefLarge(buffer1)) => ubig::bitxor_large_dword(buffer1.into(), dword0),
            (Large(buffer0), RefSmall(dword1)) => ubig::bitxor_large_dword(buffer0, dword1),
            (Large(buffer0), RefLarge(buffer1)) => ubig::bitxor_large(buffer0, buffer1),
        }
    }
}

impl BitXor<UBig> for &UBig {
    type Output = UBig;

    #[inline]
    fn bitxor(self, rhs: UBig) -> UBig {
        rhs.bitxor(self)
    }
}

impl BitXor<&UBig> for &UBig {
    type Output = UBig;

    #[inline]
    fn bitxor(self, rhs: &UBig) -> UBig {
        match (self.repr(), rhs.repr()) {
            (RefSmall(dword0), RefSmall(dword1)) => UBig::from(dword0 ^ dword1),
            (RefSmall(dword0), RefLarge(buffer1)) => {
                ubig::bitxor_large_dword(buffer1.into(), dword0)
            }
            (RefLarge(buffer0), RefSmall(dword1)) => {
                ubig::bitxor_large_dword(buffer0.into(), dword1)
            }
            (RefLarge(buffer0), RefLarge(buffer1)) => {
                if buffer0.len() >= buffer1.len() {
                    ubig::bitxor_large(buffer0.into(), buffer1)
                } else {
                    ubig::bitxor_large(buffer1.into(), buffer0)
                }
            }
        }
    }
}

impl BitXorAssign<UBig> for UBig {
    #[inline]
    fn bitxor_assign(&mut self, rhs: UBig) {
        *self = mem::take(self) ^ rhs;
    }
}

impl BitXorAssign<&UBig> for UBig {
    #[inline]
    fn bitxor_assign(&mut self, rhs: &UBig) {
        *self = mem::take(self) ^ rhs;
    }
}

impl AndNot<UBig> for UBig {
    type Output = UBig;

    #[inline]
    fn and_not(self, rhs: UBig) -> UBig {
        match (self.into_repr(), rhs.into_repr()) {
            (Small(dword0), Small(dword1)) => UBig::from(dword0 & !dword1),
            (Small(dword0), Large(buffer1)) => UBig::from(dword0 & !buffer1.front_dword()),
            (Large(buffer0), Small(dword1)) => ubig::and_not_large_dword(buffer0, dword1),
            (Large(buffer0), Large(buffer1)) => ubig::and_not_large(buffer0, &buffer1),
        }
    }
}

impl AndNot<&UBig> for UBig {
    type Output = UBig;

    #[inline]
    fn and_not(self, rhs: &UBig) -> UBig {
        match (self.into_repr(), rhs.repr()) {
            (Small(dword0), RefSmall(dword1)) => UBig::from(dword0 & !dword1),
            (Small(dword0), RefLarge(buffer1)) => UBig::from(dword0 & !front_dword(buffer1)),
            (Large(buffer0), RefSmall(dword1)) => ubig::and_not_large_dword(buffer0, dword1),
            (Large(buffer0), RefLarge(buffer1)) => ubig::and_not_large(buffer0, buffer1),
        }
    }
}

impl AndNot<UBig> for &UBig {
    type Output = UBig;

    #[inline]
    fn and_not(self, rhs: UBig) -> UBig {
        match (self.repr(), rhs.into_repr()) {
            (RefSmall(dword0), Small(dword1)) => UBig::from(dword0 & !dword1),
            (RefSmall(dword0), Large(buffer1)) => UBig::from(dword0 & !buffer1.front_dword()),
            (RefLarge(buffer0), Small(dword1)) => ubig::and_not_large_dword(buffer0.into(), dword1),
            (RefLarge(buffer0), Large(buffer1)) => ubig::and_not_large(buffer0.into(), &buffer1),
        }
    }
}

impl AndNot<&UBig> for &UBig {
    type Output = UBig;

    #[inline]
    fn and_not(self, rhs: &UBig) -> UBig {
        match (self.repr(), rhs.repr()) {
            (RefSmall(dword0), RefSmall(dword1)) => UBig::from(dword0 & !dword1),
            (RefSmall(dword0), RefLarge(buffer1)) => UBig::from(dword0 & !front_dword(buffer1)),
            (RefLarge(buffer0), RefSmall(dword1)) => {
                ubig::and_not_large_dword(buffer0.into(), dword1)
            }
            (RefLarge(buffer0), RefLarge(buffer1)) => ubig::and_not_large(buffer0.into(), buffer1),
        }
    }
}

mod ubig {
    use super::*;

    pub(crate) fn with_bit_dword_spilled(dword: DoubleWord, n: usize) -> UBig {
        debug_assert!(n >= DWORD_BITS_USIZE);
        let idx = n / WORD_BITS_USIZE;
        let mut buffer = Buffer::allocate(idx + 1);
        let (lo, hi) = split_dword(dword);
        buffer.push(lo);
        buffer.push(hi);
        buffer.push_zeros(idx - 2);
        buffer.push(1 << (n % WORD_BITS_USIZE));
        buffer.into()
    }

    pub(crate) fn with_bit_large(mut buffer: Buffer, n: usize) -> UBig {
        let idx = n / WORD_BITS_USIZE;
        if idx < buffer.len() {
            buffer[idx] |= 1 << (n % WORD_BITS_USIZE);
        } else {
            buffer.ensure_capacity(idx + 1);
            buffer.push_zeros(idx - buffer.len());
            buffer.push(1 << (n % WORD_BITS_USIZE));
        }
        buffer.into()
    }

    pub(crate) fn next_power_of_two_large(mut buffer: Buffer) -> UBig {
        debug_assert!(*buffer.last().unwrap() != 0);

        let n = buffer.len();
        let mut iter = buffer[..n - 1].iter_mut().skip_while(|x| **x == 0);

        let carry = match iter.next() {
            None => 0,
            Some(x) => {
                *x = 0;
                for x in iter {
                    *x = 0;
                }
                1
            }
        };

        let last = buffer.last_mut().unwrap();
        match last
            .checked_add(carry)
            .and_then(|x| x.checked_next_power_of_two())
        {
            Some(p) => *last = p,
            None => {
                *last = 0;
                buffer.ensure_capacity(n + 1);
                buffer.push(1);
            }
        }

        buffer.into()
    }

    pub(crate) fn bitand_large(mut buffer: Buffer, rhs: &[Word]) -> UBig {
        if buffer.len() > rhs.len() {
            buffer.truncate(rhs.len());
        }
        for (x, y) in buffer.iter_mut().zip(rhs.iter()) {
            *x &= *y;
        }
        buffer.into()
    }

    pub(crate) fn bitor_large_dword(mut buffer: Buffer, rhs: DoubleWord) -> UBig {
        debug_assert!(buffer.len() >= 2);

        let (lo, hi) = split_dword(rhs);
        unsafe {
            *buffer.get_unchecked_mut(0) |= lo;
            *buffer.get_unchecked_mut(1) |= hi;
        }
        buffer.into()
    }

    pub(crate) fn bitor_large(mut buffer: Buffer, rhs: &[Word]) -> UBig {
        for (x, y) in buffer.iter_mut().zip(rhs.iter()) {
            *x |= *y;
        }
        if rhs.len() > buffer.len() {
            buffer.ensure_capacity(rhs.len());
            buffer.push_slice(&rhs[buffer.len()..]);
        }
        buffer.into()
    }

    pub(crate) fn bitxor_large_dword(mut buffer: Buffer, rhs: DoubleWord) -> UBig {
        debug_assert!(buffer.len() >= 2);

        let (lo, hi) = split_dword(rhs);
        unsafe {
            *buffer.get_unchecked_mut(0) ^= lo;
            *buffer.get_unchecked_mut(1) ^= hi;
        }
        buffer.into()
    }

    pub(crate) fn bitxor_large(mut buffer: Buffer, rhs: &[Word]) -> UBig {
        for (x, y) in buffer.iter_mut().zip(rhs.iter()) {
            *x ^= *y;
        }
        if rhs.len() > buffer.len() {
            buffer.ensure_capacity(rhs.len());
            buffer.push_slice(&rhs[buffer.len()..]);
        }
        buffer.into()
    }

    pub(crate) fn and_not_large_dword(mut buffer: Buffer, rhs: DoubleWord) -> UBig {
        debug_assert!(buffer.len() >= 2);

        let (lo, hi) = split_dword(rhs);
        unsafe {
            *buffer.get_unchecked_mut(0) &= !lo;
            *buffer.get_unchecked_mut(1) &= !hi;
        }
        buffer.into()
    }

    pub(crate) fn and_not_large(mut buffer: Buffer, rhs: &[Word]) -> UBig {
        for (x, y) in buffer.iter_mut().zip(rhs.iter()) {
            *x &= !*y;
        }
        buffer.into()
    }
}

impl Not for IBig {
    type Output = IBig;

    #[inline]
    fn not(self) -> IBig {
        -(self + IBig::from(1u8))
    }
}

impl Not for &IBig {
    type Output = IBig;

    #[inline]
    fn not(self) -> IBig {
        -(self + IBig::from(1u8))
    }
}

// TODO: implement ops in IBig with functions on repr instead of using unsigned_abs(), and try
// to optimize intermediate operations.

impl BitAnd<IBig> for IBig {
    type Output = IBig;

    #[inline]
    fn bitand(self, rhs: IBig) -> IBig {
        match (self.sign(), rhs.sign()) {
            (Positive, Positive) => IBig::from(self.unsigned_abs() & rhs.unsigned_abs()),
            (Positive, Negative) => IBig::from(self.unsigned_abs().and_not((!rhs).unsigned_abs())),
            (Negative, Positive) => IBig::from(rhs.unsigned_abs().and_not((!self).unsigned_abs())),
            (Negative, Negative) => !IBig::from((!self).unsigned_abs() | (!rhs).unsigned_abs()),
        }
    }
}

impl BitAnd<&IBig> for IBig {
    type Output = IBig;

    #[inline]
    fn bitand(self, rhs: &IBig) -> IBig {
        match (self.sign(), rhs.sign()) {
            (Positive, Positive) => IBig::from(self.unsigned_abs() & rhs.unsigned_abs()),
            (Positive, Negative) => IBig::from(self.unsigned_abs().and_not((!rhs).unsigned_abs())),
            (Negative, Positive) => IBig::from(rhs.unsigned_abs().and_not((!self).unsigned_abs())),
            (Negative, Negative) => !IBig::from((!self).unsigned_abs() | (!rhs).unsigned_abs()),
        }
    }
}

impl BitAnd<IBig> for &IBig {
    type Output = IBig;

    #[inline]
    fn bitand(self, rhs: IBig) -> IBig {
        rhs.bitand(self)
    }
}

impl BitAnd<&IBig> for &IBig {
    type Output = IBig;

    #[inline]
    fn bitand(self, rhs: &IBig) -> IBig {
        match (self.sign(), rhs.sign()) {
            (Positive, Positive) => IBig::from(self.unsigned_abs() & rhs.unsigned_abs()),
            (Positive, Negative) => IBig::from(self.unsigned_abs().and_not((!rhs).unsigned_abs())),
            (Negative, Positive) => IBig::from(rhs.unsigned_abs().and_not((!self).unsigned_abs())),
            (Negative, Negative) => !IBig::from((!self).unsigned_abs() | (!rhs).unsigned_abs()),
        }
    }
}

impl BitAndAssign<IBig> for IBig {
    #[inline]
    fn bitand_assign(&mut self, rhs: IBig) {
        *self = mem::take(self) & rhs;
    }
}

impl BitAndAssign<&IBig> for IBig {
    #[inline]
    fn bitand_assign(&mut self, rhs: &IBig) {
        *self = mem::take(self) & rhs;
    }
}

impl BitOr<IBig> for IBig {
    type Output = IBig;

    #[inline]
    fn bitor(self, rhs: IBig) -> IBig {
        match (self.sign(), rhs.sign()) {
            (Positive, Positive) => IBig::from(self.unsigned_abs() | rhs.unsigned_abs()),
            (Positive, Negative) => !IBig::from((!rhs).unsigned_abs().and_not(self.unsigned_abs())),
            (Negative, Positive) => !IBig::from((!self).unsigned_abs().and_not(rhs.unsigned_abs())),
            (Negative, Negative) => !IBig::from((!self).unsigned_abs() & (!rhs).unsigned_abs()),
        }
    }
}

impl BitOr<&IBig> for IBig {
    type Output = IBig;

    #[inline]
    fn bitor(self, rhs: &IBig) -> IBig {
        match (self.sign(), rhs.sign()) {
            (Positive, Positive) => IBig::from(self.unsigned_abs() | rhs.unsigned_abs()),
            (Positive, Negative) => !IBig::from((!rhs).unsigned_abs().and_not(self.unsigned_abs())),
            (Negative, Positive) => !IBig::from((!self).unsigned_abs().and_not(rhs.unsigned_abs())),
            (Negative, Negative) => !IBig::from((!self).unsigned_abs() & (!rhs).unsigned_abs()),
        }
    }
}

impl BitOr<IBig> for &IBig {
    type Output = IBig;

    #[inline]
    fn bitor(self, rhs: IBig) -> IBig {
        rhs.bitor(self)
    }
}

impl BitOr<&IBig> for &IBig {
    type Output = IBig;

    #[inline]
    fn bitor(self, rhs: &IBig) -> IBig {
        match (self.sign(), rhs.sign()) {
            (Positive, Positive) => IBig::from(self.unsigned_abs() | rhs.unsigned_abs()),
            (Positive, Negative) => !IBig::from((!rhs).unsigned_abs().and_not(self.unsigned_abs())),
            (Negative, Positive) => !IBig::from((!self).unsigned_abs().and_not(rhs.unsigned_abs())),
            (Negative, Negative) => !IBig::from((!self).unsigned_abs() & (!rhs).unsigned_abs()),
        }
    }
}

impl BitOrAssign<IBig> for IBig {
    #[inline]
    fn bitor_assign(&mut self, rhs: IBig) {
        *self = mem::take(self) | rhs;
    }
}

impl BitOrAssign<&IBig> for IBig {
    #[inline]
    fn bitor_assign(&mut self, rhs: &IBig) {
        *self = mem::take(self) | rhs;
    }
}

impl BitXor<IBig> for IBig {
    type Output = IBig;

    #[inline]
    fn bitxor(self, rhs: IBig) -> IBig {
        match (self.sign(), rhs.sign()) {
            (Positive, Positive) => IBig::from(self.unsigned_abs() ^ rhs.unsigned_abs()),
            (Positive, Negative) => !IBig::from(self.unsigned_abs() ^ (!rhs).unsigned_abs()),
            (Negative, Positive) => !IBig::from((!self).unsigned_abs() ^ rhs.unsigned_abs()),
            (Negative, Negative) => IBig::from((!self).unsigned_abs() ^ (!rhs).unsigned_abs()),
        }
    }
}

impl BitXor<&IBig> for IBig {
    type Output = IBig;

    #[inline]
    fn bitxor(self, rhs: &IBig) -> IBig {
        match (self.sign(), rhs.sign()) {
            (Positive, Positive) => IBig::from(self.unsigned_abs() ^ rhs.unsigned_abs()),
            (Positive, Negative) => !IBig::from(self.unsigned_abs() ^ (!rhs).unsigned_abs()),
            (Negative, Positive) => !IBig::from((!self).unsigned_abs() ^ rhs.unsigned_abs()),
            (Negative, Negative) => IBig::from((!self).unsigned_abs() ^ (!rhs).unsigned_abs()),
        }
    }
}

impl BitXor<IBig> for &IBig {
    type Output = IBig;

    #[inline]
    fn bitxor(self, rhs: IBig) -> IBig {
        rhs.bitxor(self)
    }
}

impl BitXor<&IBig> for &IBig {
    type Output = IBig;

    #[inline]
    fn bitxor(self, rhs: &IBig) -> IBig {
        match (self.sign(), rhs.sign()) {
            (Positive, Positive) => IBig::from(self.unsigned_abs() ^ rhs.unsigned_abs()),
            (Positive, Negative) => !IBig::from(self.unsigned_abs() ^ (!rhs).unsigned_abs()),
            (Negative, Positive) => !IBig::from((!self).unsigned_abs() ^ rhs.unsigned_abs()),
            (Negative, Negative) => IBig::from((!self).unsigned_abs() ^ (!rhs).unsigned_abs()),
        }
    }
}

impl BitXorAssign<IBig> for IBig {
    #[inline]
    fn bitxor_assign(&mut self, rhs: IBig) {
        *self = mem::take(self) ^ rhs;
    }
}

impl BitXorAssign<&IBig> for IBig {
    #[inline]
    fn bitxor_assign(&mut self, rhs: &IBig) {
        *self = mem::take(self) ^ rhs;
    }
}

impl AndNot<IBig> for IBig {
    type Output = IBig;

    #[inline]
    fn and_not(self, rhs: IBig) -> IBig {
        match (self.sign(), rhs.sign()) {
            (Positive, Positive) => IBig::from(self.unsigned_abs().and_not(rhs.unsigned_abs())),
            (Positive, Negative) => IBig::from(self.unsigned_abs() & (!rhs).unsigned_abs()),
            (Negative, Positive) => !IBig::from((!self).unsigned_abs() | rhs.unsigned_abs()),
            (Negative, Negative) => {
                IBig::from((!rhs).unsigned_abs().and_not((!self).unsigned_abs()))
            }
        }
    }
}

impl AndNot<&IBig> for IBig {
    type Output = IBig;

    #[inline]
    fn and_not(self, rhs: &IBig) -> IBig {
        match (self.sign(), rhs.sign()) {
            (Positive, Positive) => IBig::from(self.unsigned_abs().and_not(rhs.unsigned_abs())),
            (Positive, Negative) => IBig::from(self.unsigned_abs() & (!rhs).unsigned_abs()),
            (Negative, Positive) => !IBig::from((!self).unsigned_abs() | rhs.unsigned_abs()),
            (Negative, Negative) => {
                IBig::from((!rhs).unsigned_abs().and_not((!self).unsigned_abs()))
            }
        }
    }
}

impl AndNot<IBig> for &IBig {
    type Output = IBig;

    #[inline]
    fn and_not(self, rhs: IBig) -> IBig {
        match (self.sign(), rhs.sign()) {
            (Positive, Positive) => IBig::from(self.unsigned_abs().and_not(rhs.unsigned_abs())),
            (Positive, Negative) => IBig::from(self.unsigned_abs() & (!rhs).unsigned_abs()),
            (Negative, Positive) => !IBig::from((!self).unsigned_abs() | rhs.unsigned_abs()),
            (Negative, Negative) => {
                IBig::from((!rhs).unsigned_abs().and_not((!self).unsigned_abs()))
            }
        }
    }
}

impl AndNot<&IBig> for &IBig {
    type Output = IBig;

    #[inline]
    fn and_not(self, rhs: &IBig) -> IBig {
        match (self.sign(), rhs.sign()) {
            (Positive, Positive) => IBig::from(self.unsigned_abs().and_not(rhs.unsigned_abs())),
            (Positive, Negative) => IBig::from(self.unsigned_abs() & (!rhs).unsigned_abs()),
            (Negative, Positive) => !IBig::from((!self).unsigned_abs() | rhs.unsigned_abs()),
            (Negative, Negative) => {
                IBig::from((!rhs).unsigned_abs().and_not((!self).unsigned_abs()))
            }
        }
    }
}

macro_rules! impl_bit_ops_ubig_unsigned {
    ($t:ty) => {
        impl BitAnd<$t> for UBig {
            type Output = $t;

            #[inline]
            fn bitand(self, rhs: $t) -> $t {
                self.bitand(UBig::from_unsigned(rhs))
                    .try_to_unsigned()
                    .unwrap()
            }
        }

        impl BitAnd<$t> for &UBig {
            type Output = $t;

            #[inline]
            fn bitand(self, rhs: $t) -> $t {
                self.bitand(UBig::from_unsigned(rhs))
                    .try_to_unsigned()
                    .unwrap()
            }
        }

        helper_macros::forward_binop_second_arg_by_value!(impl BitAnd<$t> for UBig, bitand);
        helper_macros::forward_binop_swap_args!(impl BitAnd<UBig> for $t, bitand);

        impl BitAndAssign<$t> for UBig {
            #[inline]
            fn bitand_assign(&mut self, rhs: $t) {
                self.bitand_assign(UBig::from_unsigned(rhs))
            }
        }

        helper_macros::forward_binop_assign_arg_by_value!(impl BitAndAssign<$t> for UBig, bitand_assign);

        impl BitOr<$t> for UBig {
            type Output = UBig;

            #[inline]
            fn bitor(self, rhs: $t) -> UBig {
                self.bitor(UBig::from_unsigned(rhs))
            }
        }

        impl BitOr<$t> for &UBig {
            type Output = UBig;

            #[inline]
            fn bitor(self, rhs: $t) -> UBig {
                self.bitor(UBig::from_unsigned(rhs))
            }
        }

        helper_macros::forward_binop_second_arg_by_value!(impl BitOr<$t> for UBig, bitor);
        helper_macros::forward_binop_swap_args!(impl BitOr<UBig> for $t, bitor);

        impl BitOrAssign<$t> for UBig {
            #[inline]
            fn bitor_assign(&mut self, rhs: $t) {
                self.bitor_assign(UBig::from_unsigned(rhs))
            }
        }

        helper_macros::forward_binop_assign_arg_by_value!(impl BitOrAssign<$t> for UBig, bitor_assign);

        impl BitXor<$t> for UBig {
            type Output = UBig;

            #[inline]
            fn bitxor(self, rhs: $t) -> UBig {
                self.bitxor(UBig::from_unsigned(rhs))
            }
        }

        impl BitXor<$t> for &UBig {
            type Output = UBig;

            #[inline]
            fn bitxor(self, rhs: $t) -> UBig {
                self.bitxor(UBig::from_unsigned(rhs))
            }
        }

        helper_macros::forward_binop_second_arg_by_value!(impl BitXor<$t> for UBig, bitxor);
        helper_macros::forward_binop_swap_args!(impl BitXor<UBig> for $t, bitxor);

        impl BitXorAssign<$t> for UBig {
            #[inline]
            fn bitxor_assign(&mut self, rhs: $t) {
                self.bitxor_assign(UBig::from_unsigned(rhs))
            }
        }

        helper_macros::forward_binop_assign_arg_by_value!(impl BitXorAssign<$t> for UBig, bitxor_assign);

        impl AndNot<$t> for UBig {
            type Output = UBig;

            #[inline]
            fn and_not(self, rhs: $t) -> UBig {
                self.and_not(UBig::from_unsigned(rhs))
            }
        }

        impl AndNot<$t> for &UBig {
            type Output = UBig;

            #[inline]
            fn and_not(self, rhs: $t) -> UBig {
                self.and_not(UBig::from_unsigned(rhs))
            }
        }

        helper_macros::forward_binop_second_arg_by_value!(impl AndNot<$t> for UBig, and_not);
    };
}

impl_bit_ops_ubig_unsigned!(u8);
impl_bit_ops_ubig_unsigned!(u16);
impl_bit_ops_ubig_unsigned!(u32);
impl_bit_ops_ubig_unsigned!(u64);
impl_bit_ops_ubig_unsigned!(u128);
impl_bit_ops_ubig_unsigned!(usize);

macro_rules! impl_bit_ops_ubig_signed {
    ($t:ty) => {
        impl BitAnd<$t> for UBig {
            type Output = UBig;

            #[inline]
            fn bitand(self, rhs: $t) -> UBig {
                UBig::from_ibig(IBig::from(self) & IBig::from_signed(rhs))
            }
        }

        impl BitAnd<$t> for &UBig {
            type Output = UBig;

            #[inline]
            fn bitand(self, rhs: $t) -> UBig {
                // Avoid big copy if rhs positive.
                let rhs_signed = IBig::from_signed(rhs);
                match rhs_signed.sign() {
                    Positive => self & rhs_signed.unsigned_abs(),
                    Negative => UBig::from_ibig(IBig::from(self) & rhs_signed),
                }
            }
        }

        helper_macros::forward_binop_second_arg_by_value!(impl BitAnd<$t> for UBig, bitand);
        helper_macros::forward_binop_swap_args!(impl BitAnd<UBig> for $t, bitand);

        impl BitAndAssign<$t> for UBig {
            #[inline]
            fn bitand_assign(&mut self, rhs: $t) {
                *self = mem::take(self).bitand(rhs)
            }
        }

        helper_macros::forward_binop_assign_arg_by_value!(impl BitAndAssign<$t> for UBig, bitand_assign);

        impl BitOr<$t> for UBig {
            type Output = UBig;

            #[inline]
            fn bitor(self, rhs: $t) -> UBig {
                UBig::from_ibig(IBig::from(self) | IBig::from_signed(rhs))
            }
        }

        impl BitOr<$t> for &UBig {
            type Output = UBig;

            #[inline]
            fn bitor(self, rhs: $t) -> UBig {
                UBig::from_ibig(IBig::from(self) | IBig::from_signed(rhs))
            }
        }

        helper_macros::forward_binop_second_arg_by_value!(impl BitOr<$t> for UBig, bitor);
        helper_macros::forward_binop_swap_args!(impl BitOr<UBig> for $t, bitor);

        impl BitOrAssign<$t> for UBig {
            #[inline]
            fn bitor_assign(&mut self, rhs: $t) {
                *self = mem::take(self).bitor(rhs)
            }
        }

        helper_macros::forward_binop_assign_arg_by_value!(impl BitOrAssign<$t> for UBig, bitor_assign);

        impl BitXor<$t> for UBig {
            type Output = UBig;

            #[inline]
            fn bitxor(self, rhs: $t) -> UBig {
                UBig::from_ibig(IBig::from(self) ^ IBig::from_signed(rhs))
            }
        }

        impl BitXor<$t> for &UBig {
            type Output = UBig;

            #[inline]
            fn bitxor(self, rhs: $t) -> UBig {
                UBig::from_ibig(IBig::from(self) ^ IBig::from_signed(rhs))
            }
        }

        helper_macros::forward_binop_second_arg_by_value!(impl BitXor<$t> for UBig, bitxor);
        helper_macros::forward_binop_swap_args!(impl BitXor<UBig> for $t, bitxor);

        impl BitXorAssign<$t> for UBig {
            #[inline]
            fn bitxor_assign(&mut self, rhs: $t) {
                *self = mem::take(self).bitxor(rhs)
            }
        }

        helper_macros::forward_binop_assign_arg_by_value!(impl BitXorAssign<$t> for UBig, bitxor_assign);

        impl AndNot<$t> for UBig {
            type Output = UBig;

            #[inline]
            fn and_not(self, rhs: $t) -> UBig {
                UBig::from_ibig(IBig::from(self).and_not(IBig::from_signed(rhs)))
            }
        }

        impl AndNot<$t> for &UBig {
            type Output = UBig;

            #[inline]
            fn and_not(self, rhs: $t) -> UBig {
                UBig::from_ibig(IBig::from(self).and_not(IBig::from_signed(rhs)))
            }
        }

        helper_macros::forward_binop_second_arg_by_value!(impl AndNot<$t> for UBig, and_not);
    };
}

impl_bit_ops_ubig_signed!(i8);
impl_bit_ops_ubig_signed!(i16);
impl_bit_ops_ubig_signed!(i32);
impl_bit_ops_ubig_signed!(i64);
impl_bit_ops_ubig_signed!(i128);
impl_bit_ops_ubig_signed!(isize);

macro_rules! impl_bit_ops_ibig_unsigned {
    ($t:ty) => {
        impl BitAnd<$t> for IBig {
            type Output = $t;

            #[inline]
            fn bitand(self, rhs: $t) -> $t {
                self.bitand(IBig::from_unsigned(rhs))
                    .try_to_unsigned()
                    .unwrap()
            }
        }

        impl BitAnd<$t> for &IBig {
            type Output = $t;

            #[inline]
            fn bitand(self, rhs: $t) -> $t {
                self.bitand(IBig::from_unsigned(rhs))
                    .try_to_unsigned()
                    .unwrap()
            }
        }
    };
}

impl_bit_ops_ibig_unsigned!(u8);
impl_bit_ops_ibig_unsigned!(u16);
impl_bit_ops_ibig_unsigned!(u32);
impl_bit_ops_ibig_unsigned!(u64);
impl_bit_ops_ibig_unsigned!(u128);
impl_bit_ops_ibig_unsigned!(usize);

macro_rules! impl_bit_ops_ibig_signed {
    ($t:ty) => {
        impl BitAnd<$t> for IBig {
            type Output = IBig;

            #[inline]
            fn bitand(self, rhs: $t) -> IBig {
                self.bitand(IBig::from_signed(rhs))
            }
        }

        impl BitAnd<$t> for &IBig {
            type Output = IBig;

            #[inline]
            fn bitand(self, rhs: $t) -> IBig {
                self.bitand(IBig::from_signed(rhs))
            }
        }
    };
}

impl_bit_ops_ibig_signed!(i8);
impl_bit_ops_ibig_signed!(i16);
impl_bit_ops_ibig_signed!(i32);
impl_bit_ops_ibig_signed!(i64);
impl_bit_ops_ibig_signed!(i128);
impl_bit_ops_ibig_signed!(isize);

macro_rules! impl_bit_ops_ibig_primitive {
    ($t:ty) => {
        helper_macros::forward_binop_second_arg_by_value!(impl BitAnd<$t> for IBig, bitand);
        helper_macros::forward_binop_swap_args!(impl BitAnd<IBig> for $t, bitand);

        impl BitAndAssign<$t> for IBig {
            #[inline]
            fn bitand_assign(&mut self, rhs: $t) {
                self.bitand_assign(IBig::from(rhs))
            }
        }

        helper_macros::forward_binop_assign_arg_by_value!(impl BitAndAssign<$t> for IBig, bitand_assign);

        impl BitOr<$t> for IBig {
            type Output = IBig;

            #[inline]
            fn bitor(self, rhs: $t) -> IBig {
                self.bitor(IBig::from(rhs))
            }
        }

        impl BitOr<$t> for &IBig {
            type Output = IBig;

            #[inline]
            fn bitor(self, rhs: $t) -> IBig {
                self.bitor(IBig::from(rhs))
            }
        }

        helper_macros::forward_binop_second_arg_by_value!(impl BitOr<$t> for IBig, bitor);
        helper_macros::forward_binop_swap_args!(impl BitOr<IBig> for $t, bitor);

        impl BitOrAssign<$t> for IBig {
            #[inline]
            fn bitor_assign(&mut self, rhs: $t) {
                self.bitor_assign(IBig::from(rhs))
            }
        }

        helper_macros::forward_binop_assign_arg_by_value!(impl BitOrAssign<$t> for IBig, bitor_assign);

        impl BitXor<$t> for IBig {
            type Output = IBig;

            #[inline]
            fn bitxor(self, rhs: $t) -> IBig {
                self.bitxor(IBig::from(rhs))
            }
        }

        impl BitXor<$t> for &IBig {
            type Output = IBig;

            #[inline]
            fn bitxor(self, rhs: $t) -> IBig {
                self.bitxor(IBig::from(rhs))
            }
        }

        helper_macros::forward_binop_second_arg_by_value!(impl BitXor<$t> for IBig, bitxor);
        helper_macros::forward_binop_swap_args!(impl BitXor<IBig> for $t, bitxor);

        impl BitXorAssign<$t> for IBig {
            #[inline]
            fn bitxor_assign(&mut self, rhs: $t) {
                self.bitxor_assign(IBig::from(rhs))
            }
        }

        helper_macros::forward_binop_assign_arg_by_value!(impl BitXorAssign<$t> for IBig, bitxor_assign);

        impl AndNot<$t> for IBig {
            type Output = IBig;

            #[inline]
            fn and_not(self, rhs: $t) -> IBig {
                self.and_not(IBig::from(rhs))
            }
        }

        impl AndNot<$t> for &IBig {
            type Output = IBig;

            #[inline]
            fn and_not(self, rhs: $t) -> IBig {
                self.and_not(IBig::from(rhs))
            }
        }

        helper_macros::forward_binop_second_arg_by_value!(impl AndNot<$t> for IBig, and_not);
    };
}

impl_bit_ops_ibig_primitive!(u8);
impl_bit_ops_ibig_primitive!(u16);
impl_bit_ops_ibig_primitive!(u32);
impl_bit_ops_ibig_primitive!(u64);
impl_bit_ops_ibig_primitive!(u128);
impl_bit_ops_ibig_primitive!(usize);
impl_bit_ops_ibig_primitive!(i8);
impl_bit_ops_ibig_primitive!(i16);
impl_bit_ops_ibig_primitive!(i32);
impl_bit_ops_ibig_primitive!(i64);
impl_bit_ops_ibig_primitive!(i128);
impl_bit_ops_ibig_primitive!(isize);
