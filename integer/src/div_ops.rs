//! Division operators.

use crate::{
    arch::word::{DoubleWord, Word},
    div, helper_macros,
    ibig::IBig,
    memory::MemoryAllocation,
    ops::{Abs, DivEuclid, DivRem, DivRemEuclid, RemEuclid},
    repr::{Buffer, TypedRepr::*, TypedReprRef::*},
    shift,
    sign::Sign::*,
    ubig::UBig,
};
use core::{
    convert::TryFrom,
    ops::{Div, DivAssign, Rem, RemAssign},
};

helper_macros::forward_ubig_binop_to_repr!(impl Div, div);
helper_macros::forward_ubig_binop_to_repr!(impl Rem, rem);
helper_macros::forward_ubig_binop_to_repr!(impl DivRem as divrem, div_rem);
helper_macros::forward_ubig_binop_to_repr!(impl DivEuclid, div_euclid, div);
helper_macros::forward_ubig_binop_to_repr!(impl RemEuclid, rem_euclid, rem);
helper_macros::forward_ubig_binop_to_repr!(impl DivRemEuclid as divrem, div_rem_euclid, div_rem);
helper_macros::forward_binop_assign_by_taking!(impl DivAssign<UBig> for UBig, div_assign, div);
helper_macros::forward_binop_assign_by_taking!(impl RemAssign<UBig> for UBig, rem_assign, rem);

macro_rules! impl_ibig_div {
    ($sign0:ident, $mag0:ident, $sign1:ident, $mag1:ident) => {
        // truncate towards 0.
        IBig($mag0.div($mag1).with_sign($sign0 * $sign1))
    };
}
macro_rules! impl_ibig_rem {
    ($sign0:ident, $mag0:ident, $sign1:ident, $mag1:ident) => {{
        let _sign1 = $sign1; // unused
        // remainder with truncating division has same sign as lhs.
        IBig($mag0.rem($mag1).with_sign($sign0))
    }};
}
helper_macros::forward_ibig_binop_to_repr!(impl Div, div, impl_ibig_div);
helper_macros::forward_ibig_binop_to_repr!(impl Rem, rem, impl_ibig_rem);

helper_macros::forward_binop_assign_by_taking!(impl DivAssign<IBig> for IBig, div_assign, div);
helper_macros::forward_binop_assign_by_taking!(impl RemAssign<IBig> for IBig, rem_assign, rem);

macro_rules! impl_ibig_div_rem {
    ($sign0:ident, $mag0:ident, $sign1:ident, $mag1:ident) => {{
        // truncate towards 0.
        let (q, r) = $mag0.div_rem($mag1);
        (
            IBig(q.with_sign($sign0 * $sign1)),
            IBig(r.with_sign($sign0)),
        )
    }};
}
helper_macros::forward_ibig_binop_to_repr!(impl DivRem as divrem, div_rem, impl_ibig_div_rem);

impl DivEuclid<IBig> for IBig {
    type Output = IBig;

    #[inline]
    fn div_euclid(self, rhs: IBig) -> IBig {
        let s = rhs.signum();
        let (q, r) = self.div_rem(rhs);
        match r.sign() {
            Positive => q,
            Negative => q - s,
        }
    }
}

impl DivEuclid<&IBig> for IBig {
    type Output = IBig;

    #[inline]
    fn div_euclid(self, rhs: &IBig) -> IBig {
        let (q, r) = self.div_rem(rhs);
        match r.sign() {
            Positive => q,
            Negative => q - rhs.signum(),
        }
    }
}

impl DivEuclid<IBig> for &IBig {
    type Output = IBig;

    #[inline]
    fn div_euclid(self, rhs: IBig) -> IBig {
        let s = rhs.signum();
        let (q, r) = self.div_rem(rhs);
        match r.sign() {
            Positive => q,
            Negative => q - s,
        }
    }
}

impl DivEuclid<&IBig> for &IBig {
    type Output = IBig;

    #[inline]
    fn div_euclid(self, rhs: &IBig) -> IBig {
        let (q, r) = self.div_rem(rhs);
        match r.sign() {
            Positive => q,
            Negative => q - rhs.signum(),
        }
    }
}

impl RemEuclid<IBig> for IBig {
    type Output = IBig;

    #[inline]
    fn rem_euclid(self, rhs: IBig) -> IBig {
        let r = self % &rhs;
        match r.sign() {
            Positive => r,
            Negative => r + rhs.abs(),
        }
    }
}

impl RemEuclid<&IBig> for IBig {
    type Output = IBig;

    #[inline]
    fn rem_euclid(self, rhs: &IBig) -> IBig {
        let r = self % rhs;
        match r.sign() {
            Positive => r,
            Negative => r + rhs.abs(),
        }
    }
}

impl RemEuclid<IBig> for &IBig {
    type Output = IBig;

    #[inline]
    fn rem_euclid(self, rhs: IBig) -> IBig {
        let r = self % &rhs;
        match r.sign() {
            Positive => r,
            Negative => r + rhs.abs(),
        }
    }
}

impl RemEuclid<&IBig> for &IBig {
    type Output = IBig;

    #[inline]
    fn rem_euclid(self, rhs: &IBig) -> IBig {
        let r = self % rhs;
        match r.sign() {
            Positive => r,
            Negative => r + rhs.abs(),
        }
    }
}

impl DivRemEuclid<IBig> for IBig {
    type OutputDiv = IBig;
    type OutputRem = IBig;

    #[inline]
    fn div_rem_euclid(self, rhs: IBig) -> (IBig, IBig) {
        let (q, r) = self.div_rem(&rhs);
        match r.sign() {
            Positive => (q, r),
            Negative => (q - rhs.signum(), r + rhs.abs()),
        }
    }
}

impl DivRemEuclid<&IBig> for IBig {
    type OutputDiv = IBig;
    type OutputRem = IBig;

    #[inline]
    fn div_rem_euclid(self, rhs: &IBig) -> (IBig, IBig) {
        let (q, r) = self.div_rem(rhs);
        match r.sign() {
            Positive => (q, r),
            Negative => (q - rhs.signum(), r + rhs.abs()),
        }
    }
}

impl DivRemEuclid<IBig> for &IBig {
    type OutputDiv = IBig;
    type OutputRem = IBig;

    #[inline]
    fn div_rem_euclid(self, rhs: IBig) -> (IBig, IBig) {
        let (q, r) = self.div_rem(&rhs);
        match r.sign() {
            Positive => (q, r),
            Negative => (q - rhs.signum(), r + rhs.abs()),
        }
    }
}

impl DivRemEuclid<&IBig> for &IBig {
    type OutputDiv = IBig;
    type OutputRem = IBig;

    #[inline]
    fn div_rem_euclid(self, rhs: &IBig) -> (IBig, IBig) {
        let (q, r) = self.div_rem(rhs);
        match r.sign() {
            Positive => (q, r),
            Negative => (q - rhs.signum(), r + rhs.abs()),
        }
    }
}

macro_rules! impl_div_ubig_unsigned {
    ($t:ty) => {
        impl Div<$t> for UBig {
            type Output = UBig;

            #[inline]
            fn div(self, rhs: $t) -> UBig {
                self / UBig::from_unsigned(rhs)
            }
        }

        impl Div<$t> for &UBig {
            type Output = UBig;

            #[inline]
            fn div(self, rhs: $t) -> UBig {
                self / UBig::from_unsigned(rhs)
            }
        }

        helper_macros::forward_binop_second_arg_by_value!(impl Div<$t> for UBig, div);

        impl DivAssign<$t> for UBig {
            #[inline]
            fn div_assign(&mut self, rhs: $t) {
                self.div_assign(UBig::from_unsigned(rhs))
            }
        }

        helper_macros::forward_binop_assign_arg_by_value!(impl DivAssign<$t> for UBig, div_assign);

        impl Rem<$t> for UBig {
            type Output = $t;

            #[inline]
            fn rem(self, rhs: $t) -> $t {
                (self % UBig::from_unsigned(rhs)).try_to_unsigned().unwrap()
            }
        }

        impl Rem<$t> for &UBig {
            type Output = $t;

            #[inline]
            fn rem(self, rhs: $t) -> $t {
                (self % UBig::from_unsigned(rhs)).try_to_unsigned().unwrap()
            }
        }

        helper_macros::forward_binop_second_arg_by_value!(impl Rem<$t> for UBig, rem);

        impl RemAssign<$t> for UBig {
            #[inline]
            fn rem_assign(&mut self, rhs: $t) {
                self.rem_assign(UBig::from_unsigned(rhs))
            }
        }

        helper_macros::forward_binop_assign_arg_by_value!(impl RemAssign<$t> for UBig, rem_assign);

        impl DivRem<$t> for UBig {
            type OutputDiv = UBig;
            type OutputRem = $t;

            #[inline]
            fn div_rem(self, rhs: $t) -> (UBig, $t) {
                let (q, r) = self.div_rem(UBig::from_unsigned(rhs));
                (q, r.try_to_unsigned().unwrap())
            }
        }

        impl DivRem<$t> for &UBig {
            type OutputDiv = UBig;
            type OutputRem = $t;

            #[inline]
            fn div_rem(self, rhs: $t) -> (UBig, $t) {
                let (q, r) = self.div_rem(UBig::from_unsigned(rhs));
                (q, r.try_to_unsigned().unwrap())
            }
        }

        helper_macros::forward_div_rem_second_arg_by_value!(impl DivRem<$t> for UBig, div_rem);

         impl DivEuclid<$t> for UBig {
            type Output = UBig;

            #[inline]
            fn div_euclid(self, rhs: $t) -> UBig {
                UBig::from_ibig(IBig::from(self) / IBig::from_unsigned(rhs))
            }
        }

        impl DivEuclid<$t> for &UBig {
            type Output = UBig;

            #[inline]
            fn div_euclid(self, rhs: $t) -> UBig {
                UBig::from_ibig(IBig::from(self) / IBig::from_unsigned(rhs))
            }
        }
        helper_macros::forward_binop_second_arg_by_value!(impl DivEuclid<$t> for UBig, div_euclid);

        impl RemEuclid<$t> for UBig {
            type Output = $t;

            #[inline]
            fn rem_euclid(self, rhs: $t) -> $t {
                self % rhs
            }
        }

        impl RemEuclid<$t> for &UBig {
            type Output = $t;

            #[inline]
            fn rem_euclid(self, rhs: $t) -> $t {
                self % rhs
            }
        }

        helper_macros::forward_binop_second_arg_by_value!(impl RemEuclid<$t> for UBig, rem_euclid);

        impl DivRemEuclid<$t> for UBig {
            type OutputDiv = UBig;
            type OutputRem = $t;

            #[inline]
            fn div_rem_euclid(self, rhs: $t) -> (UBig, $t) {
                self.div_rem(rhs)
            }
        }

        impl DivRemEuclid<$t> for &UBig {
            type OutputDiv = UBig;
            type OutputRem = $t;

            #[inline]
            fn div_rem_euclid(self, rhs: $t) -> (UBig, $t) {
                self.div_rem(rhs)
            }
        }

        helper_macros::forward_div_rem_second_arg_by_value!(impl DivRemEuclid<$t> for UBig, div_rem_euclid);
    };
}

impl_div_ubig_unsigned!(u8);
impl_div_ubig_unsigned!(u16);
impl_div_ubig_unsigned!(u32);
impl_div_ubig_unsigned!(u64);
impl_div_ubig_unsigned!(u128);
impl_div_ubig_unsigned!(usize);

macro_rules! impl_div_ibig_signed {
    ($t:ty) => {
        impl Rem<$t> for IBig {
            type Output = $t;

            #[inline]
            fn rem(self, rhs: $t) -> $t {
                (self % IBig::from_signed(rhs)).try_to_signed().unwrap()
            }
        }

        impl Rem<$t> for &IBig {
            type Output = $t;

            #[inline]
            fn rem(self, rhs: $t) -> $t {
                (self % IBig::from_signed(rhs)).try_to_signed().unwrap()
            }
        }

        impl DivRem<$t> for IBig {
            type OutputDiv = IBig;
            type OutputRem = $t;

            #[inline]
            fn div_rem(self, rhs: $t) -> (IBig, $t) {
                let (q, r) = self.div_rem(IBig::from_signed(rhs));
                (q, r.try_to_signed().unwrap())
            }
        }

        impl DivRem<$t> for &IBig {
            type OutputDiv = IBig;
            type OutputRem = $t;

            #[inline]
            fn div_rem(self, rhs: $t) -> (IBig, $t) {
                let (q, r) = self.div_rem(IBig::from_signed(rhs));
                (q, r.try_to_signed().unwrap())
            }
        }

        impl_div_ibig_primitive!($t);
    };
}

macro_rules! impl_div_ibig_primitive {
    ($t:ty) => {
        impl Div<$t> for IBig {
            type Output = IBig;

            #[inline]
            fn div(self, rhs: $t) -> IBig {
                self.div(IBig::from(rhs))
            }
        }

        impl Div<$t> for &IBig {
            type Output = IBig;

            #[inline]
            fn div(self, rhs: $t) -> IBig {
                self.div(IBig::from(rhs))
            }
        }

        helper_macros::forward_binop_second_arg_by_value!(impl Div<$t> for IBig, div);

        impl DivAssign<$t> for IBig {
            #[inline]
            fn div_assign(&mut self, rhs: $t) {
                self.div_assign(IBig::from(rhs))
            }
        }

        helper_macros::forward_binop_assign_arg_by_value!(impl DivAssign<$t> for IBig, div_assign);

        helper_macros::forward_binop_second_arg_by_value!(impl Rem<$t> for IBig, rem);

        impl RemAssign<$t> for IBig {
            #[inline]
            fn rem_assign(&mut self, rhs: $t) {
                self.rem_assign(IBig::from(rhs))
            }
        }

        helper_macros::forward_binop_assign_arg_by_value!(impl RemAssign<$t> for IBig, rem_assign);

        helper_macros::forward_div_rem_second_arg_by_value!(impl DivRem<$t> for IBig, div_rem);

        impl DivEuclid<$t> for IBig {
            type Output = IBig;

            #[inline]
            fn div_euclid(self, rhs: $t) -> IBig {
                self.div_euclid(IBig::from(rhs))
            }
        }

        impl DivEuclid<$t> for &IBig {
            type Output = IBig;

            #[inline]
            fn div_euclid(self, rhs: $t) -> IBig {
                self.div_euclid(IBig::from(rhs))
            }
        }

        helper_macros::forward_binop_second_arg_by_value!(impl DivEuclid<$t> for IBig, div_euclid);

        impl RemEuclid<$t> for IBig {
            type Output = $t;

            #[inline]
            fn rem_euclid(self, rhs: $t) -> $t {
                <$t>::try_from(self.rem_euclid(IBig::from(rhs))).unwrap()
            }
        }

        impl RemEuclid<$t> for &IBig {
            type Output = $t;

            #[inline]
            fn rem_euclid(self, rhs: $t) -> $t {
                <$t>::try_from(self.rem_euclid(IBig::from(rhs))).unwrap()
            }
        }

        helper_macros::forward_binop_second_arg_by_value!(impl RemEuclid<$t> for IBig, rem_euclid);

        impl DivRemEuclid<$t> for IBig {
            type OutputDiv = IBig;
            type OutputRem = $t;

            #[inline]
            fn div_rem_euclid(self, rhs: $t) -> (IBig, $t) {
                let (q, r) = self.div_rem_euclid(IBig::from(rhs));
                (q, <$t>::try_from(r).unwrap())
            }
        }

        impl DivRemEuclid<$t> for &IBig {
            type OutputDiv = IBig;
            type OutputRem = $t;

            #[inline]
            fn div_rem_euclid(self, rhs: $t) -> (IBig, $t) {
                let (q, r) = self.div_rem_euclid(IBig::from(rhs));
                (q, <$t>::try_from(r).unwrap())
            }
        }

        helper_macros::forward_div_rem_second_arg_by_value!(impl DivRemEuclid<$t> for IBig, div_rem_euclid);
    };
}

impl_div_ibig_signed!(i8);
impl_div_ibig_signed!(i16);
impl_div_ibig_signed!(i32);
impl_div_ibig_signed!(i64);
impl_div_ibig_signed!(i128);
impl_div_ibig_signed!(isize);

mod ubig {
    use super::*;
    use crate::{
        primitive::shrink_dword,
        repr::{TypedRepr, TypedReprRef, Repr},
    };

    impl DivRem<TypedRepr> for TypedRepr {
        type OutputDiv = Repr;
        type OutputRem = Repr;

        #[inline]
        fn div_rem(self, rhs: TypedRepr) -> (Repr, Repr) {
            match (self, rhs) {
                (Small(dword0), Small(dword1)) => div_rem_dword(dword0, dword1),
                (Small(dword0), Large(_)) => (Repr::zero(), Repr::from_dword(dword0)),
                (Large(buffer0), Small(dword1)) => div_rem_large_dword(buffer0, dword1),
                (Large(buffer0), Large(buffer1)) => {
                    if buffer0.len() >= buffer1.len() {
                        div_rem_large(buffer0, buffer1)
                    } else {
                        (Repr::zero(), Repr::from_buffer(buffer0.into()))
                    }
                }
            }
        }
    }

    impl<'l> DivRem<TypedRepr> for TypedReprRef<'l> {
        type OutputDiv = Repr;
        type OutputRem = Repr;

        #[inline]
        fn div_rem(self, rhs: TypedRepr) -> (Repr, Repr) {
            match (self, rhs) {
                (RefSmall(dword0), Small(dword1)) => div_rem_dword(dword0, dword1),
                (RefSmall(dword0), Large(_)) => (Repr::zero(), Repr::from_dword(dword0)),
                (RefLarge(buffer0), Small(dword1)) => div_rem_large_dword(buffer0.into(), dword1),
                (RefLarge(buffer0), Large(mut buffer1)) => {
                    if buffer0.len() >= buffer1.len() {
                        div_rem_large(buffer0.into(), buffer1)
                    } else {
                        // Reuse buffer1 for the remainder.
                        buffer1.clone_from_slice(buffer0);
                        (Repr::zero(), Repr::from_buffer(buffer1))
                    }
                }
            }
        }
    }

    impl<'r> DivRem<TypedReprRef<'r>> for TypedRepr {
        type OutputDiv = Repr;
        type OutputRem = Repr;

        #[inline]
        fn div_rem(self, rhs: TypedReprRef) -> (Repr, Repr) {
            match (self, rhs) {
                (Small(dword0), RefSmall(dword1)) => div_rem_dword(dword0, dword1),
                (Small(dword0), RefLarge(_)) => (Repr::zero(), Repr::from_dword(dword0)),
                (Large(buffer0), RefSmall(dword1)) => div_rem_large_dword(buffer0, dword1),
                (Large(buffer0), RefLarge(buffer1)) => {
                    if buffer0.len() >= buffer1.len() {
                        div_rem_large(buffer0, buffer1.into())
                    } else {
                        (Repr::zero(), Repr::from_buffer(buffer0))
                    }
                }
            }
        }
    }

    impl<'l, 'r> DivRem<TypedReprRef<'r>> for TypedReprRef<'l> {
        type OutputDiv = Repr;
        type OutputRem = Repr;

        #[inline]
        fn div_rem(self, rhs: TypedReprRef) -> (Repr, Repr) {
            match (self, rhs) {
                (RefSmall(dword0), RefSmall(dword1)) => div_rem_dword(dword0, dword1),
                (RefSmall(dword0), RefLarge(_)) => (Repr::zero(), Repr::from_dword(dword0)),
                (RefLarge(buffer0), RefSmall(dword1)) => div_rem_large_dword(buffer0.into(), dword1),
                (RefLarge(buffer0), RefLarge(buffer1)) => {
                    if buffer0.len() >= buffer1.len() {
                        div_rem_large(buffer0.into(), buffer1.into())
                    } else {
                        (Repr::zero(), Repr::from_buffer(buffer0.into()))
                    }
                }
            }
        }
    }

    #[inline]
    fn div_rem_dword(lhs: DoubleWord, rhs: DoubleWord) -> (Repr, Repr) {
        // If division works, remainder also works.
        match lhs.checked_div(rhs) {
            Some(res) => (Repr::from_dword(res), Repr::from_dword(lhs % rhs)),
            None => panic_divide_by_0(),
        }
    }

    fn div_rem_large_dword(mut buffer: Buffer, rhs: DoubleWord) -> (Repr, Repr) {
        if rhs == 0 {
            panic_divide_by_0();
        }
        if let Some(word) = shrink_dword(rhs) {
            let rem = div::div_by_word_in_place(&mut buffer, word);
            (Repr::from_buffer(buffer), Repr::from_word(rem))
        } else {
            let rem = div::div_by_dword_in_place(&mut buffer, rhs);
            (Repr::from_buffer(buffer), Repr::from_dword(rem))
        }
    }
    
    fn div_rem_large(mut lhs: Buffer, mut rhs: Buffer) -> (Repr, Repr) {
        let shift = div_rem_in_lhs(&mut lhs, &mut rhs);
        let n = rhs.len();
        rhs.copy_from_slice(&lhs[..n]);
        let low_bits = shift::shr_in_place(&mut rhs, shift);
        debug_assert!(low_bits == 0);
        lhs.erase_front(n);
        (Repr::from_buffer(lhs), Repr::from_buffer(rhs))
    }

    /// lhs = (lhs / rhs, lhs % rhs)
    ///
    /// Returns shift.
    fn div_rem_in_lhs(lhs: &mut Buffer, rhs: &mut Buffer) -> u32 {
        let (shift, fast_div_rhs_top) = div::normalize_large(rhs);
        let lhs_carry = shift::shl_in_place(lhs, shift);
        if lhs_carry != 0 {
            lhs.push_resizing(lhs_carry);
        }
        let mut allocation =
            MemoryAllocation::new(div::memory_requirement_exact(lhs.len(), rhs.len()));
        let mut memory = allocation.memory();
        let overflow = div::div_rem_in_place(lhs, rhs, fast_div_rhs_top, &mut memory);
        if overflow {
            lhs.push_resizing(1);
        }
        shift
    }

    impl Div<TypedRepr> for TypedRepr {
        type Output = Repr;

        #[inline]
        fn div(self, rhs: TypedRepr) -> Repr {
            match (self, rhs) {
                (Small(dword0), Small(dword1)) => div_dword(dword0, dword1),
                (Small(_), Large(_)) => Repr::zero(),
                (Large(buffer0), Small(dword1)) => div_large_dword(buffer0, dword1),
                (Large(buffer0), Large(buffer1)) => {
                    if buffer0.len() >= buffer1.len() {
                        div_large(buffer0, buffer1)
                    } else {
                        Repr::zero()
                    }
                }
            }
        }
    }

    impl<'r> Div<TypedReprRef<'r>> for TypedRepr {
        type Output = Repr;

        #[inline]
        fn div(self, rhs: TypedReprRef<'r>) -> Repr {
            match (self, rhs) {
                (Small(dword0), RefSmall(dword1)) => div_dword(dword0, dword1),
                (Small(_), RefLarge(_)) => Repr::zero(),
                (Large(buffer0), RefSmall(dword1)) => div_large_dword(buffer0, dword1),
                (Large(buffer0), RefLarge(buffer1)) => {
                    if buffer0.len() >= buffer1.len() {
                        div_large(buffer0, buffer1.into())
                    } else {
                        Repr::zero()
                    }
                }
            }
        }
    }

    impl<'l> Div<TypedRepr> for TypedReprRef<'l> {
        type Output = Repr;

        #[inline]
        fn div(self, rhs: TypedRepr) -> Repr {
            match (self, rhs) {
                (RefSmall(dword0), Small(dword1)) => div_dword(dword0, dword1),
                (RefSmall(_), Large(_)) => Repr::zero(),
                (RefLarge(buffer0), Small(dword1)) => div_large_dword(buffer0.into(), dword1),
                (RefLarge(buffer0), Large(buffer1)) => {
                    if buffer0.len() >= buffer1.len() {
                        div_large(buffer0.into(), buffer1)
                    } else {
                        Repr::zero()
                    }
                }
            }
        }
    }

    impl<'l, 'r> Div<TypedReprRef<'r>> for TypedReprRef<'l> {
        type Output = Repr;

        #[inline]
        fn div(self, rhs: TypedReprRef) -> Repr {
            match (self, rhs) {
                (RefSmall(dword0), RefSmall(dword1)) => div_dword(dword0, dword1),
                (RefSmall(_), RefLarge(_)) => Repr::zero(),
                (RefLarge(buffer0), RefSmall(dword1)) => div_large_dword(buffer0.into(), dword1),
                (RefLarge(buffer0), RefLarge(buffer1)) => {
                    if buffer0.len() >= buffer1.len() {
                        div_large(buffer0.into(), buffer1.into())
                    } else {
                        Repr::zero()
                    }
                }
            }
        }
    }

    #[inline]
    fn div_dword(lhs: DoubleWord, rhs: DoubleWord) -> Repr {
        match lhs.checked_div(rhs) {
            Some(res) => Repr::from_dword(res),
            None => panic_divide_by_0(),
        }
    }

    #[inline]
    fn div_large_dword(lhs: Buffer, rhs: DoubleWord) -> Repr {
        let (q, _) = div_rem_large_dword(lhs, rhs);
        q
    }
    
    fn div_large(mut lhs: Buffer, mut rhs: Buffer) -> Repr {
        let _shift = div_rem_in_lhs(&mut lhs, &mut rhs);
        lhs.erase_front(rhs.len());
        Repr::from_buffer(lhs)
    }

    impl Rem<TypedRepr> for TypedRepr {
        type Output = Repr;

        #[inline]
        fn rem(self, rhs: TypedRepr) -> Repr {
            match (self, rhs) {
                (Small(dword0), Small(dword1)) => rem_dword(dword0, dword1),
                (Small(dword0), Large(_)) => Repr::from_dword(dword0),
                (Large(buffer0), Small(dword1)) => rem_large_dword(&buffer0, dword1),
                (Large(buffer0), Large(buffer1)) => {
                    if buffer0.len() >= buffer1.len() {
                        rem_large(buffer0, buffer1)
                    } else {
                        Repr::from_buffer(buffer0)
                    }
                }
            }
        }
    }

    impl<'r> Rem<TypedReprRef<'r>> for TypedRepr {
        type Output = Repr;

        #[inline]
        fn rem(self, rhs: TypedReprRef) -> Repr {
            match (self, rhs) {
                (Small(dword0), RefSmall(dword1)) => rem_dword(dword0, dword1),
                (Small(dword0), RefLarge(_)) => Repr::from_dword(dword0),
                (Large(buffer0), RefSmall(dword1)) => rem_large_dword(&buffer0, dword1),
                (Large(buffer0), RefLarge(buffer1)) => {
                    if buffer0.len() >= buffer1.len() {
                        rem_large(buffer0, buffer1.into())
                    } else {
                        Repr::from_buffer(buffer0.into())
                    }
                }
            }
        }
    }

    impl<'l> Rem<TypedRepr> for TypedReprRef<'l> {
        type Output = Repr;

        #[inline]
        fn rem(self, rhs: TypedRepr) -> Repr {
            match (self, rhs) {
                (RefSmall(dword0), Small(dword1)) => rem_dword(dword0, dword1),
                (RefSmall(dword0), Large(_)) => Repr::from_dword(dword0),
                (RefLarge(buffer0), Small(dword1)) => rem_large_dword(buffer0, dword1),
                (RefLarge(buffer0), Large(mut buffer1)) => {
                    if buffer0.len() >= buffer1.len() {
                        rem_large(buffer0.into(), buffer1)
                    } else {
                        // Reuse buffer1 for the remainder.
                        buffer1.clone_from_slice(buffer0);
                        Repr::from_buffer(buffer1)
                    }
                }
            }
        }
    }

    impl<'l, 'r> Rem<TypedReprRef<'r>> for TypedReprRef<'l> {
        type Output = Repr;

        #[inline]
        fn rem(self, rhs: TypedReprRef) -> Repr {
            match (self, rhs) {
                (RefSmall(dword0), RefSmall(dword1)) => rem_dword(dword0, dword1),
                (RefSmall(dword0), RefLarge(_)) => Repr::from_dword(dword0),
                (RefLarge(buffer0), RefSmall(dword1)) => rem_large_dword(buffer0, dword1),
                (RefLarge(buffer0), RefLarge(buffer1)) => {
                    if buffer0.len() >= buffer1.len() {
                        rem_large(buffer0.into(), buffer1.into())
                    } else {
                        Repr::from_buffer(buffer0.into())
                    }
                }
            }
        }
    }

    #[inline]
    fn rem_dword(lhs: DoubleWord, rhs: DoubleWord) -> Repr {
        match lhs.checked_rem(rhs) {
            Some(res) => Repr::from_dword(res),
            None => panic_divide_by_0(),
        }
    }

    #[inline]
    fn rem_large_dword(lhs: &[Word], rhs: DoubleWord) -> Repr {
        if rhs == 0 {
            panic_divide_by_0();
        }
        if let Some(word) = shrink_dword(rhs) {
            Repr::from_word(div::rem_by_word(lhs, word))
        } else {
            Repr::from_dword(div::rem_by_dword(lhs, rhs))
        }
    }

    fn rem_large(mut lhs: Buffer, mut rhs: Buffer) -> Repr {
        let shift = div_rem_in_lhs(&mut lhs, &mut rhs);
        let n = rhs.len();
        rhs.copy_from_slice(&lhs[..n]);
        let low_bits = shift::shr_in_place(&mut rhs, shift);
        debug_assert!(low_bits == 0);
        Repr::from_buffer(rhs)
    }
}

// TODO: organize all panic into error.rs
fn panic_divide_by_0() -> ! {
    panic!("divide by 0")
}
