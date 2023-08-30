//! Addition and subtraction operators.

use crate::{helper_macros, ibig::IBig, ubig::UBig, Sign::*};
use core::ops::{Add, AddAssign, Sub, SubAssign};

helper_macros::forward_ubig_binop_to_repr!(impl Add, add);
helper_macros::forward_ubig_binop_to_repr!(impl Sub, sub);
helper_macros::impl_binop_assign_by_taking!(impl AddAssign<UBig> for UBig, add_assign, add);
helper_macros::impl_binop_assign_by_taking!(impl SubAssign<UBig> for UBig, sub_assign, sub);

macro_rules! impl_ibig_add {
    ($sign0:ident, $mag0:ident, $sign1:ident, $mag1:ident) => {
        match ($sign0, $sign1) {
            (Positive, Positive) => IBig($mag0.add($mag1)),
            (Positive, Negative) => IBig($mag0.sub_signed($mag1)),
            (Negative, Positive) => IBig($mag1.sub_signed($mag0)),
            (Negative, Negative) => IBig($mag0.add($mag1).with_sign(Negative)),
        }
    };
}
macro_rules! impl_ibig_sub {
    ($sign0:ident, $mag0:ident, $sign1:ident, $mag1:ident) => {
        match ($sign0, $sign1) {
            (Positive, Positive) => IBig($mag0.sub_signed($mag1)),
            (Positive, Negative) => IBig($mag0.add($mag1)),
            (Negative, Positive) => IBig($mag0.add($mag1).with_sign(Negative)),
            (Negative, Negative) => IBig($mag1.sub_signed($mag0)),
        }
    };
}
helper_macros::forward_ibig_binop_to_repr!(impl Add, add, Output = IBig, impl_ibig_add);
helper_macros::forward_ibig_binop_to_repr!(impl Sub, sub, Output = IBig, impl_ibig_sub);
helper_macros::impl_binop_assign_by_taking!(impl AddAssign<IBig> for IBig, add_assign, add);
helper_macros::impl_binop_assign_by_taking!(impl SubAssign<IBig> for IBig, sub_assign, sub);

macro_rules! impl_ubig_ibig_add {
    ($mag0:ident, $sign1:ident, $mag1:ident) => {
        match ($sign1) {
            Positive => IBig($mag0.add($mag1)),
            Negative => IBig($mag0.sub_signed($mag1)),
        }
    };
}
macro_rules! impl_ubig_ibig_sub {
    ($mag0:ident, $sign1:ident, $mag1:ident) => {
        match ($sign1) {
            Positive => IBig($mag0.sub_signed($mag1)),
            Negative => IBig($mag0.add($mag1)),
        }
    };
}
helper_macros::forward_ubig_ibig_binop_to_repr!(impl Add, add, Output = IBig, impl_ubig_ibig_add);
helper_macros::forward_ubig_ibig_binop_to_repr!(impl Sub, sub, Output = IBig, impl_ubig_ibig_sub);

macro_rules! impl_ibig_ubig_add {
    ($sign0:ident, $mag0:ident, $mag1:ident) => {
        match ($sign0) {
            Positive => IBig($mag0.add($mag1)),
            Negative => IBig($mag1.sub_signed($mag0)),
        }
    };
}
macro_rules! impl_ibig_ubig_sub {
    ($sign0:ident, $mag0:ident, $mag1:ident) => {
        match ($sign0) {
            Positive => IBig($mag0.sub_signed($mag1)),
            Negative => IBig($mag0.add($mag1).with_sign(Negative)),
        }
    };
}
helper_macros::forward_ibig_ubig_binop_to_repr!(impl Add, add, Output = IBig, impl_ibig_ubig_add);
helper_macros::forward_ibig_ubig_binop_to_repr!(impl Sub, sub, Output = IBig, impl_ibig_ubig_sub);
helper_macros::impl_binop_assign_by_taking!(impl AddAssign<UBig> for IBig, add_assign, add);
helper_macros::impl_binop_assign_by_taking!(impl SubAssign<UBig> for IBig, sub_assign, sub);

// Ops with primitives

macro_rules! impl_add_sub_primitive_with_ubig {
    ($($t:ty)*) => {$(
        helper_macros::impl_commutative_binop_with_primitive!(impl Add<$t> for UBig, add);
        helper_macros::impl_binop_assign_with_primitive!(impl AddAssign<$t> for UBig, add_assign);
        helper_macros::impl_commutative_binop_with_primitive!(impl Sub<$t> for UBig, sub);
        helper_macros::impl_binop_assign_with_primitive!(impl SubAssign<$t> for UBig, sub_assign);
    )*};
}
impl_add_sub_primitive_with_ubig!(u8 u16 u32 u64 u128 usize);

macro_rules! impl_add_sub_primitive_with_ibig {
    ($($t:ty)*) => {$(
        helper_macros::impl_commutative_binop_with_primitive!(impl Add<$t> for IBig, add);
        helper_macros::impl_binop_assign_with_primitive!(impl AddAssign<$t> for IBig, add_assign);
        helper_macros::impl_commutative_binop_with_primitive!(impl Sub<$t> for IBig, sub);
        helper_macros::impl_binop_assign_with_primitive!(impl SubAssign<$t> for IBig, sub_assign);
    )*};
}
impl_add_sub_primitive_with_ibig!(u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize);

pub mod repr {
    use super::*;
    use crate::{
        add,
        arch::word::{DoubleWord, Word},
        buffer::Buffer,
        error::panic_negative_ubig,
        primitive::split_dword,
        repr::{
            Repr,
            TypedRepr::{self, *},
            TypedReprRef::{self, *},
        },
    };

    impl<'l, 'r> Add<TypedReprRef<'r>> for TypedReprRef<'l> {
        type Output = Repr;
        #[inline]
        fn add(self, rhs: TypedReprRef) -> Repr {
            match (self, rhs) {
                (RefSmall(dword0), RefSmall(dword1)) => add_dword(dword0, dword1),
                (RefSmall(dword0), RefLarge(words1)) => add_large_dword(words1.into(), dword0),
                (RefLarge(words0), RefSmall(dword1)) => add_large_dword(words0.into(), dword1),
                (RefLarge(words0), RefLarge(words1)) => {
                    if words0.len() >= words1.len() {
                        add_large(words0.into(), words1)
                    } else {
                        add_large(words1.into(), words0)
                    }
                }
            }
        }
    }

    impl<'l> Add<TypedRepr> for TypedReprRef<'l> {
        type Output = Repr;
        #[inline]
        fn add(self, rhs: TypedRepr) -> Repr {
            match (self, rhs) {
                (RefSmall(dword0), Small(dword1)) => add_dword(dword0, dword1),
                (RefSmall(dword0), Large(buffer1)) => add_large_dword(buffer1, dword0),
                (RefLarge(words0), Small(dword1)) => add_large_dword(words0.into(), dword1),
                (RefLarge(words0), Large(buffer1)) => add_large(buffer1, words0),
            }
        }
    }

    impl<'r> Add<TypedReprRef<'r>> for TypedRepr {
        type Output = Repr;
        #[inline]
        fn add(self, rhs: TypedReprRef) -> Repr {
            // add is commutative
            rhs.add(self)
        }
    }

    impl Add<TypedRepr> for TypedRepr {
        type Output = Repr;
        #[inline]
        fn add(self, rhs: TypedRepr) -> Repr {
            match (self, rhs) {
                (Small(dword0), Small(dword1)) => add_dword(dword0, dword1),
                (Small(dword0), Large(buffer1)) => add_large_dword(buffer1, dword0),
                (Large(buffer0), Small(dword1)) => add_large_dword(buffer0, dword1),
                (Large(buffer0), Large(buffer1)) => {
                    if buffer0.len() >= buffer1.len() {
                        add_large(buffer0, &buffer1)
                    } else {
                        add_large(buffer1, &buffer0)
                    }
                }
            }
        }
    }

    #[inline]
    fn add_dword(a: DoubleWord, b: DoubleWord) -> Repr {
        let (res, overflow) = a.overflowing_add(b);
        if overflow {
            // spilled
            let (lo, hi) = split_dword(res);
            let mut buffer = Buffer::allocate(3);
            buffer.push(lo);
            buffer.push(hi);
            buffer.push(1);
            Repr::from_buffer(buffer)
        } else {
            Repr::from_dword(res)
        }
    }

    #[inline]
    fn add_large_dword(mut buffer: Buffer, rhs: DoubleWord) -> Repr {
        debug_assert!(buffer.len() >= 3);
        if add::add_dword_in_place(&mut buffer, rhs) {
            buffer.push_resizing(1);
        }
        Repr::from_buffer(buffer)
    }

    #[inline]
    fn add_large(mut buffer: Buffer, rhs: &[Word]) -> Repr {
        let n = buffer.len().min(rhs.len());
        let overflow = add::add_same_len_in_place(&mut buffer[..n], &rhs[..n]);
        if rhs.len() > n {
            buffer.ensure_capacity(rhs.len());
            buffer.push_slice(&rhs[n..]);
        }
        if overflow && add::add_one_in_place(&mut buffer[n..]) {
            buffer.push_resizing(1);
        }
        Repr::from_buffer(buffer)
    }

    impl<'l, 'r> Sub<TypedReprRef<'r>> for TypedReprRef<'l> {
        type Output = Repr;
        #[inline]
        fn sub(self, rhs: TypedReprRef) -> Repr {
            match (self, rhs) {
                (RefSmall(dword0), RefSmall(dword1)) => sub_dword(dword0, dword1),
                (RefSmall(_), RefLarge(_)) => panic_negative_ubig(),
                (RefLarge(buffer0), RefSmall(dword1)) => sub_large_dword(buffer0.into(), dword1),
                (RefLarge(buffer0), RefLarge(buffer1)) => sub_large(buffer0.into(), buffer1),
            }
        }
    }

    impl<'r> Sub<TypedReprRef<'r>> for TypedRepr {
        type Output = Repr;
        #[inline]
        fn sub(self, rhs: TypedReprRef) -> Repr {
            match (self, rhs) {
                (Small(dword0), RefSmall(dword1)) => sub_dword(dword0, dword1),
                (Small(_), RefLarge(_)) => panic_negative_ubig(),
                (Large(buffer0), RefSmall(dword1)) => sub_large_dword(buffer0, dword1),
                (Large(buffer0), RefLarge(buffer1)) => sub_large(buffer0, buffer1),
            }
        }
    }

    impl<'l> Sub<TypedRepr> for TypedReprRef<'l> {
        type Output = Repr;
        #[inline]
        fn sub(self, rhs: TypedRepr) -> Repr {
            match (self, rhs) {
                (RefSmall(dword0), Small(dword1)) => sub_dword(dword0, dword1),
                (RefSmall(_), Large(_)) => panic_negative_ubig(),
                (RefLarge(buffer0), Small(dword1)) => sub_large_dword(buffer0.into(), dword1),
                (RefLarge(buffer0), Large(buffer1)) => sub_large_ref_val(buffer0, buffer1),
            }
        }
    }

    impl Sub<TypedRepr> for TypedRepr {
        type Output = Repr;
        #[inline]
        fn sub(self, rhs: TypedRepr) -> Repr {
            match (self, rhs) {
                (Small(dword0), Small(dword1)) => sub_dword(dword0, dword1),
                (Small(_), Large(_)) => panic_negative_ubig(),
                (Large(buffer0), Small(dword1)) => sub_large_dword(buffer0, dword1),
                (Large(buffer0), Large(buffer1)) => sub_large(buffer0, &buffer1),
            }
        }
    }

    #[inline]
    fn sub_dword(a: DoubleWord, b: DoubleWord) -> Repr {
        match a.checked_sub(b) {
            Some(res) => Repr::from_dword(res),
            None => panic_negative_ubig(),
        }
    }

    #[inline]
    pub(crate) fn sub_large_dword(mut lhs: Buffer, rhs: DoubleWord) -> Repr {
        let overflow = add::sub_dword_in_place(&mut lhs, rhs);
        debug_assert!(!overflow);
        Repr::from_buffer(lhs)
    }

    #[inline]
    pub(crate) fn sub_large(mut lhs: Buffer, rhs: &[Word]) -> Repr {
        if lhs.len() < rhs.len() || add::sub_in_place(&mut lhs, rhs) {
            panic_negative_ubig();
        }
        Repr::from_buffer(lhs)
    }

    #[inline]
    pub(crate) fn sub_large_ref_val(lhs: &[Word], mut rhs: Buffer) -> Repr {
        let n = rhs.len();
        if lhs.len() < n {
            panic_negative_ubig();
        }
        let borrow = add::sub_same_len_in_place_swap(&lhs[..n], &mut rhs);
        rhs.ensure_capacity(lhs.len());
        rhs.push_slice(&lhs[n..]);
        if borrow && add::sub_one_in_place(&mut rhs[n..]) {
            panic_negative_ubig();
        }
        Repr::from_buffer(rhs)
    }

    impl<'a> TypedReprRef<'a> {
        /// Add one to the number
        pub fn add_one(self) -> Repr {
            match self {
                RefSmall(dword) => add_dword(dword, 1),
                RefLarge(buffer) => add_large_one(buffer.into()),
            }
        }

        /// Subtract one from the number
        pub fn sub_one(self) -> Repr {
            match self {
                RefSmall(dword) => Repr::from_dword(dword - 1),
                RefLarge(buffer) => sub_large_one(buffer.into()),
            }
        }
    }

    impl TypedRepr {
        /// Add one to the number
        pub fn add_one(self) -> Repr {
            match self {
                Small(dword) => add_dword(dword, 1),
                Large(buffer) => add_large_one(buffer),
            }
        }

        /// Subtract one from the number
        pub fn sub_one(self) -> Repr {
            match self {
                Small(dword) => Repr::from_dword(dword - 1),
                Large(buffer) => sub_large_one(buffer),
            }
        }
    }

    #[inline]
    fn add_large_one(mut buffer: Buffer) -> Repr {
        if add::add_one_in_place(&mut buffer) {
            buffer.push_resizing(1);
        }
        Repr::from_buffer(buffer)
    }

    #[inline]
    fn sub_large_one(mut buffer: Buffer) -> Repr {
        let overflow = add::sub_one_in_place(&mut buffer);
        debug_assert!(!overflow);
        Repr::from_buffer(buffer)
    }
}

/// This trait is for internal use only, it's used for distinguishing
/// between subtraction with sign and without sign.
trait SubSigned<Rhs> {
    type Output;
    fn sub_signed(self, rhs: Rhs) -> Self::Output;
}

mod repr_signed {
    use super::*;
    use crate::{
        add,
        arch::word::{DoubleWord, Word},
        buffer::Buffer,
        repr::{
            Repr,
            TypedRepr::{self, *},
            TypedReprRef::{self, *},
        },
    };

    impl<'l, 'r> SubSigned<TypedReprRef<'r>> for TypedReprRef<'l> {
        type Output = Repr;
        #[inline]
        fn sub_signed(self, rhs: TypedReprRef<'r>) -> Repr {
            match (self, rhs) {
                (RefSmall(dword0), RefSmall(dword1)) => sub_dword(dword0, dword1),
                (RefSmall(dword0), RefLarge(buffer1)) => {
                    sub_large_dword(buffer1.into(), dword0).neg()
                }
                (RefLarge(words0), RefSmall(words1)) => sub_large_dword(words0.into(), words1),
                (RefLarge(words0), RefLarge(words1)) => {
                    if words0.len() >= words1.len() {
                        sub_large(words0.into(), words1)
                    } else {
                        sub_large(words1.into(), words0).neg()
                    }
                }
            }
        }
    }

    impl<'l> SubSigned<TypedRepr> for TypedReprRef<'l> {
        type Output = Repr;
        #[inline]
        fn sub_signed(self, rhs: TypedRepr) -> Self::Output {
            match (self, rhs) {
                (RefSmall(dword0), Small(dword1)) => sub_dword(dword0, dword1),
                (RefSmall(dword0), Large(buffer1)) => sub_large_dword(buffer1, dword0).neg(),
                (RefLarge(words0), Small(dword1)) => sub_large_dword(words0.into(), dword1),
                (RefLarge(words0), Large(buffer1)) => sub_large(buffer1, words0).neg(),
            }
        }
    }

    impl<'r> SubSigned<TypedReprRef<'r>> for TypedRepr {
        type Output = Repr;
        #[inline]
        fn sub_signed(self, rhs: TypedReprRef) -> Self::Output {
            match (self, rhs) {
                (Small(dword0), RefSmall(dword1)) => sub_dword(dword0, dword1),
                (Small(dword0), RefLarge(words1)) => sub_large_dword(words1.into(), dword0).neg(),
                (Large(buffer0), RefSmall(dword1)) => sub_large_dword(buffer0, dword1),
                (Large(buffer0), RefLarge(words1)) => sub_large(buffer0, words1),
            }
        }
    }

    impl SubSigned<TypedRepr> for TypedRepr {
        type Output = Repr;
        #[inline]
        fn sub_signed(self, rhs: TypedRepr) -> Self::Output {
            match (self, rhs) {
                (Small(dword0), Small(dword1)) => sub_dword(dword0, dword1),
                (Small(dword0), Large(buffer1)) => sub_large_dword(buffer1, dword0).neg(),
                (Large(buffer0), Small(dword1)) => sub_large_dword(buffer0, dword1),
                (Large(buffer0), Large(buffer1)) => {
                    if buffer0.len() >= buffer1.len() {
                        sub_large(buffer0, &buffer1)
                    } else {
                        sub_large(buffer1, &buffer0).neg()
                    }
                }
            }
        }
    }

    #[inline]
    fn sub_dword(lhs: DoubleWord, rhs: DoubleWord) -> Repr {
        let (val, overflow) = lhs.overflowing_sub(rhs);
        if !overflow {
            Repr::from_dword(val)
        } else {
            Repr::from_dword(val.wrapping_neg()).neg()
        }
    }

    #[inline]
    fn sub_large_dword(lhs: Buffer, rhs: DoubleWord) -> Repr {
        super::repr::sub_large_dword(lhs, rhs)
    }

    #[inline]
    fn sub_large(mut lhs: Buffer, rhs: &[Word]) -> Repr {
        if lhs.len() >= rhs.len() {
            let sign = add::sub_in_place_with_sign(&mut lhs, rhs);
            Repr::from_buffer(lhs).with_sign(sign)
        } else {
            super::repr::sub_large_ref_val(rhs, lhs).with_sign(Negative)
        }
    }
}
