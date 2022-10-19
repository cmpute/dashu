//! Bitwise operators.

use crate::{arch::word::Word, helper_macros, ibig::IBig, ops::PowerOfTwo, ubig::UBig, Sign::*};
use core::{
    mem,
    ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not},
};

impl UBig {
    /// Returns true if the `n`-th bit is set, n starts from 0.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::UBig;
    /// assert_eq!(UBig::from(0b10010u16).bit(1), true);
    /// assert_eq!(UBig::from(0b10010u16).bit(3), false);
    /// assert_eq!(UBig::from(0b10010u16).bit(100), false);
    /// ```
    #[inline]
    pub fn bit(&self, n: usize) -> bool {
        self.repr().bit(n)
    }

    /// Set the `n`-th bit, n starts from 0.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::UBig;
    ///
    /// let mut a = UBig::from(0b100u8);
    /// a.set_bit(0);
    /// assert_eq!(a, UBig::from(0b101u8));
    /// a.set_bit(10);
    /// assert_eq!(a, UBig::from(0b10000000101u16));
    /// ```
    #[inline]
    pub fn set_bit(&mut self, n: usize) {
        self.0 = mem::take(self).into_repr().set_bit(n);
    }

    /// Clear the `n`-th bit, n starts from 0.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::UBig;
    /// let mut a = UBig::from(0b101u8);
    /// a.clear_bit(0);
    /// assert_eq!(a, UBig::from(0b100u8));
    /// ```
    #[inline]
    pub fn clear_bit(&mut self, n: usize) {
        self.0 = mem::take(self).into_repr().clear_bit(n);
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
    /// # use dashu_int::UBig;
    /// assert_eq!(UBig::from(17u8).trailing_zeros(), Some(0));
    /// assert_eq!(UBig::from(48u8).trailing_zeros(), Some(4));
    /// assert_eq!(UBig::from(0b101000000u16).trailing_zeros(), Some(6));
    /// assert_eq!(UBig::ZERO.trailing_zeros(), None);
    /// ```
    // TODO(v0.3): expose this function through BitTest trait
    #[inline]
    pub fn trailing_zeros(&self) -> Option<usize> {
        self.repr().trailing_zeros()
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
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::UBig;
    /// assert_eq!(UBig::from(17u8).bit_len(), 5);
    /// assert_eq!(UBig::from(0b101000000u16).bit_len(), 9);
    /// assert_eq!(UBig::ZERO.bit_len(), 0);
    /// let x = UBig::from(0x90ffff3450897234u64);
    /// assert_eq!(x.bit_len(), x.in_radix(2).to_string().len());
    /// ```
    #[inline]
    pub fn bit_len(&self) -> usize {
        self.repr().bit_len()
    }

    /// Split this integer into low bits and high bits.
    ///
    /// Its returns are equal to `(self & ((1 << n) - 1), self >> n)`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::UBig;
    /// let (lo, hi) = UBig::from(0b10100011u8).split_bits(4);
    /// assert_eq!(hi, UBig::from(0b1010u8));
    /// assert_eq!(lo, UBig::from(0b0011u8));
    ///
    /// let x = UBig::from(0x90ffff3450897234u64);
    /// let (lo, hi) = x.clone().split_bits(21);
    /// assert_eq!(hi, (&x) >> 21);
    /// assert_eq!(lo, x & ((UBig::ONE << 21) - 1u8));
    /// ```
    #[inline]
    pub fn split_bits(self, n: usize) -> (UBig, UBig) {
        let (lo, hi) = self.into_repr().split_bits(n);
        (UBig(lo), UBig(hi))
    }

    /// Clear the high bits from (n+1)-th bit.
    ///
    /// This operation is equivalent to getting the lowest n bits on the integer
    /// i.e. `self &= ((1 << n) - 1)`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::UBig;
    /// let mut x = UBig::from(0b10100011u8);
    /// x.clear_high_bits(4);
    /// assert_eq!(x, UBig::from(0b0011u8));
    ///
    /// let mut x = UBig::from(0x90ffff3450897234u64);
    /// let lo = (&x) & ((UBig::ONE << 21) - 1u8);
    /// x.clear_high_bits(21);
    /// assert_eq!(x, lo);
    /// ```
    #[inline]
    pub fn clear_high_bits(&mut self, n: usize) {
        self.0 = mem::take(self).into_repr().clear_high_bits(n);
    }
}

helper_macros::forward_ubig_binop_to_repr!(impl BitAnd, bitand);
helper_macros::forward_ubig_binop_to_repr!(impl BitOr, bitor);
helper_macros::forward_ubig_binop_to_repr!(impl BitXor, bitxor);
helper_macros::forward_ubig_binop_to_repr!(impl AndNot, and_not);
helper_macros::impl_binop_assign_by_taking!(impl BitAndAssign<UBig> for UBig, bitand_assign, bitand);
helper_macros::impl_binop_assign_by_taking!(impl BitOrAssign<UBig> for UBig, bitor_assign, bitor);
helper_macros::impl_binop_assign_by_taking!(impl BitXorAssign<UBig> for UBig, bitxor_assign, bitxor);

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
    /// # use dashu_int::IBig;
    /// assert_eq!(IBig::from(17).trailing_zeros(), Some(0));
    /// assert_eq!(IBig::from(-48).trailing_zeros(), Some(4));
    /// assert_eq!(IBig::from(-0b101000000).trailing_zeros(), Some(6));
    /// assert_eq!(IBig::ZERO.trailing_zeros(), None);
    /// ```
    #[inline]
    pub fn trailing_zeros(&self) -> Option<usize> {
        self.as_sign_repr().1.trailing_zeros()
    }
}

impl PowerOfTwo for UBig {
    #[inline]
    fn is_power_of_two(&self) -> bool {
        self.repr().is_power_of_two()
    }

    #[inline]
    fn next_power_of_two(self) -> UBig {
        UBig(self.into_repr().next_power_of_two())
    }
}

/// Bitwise AND NOT operation. For internal use only, used for implementing
/// bit operations on IBig.
///
/// `x.and_not(y)` is equivalent to `x & !y` for primitive integers.
trait AndNot<Rhs = Self> {
    type Output;

    fn and_not(self, rhs: Rhs) -> Self::Output;
}

mod repr {
    use super::*;
    use crate::{
        arch::word::DoubleWord,
        buffer::Buffer,
        math::{self, ceil_div, ones_dword, ones_word},
        primitive::{lowest_dword, split_dword, DWORD_BITS_USIZE, WORD_BITS_USIZE},
        repr::{
            Repr,
            TypedRepr::{self, *},
            TypedReprRef::{self, *},
        },
        shift_ops,
    };

    impl<'a> TypedReprRef<'a> {
        #[inline]
        pub fn bit(self, n: usize) -> bool {
            match self {
                RefSmall(dword) => n < DWORD_BITS_USIZE && dword & 1 << n != 0,
                RefLarge(buffer) => {
                    let idx = n / WORD_BITS_USIZE;
                    idx < buffer.len() && buffer[idx] & 1 << (n % WORD_BITS_USIZE) != 0
                }
            }
        }

        #[inline]
        pub fn bit_len(self) -> usize {
            match self {
                RefSmall(dword) => math::bit_len(dword) as usize,
                RefLarge(words) => {
                    words.len() * WORD_BITS_USIZE - words.last().unwrap().leading_zeros() as usize
                }
            }
        }

        /// Check if low n-bits are not all zeros
        #[inline]
        pub fn are_low_bits_nonzero(self, n: usize) -> bool {
            match self {
                Self::RefSmall(dword) => are_dword_low_bits_nonzero(dword, n),
                Self::RefLarge(words) => are_slice_low_bits_nonzero(words, n),
            }
        }

        /// Check if the underlying number is a power of two
        #[inline]
        pub fn is_power_of_two(self) -> bool {
            match self {
                RefSmall(dword) => dword.is_power_of_two(),
                RefLarge(words) => {
                    words[..words.len() - 1].iter().all(|x| *x == 0)
                        && words.last().unwrap().is_power_of_two()
                }
            }
        }

        pub fn trailing_zeros(self) -> Option<usize> {
            match self {
                RefSmall(0) => None,
                RefSmall(dword) => Some(dword.trailing_zeros() as usize),
                RefLarge(words) => Some(trailing_zeros_large(words)),
            }
        }
    }

    impl TypedRepr {
        #[inline]
        pub fn next_power_of_two(self) -> Repr {
            match self {
                Small(dword) => match dword.checked_next_power_of_two() {
                    Some(p) => Repr::from_dword(p),
                    None => {
                        let mut buffer = Buffer::allocate(3);
                        buffer.push_zeros(2);
                        buffer.push(1);
                        Repr::from_buffer(buffer)
                    }
                },
                Large(buffer) => next_power_of_two_large(buffer),
            }
        }

        pub fn set_bit(self, n: usize) -> Repr {
            match self {
                Small(dword) => {
                    if n < DWORD_BITS_USIZE {
                        Repr::from_dword(dword | 1 << n)
                    } else {
                        with_bit_dword_spilled(dword, n)
                    }
                }
                Large(buffer) => with_bit_large(buffer, n),
            }
        }

        pub fn clear_bit(self, n: usize) -> Repr {
            match self {
                Small(dword) => {
                    if n < DWORD_BITS_USIZE {
                        Repr::from_dword(dword & !(1 << n))
                    } else {
                        Repr::from_dword(dword)
                    }
                }
                Large(mut buffer) => {
                    let idx = n / WORD_BITS_USIZE;
                    if idx < buffer.len() {
                        buffer[idx] &= !(1 << (n % WORD_BITS_USIZE));
                    }
                    Repr::from_buffer(buffer)
                }
            }
        }

        pub fn clear_high_bits(self, n: usize) -> Repr {
            match self {
                Small(dword) => {
                    if n < DWORD_BITS_USIZE {
                        Repr::from_dword(dword & ones_dword(n as u32))
                    } else {
                        Repr::from_dword(dword)
                    }
                }
                Large(buffer) => clear_high_bits_large(buffer, n),
            }
        }

        pub fn split_bits(self, n: usize) -> (Repr, Repr) {
            match self {
                Small(dword) => {
                    if n < DWORD_BITS_USIZE {
                        (
                            Repr::from_dword(dword & ones_dword(n as u32)),
                            Repr::from_dword(dword >> n),
                        )
                    } else {
                        (Repr::from_dword(dword), Repr::zero())
                    }
                }
                Large(buffer) => {
                    if n == 0 {
                        (Repr::zero(), Repr::from_buffer(buffer))
                    } else {
                        let hi = shift_ops::repr::shr_large_ref(&buffer, n);
                        let lo = clear_high_bits_large(buffer, n);
                        (lo, hi)
                    }
                }
            }
        }
    }

    #[inline]
    fn are_dword_low_bits_nonzero(dword: DoubleWord, n: usize) -> bool {
        let n = n.min(WORD_BITS_USIZE) as u32;
        dword & ones_dword(n) != 0
    }

    fn are_slice_low_bits_nonzero(words: &[Word], n: usize) -> bool {
        let n_words = n / WORD_BITS_USIZE;
        if n_words >= words.len() {
            true
        } else {
            let n_top = (n % WORD_BITS_USIZE) as u32;
            words[..n_words].iter().any(|x| *x != 0) || words[n_words] & ones_word(n_top) != 0
        }
    }

    fn next_power_of_two_large(mut buffer: Buffer) -> Repr {
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
                buffer.push_resizing(1);
            }
        }

        Repr::from_buffer(buffer)
    }

    fn with_bit_dword_spilled(dword: DoubleWord, n: usize) -> Repr {
        debug_assert!(n >= DWORD_BITS_USIZE);
        let idx = n / WORD_BITS_USIZE;
        let mut buffer = Buffer::allocate(idx + 1);
        let (lo, hi) = split_dword(dword);
        buffer.push(lo);
        buffer.push(hi);
        buffer.push_zeros(idx - 2);
        buffer.push(1 << (n % WORD_BITS_USIZE));
        Repr::from_buffer(buffer)
    }

    fn with_bit_large(mut buffer: Buffer, n: usize) -> Repr {
        let idx = n / WORD_BITS_USIZE;
        if idx < buffer.len() {
            buffer[idx] |= 1 << (n % WORD_BITS_USIZE);
        } else {
            buffer.ensure_capacity(idx + 1);
            buffer.push_zeros(idx - buffer.len());
            buffer.push(1 << (n % WORD_BITS_USIZE));
        }
        Repr::from_buffer(buffer)
    }

    /// Count the trailing zero bits in the words.
    /// Panics if the input is zero.
    #[inline]
    fn trailing_zeros_large(words: &[Word]) -> usize {
        let zero_words = words.iter().position(|&word| word != 0).unwrap();
        let zero_bits = words[zero_words].trailing_zeros() as usize;
        zero_words * WORD_BITS_USIZE + zero_bits
    }

    #[inline]
    fn clear_high_bits_large(mut buffer: Buffer, n: usize) -> Repr {
        let n_words = ceil_div(n, WORD_BITS_USIZE);
        if n_words > buffer.len() {
            Repr::from_buffer(buffer)
        } else {
            buffer.truncate(n_words);
            if let Some(last) = buffer.last_mut() {
                *last &= ones_word((n % WORD_BITS_USIZE) as u32);
            }
            Repr::from_buffer(buffer)
        }
    }

    impl BitAnd<TypedRepr> for TypedRepr {
        type Output = Repr;

        #[inline]
        fn bitand(self, rhs: TypedRepr) -> Repr {
            match (self, rhs) {
                (Small(dword0), Small(dword1)) => Repr::from_dword(dword0 & dword1),
                (Small(dword0), Large(buffer1)) => {
                    Repr::from_dword(dword0 & buffer1.lowest_dword())
                }
                (Large(buffer0), Small(dword1)) => {
                    Repr::from_dword(buffer0.lowest_dword() & dword1)
                }
                (Large(buffer0), Large(buffer1)) => {
                    if buffer0.len() <= buffer1.len() {
                        bitand_large(buffer0, &buffer1)
                    } else {
                        bitand_large(buffer1, &buffer0)
                    }
                }
            }
        }
    }

    impl<'r> BitAnd<TypedReprRef<'r>> for TypedRepr {
        type Output = Repr;

        #[inline]
        fn bitand(self, rhs: TypedReprRef) -> Repr {
            match (self, rhs) {
                (Small(dword0), RefSmall(dword1)) => Repr::from_dword(dword0 & dword1),
                (Small(dword0), RefLarge(buffer1)) => {
                    Repr::from_dword(dword0 & lowest_dword(buffer1))
                }
                (Large(buffer0), RefSmall(dword1)) => {
                    Repr::from_dword(buffer0.lowest_dword() & dword1)
                }
                (Large(buffer0), RefLarge(buffer1)) => bitand_large(buffer0, buffer1),
            }
        }
    }

    impl<'l> BitAnd<TypedRepr> for TypedReprRef<'l> {
        type Output = Repr;

        #[inline]
        fn bitand(self, rhs: TypedRepr) -> Repr {
            // bitand is commutative
            rhs.bitand(self)
        }
    }

    impl<'l, 'r> BitAnd<TypedReprRef<'r>> for TypedReprRef<'l> {
        type Output = Repr;

        #[inline]
        fn bitand(self, rhs: TypedReprRef) -> Repr {
            match (self, rhs) {
                (RefSmall(dword0), RefSmall(dword1)) => Repr::from_dword(dword0 & dword1),
                (RefSmall(dword0), RefLarge(buffer1)) => {
                    Repr::from_dword(dword0 & lowest_dword(buffer1))
                }
                (RefLarge(buffer0), RefSmall(dword1)) => {
                    Repr::from_dword(lowest_dword(buffer0) & dword1)
                }
                (RefLarge(buffer0), RefLarge(buffer1)) => {
                    if buffer0.len() <= buffer1.len() {
                        bitand_large(buffer0.into(), buffer1)
                    } else {
                        bitand_large(buffer1.into(), buffer0)
                    }
                }
            }
        }
    }

    fn bitand_large(mut buffer: Buffer, rhs: &[Word]) -> Repr {
        if buffer.len() > rhs.len() {
            buffer.truncate(rhs.len());
        }
        for (x, y) in buffer.iter_mut().zip(rhs.iter()) {
            *x &= *y;
        }
        Repr::from_buffer(buffer)
    }

    impl BitOr<TypedRepr> for TypedRepr {
        type Output = Repr;

        #[inline]
        fn bitor(self, rhs: TypedRepr) -> Repr {
            match (self, rhs) {
                (Small(dword0), Small(dword1)) => Repr::from_dword(dword0 | dword1),
                (Small(dword0), Large(buffer1)) => bitor_large_dword(buffer1, dword0),
                (Large(buffer0), Small(dword1)) => bitor_large_dword(buffer0, dword1),
                (Large(buffer0), Large(buffer1)) => {
                    if buffer0.len() >= buffer1.len() {
                        bitor_large(buffer0, &buffer1)
                    } else {
                        bitor_large(buffer1, &buffer0)
                    }
                }
            }
        }
    }

    impl<'r> BitOr<TypedReprRef<'r>> for TypedRepr {
        type Output = Repr;

        #[inline]
        fn bitor(self, rhs: TypedReprRef) -> Repr {
            match (self, rhs) {
                (Small(dword0), RefSmall(dword1)) => Repr::from_dword(dword0 | dword1),
                (Small(dword0), RefLarge(buffer1)) => bitor_large_dword(buffer1.into(), dword0),
                (Large(buffer0), RefSmall(dword1)) => bitor_large_dword(buffer0, dword1),
                (Large(buffer0), RefLarge(buffer1)) => bitor_large(buffer0, buffer1),
            }
        }
    }

    impl<'l> BitOr<TypedRepr> for TypedReprRef<'l> {
        type Output = Repr;

        #[inline]
        fn bitor(self, rhs: TypedRepr) -> Repr {
            // bitor is commutative
            rhs.bitor(self)
        }
    }

    impl<'l, 'r> BitOr<TypedReprRef<'r>> for TypedReprRef<'l> {
        type Output = Repr;

        #[inline]
        fn bitor(self, rhs: TypedReprRef) -> Repr {
            match (self, rhs) {
                (RefSmall(dword0), RefSmall(dword1)) => Repr::from_dword(dword0 | dword1),
                (RefSmall(dword0), RefLarge(buffer1)) => bitor_large_dword(buffer1.into(), dword0),
                (RefLarge(buffer0), RefSmall(dword1)) => bitor_large_dword(buffer0.into(), dword1),
                (RefLarge(buffer0), RefLarge(buffer1)) => {
                    if buffer0.len() >= buffer1.len() {
                        bitor_large(buffer0.into(), buffer1)
                    } else {
                        bitor_large(buffer1.into(), buffer0)
                    }
                }
            }
        }
    }

    fn bitor_large_dword(mut buffer: Buffer, rhs: DoubleWord) -> Repr {
        debug_assert!(buffer.len() >= 2);

        let (lo, hi) = split_dword(rhs);
        let (b_lo, b_hi) = buffer.lowest_dword_mut();
        *b_lo |= lo;
        *b_hi |= hi;
        Repr::from_buffer(buffer)
    }

    fn bitor_large(mut buffer: Buffer, rhs: &[Word]) -> Repr {
        for (x, y) in buffer.iter_mut().zip(rhs.iter()) {
            *x |= *y;
        }
        if rhs.len() > buffer.len() {
            buffer.ensure_capacity(rhs.len());
            buffer.push_slice(&rhs[buffer.len()..]);
        }
        Repr::from_buffer(buffer)
    }

    impl BitXor<TypedRepr> for TypedRepr {
        type Output = Repr;

        #[inline]
        fn bitxor(self, rhs: TypedRepr) -> Repr {
            match (self, rhs) {
                (Small(dword0), Small(dword1)) => Repr::from_dword(dword0 ^ dword1),
                (Small(dword0), Large(buffer1)) => bitxor_large_dword(buffer1, dword0),
                (Large(buffer0), Small(dword1)) => bitxor_large_dword(buffer0, dword1),
                (Large(buffer0), Large(buffer1)) => {
                    if buffer0.len() >= buffer1.len() {
                        bitxor_large(buffer0, &buffer1)
                    } else {
                        bitxor_large(buffer1, &buffer0)
                    }
                }
            }
        }
    }

    impl<'r> BitXor<TypedReprRef<'r>> for TypedRepr {
        type Output = Repr;

        #[inline]
        fn bitxor(self, rhs: TypedReprRef) -> Repr {
            match (self, rhs) {
                (Small(dword0), RefSmall(dword1)) => Repr::from_dword(dword0 ^ dword1),
                (Small(dword0), RefLarge(buffer1)) => bitxor_large_dword(buffer1.into(), dword0),
                (Large(buffer0), RefSmall(dword1)) => bitxor_large_dword(buffer0, dword1),
                (Large(buffer0), RefLarge(buffer1)) => bitxor_large(buffer0, buffer1),
            }
        }
    }

    impl<'l> BitXor<TypedRepr> for TypedReprRef<'l> {
        type Output = Repr;

        #[inline]
        fn bitxor(self, rhs: TypedRepr) -> Repr {
            // bitxor is commutative
            rhs.bitxor(self)
        }
    }

    impl<'l, 'r> BitXor<TypedReprRef<'r>> for TypedReprRef<'l> {
        type Output = Repr;

        #[inline]
        fn bitxor(self, rhs: TypedReprRef) -> Repr {
            match (self, rhs) {
                (RefSmall(dword0), RefSmall(dword1)) => Repr::from_dword(dword0 ^ dword1),
                (RefSmall(dword0), RefLarge(buffer1)) => bitxor_large_dword(buffer1.into(), dword0),
                (RefLarge(buffer0), RefSmall(dword1)) => bitxor_large_dword(buffer0.into(), dword1),
                (RefLarge(buffer0), RefLarge(buffer1)) => {
                    if buffer0.len() >= buffer1.len() {
                        bitxor_large(buffer0.into(), buffer1)
                    } else {
                        bitxor_large(buffer1.into(), buffer0)
                    }
                }
            }
        }
    }

    fn bitxor_large_dword(mut buffer: Buffer, rhs: DoubleWord) -> Repr {
        debug_assert!(buffer.len() >= 2);

        let (lo, hi) = split_dword(rhs);
        let (b_lo, b_hi) = buffer.lowest_dword_mut();
        *b_lo ^= lo;
        *b_hi ^= hi;
        Repr::from_buffer(buffer)
    }

    fn bitxor_large(mut buffer: Buffer, rhs: &[Word]) -> Repr {
        for (x, y) in buffer.iter_mut().zip(rhs.iter()) {
            *x ^= *y;
        }
        if rhs.len() > buffer.len() {
            buffer.ensure_capacity(rhs.len());
            buffer.push_slice(&rhs[buffer.len()..]);
        }
        Repr::from_buffer(buffer)
    }

    impl AndNot<TypedRepr> for TypedRepr {
        type Output = Repr;

        #[inline]
        fn and_not(self, rhs: TypedRepr) -> Repr {
            match (self, rhs) {
                (Small(dword0), Small(dword1)) => Repr::from_dword(dword0 & !dword1),
                (Small(dword0), Large(buffer1)) => {
                    Repr::from_dword(dword0 & !buffer1.lowest_dword())
                }
                (Large(buffer0), Small(dword1)) => and_not_large_dword(buffer0, dword1),
                (Large(buffer0), Large(buffer1)) => and_not_large(buffer0, &buffer1),
            }
        }
    }

    impl<'r> AndNot<TypedReprRef<'r>> for TypedRepr {
        type Output = Repr;

        #[inline]
        fn and_not(self, rhs: TypedReprRef) -> Repr {
            match (self, rhs) {
                (Small(dword0), RefSmall(dword1)) => Repr::from_dword(dword0 & !dword1),
                (Small(dword0), RefLarge(buffer1)) => {
                    Repr::from_dword(dword0 & !lowest_dword(buffer1))
                }
                (Large(buffer0), RefSmall(dword1)) => and_not_large_dword(buffer0, dword1),
                (Large(buffer0), RefLarge(buffer1)) => and_not_large(buffer0, buffer1),
            }
        }
    }

    impl<'l> AndNot<TypedRepr> for TypedReprRef<'l> {
        type Output = Repr;

        #[inline]
        fn and_not(self, rhs: TypedRepr) -> Repr {
            match (self, rhs) {
                (RefSmall(dword0), Small(dword1)) => Repr::from_dword(dword0 & !dword1),
                (RefSmall(dword0), Large(buffer1)) => {
                    Repr::from_dword(dword0 & !buffer1.lowest_dword())
                }
                (RefLarge(buffer0), Small(dword1)) => and_not_large_dword(buffer0.into(), dword1),
                (RefLarge(buffer0), Large(buffer1)) => and_not_large(buffer0.into(), &buffer1),
            }
        }
    }

    impl<'l, 'r> AndNot<TypedReprRef<'r>> for TypedReprRef<'l> {
        type Output = Repr;

        #[inline]
        fn and_not(self, rhs: TypedReprRef) -> Repr {
            match (self, rhs) {
                (RefSmall(dword0), RefSmall(dword1)) => Repr::from_dword(dword0 & !dword1),
                (RefSmall(dword0), RefLarge(buffer1)) => {
                    Repr::from_dword(dword0 & !lowest_dword(buffer1))
                }
                (RefLarge(buffer0), RefSmall(dword1)) => {
                    and_not_large_dword(buffer0.into(), dword1)
                }
                (RefLarge(buffer0), RefLarge(buffer1)) => and_not_large(buffer0.into(), buffer1),
            }
        }
    }

    fn and_not_large_dword(mut buffer: Buffer, rhs: DoubleWord) -> Repr {
        debug_assert!(buffer.len() >= 2);

        let (lo, hi) = split_dword(rhs);
        let (b_lo, b_hi) = buffer.lowest_dword_mut();
        *b_lo &= !lo;
        *b_hi &= !hi;
        Repr::from_buffer(buffer)
    }

    fn and_not_large(mut buffer: Buffer, rhs: &[Word]) -> Repr {
        for (x, y) in buffer.iter_mut().zip(rhs.iter()) {
            *x &= !*y;
        }
        Repr::from_buffer(buffer)
    }
}

impl Not for IBig {
    type Output = IBig;

    #[inline]
    fn not(self) -> IBig {
        let (sign, mag) = self.into_sign_repr();
        match sign {
            Positive => IBig(mag.add_one().with_sign(Negative)),
            Negative => IBig(mag.sub_one().with_sign(Positive)),
        }
    }
}

impl Not for &IBig {
    type Output = IBig;

    #[inline]
    fn not(self) -> IBig {
        let (sign, mag) = self.as_sign_repr();
        match sign {
            Positive => IBig(mag.add_one().with_sign(Negative)),
            Negative => IBig(mag.sub_one().with_sign(Positive)),
        }
    }
}

macro_rules! impl_ibig_bitand {
    ($sign0:ident, $mag0:ident, $sign1:ident, $mag1:ident) => {
        match ($sign0, $sign1) {
            (Positive, Positive) => IBig($mag0.bitand($mag1)),
            (Positive, Negative) => IBig($mag0.and_not($mag1.sub_one().into_typed())),
            (Negative, Positive) => IBig($mag1.and_not($mag0.sub_one().into_typed())),
            (Negative, Negative) => !IBig(
                $mag0
                    .sub_one()
                    .into_typed()
                    .bitor($mag1.sub_one().into_typed()),
            ),
        }
    };
}
macro_rules! impl_ibig_bitor {
    ($sign0:ident, $mag0:ident, $sign1:ident, $mag1:ident) => {
        match ($sign0, $sign1) {
            (Positive, Positive) => IBig($mag0.bitor($mag1)),
            (Positive, Negative) => !IBig($mag1.sub_one().into_typed().and_not($mag0)),
            (Negative, Positive) => !IBig($mag0.sub_one().into_typed().and_not($mag1)),
            (Negative, Negative) => !IBig(
                $mag0
                    .sub_one()
                    .into_typed()
                    .bitand($mag1.sub_one().into_typed()),
            ),
        }
    };
}
macro_rules! impl_ibig_bitxor {
    ($sign0:ident, $mag0:ident, $sign1:ident, $mag1:ident) => {
        match ($sign0, $sign1) {
            (Positive, Positive) => IBig($mag0.bitxor($mag1)),
            (Positive, Negative) => !IBig($mag0.bitxor($mag1.sub_one().into_typed())),
            (Negative, Positive) => !IBig($mag0.sub_one().into_typed().bitxor($mag1)),
            (Negative, Negative) => IBig(
                $mag0
                    .sub_one()
                    .into_typed()
                    .bitxor($mag1.sub_one().into_typed()),
            ),
        }
    };
}
helper_macros::forward_ibig_binop_to_repr!(impl BitAnd, bitand, Output = IBig, impl_ibig_bitand);
helper_macros::forward_ibig_binop_to_repr!(impl BitOr, bitor, Output = IBig, impl_ibig_bitor);
helper_macros::forward_ibig_binop_to_repr!(impl BitXor, bitxor, Output = IBig, impl_ibig_bitxor);
helper_macros::impl_binop_assign_by_taking!(impl BitAndAssign<IBig> for IBig, bitand_assign, bitand);
helper_macros::impl_binop_assign_by_taking!(impl BitOrAssign<IBig> for IBig, bitor_assign, bitor);
helper_macros::impl_binop_assign_by_taking!(impl BitXorAssign<IBig> for IBig, bitxor_assign, bitxor);

macro_rules! impl_bit_ops_primitive_with_ubig {
    ($($t:ty)*) => {$(
        helper_macros::impl_commutative_binop_with_primitive!(impl BitAnd<$t> for UBig, bitand -> $t);
        helper_macros::impl_commutative_binop_with_primitive!(impl BitOr<$t> for UBig, bitor);
        helper_macros::impl_commutative_binop_with_primitive!(impl BitXor<$t> for UBig, bitxor);
        helper_macros::impl_binop_assign_with_primitive!(impl BitAndAssign<$t> for UBig, bitand_assign);
        helper_macros::impl_binop_assign_with_primitive!(impl BitOrAssign<$t> for UBig, bitor_assign);
        helper_macros::impl_binop_assign_with_primitive!(impl BitXorAssign<$t> for UBig, bitxor_assign);
    )*};
}
impl_bit_ops_primitive_with_ubig!(u8 u16 u32 u64 u128 usize);

macro_rules! impl_bit_ops_primitive_with_ibig {
    ($($t:ty)*) => {$(
        helper_macros::impl_commutative_binop_with_primitive!(impl BitAnd<$t> for IBig, bitand);
        helper_macros::impl_commutative_binop_with_primitive!(impl BitOr<$t> for IBig, bitor);
        helper_macros::impl_commutative_binop_with_primitive!(impl BitXor<$t> for IBig, bitxor);
        helper_macros::impl_binop_assign_with_primitive!(impl BitAndAssign<$t> for IBig, bitand_assign);
        helper_macros::impl_binop_assign_with_primitive!(impl BitOrAssign<$t> for IBig, bitor_assign);
        helper_macros::impl_binop_assign_with_primitive!(impl BitXorAssign<$t> for IBig, bitxor_assign);
    )*};
}
impl_bit_ops_primitive_with_ibig!(i8 i16 i32 i64 i128 isize);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_and_not() {
        let cases = [
            (UBig::from(0xf0f0u16), UBig::from(0xff00u16), UBig::from(0xf0u16)),
            (
                UBig::from(0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeu128),
                UBig::from(0xffu8),
                UBig::from(0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeee00u128),
            ),
            (
                UBig::from(0xffu8),
                UBig::from(0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeu128),
                UBig::from(0x11u8),
            ),
            (
                UBig::from_str_radix("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee", 16).unwrap(),
                UBig::from_str_radix(
                    "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
                    16,
                )
                .unwrap(),
                UBig::from_str_radix("22222222222222222222222222222222", 16).unwrap(),
            ),
            (
                UBig::from_str_radix(
                    "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
                    16,
                )
                .unwrap(),
                UBig::from_str_radix("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee", 16).unwrap(),
                UBig::from_str_radix(
                    "dddddddddddddddddddddddddddddddd11111111111111111111111111111111",
                    16,
                )
                .unwrap(),
            ),
        ];

        for (a, b, c) in cases.iter() {
            assert_eq!(UBig(a.repr().and_not(b.repr())), *c);
            assert_eq!(UBig(a.clone().into_repr().and_not(b.repr())), *c);
            assert_eq!(UBig(a.repr().and_not(b.clone().into_repr())), *c);
            let (a, b) = (a.clone(), b.clone());
            assert_eq!(UBig(a.into_repr().and_not(b.into_repr())), *c);
        }
    }
}
