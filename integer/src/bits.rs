//! Bitwise operators.

use crate::{
    arch::word::{DoubleWord, Word},
    repr::{Buffer, TypedRepr::*, TypedReprRef::*},
    helper_macros,
    ibig::IBig,
    math,
    ops::{AndNot, PowerOfTwo, UnsignedAbs},
    primitive::{first_dword, split_dword, DWORD_BITS_USIZE, WORD_BITS_USIZE},
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
                    *self = UBig::with_bit_dword_spilled(dword, n);
                }
            }
            Large(buffer) => {
                *self = UBig::with_bit_large(buffer, n);
            }
        }
    }

    fn with_bit_dword_spilled(dword: DoubleWord, n: usize) -> UBig {
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

    fn with_bit_large(mut buffer: Buffer, n: usize) -> UBig {
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

helper_macros::forward_ubig_binop_to_repr!(impl BitAnd, bitand);
helper_macros::forward_ubig_binop_to_repr!(impl BitOr, bitor);
helper_macros::forward_ubig_binop_to_repr!(impl BitXor, bitxor);
helper_macros::forward_ubig_binop_to_repr!(impl AndNot, and_not);
helper_macros::forward_binop_assign_by_taking!(impl BitAndAssign<UBig> for UBig, bitand_assign, bitand);
helper_macros::forward_binop_assign_by_taking!(impl BitOrAssign<UBig> for UBig, bitor_assign, bitor);
helper_macros::forward_binop_assign_by_taking!(impl BitXorAssign<UBig> for UBig, bitxor_assign, bitxor);

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
        self.repr().is_power_of_two()
    }

    #[inline]
    fn next_power_of_two(self) -> UBig {
        UBig(self.into_repr().next_power_of_two())
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
        pub(crate) fn are_low_bits_nonzero(self, n: usize) -> bool {
            match self {
                Self::RefSmall(dword) => are_dword_low_bits_nonzero(dword, n),
                Self::RefLarge(buffer) => are_slice_low_bits_nonzero(buffer, n),
            }
        }

        /// Check if the underlying number is a power of two
        #[inline]
        pub(crate) fn is_power_of_two(self) -> bool {
            match self {
                RefSmall(dword) => dword.is_power_of_two(),
                RefLarge(buffer) => {
                    buffer[..buffer.len() - 1].iter().all(|x| *x == 0)
                        && buffer.last().unwrap().is_power_of_two()
                }
            }
        }
    }

    impl TypedRepr {
        #[inline]
        pub(crate) fn next_power_of_two(self) -> Repr {
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
    }

    #[inline]
    fn are_dword_low_bits_nonzero(dword: DoubleWord, n: usize) -> bool {
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
                buffer.ensure_capacity(n + 1);
                buffer.push(1);
            }
        }

        Repr::from_buffer(buffer)
    }

    impl BitAnd<TypedRepr> for TypedRepr {
        type Output = Repr;

        #[inline]
        fn bitand(self, rhs: TypedRepr) -> Repr {
            match (self, rhs) {
                (Small(dword0), Small(dword1)) => Repr::from_dword(dword0 & dword1),
                (Small(dword0), Large(buffer1)) => Repr::from_dword(dword0 & buffer1.first_dword()),
                (Large(buffer0), Small(dword1)) => Repr::from_dword(buffer0.first_dword() & dword1),
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
                (Small(dword0), RefLarge(buffer1)) => Repr::from_dword(dword0 & first_dword(buffer1)),
                (Large(buffer0), RefSmall(dword1)) => Repr::from_dword(buffer0.first_dword() & dword1),
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
                (RefSmall(dword0), RefLarge(buffer1)) => Repr::from_dword(dword0 & first_dword(buffer1)),
                (RefLarge(buffer0), RefSmall(dword1)) => Repr::from_dword(first_dword(buffer0) & dword1),
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
        let (b_lo, b_hi) = buffer.first_dword_mut();
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
                (RefSmall(dword0), RefLarge(buffer1)) => 
                    bitxor_large_dword(buffer1.into(), dword0),
                (RefLarge(buffer0), RefSmall(dword1)) => 
                    bitxor_large_dword(buffer0.into(), dword1),
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
        let (b_lo, b_hi) = buffer.first_dword_mut();
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
                (Small(dword0), Large(buffer1)) => Repr::from_dword(dword0 & !buffer1.first_dword()),
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
                (Small(dword0), RefLarge(buffer1)) => Repr::from_dword(dword0 & !first_dword(buffer1)),
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
                (RefSmall(dword0), Large(buffer1)) => Repr::from_dword(dword0 & !buffer1.first_dword()),
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
                (RefSmall(dword0), RefLarge(buffer1)) => Repr::from_dword(dword0 & !first_dword(buffer1)),
                (RefLarge(buffer0), RefSmall(dword1)) => and_not_large_dword(buffer0.into(), dword1),
                (RefLarge(buffer0), RefLarge(buffer1)) => and_not_large(buffer0.into(), buffer1),
            }
        }
    }
    
    fn and_not_large_dword(mut buffer: Buffer, rhs: DoubleWord) -> Repr {
        debug_assert!(buffer.len() >= 2);

        let (lo, hi) = split_dword(rhs);
        let (b_lo, b_hi) = buffer.first_dword_mut();
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
        // TODO: implement "add_one" and then implement this more efficiently
        -(self + IBig::one())
    }
}

impl Not for &IBig {
    type Output = IBig;

    #[inline]
    fn not(self) -> IBig {
        -(self + IBig::one())
    }
}

impl BitAnd<IBig> for IBig {
    type Output = IBig;

    #[inline]
    fn bitand(self, rhs: IBig) -> IBig {
        let (sign0, mag0) = self.into_sign_repr();
        let (sign1, mag1) = rhs.into_sign_repr();
        helper_macros::forword_ibig_bitand_to_repr!(sign0, mag0, sign1, mag1)
    }
}

impl BitAnd<&IBig> for IBig {
    type Output = IBig;

    #[inline]
    fn bitand(self, rhs: &IBig) -> IBig {
        let (sign0, mag0) = self.into_sign_repr();
        let (sign1, mag1) = rhs.as_sign_repr();
        helper_macros::forword_ibig_bitand_to_repr!(sign0, mag0, sign1, mag1)
    }
}

impl BitAnd<IBig> for &IBig {
    type Output = IBig;

    #[inline]
    fn bitand(self, rhs: IBig) -> IBig {
        let (sign0, mag0) = self.as_sign_repr();
        let (sign1, mag1) = rhs.into_sign_repr();
        helper_macros::forword_ibig_bitand_to_repr!(sign0, mag0, sign1, mag1)
    }
}

impl BitAnd<&IBig> for &IBig {
    type Output = IBig;

    #[inline]
    fn bitand(self, rhs: &IBig) -> IBig {
        let (sign0, mag0) = self.as_sign_repr();
        let (sign1, mag1) = rhs.as_sign_repr();
        helper_macros::forword_ibig_bitand_to_repr!(sign0, mag0, sign1, mag1)
    }
}

impl BitOr<IBig> for IBig {
    type Output = IBig;

    #[inline]
    fn bitor(self, rhs: IBig) -> IBig {
        let (sign0, mag0) = self.into_sign_repr();
        let (sign1, mag1) = rhs.into_sign_repr();
        helper_macros::forword_ibig_bitor_to_repr!(sign0, mag0, sign1, mag1)
    }
}

impl BitOr<&IBig> for IBig {
    type Output = IBig;

    #[inline]
    fn bitor(self, rhs: &IBig) -> IBig {
        let (sign0, mag0) = self.into_sign_repr();
        let (sign1, mag1) = rhs.as_sign_repr();
        helper_macros::forword_ibig_bitor_to_repr!(sign0, mag0, sign1, mag1)
    }
}

impl BitOr<IBig> for &IBig {
    type Output = IBig;

    #[inline]
    fn bitor(self, rhs: IBig) -> IBig {
        let (sign0, mag0) = self.as_sign_repr();
        let (sign1, mag1) = rhs.into_sign_repr();
        helper_macros::forword_ibig_bitor_to_repr!(sign0, mag0, sign1, mag1)
    }
}

impl BitOr<&IBig> for &IBig {
    type Output = IBig;

    #[inline]
    fn bitor(self, rhs: &IBig) -> IBig {
        let (sign0, mag0) = self.as_sign_repr();
        let (sign1, mag1) = rhs.as_sign_repr();
        helper_macros::forword_ibig_bitor_to_repr!(sign0, mag0, sign1, mag1)
    }
}

impl BitXor<IBig> for IBig {
    type Output = IBig;

    #[inline]
    fn bitxor(self, rhs: IBig) -> IBig {
        let (sign0, mag0) = self.into_sign_repr();
        let (sign1, mag1) = rhs.into_sign_repr();
        helper_macros::forword_ibig_bitxor_to_repr!(sign0, mag0, sign1, mag1)
    }
}

impl BitXor<&IBig> for IBig {
    type Output = IBig;

    #[inline]
    fn bitxor(self, rhs: &IBig) -> IBig {
        let (sign0, mag0) = self.into_sign_repr();
        let (sign1, mag1) = rhs.as_sign_repr();
        helper_macros::forword_ibig_bitxor_to_repr!(sign0, mag0, sign1, mag1)
    }
}

impl BitXor<IBig> for &IBig {
    type Output = IBig;

    #[inline]
    fn bitxor(self, rhs: IBig) -> IBig {
        let (sign0, mag0) = self.as_sign_repr();
        let (sign1, mag1) = rhs.into_sign_repr();
        helper_macros::forword_ibig_bitxor_to_repr!(sign0, mag0, sign1, mag1)
    }
}

impl BitXor<&IBig> for &IBig {
    type Output = IBig;

    #[inline]
    fn bitxor(self, rhs: &IBig) -> IBig {
        let (sign0, mag0) = self.as_sign_repr();
        let (sign1, mag1) = rhs.as_sign_repr();
        helper_macros::forword_ibig_bitxor_to_repr!(sign0, mag0, sign1, mag1)
    }
}

helper_macros::forward_binop_assign_by_taking!(impl BitAndAssign<IBig> for IBig, bitand_assign, bitand);
helper_macros::forward_binop_assign_by_taking!(impl BitOrAssign<IBig> for IBig, bitor_assign, bitor);
helper_macros::forward_binop_assign_by_taking!(impl BitXorAssign<IBig> for IBig, bitxor_assign, bitxor);

// TODO: implement ops in IBig with functions on repr instead of using unsigned_abs(), and try
// to optimize intermediate operations.

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

        helper_macros::forward_binop_second_arg_by_value!(impl BitAnd<$t> for IBig, bitand);
        helper_macros::forward_binop_swap_args!(impl BitAnd<IBig> for $t, bitand);

        impl BitAndAssign<$t> for IBig {
            #[inline]
            fn bitand_assign(&mut self, rhs: $t) {
                self.bitand_assign(IBig::from_signed(rhs))
            }
        }

        helper_macros::forward_binop_assign_arg_by_value!(impl BitAndAssign<$t> for IBig, bitand_assign);

        impl BitOr<$t> for IBig {
            type Output = IBig;

            #[inline]
            fn bitor(self, rhs: $t) -> IBig {
                self.bitor(IBig::from_signed(rhs))
            }
        }

        impl BitOr<$t> for &IBig {
            type Output = IBig;

            #[inline]
            fn bitor(self, rhs: $t) -> IBig {
                self.bitor(IBig::from_signed(rhs))
            }
        }

        helper_macros::forward_binop_second_arg_by_value!(impl BitOr<$t> for IBig, bitor);
        helper_macros::forward_binop_swap_args!(impl BitOr<IBig> for $t, bitor);

        impl BitOrAssign<$t> for IBig {
            #[inline]
            fn bitor_assign(&mut self, rhs: $t) {
                self.bitor_assign(IBig::from_signed(rhs))
            }
        }

        helper_macros::forward_binop_assign_arg_by_value!(impl BitOrAssign<$t> for IBig, bitor_assign);

        impl BitXor<$t> for IBig {
            type Output = IBig;

            #[inline]
            fn bitxor(self, rhs: $t) -> IBig {
                self.bitxor(IBig::from_signed(rhs))
            }
        }

        impl BitXor<$t> for &IBig {
            type Output = IBig;

            #[inline]
            fn bitxor(self, rhs: $t) -> IBig {
                self.bitxor(IBig::from_signed(rhs))
            }
        }

        helper_macros::forward_binop_second_arg_by_value!(impl BitXor<$t> for IBig, bitxor);
        helper_macros::forward_binop_swap_args!(impl BitXor<IBig> for $t, bitxor);

        impl BitXorAssign<$t> for IBig {
            #[inline]
            fn bitxor_assign(&mut self, rhs: $t) {
                self.bitxor_assign(IBig::from_signed(rhs))
            }
        }

        helper_macros::forward_binop_assign_arg_by_value!(impl BitXorAssign<$t> for IBig, bitxor_assign);

        impl AndNot<$t> for IBig {
            type Output = IBig;

            #[inline]
            fn and_not(self, rhs: $t) -> IBig {
                self.and_not(IBig::from_signed(rhs))
            }
        }

        impl AndNot<$t> for &IBig {
            type Output = IBig;

            #[inline]
            fn and_not(self, rhs: $t) -> IBig {
                self.and_not(IBig::from_signed(rhs))
            }
        }

        helper_macros::forward_binop_second_arg_by_value!(impl AndNot<$t> for IBig, and_not);
    };
}

impl_bit_ops_ibig_signed!(i8);
impl_bit_ops_ibig_signed!(i16);
impl_bit_ops_ibig_signed!(i32);
impl_bit_ops_ibig_signed!(i64);
impl_bit_ops_ibig_signed!(i128);
impl_bit_ops_ibig_signed!(isize);
