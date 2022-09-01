//! Multiplication and squaring operators.

use crate::{helper_macros, ibig::IBig, ubig::UBig};
use core::ops::{Mul, MulAssign};

helper_macros::forward_ubig_binop_to_repr!(impl Mul, mul);
helper_macros::impl_binop_assign_by_taking!(impl MulAssign<UBig> for UBig, mul_assign, mul);

macro_rules! impl_ibig_mul {
    ($sign0:ident, $mag0:ident, $sign1:ident, $mag1:ident) => {
        IBig($mag0.mul($mag1).with_sign($sign0 * $sign1))
    };
}
helper_macros::forward_ibig_binop_to_repr!(impl Mul, mul, Output = IBig, impl_ibig_mul);
helper_macros::impl_binop_assign_by_taking!(impl MulAssign<IBig> for IBig, mul_assign, mul);

macro_rules! impl_ubig_ibig_mul {
    ($mag0:ident, $sign1:ident, $mag1:ident) => {
        IBig($mag0.mul($mag1).with_sign($sign1))
    };
}
helper_macros::forward_ubig_ibig_binop_to_repr!(impl Mul, mul, Output = IBig, impl_ubig_ibig_mul);

macro_rules! impl_ibig_ubig_mul {
    ($sign0:ident, $mag0:ident, $mag1:ident) => {
        IBig($mag0.mul($mag1).with_sign($sign0))
    };
}
helper_macros::forward_ibig_ubig_binop_to_repr!(impl Mul, mul, Output = IBig, impl_ibig_ubig_mul);
helper_macros::impl_binop_assign_by_taking!(impl MulAssign<UBig> for IBig, mul_assign, mul);

// Ops with primitives

macro_rules! impl_div_primitive_with_ubig {
    ($($t:ty)*) => {$(
        helper_macros::impl_commutative_binop_with_primitive!(impl Mul<$t> for UBig, mul);
        helper_macros::impl_binop_assign_with_primitive!(impl MulAssign<$t> for UBig, mul_assign);
    )*};
}
impl_div_primitive_with_ubig!(u8 u16 u32 u64 u128 usize);

macro_rules! impl_div_primitive_with_ibig {
    ($($t:ty)*) => {$(
        helper_macros::impl_commutative_binop_with_primitive!(impl Mul<$t> for IBig, mul);
        helper_macros::impl_binop_assign_with_primitive!(impl MulAssign<$t> for IBig, mul_assign);
    )*};
}
impl_div_primitive_with_ibig!(u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize);

impl UBig {
    /// Calculate the squared number (x * x).
    ///
    /// # Example
    ///
    /// ```
    /// # use dashu_int::UBig;
    /// assert_eq!(UBig::from(3u8).square(), 9);
    /// ```
    #[inline]
    pub fn square(&self) -> UBig {
        UBig(self.repr().square())
    }
}

impl IBig {
    /// Calculate the squared number (x * x).
    ///
    /// # Example
    ///
    /// ```
    /// # use dashu_int::IBig;
    /// assert_eq!(IBig::from(-3).square(), 9);
    /// ```
    #[inline]
    pub fn square(&self) -> IBig {
        IBig(self.as_sign_repr().1.square())
    }
}

pub(crate) mod repr {
    use super::*;
    use crate::{
        arch::word::{DoubleWord, Word},
        buffer::Buffer,
        cmp::cmp_in_place,
        math,
        memory::MemoryAllocation,
        mul,
        primitive::{extend_word, shrink_dword, split_dword},
        repr::{
            Repr,
            TypedRepr::{self, *},
            TypedReprRef::{self, *},
        },
        shift, sqr,
    };

    impl Mul<TypedRepr> for TypedRepr {
        type Output = Repr;

        #[inline]
        fn mul(self, rhs: TypedRepr) -> Repr {
            match (self, rhs) {
                (Small(dword0), Small(dword1)) => mul_dword(dword0, dword1),
                (Small(dword0), Large(buffer1)) => mul_large_dword(buffer1, dword0),
                (Large(buffer0), Small(dword1)) => mul_large_dword(buffer0, dword1),
                (Large(buffer0), Large(buffer1)) => mul_large(&buffer0, &buffer1),
            }
        }
    }

    impl<'l> Mul<TypedRepr> for TypedReprRef<'l> {
        type Output = Repr;

        #[inline]
        fn mul(self, rhs: TypedRepr) -> Repr {
            match (self, rhs) {
                (RefSmall(dword0), Small(dword1)) => mul_dword(dword0, dword1),
                (RefSmall(dword0), Large(buffer1)) => mul_large_dword(buffer1, dword0),
                (RefLarge(buffer0), Small(dword1)) => mul_large_dword(buffer0.into(), dword1),
                (RefLarge(buffer0), Large(buffer1)) => mul_large(buffer0, &buffer1),
            }
        }
    }

    impl<'r> Mul<TypedReprRef<'r>> for TypedRepr {
        type Output = Repr;

        #[inline]
        fn mul(self, rhs: TypedReprRef) -> Self::Output {
            // mul is commutative
            rhs.mul(self)
        }
    }

    impl<'l, 'r> Mul<TypedReprRef<'r>> for TypedReprRef<'l> {
        type Output = Repr;

        #[inline]
        fn mul(self, rhs: TypedReprRef) -> Repr {
            match (self, rhs) {
                (RefSmall(dword0), RefSmall(dword1)) => mul_dword(dword0, dword1),
                (RefSmall(dword0), RefLarge(buffer1)) => mul_large_dword(buffer1.into(), dword0),
                (RefLarge(buffer0), RefSmall(dword1)) => mul_large_dword(buffer0.into(), dword1),
                (RefLarge(buffer0), RefLarge(buffer1)) => mul_large(buffer0, buffer1),
            }
        }
    }

    /// Multiply two `DoubleWord`s.
    #[inline]
    fn mul_dword(a: DoubleWord, b: DoubleWord) -> Repr {
        if a <= Word::MAX as DoubleWord && b <= Word::MAX as DoubleWord {
            Repr::from_dword(a * b)
        } else {
            mul_dword_spilled(a, b)
        }
    }

    fn mul_dword_spilled(lhs: DoubleWord, rhs: DoubleWord) -> Repr {
        let (lo, hi) = math::mul_add_carry_dword(lhs, rhs, 0);
        let mut buffer = Buffer::allocate(4);
        let (n0, n1) = split_dword(lo);
        buffer.push(n0);
        buffer.push(n1);
        let (n2, n3) = split_dword(hi);
        buffer.push(n2);
        buffer.push(n3);
        Repr::from_buffer(buffer)
    }

    /// Multiply a large number by a `DoubleWord`.
    pub(crate) fn mul_large_dword(mut buffer: Buffer, rhs: DoubleWord) -> Repr {
        match rhs {
            0 => Repr::zero(),
            1 => Repr::from_buffer(buffer),
            dw => {
                if let Some(word) = shrink_dword(dw) {
                    let carry = if dw.is_power_of_two() {
                        shift::shl_in_place(&mut buffer, dw.trailing_zeros())
                    } else {
                        mul::mul_word_in_place(&mut buffer, word)
                    };
                    buffer.push_resizing(carry);
                    Repr::from_buffer(buffer)
                } else {
                    let carry = mul::mul_dword_in_place(&mut buffer, dw);
                    if carry != 0 {
                        let (lo, hi) = split_dword(carry);
                        buffer.ensure_capacity(buffer.len() + 2);
                        buffer.push(lo);
                        buffer.push(hi);
                    }
                    Repr::from_buffer(buffer)
                }
            }
        }
    }

    /// Multiply two large numbers.
    pub(crate) fn mul_large(lhs: &[Word], rhs: &[Word]) -> Repr {
        debug_assert!(lhs.len() >= 2 && rhs.len() >= 2);

        // shortcut to square if two operands are equal
        if cmp_in_place(lhs, rhs).is_eq() {
            return square_large(lhs);
        }

        let res_len = lhs.len() + rhs.len();
        let mut buffer = Buffer::allocate(res_len);
        buffer.push_zeros(res_len);

        let mut allocation =
            MemoryAllocation::new(mul::memory_requirement_exact(res_len, lhs.len().min(rhs.len())));
        mul::multiply(&mut buffer, lhs, rhs, &mut allocation.memory());
        Repr::from_buffer(buffer)
    }

    impl TypedReprRef<'_> {
        pub fn square(&self) -> Repr {
            match self {
                TypedReprRef::RefSmall(dword) => {
                    if let Some(word) = shrink_dword(*dword) {
                        Repr::from_dword(extend_word(word) * extend_word(word))
                    } else {
                        square_dword_spilled(*dword)
                    }
                }
                TypedReprRef::RefLarge(words) => square_large(words),
            }
        }
    }

    fn square_dword_spilled(dw: DoubleWord) -> Repr {
        let (lo, hi) = math::mul_add_carry_dword(dw, dw, 0);
        let mut buffer = Buffer::allocate(4);
        let (n0, n1) = split_dword(lo);
        buffer.push(n0);
        buffer.push(n1);
        let (n2, n3) = split_dword(hi);
        buffer.push(n2);
        buffer.push(n3);
        Repr::from_buffer(buffer)
    }

    pub(crate) fn square_large(words: &[Word]) -> Repr {
        debug_assert!(words.len() >= 2);

        let mut buffer = Buffer::allocate(words.len() * 2);
        buffer.push_zeros(words.len() * 2);

        let mut allocation = MemoryAllocation::new(sqr::memory_requirement_exact(words.len()));
        sqr::square(&mut buffer, words, &mut allocation.memory());
        Repr::from_buffer(buffer)
    }
}
