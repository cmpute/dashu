//! Multiplication operators.

use crate::{
    arch::word::{Word, DoubleWord},
    buffer::{Buffer, TypedRepr::*, TypedReprRef::*},
    helper_macros,
    ibig::IBig,
    memory::MemoryAllocation,
    mul,
    sign::Sign::{self, *},
    ubig::UBig,
};
use core::{
    mem,
    ops::{Mul, MulAssign},
};
use static_assertions::const_assert;

impl Mul<UBig> for UBig {
    type Output = UBig;

    #[inline]
    fn mul(self, rhs: UBig) -> UBig {
        ubig::mul_repr_val_val(self.into_repr(), rhs.into_repr())
    }
}

impl Mul<&UBig> for UBig {
    type Output = UBig;

    #[inline]
    fn mul(self, rhs: &UBig) -> UBig {
        ubig::mul_repr_ref_val(rhs.repr(), self.into_repr())
    }
}

impl Mul<UBig> for &UBig {
    type Output = UBig;

    #[inline]
    fn mul(self, rhs: UBig) -> UBig {
        ubig::mul_repr_ref_val(self.repr(), rhs.into_repr())
    }
}

impl Mul<&UBig> for &UBig {
    type Output = UBig;

    #[inline]
    fn mul(self, rhs: &UBig) -> UBig {
        ubig::mul_repr_ref_ref(self.repr(), rhs.repr())
    }
}

impl MulAssign<UBig> for UBig {
    #[inline]
    fn mul_assign(&mut self, rhs: UBig) {
        *self = mem::take(self) * rhs;
    }
}

impl MulAssign<&UBig> for UBig {
    #[inline]
    fn mul_assign(&mut self, rhs: &UBig) {
        *self = mem::take(self) * rhs;
    }
}

impl Mul<IBig> for IBig {
    type Output = IBig;

    #[inline]
    fn mul(self, rhs: IBig) -> IBig {
        let (sign0, mag0) = self.into_sign_repr();
        let (sign1, mag1) = rhs.into_sign_repr();
        IBig::from_sign_magnitude(sign0 * sign1, ubig::mul_repr_val_val(mag0, mag1))
    }
}

impl Mul<IBig> for &IBig {
    type Output = IBig;

    #[inline]
    fn mul(self, rhs: IBig) -> IBig {
        let (sign0, mag0) = self.as_sign_repr();
        let (sign1, mag1) = rhs.into_sign_repr();
        IBig::from_sign_magnitude(sign0 * sign1, ubig::mul_repr_ref_val(mag0, mag1))
    }
}

impl Mul<&IBig> for IBig {
    type Output = IBig;

    #[inline]
    fn mul(self, rhs: &IBig) -> IBig {
        rhs.mul(self)
    }
}

impl Mul<&IBig> for &IBig {
    type Output = IBig;

    #[inline]
    fn mul(self, rhs: &IBig) -> IBig {
        let (sign0, mag0) = self.as_sign_repr();
        let (sign1, mag1) = rhs.as_sign_repr();
        IBig::from_sign_magnitude(sign0 * sign1, ubig::mul_repr_ref_ref(mag0, mag1))
    }
}

impl MulAssign<IBig> for IBig {
    #[inline]
    fn mul_assign(&mut self, rhs: IBig) {
        *self = mem::take(self) * rhs;
    }
}

impl MulAssign<&IBig> for IBig {
    #[inline]
    fn mul_assign(&mut self, rhs: &IBig) {
        *self = mem::take(self) * rhs;
    }
}

impl Mul<Sign> for Sign {
    type Output = Sign;

    #[inline]
    fn mul(self, rhs: Sign) -> Sign {
        match (self, rhs) {
            (Positive, Positive) => Positive,
            (Positive, Negative) => Negative,
            (Negative, Positive) => Negative,
            (Negative, Negative) => Positive,
        }
    }
}

impl MulAssign<Sign> for Sign {
    #[inline]
    fn mul_assign(&mut self, rhs: Sign) {
        *self = *self * rhs;
    }
}

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

macro_rules! impl_mul_ubig_signed {
    ($t:ty) => {
        impl Mul<$t> for UBig {
            type Output = UBig;

            #[inline]
            fn mul(self, rhs: $t) -> UBig {
                UBig::from_ibig(IBig::from(self) * IBig::from_signed(rhs))
            }
        }

        impl Mul<$t> for &UBig {
            type Output = UBig;

            #[inline]
            fn mul(self, rhs: $t) -> UBig {
                UBig::from_ibig(IBig::from(self) * IBig::from_signed(rhs))
            }
        }

        helper_macros::forward_binop_second_arg_by_value!(impl Mul<$t> for UBig, mul);
        helper_macros::forward_binop_swap_args!(impl Mul<UBig> for $t, mul);

        impl MulAssign<$t> for UBig {
            #[inline]
            fn mul_assign(&mut self, rhs: $t) {
                *self = mem::take(self) * rhs
            }
        }

        helper_macros::forward_binop_assign_arg_by_value!(impl MulAssign<$t> for UBig, mul_assign);
    };
}

impl_mul_ubig_signed!(i8);
impl_mul_ubig_signed!(i16);
impl_mul_ubig_signed!(i32);
impl_mul_ubig_signed!(i64);
impl_mul_ubig_signed!(i128);
impl_mul_ubig_signed!(isize);

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

impl_mul_ibig_primitive!(u8);
impl_mul_ibig_primitive!(u16);
impl_mul_ibig_primitive!(u32);
impl_mul_ibig_primitive!(u64);
impl_mul_ibig_primitive!(u128);
impl_mul_ibig_primitive!(usize);
impl_mul_ibig_primitive!(i8);
impl_mul_ibig_primitive!(i16);
impl_mul_ibig_primitive!(i32);
impl_mul_ibig_primitive!(i64);
impl_mul_ibig_primitive!(i128);
impl_mul_ibig_primitive!(isize);

mod ubig {
    use crate::buffer::{TypedRepr, TypedReprRef};
    use crate::math;
    use crate::primitive::{split_dword, shrink_dword};
    use super::*;

    #[inline]
    pub(crate) fn mul_repr_val_val(lhs: TypedRepr, rhs: TypedRepr) -> UBig {
        match (lhs, rhs) {
            (Small(dword0), Small(dword1)) => ubig::mul_dword(dword0, dword1),
            (Small(dword0), Large(buffer1)) => ubig::mul_large_dword(buffer1, dword0),
            (Large(buffer0), Small(dword1)) => ubig::mul_large_dword(buffer0, dword1),
            (Large(buffer0), Large(buffer1)) => ubig::mul_large(&buffer0, &buffer1),
        }
    }

    #[inline]
    pub(crate) fn mul_repr_ref_val(lhs: TypedReprRef, rhs: TypedRepr) -> UBig {
        match (lhs, rhs) {
            (RefSmall(dword0), Small(dword1)) => ubig::mul_dword(dword0, dword1),
            (RefSmall(dword0), Large(buffer1)) => ubig::mul_large_dword(buffer1, dword0),
            (RefLarge(buffer0), Small(dword1)) => ubig::mul_large_dword(buffer0.into(), dword1),
            (RefLarge(buffer0), Large(buffer1)) => ubig::mul_large(buffer0, &buffer1),
        }
    }

    #[inline]
    pub(crate) fn mul_repr_ref_ref(lhs: TypedReprRef, rhs: TypedReprRef) -> UBig {
        match (lhs, rhs) {
            (RefSmall(dword0), RefSmall(dword1)) => ubig::mul_dword(dword0, dword1),
            (RefSmall(dword0), RefLarge(buffer1)) => ubig::mul_large_dword(buffer1.into(), dword0),
            (RefLarge(buffer0), RefSmall(dword1)) => ubig::mul_large_dword(buffer0.into(), dword1),
            (RefLarge(buffer0), RefLarge(buffer1)) => ubig::mul_large(buffer0, buffer1),
        }
    }

    /// Multiply two `DoubleWord`s.
    #[inline]
    fn mul_dword(a: DoubleWord, b: DoubleWord) -> UBig {
        if a <= Word::MAX as DoubleWord && b <= Word::MAX as DoubleWord {
            UBig::from(a * b)
        } else {
            mul_dword_slow(a, b)
        }
    }

    fn mul_dword_slow(lhs: DoubleWord, rhs: DoubleWord) -> UBig {
        let (lo, hi) = math::mul_add_carry_dword(lhs, rhs);
        let mut buffer = Buffer::allocate(4);
        let (n0, n1) = split_dword(lo);
        buffer.push(n0);
        buffer.push(n1);
        let (n2, n3) = split_dword(hi);
        buffer.push(n2);
        buffer.push(n3);
        buffer.into()
    }

    /// Multiply a large number by a `DoubleWord`.
    fn mul_large_dword(mut buffer: Buffer, rhs: DoubleWord) -> UBig {
        match rhs {
            0 => UBig::zero(),
            1 => buffer.into(),
            a if a <= Word::MAX as DoubleWord => {
                let carry = mul::mul_word_in_place(&mut buffer, shrink_dword(a));
                if carry != 0 {
                    buffer.push_resizing(carry);
                }
                buffer.into()
            },
            b => {
                let carry = mul::mul_dword_in_place(&mut buffer, b);
                if carry != 0 {
                    let (lo, hi) = split_dword(carry);
                    buffer.ensure_capacity(buffer.len() + 2);
                    buffer.push(lo);
                    buffer.push(hi);
                }
                buffer.into()
            }
        }
    }

    /// Multiply two large numbers.
    fn mul_large(lhs: &[Word], rhs: &[Word]) -> UBig {
        debug_assert!(lhs.len() >= 2 && rhs.len() >= 2);

        // This may be 1 too large.
        const_assert!(Buffer::MAX_CAPACITY - UBig::MAX_LEN >= 1);
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
        buffer.into()
    }
}
