//! Multiplication operators.

use crate::{
    arch::word::{DoubleWord, Word},
    helper_macros,
    ibig::IBig,
    memory::MemoryAllocation,
    mul,
    repr::{Buffer, TypedRepr::*, TypedReprRef::*},
    sign::Sign::*,
    ubig::UBig,
};
use core::ops::{Mul, MulAssign};

helper_macros::forward_ubig_binop_to_repr!(impl Mul, mul);
helper_macros::forward_binop_assign_by_taking!(impl MulAssign<UBig> for UBig, mul_assign, mul);

macro_rules! impl_ibig_mul {
    ($sign0:ident, $mag0:ident, $sign1:ident, $mag1:ident) => {
        IBig($mag0.mul($mag1).with_sign($sign0 * $sign1))
    };
}
helper_macros::forward_ibig_binop_to_repr!(impl Mul, mul, impl_ibig_mul);
helper_macros::forward_binop_assign_by_taking!(impl MulAssign<IBig> for IBig, mul_assign, mul);

macro_rules! impl_ubig_ibig_mul {
    ($mag0:ident, $sign1:ident, $mag1:ident) => {
        IBig($mag0.mul($mag1).with_sign($sign1))
    };
}
helper_macros::forward_ubig_ibig_binop_to_repr!(impl Mul, mul, impl_ubig_ibig_mul);

macro_rules! impl_ibig_ubig_mul {
    ($sign0:ident, $mag0:ident, $mag1:ident) => {
        IBig($mag0.mul($mag1).with_sign($sign0))
    };
}
helper_macros::forward_ibig_ubig_binop_to_repr!(impl Mul, mul, impl_ibig_ubig_mul);
helper_macros::forward_binop_assign_by_taking!(impl MulAssign<UBig> for IBig, mul_assign, mul);

macro_rules! impl_mul_ubig_unsigned {
    ($t:ty) => {
        impl Mul<$t> for UBig {
            type Output = UBig;

            #[inline]
            fn mul(self, rhs: $t) -> UBig {
                self * UBig::from_unsigned(rhs)
            }
        }

        impl Mul<$t> for &UBig {
            type Output = UBig;

            #[inline]
            fn mul(self, rhs: $t) -> UBig {
                self * UBig::from_unsigned(rhs)
            }
        }

        helper_macros::forward_binop_second_arg_by_value!(impl Mul<$t> for UBig, mul);
        helper_macros::forward_binop_swap_args!(impl Mul<UBig> for $t, mul);

        impl MulAssign<$t> for UBig {
            #[inline]
            fn mul_assign(&mut self, rhs: $t) {
                *self *= UBig::from_unsigned(rhs)
            }
        }

        helper_macros::forward_binop_assign_arg_by_value!(impl MulAssign<$t> for UBig, mul_assign);
    };
}

impl_mul_ubig_unsigned!(u8);
impl_mul_ubig_unsigned!(u16);
impl_mul_ubig_unsigned!(u32);
impl_mul_ubig_unsigned!(u64);
impl_mul_ubig_unsigned!(u128);
impl_mul_ubig_unsigned!(usize);

macro_rules! impl_mul_ibig_primitive {
    ($t:ty) => {
        impl Mul<$t> for IBig {
            type Output = IBig;

            #[inline]
            fn mul(self, rhs: $t) -> IBig {
                self * IBig::from(rhs)
            }
        }

        impl Mul<$t> for &IBig {
            type Output = IBig;

            #[inline]
            fn mul(self, rhs: $t) -> IBig {
                self * IBig::from(rhs)
            }
        }

        helper_macros::forward_binop_second_arg_by_value!(impl Mul<$t> for IBig, mul);
        helper_macros::forward_binop_swap_args!(impl Mul<IBig> for $t, mul);

        impl MulAssign<$t> for IBig {
            #[inline]
            fn mul_assign(&mut self, rhs: $t) {
                *self *= IBig::from(rhs)
            }
        }

        helper_macros::forward_binop_assign_arg_by_value!(impl MulAssign<$t> for IBig, mul_assign);
    };
}

impl_mul_ibig_primitive!(i8);
impl_mul_ibig_primitive!(i16);
impl_mul_ibig_primitive!(i32);
impl_mul_ibig_primitive!(i64);
impl_mul_ibig_primitive!(i128);
impl_mul_ibig_primitive!(isize);

mod repr {
    use super::*;
    use crate::primitive::{shrink_dword, split_dword};
    use crate::repr::{Repr, TypedRepr, TypedReprRef};
    use crate::{math, shift};

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
    fn mul_large_dword(mut buffer: Buffer, rhs: DoubleWord) -> Repr {
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
                    if carry != 0 {
                        buffer.push_resizing(carry);
                    }
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
    fn mul_large(lhs: &[Word], rhs: &[Word]) -> Repr {
        debug_assert!(lhs.len() >= 2 && rhs.len() >= 2);

        let res_len = lhs.len() + rhs.len();
        let mut buffer = Buffer::allocate(res_len);
        buffer.push_zeros(res_len);

        let mut allocation = MemoryAllocation::new(mul::memory_requirement_exact(
            res_len,
            lhs.len().min(rhs.len()),
        ));
        let mut memory = allocation.memory();
        let overflow = mul::add_signed_mul(&mut buffer, Positive, lhs, rhs, &mut memory);
        assert!(overflow == 0);
        Repr::from_buffer(buffer)
    }
}
