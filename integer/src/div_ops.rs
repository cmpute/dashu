//! Division operators.

use crate::{
    helper_macros,
    ibig::IBig,
    ops::{DivEuclid, DivRem, DivRemAssign, DivRemEuclid, RemEuclid},
    ubig::UBig,
    Sign::*,
};
use core::ops::{Div, DivAssign, Rem, RemAssign};

// Ops for UBig

helper_macros::forward_ubig_binop_to_repr!(impl Div, div);
helper_macros::forward_ubig_binop_to_repr!(impl Rem, rem);
helper_macros::forward_ubig_binop_to_repr!(impl DivEuclid, div_euclid, div);
helper_macros::forward_ubig_binop_to_repr!(impl RemEuclid, rem_euclid, rem);
helper_macros::impl_binop_assign_by_taking!(impl DivAssign<UBig> for UBig, div_assign, div);
helper_macros::impl_binop_assign_by_taking!(impl RemAssign<UBig> for UBig, rem_assign, rem);

macro_rules! impl_ubig_divrem {
    ($repr0:ident, $repr1:ident) => {{
        let (q, r) = $repr0.div_rem($repr1);
        (UBig(q), UBig(r))
    }};
}
helper_macros::forward_ubig_binop_to_repr!(
    impl DivRem, div_rem -> (UBig, UBig),
    OutputDiv = UBig, OutputRem = UBig,
    impl_ubig_divrem
);
helper_macros::forward_ubig_binop_to_repr!(
    impl DivRemEuclid,
    div_rem_euclid -> (UBig, UBig),
    OutputDiv = UBig, OutputRem = UBig,
    impl_ubig_divrem
);
helper_macros::impl_binop_assign_by_taking!(
    impl DivRemAssign<UBig> for UBig, div_rem_assign,
    OutputRem = UBig, div_rem
);

// Ops for IBig

macro_rules! impl_ibig_div {
    ($sign0:ident, $mag0:ident, $sign1:ident, $mag1:ident) => {
        // truncate towards 0.
        IBig(($mag0 / $mag1).with_sign($sign0 * $sign1))
    };
}
macro_rules! impl_ibig_rem {
    ($sign0:ident, $mag0:ident, $sign1:ident, $mag1:ident) => {{
        let _unused = $sign1;

        // remainder with truncating division has same sign as lhs.
        IBig(($mag0 % $mag1).with_sign($sign0))
    }};
}
helper_macros::forward_ibig_binop_to_repr!(impl Div, div, Output = IBig, impl_ibig_div);
helper_macros::forward_ibig_binop_to_repr!(impl Rem, rem, Output = IBig, impl_ibig_rem);
helper_macros::impl_binop_assign_by_taking!(impl DivAssign<IBig> for IBig, div_assign, div);
helper_macros::impl_binop_assign_by_taking!(impl RemAssign<IBig> for IBig, rem_assign, rem);

macro_rules! impl_ibig_div_rem {
    ($sign0:ident, $mag0:ident, $sign1:ident, $mag1:ident) => {{
        // truncate towards 0.
        let (q, r) = $mag0.div_rem($mag1);
        (IBig(q.with_sign($sign0 * $sign1)), IBig(r.with_sign($sign0)))
    }};
}
helper_macros::forward_ibig_binop_to_repr!(
    impl DivRem, div_rem -> (IBig, IBig),
    OutputDiv = IBig, OutputRem = IBig,
    impl_ibig_div_rem
);

macro_rules! impl_ibig_div_euclid {
    ($sign0:ident, $mag0:ident, $sign1:ident, $mag1:ident) => {{
        let (q, r) = $mag0.div_rem($mag1);
        let q = match ($sign0, r.is_zero()) {
            (Positive, _) | (Negative, true) => q,
            (Negative, false) => q.into_typed().add_one(),
        };
        IBig(q.with_sign($sign0 * $sign1))
    }};
}
macro_rules! impl_ibig_rem_euclid {
    ($sign0:ident, $mag0:ident, $sign1:ident, $mag1:ident) => {{
        let _unused = $sign1;
        let repr = match $sign0 {
            Positive => $mag0 % $mag1,
            Negative => {
                let r = $mag0 % $mag1.as_ref();
                if r.is_zero() {
                    r
                } else {
                    $mag1 - r.into_typed()
                }
            }
        };
        UBig(repr)
    }};
}
helper_macros::forward_ibig_binop_to_repr!(
    impl DivEuclid,
    div_euclid,
    Output = IBig,
    impl_ibig_div_euclid
);
helper_macros::forward_ibig_binop_to_repr!(
    impl RemEuclid,
    rem_euclid,
    Output = UBig,
    impl_ibig_rem_euclid
);

macro_rules! impl_ibig_divrem_euclid {
    ($sign0:ident, $mag0:ident, $sign1:ident, $mag1:ident) => {
        match $sign0 {
            Positive => {
                let (q, r) = $mag0.div_rem($mag1);
                (IBig(q.with_sign($sign1)), UBig(r))
            }
            Negative => {
                let (mut q, mut r) = $mag0.div_rem($mag1.as_ref());
                if !r.is_zero() {
                    q = q.into_typed().add_one();
                    r = $mag1 - r.into_typed();
                }
                (IBig(q.with_sign(-$sign1)), UBig(r))
            }
        }
    };
}
helper_macros::forward_ibig_binop_to_repr!(
    impl DivRemEuclid, div_rem_euclid -> (IBig, UBig),
    OutputDiv = IBig, OutputRem = UBig,
    impl_ibig_divrem_euclid
);
helper_macros::impl_binop_assign_by_taking!(
    impl DivRemAssign<IBig> for IBig, div_rem_assign,
    OutputRem = IBig, div_rem
);

// Ops between UBig & IBig

macro_rules! impl_ubig_ibig_div {
    ($mag0:ident, $sign1:ident, $mag1:ident) => {
        IBig(($mag0 / $mag1).with_sign($sign1))
    };
}
macro_rules! impl_ubig_ibig_rem {
    ($mag0:ident, $sign1:ident, $mag1:ident) => {{
        let _unused = $sign1;
        UBig($mag0 % $mag1)
    }};
}
macro_rules! impl_ubig_ibig_divrem {
    ($mag0:ident, $sign1:ident, $mag1:ident) => {{
        let (q, r) = $mag0.div_rem($mag1);
        (IBig(q.with_sign($sign1)), UBig(r))
    }};
}
helper_macros::forward_ubig_ibig_binop_to_repr!(impl Div, div, Output = IBig, impl_ubig_ibig_div);
helper_macros::forward_ubig_ibig_binop_to_repr!(impl Rem, rem, Output = UBig, impl_ubig_ibig_rem);
helper_macros::forward_ubig_ibig_binop_to_repr!(impl DivRem, div_rem -> (IBig, UBig), OutputDiv = IBig, OutputRem = UBig, impl_ubig_ibig_divrem);
helper_macros::impl_binop_assign_by_taking!(impl RemAssign<IBig> for UBig, rem_assign, rem);

macro_rules! impl_ibig_ubig_div {
    ($sign0:ident, $mag0:ident, $mag1:ident) => {
        // truncate towards 0.
        IBig(($mag0 / $mag1).with_sign($sign0))
    };
}
macro_rules! impl_ibig_ubig_rem {
    ($sign0:ident, $mag0:ident, $mag1:ident) => {{
        // remainder with truncating division has same sign as lhs.
        IBig(($mag0 % $mag1).with_sign($sign0))
    }};
}
macro_rules! impl_ibig_ubig_divrem {
    ($sign0:ident, $mag0:ident, $mag1:ident) => {{
        // remainder with truncating division has same sign as lhs.
        let (q, r) = $mag0.div_rem($mag1);
        (IBig(q.with_sign($sign0)), IBig(r.with_sign($sign0)))
    }};
}
helper_macros::forward_ibig_ubig_binop_to_repr!(impl Div, div, Output = IBig, impl_ibig_ubig_div);
helper_macros::forward_ibig_ubig_binop_to_repr!(impl Rem, rem, Output = IBig, impl_ibig_ubig_rem);
helper_macros::forward_ibig_ubig_binop_to_repr!(impl DivRem, div_rem -> (IBig, IBig), OutputDiv = IBig, OutputRem = IBig, impl_ibig_ubig_divrem);
helper_macros::impl_binop_assign_by_taking!(impl DivAssign<UBig> for IBig, div_assign, div);
helper_macros::impl_binop_assign_by_taking!(impl RemAssign<UBig> for IBig, rem_assign, rem);

// Ops with primitives

macro_rules! impl_divrem_with_primitive {
    (impl <$target:ty> for $t:ty) => {
        impl DivRem<$target> for $t {
            type OutputDiv = $t;
            type OutputRem = $target;

            #[inline]
            fn div_rem(self, rhs: $target) -> ($t, $target) {
                let (q, r) = self.div_rem(<$t>::from(rhs));
                (q, r.try_into().unwrap())
            }
        }

        impl<'l> DivRem<$target> for &'l $t {
            type OutputDiv = $t;
            type OutputRem = $target;

            #[inline]
            fn div_rem(self, rhs: $target) -> ($t, $target) {
                let (q, r) = self.div_rem(<$t>::from(rhs));
                (q, r.try_into().unwrap())
            }
        }

        impl<'r> DivRem<&'r $target> for $t {
            type OutputDiv = $t;
            type OutputRem = $target;

            #[inline]
            fn div_rem(self, rhs: &$target) -> ($t, $target) {
                let (q, r) = self.div_rem(<$t>::from(*rhs));
                (q, r.try_into().unwrap())
            }
        }

        impl<'l, 'r> DivRem<&'r $target> for &'l $t {
            type OutputDiv = $t;
            type OutputRem = $target;

            #[inline]
            fn div_rem(self, rhs: &$target) -> ($t, $target) {
                let (q, r) = self.div_rem(<$t>::from(*rhs));
                (q, r.try_into().unwrap())
            }
        }
    };
}

macro_rules! impl_div_primitive_with_ubig {
    ($($t:ty)*) => {$(
        helper_macros::impl_binop_with_primitive!(impl Div<$t> for UBig, div);
        helper_macros::impl_binop_with_primitive!(impl Rem<$t> for UBig, rem -> $t);
        helper_macros::impl_binop_assign_with_primitive!(impl DivAssign<$t> for UBig, div_assign);

        impl_divrem_with_primitive!(impl <$t> for UBig);
        helper_macros::impl_binop_assign_with_primitive!(impl DivRemAssign<$t> for UBig, div_rem_assign, OutputRem = $t);
    )*};
}
impl_div_primitive_with_ubig!(u8 u16 u32 u64 u128 usize);

macro_rules! impl_div_primitive_with_ibig {
    ($($t:ty)*) => {$(
        helper_macros::impl_binop_with_primitive!(impl Div<$t> for IBig, div);
        helper_macros::impl_binop_with_primitive!(impl Rem<$t> for IBig, rem -> $t);
        helper_macros::impl_binop_assign_with_primitive!(impl DivAssign<$t> for IBig, div_assign);

        impl_divrem_with_primitive!(impl <$t> for IBig);
        helper_macros::impl_binop_assign_with_primitive!(impl DivRemAssign<$t> for IBig, div_rem_assign, OutputRem = $t);
    )*};
}
impl_div_primitive_with_ibig!(u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize);

pub(crate) mod repr {
    use super::*;
    use crate::{
        arch::word::{DoubleWord, Word},
        buffer::Buffer,
        div,
        error::panic_divide_by_0,
        helper_macros::debug_assert_zero,
        memory::MemoryAllocation,
        primitive::{extend_word, shrink_dword},
        repr::{
            Repr,
            TypedRepr::{self, *},
            TypedReprRef::{self, *},
        },
        shift,
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
                        (Repr::zero(), Repr::from_buffer(buffer0))
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
                (RefLarge(words0), Small(dword1)) => div_rem_large_dword(words0.into(), dword1),
                (RefLarge(words0), Large(mut buffer1)) => {
                    if words0.len() >= buffer1.len() {
                        div_rem_large(words0.into(), buffer1)
                    } else {
                        // Reuse buffer1 for the remainder.
                        buffer1.clone_from_slice(words0);
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
                (Large(buffer0), RefLarge(words1)) => {
                    if buffer0.len() >= words1.len() {
                        div_rem_large(buffer0, words1.into())
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
                (RefLarge(words0), RefSmall(dword1)) => div_rem_large_dword(words0.into(), dword1),
                (RefLarge(words0), RefLarge(words1)) => {
                    if words0.len() >= words1.len() {
                        div_rem_large(words0.into(), words1.into())
                    } else {
                        (Repr::zero(), Repr::from_buffer(words0.into()))
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
        debug_assert_zero!(shift::shr_in_place(&mut rhs, shift));
        lhs.erase_front(n);
        (Repr::from_buffer(lhs), Repr::from_buffer(rhs))
    }

    /// lhs = [lhs % rhs, lhs / rhs]
    ///
    /// Returns the number of shift bits produced by normalization.
    #[inline]
    fn div_rem_in_lhs(lhs: &mut Buffer, rhs: &mut Buffer) -> u32 {
        let mut allocation =
            MemoryAllocation::new(div::memory_requirement_exact(lhs.len(), rhs.len()));
        let (shift, fast_div_top) = div::normalize(rhs);
        let quo_carry = div::div_rem_unshifted_in_place(
            lhs,
            rhs,
            shift,
            fast_div_top,
            &mut allocation.memory(),
        );
        lhs.push_resizing(quo_carry);
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
                (Large(buffer0), RefLarge(words1)) => {
                    if buffer0.len() >= words1.len() {
                        div_large(buffer0, words1.into())
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
                (RefLarge(words0), Small(dword1)) => div_large_dword(words0.into(), dword1),
                (RefLarge(words1), Large(buffer1)) => {
                    if words1.len() >= buffer1.len() {
                        div_large(words1.into(), buffer1)
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
                (RefLarge(words0), RefSmall(dword1)) => div_large_dword(words0.into(), dword1),
                (RefLarge(words0), RefLarge(words1)) => {
                    if words0.len() >= words1.len() {
                        div_large(words0.into(), words1.into())
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
                (Large(buffer0), RefLarge(words1)) => {
                    if buffer0.len() >= words1.len() {
                        rem_large(buffer0, words1.into())
                    } else {
                        Repr::from_buffer(buffer0)
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
                (RefLarge(words0), Small(dword1)) => rem_large_dword(words0, dword1),
                (RefLarge(words0), Large(mut buffer1)) => {
                    if words0.len() >= buffer1.len() {
                        rem_large(words0.into(), buffer1)
                    } else {
                        // Reuse buffer1 for the remainder.
                        buffer1.clone_from_slice(words0);
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
                (RefLarge(words0), RefSmall(dword1)) => rem_large_dword(words0, dword1),
                (RefLarge(words0), RefLarge(words1)) => {
                    if words0.len() >= words1.len() {
                        rem_large(words0.into(), words1.into())
                    } else {
                        Repr::from_buffer(words0.into())
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

    pub(crate) fn rem_large(mut lhs: Buffer, mut rhs: Buffer) -> Repr {
        let shift = div_rem_in_lhs(&mut lhs, &mut rhs);
        let n = rhs.len();
        rhs.copy_from_slice(&lhs[..n]);
        debug_assert_zero!(shift::shr_in_place(&mut rhs, shift));
        Repr::from_buffer(rhs)
    }

    #[rustversion::since(1.64)]
    impl<'a> TypedReprRef<'a> {
        pub(super) const fn is_multiple_of_dword(self, divisor: DoubleWord) -> bool {
            if let Some(w) = shrink_dword(divisor) {
                match self {
                    TypedReprRef::RefSmall(dword) => dword % extend_word(w) == 0,
                    TypedReprRef::RefLarge(words) => div::rem_by_word(words, w) == 0,
                }
            } else {
                match self {
                    TypedReprRef::RefSmall(dword) => dword % divisor == 0,
                    TypedReprRef::RefLarge(words) => div::rem_by_dword(words, divisor) == 0,
                }
            }
        }
    }
}

impl UBig {
    /// Determine whether the integer is perfectly divisible by the divisor.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::UBig;
    /// let a = UBig::from(24u8);
    /// let b = UBig::from(6u8);
    /// assert!(a.is_multiple_of(&b));
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the divisor is zero.
    #[inline]
    pub fn is_multiple_of(&self, divisor: &Self) -> bool {
        (self % divisor).is_zero()
    }

    /// A const version of [UBig::is_multiple_of], but only accepts [DoubleWord][crate::DoubleWord] divisors.
    ///
    /// # Availability
    ///
    /// Since Rust 1.64
    #[rustversion::since(1.64)]
    #[inline]
    pub const fn is_multiple_of_const(&self, divisor: crate::DoubleWord) -> bool {
        self.repr().is_multiple_of_dword(divisor)
    }
}

impl IBig {
    /// Determine whether the integer is perfectly divisible by the divisor.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::IBig;
    /// let a = IBig::from(24);
    /// let b = IBig::from(-6);
    /// assert!(a.is_multiple_of(&b));
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the divisor is zero.
    #[inline]
    pub fn is_multiple_of(&self, divisor: &Self) -> bool {
        (self % divisor).is_zero()
    }

    /// A const version of [IBig::is_multiple_of], but only accepts [DoubleWord][crate::DoubleWord] divisors.
    ///
    /// # Availability
    ///
    /// Since Rust 1.64
    #[rustversion::since(1.64)]
    #[inline]
    pub const fn is_multiple_of_const(&self, divisor: crate::DoubleWord) -> bool {
        let (_, repr) = self.as_sign_repr();
        repr.is_multiple_of_dword(divisor)
    }
}
