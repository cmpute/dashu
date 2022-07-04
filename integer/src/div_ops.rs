//! Division operators.

use crate::{
    arch::word::{Word, DoubleWord},
    buffer::{Buffer, TypedReprRef::*, TypedRepr::*},
    div, helper_macros,
    ibig::IBig,
    memory::MemoryAllocation,
    ops::{Abs, DivEuclid, DivRem, DivRemEuclid, RemEuclid},
    primitive::{PrimitiveSigned, PrimitiveUnsigned},
    shift,
    sign::Sign::*,
    ubig::UBig,
};
use core::{
    convert::TryFrom,
    fmt::Debug,
    mem,
    ops::{Div, DivAssign, Rem, RemAssign},
};

impl Div<UBig> for UBig {
    type Output = UBig;

    #[inline]
    fn div(self, rhs: UBig) -> UBig {
        ubig::div_repr_val_val(self.into_repr(), rhs.into_repr())
    }
}

impl Div<&UBig> for UBig {
    type Output = UBig;

    #[inline]
    fn div(self, rhs: &UBig) -> UBig {
        ubig::div_repr_val_ref(self.into_repr(), rhs.repr())
    }
}

impl Div<UBig> for &UBig {
    type Output = UBig;

    #[inline]
    fn div(self, rhs: UBig) -> UBig {
        ubig::div_repr_ref_val(self.repr(), rhs.into_repr())
    }
}

impl Div<&UBig> for &UBig {
    type Output = UBig;

    #[inline]
    fn div(self, rhs: &UBig) -> UBig {
        ubig::div_repr_ref_ref(self.repr(), rhs.repr())
    }
}

impl DivAssign<UBig> for UBig {
    #[inline]
    fn div_assign(&mut self, rhs: UBig) {
        *self = mem::take(self) / rhs;
    }
}

impl DivAssign<&UBig> for UBig {
    #[inline]
    fn div_assign(&mut self, rhs: &UBig) {
        *self = mem::take(self) / rhs;
    }
}

impl Rem<UBig> for UBig {
    type Output = UBig;

    #[inline]
    fn rem(self, rhs: UBig) -> UBig {
        ubig::rem_repr_val_val(self.into_repr(), rhs.into_repr())
    }
}

impl Rem<&UBig> for UBig {
    type Output = UBig;

    #[inline]
    fn rem(self, rhs: &UBig) -> UBig {
        ubig::rem_repr_val_ref(self.into_repr(), rhs.repr())
    }
}

impl Rem<UBig> for &UBig {
    type Output = UBig;

    #[inline]
    fn rem(self, rhs: UBig) -> UBig {
        ubig::rem_repr_ref_val(self.repr(), rhs.into_repr())
    }
}

impl Rem<&UBig> for &UBig {
    type Output = UBig;

    #[inline]
    fn rem(self, rhs: &UBig) -> UBig {
        ubig::rem_repr_ref_ref(self.repr(), rhs.repr())
    }
}

impl RemAssign<UBig> for UBig {
    #[inline]
    fn rem_assign(&mut self, rhs: UBig) {
        *self = mem::take(self) % rhs;
    }
}

impl RemAssign<&UBig> for UBig {
    #[inline]
    fn rem_assign(&mut self, rhs: &UBig) {
        *self = mem::take(self) % rhs;
    }
}

impl DivRem<UBig> for UBig {
    type OutputDiv = UBig;
    type OutputRem = UBig;

    #[inline]
    fn div_rem(self, rhs: UBig) -> (UBig, UBig) {
        ubig::div_rem_repr_val_val(self.into_repr(), rhs.into_repr())
    }
}

impl DivRem<&UBig> for UBig {
    type OutputDiv = UBig;
    type OutputRem = UBig;

    #[inline]
    fn div_rem(self, rhs: &UBig) -> (UBig, UBig) {
        ubig::div_rem_repr_val_ref(self.into_repr(), rhs.repr())
    }
}

impl DivRem<UBig> for &UBig {
    type OutputDiv = UBig;
    type OutputRem = UBig;

    #[inline]
    fn div_rem(self, rhs: UBig) -> (UBig, UBig) {
        ubig::div_rem_repr_ref_val(self.repr(), rhs.into_repr())
    }
}

impl DivRem<&UBig> for &UBig {
    type OutputDiv = UBig;
    type OutputRem = UBig;

    #[inline]
    fn div_rem(self, rhs: &UBig) -> (UBig, UBig) {
        ubig::div_rem_repr_ref_ref(self.repr(), rhs.repr())
    }
}

// TODO: use macros here
impl DivEuclid<UBig> for UBig {
    type Output = UBig;

    #[inline]
    fn div_euclid(self, rhs: UBig) -> UBig {
        self / rhs
    }
}

impl DivEuclid<&UBig> for UBig {
    type Output = UBig;

    #[inline]
    fn div_euclid(self, rhs: &UBig) -> UBig {
        self / rhs
    }
}

impl DivEuclid<UBig> for &UBig {
    type Output = UBig;

    #[inline]
    fn div_euclid(self, rhs: UBig) -> UBig {
        self / rhs
    }
}

impl DivEuclid<&UBig> for &UBig {
    type Output = UBig;

    #[inline]
    fn div_euclid(self, rhs: &UBig) -> UBig {
        self / rhs
    }
}

impl RemEuclid<UBig> for UBig {
    type Output = UBig;

    #[inline]
    fn rem_euclid(self, rhs: UBig) -> UBig {
        self % rhs
    }
}

impl RemEuclid<&UBig> for UBig {
    type Output = UBig;

    #[inline]
    fn rem_euclid(self, rhs: &UBig) -> UBig {
        self % rhs
    }
}

impl RemEuclid<UBig> for &UBig {
    type Output = UBig;

    #[inline]
    fn rem_euclid(self, rhs: UBig) -> UBig {
        self % rhs
    }
}

impl RemEuclid<&UBig> for &UBig {
    type Output = UBig;

    #[inline]
    fn rem_euclid(self, rhs: &UBig) -> UBig {
        self % rhs
    }
}

impl DivRemEuclid<UBig> for UBig {
    type OutputDiv = UBig;
    type OutputRem = UBig;

    #[inline]
    fn div_rem_euclid(self, rhs: UBig) -> (UBig, UBig) {
        self.div_rem(rhs)
    }
}

impl DivRemEuclid<&UBig> for UBig {
    type OutputDiv = UBig;
    type OutputRem = UBig;

    #[inline]
    fn div_rem_euclid(self, rhs: &UBig) -> (UBig, UBig) {
        self.div_rem(rhs)
    }
}

impl DivRemEuclid<UBig> for &UBig {
    type OutputDiv = UBig;
    type OutputRem = UBig;

    #[inline]
    fn div_rem_euclid(self, rhs: UBig) -> (UBig, UBig) {
        self.div_rem(rhs)
    }
}

impl DivRemEuclid<&UBig> for &UBig {
    type OutputDiv = UBig;
    type OutputRem = UBig;

    #[inline]
    fn div_rem_euclid(self, rhs: &UBig) -> (UBig, UBig) {
        self.div_rem(rhs)
    }
}

impl Div<IBig> for IBig {
    type Output = IBig;

    #[inline]
    fn div(self, rhs: IBig) -> IBig {
        // Truncate towards 0.
        let (sign0, mag0) = self.into_sign_repr();
        let (sign1, mag1) = rhs.into_sign_repr();
        IBig::from_sign_magnitude(sign0 * sign1, ubig::div_repr_val_val(mag0, mag1))
    }
}

impl Div<&IBig> for IBig {
    type Output = IBig;

    #[inline]
    fn div(self, rhs: &IBig) -> IBig {
        // Truncate towards 0.
        let (sign0, mag0) = self.into_sign_repr();
        let (sign1, mag1) = rhs.as_sign_repr();
        IBig::from_sign_magnitude(sign0 * sign1, ubig::div_repr_val_ref(mag0, mag1))
    }
}

impl Div<IBig> for &IBig {
    type Output = IBig;

    #[inline]
    fn div(self, rhs: IBig) -> IBig {
        // Truncate towards 0.
        let (sign0, mag0) = self.as_sign_repr();
        let (sign1, mag1) = rhs.into_sign_repr();
        IBig::from_sign_magnitude(sign0 * sign1, ubig::div_repr_ref_val(mag0, mag1))
    }
}

impl Div<&IBig> for &IBig {
    type Output = IBig;

    #[inline]
    fn div(self, rhs: &IBig) -> IBig {
        // Truncate towards 0.
        let (sign0, mag0) = self.as_sign_repr();
        let (sign1, mag1) = rhs.as_sign_repr();
        IBig::from_sign_magnitude(sign0 * sign1, ubig::div_repr_ref_ref(mag0, mag1))
    }
}

impl DivAssign<IBig> for IBig {
    #[inline]
    fn div_assign(&mut self, rhs: IBig) {
        *self = mem::take(self) / rhs;
    }
}

impl DivAssign<&IBig> for IBig {
    #[inline]
    fn div_assign(&mut self, rhs: &IBig) {
        *self = mem::take(self) / rhs;
    }
}

impl Rem<IBig> for IBig {
    type Output = IBig;

    #[inline]
    fn rem(self, rhs: IBig) -> IBig {
        // Remainder with truncating division has same sign as lhs.
        let (sign0, mag0) = self.into_sign_repr();
        let (_, mag1) = rhs.into_sign_repr();
        IBig::from_sign_magnitude(sign0, ubig::rem_repr_val_val(mag0, mag1))
    }
}

impl Rem<&IBig> for IBig {
    type Output = IBig;

    #[inline]
    fn rem(self, rhs: &IBig) -> IBig {
        // Remainder with truncating division has same sign as lhs.
        let (sign0, mag0) = self.into_sign_repr();
        let (_, mag1) = rhs.as_sign_repr();
        IBig::from_sign_magnitude(sign0, ubig::rem_repr_val_ref(mag0, mag1))
    }
}

impl Rem<IBig> for &IBig {
    type Output = IBig;

    #[inline]
    fn rem(self, rhs: IBig) -> IBig {
        // Remainder with truncating division has same sign as lhs.
        let (sign0, mag0) = self.as_sign_repr();
        let (_, mag1) = rhs.into_sign_repr();
        IBig::from_sign_magnitude(sign0, ubig::rem_repr_ref_val(mag0, mag1))
    }
}

impl Rem<&IBig> for &IBig {
    type Output = IBig;

    #[inline]
    fn rem(self, rhs: &IBig) -> IBig {
        // Remainder with truncating division has same sign as lhs.
        let (sign0, mag0) = self.as_sign_repr();
        let (_, mag1) = rhs.as_sign_repr();
        IBig::from_sign_magnitude(sign0, ubig::rem_repr_ref_ref(mag0, mag1))
    }
}

impl RemAssign<IBig> for IBig {
    #[inline]
    fn rem_assign(&mut self, rhs: IBig) {
        *self = mem::take(self) % rhs;
    }
}

impl RemAssign<&IBig> for IBig {
    #[inline]
    fn rem_assign(&mut self, rhs: &IBig) {
        *self = mem::take(self) % rhs;
    }
}

impl DivRem<IBig> for IBig {
    type OutputDiv = IBig;
    type OutputRem = IBig;

    #[inline]
    fn div_rem(self, rhs: IBig) -> (IBig, IBig) {
        // Truncate towards 0.
        let (sign0, mag0) = self.into_sign_repr();
        let (sign1, mag1) = rhs.into_sign_repr();
        let (q, r) = ubig::div_rem_repr_val_val(mag0, mag1);
        (
            IBig::from_sign_magnitude(sign0 * sign1, q),
            IBig::from_sign_magnitude(sign0, r),
        )
    }
}

impl DivRem<&IBig> for IBig {
    type OutputDiv = IBig;
    type OutputRem = IBig;

    #[inline]
    fn div_rem(self, rhs: &IBig) -> (IBig, IBig) {
        // Truncate towards 0.
        let (sign0, mag0) = self.into_sign_repr();
        let (sign1, mag1) = rhs.as_sign_repr();
        let (q, r) = ubig::div_rem_repr_val_ref(mag0, mag1);
        (
            IBig::from_sign_magnitude(sign0 * sign1, q),
            IBig::from_sign_magnitude(sign0, r),
        )
    }
}

impl DivRem<IBig> for &IBig {
    type OutputDiv = IBig;
    type OutputRem = IBig;

    #[inline]
    fn div_rem(self, rhs: IBig) -> (IBig, IBig) {
        // Truncate towards 0.
        let (sign0, mag0) = self.as_sign_repr();
        let (sign1, mag1) = rhs.into_sign_repr();
        let (q, r) = ubig::div_rem_repr_ref_val(mag0, mag1);
        (
            IBig::from_sign_magnitude(sign0 * sign1, q),
            IBig::from_sign_magnitude(sign0, r),
        )
    }
}

impl DivRem<&IBig> for &IBig {
    type OutputDiv = IBig;
    type OutputRem = IBig;

    #[inline]
    fn div_rem(self, rhs: &IBig) -> (IBig, IBig) {
        // Truncate towards 0.
        let (sign0, mag0) = self.as_sign_repr();
        let (sign1, mag1) = rhs.as_sign_repr();
        let (q, r) = ubig::div_rem_repr_ref_ref(mag0, mag1);
        (
            IBig::from_sign_magnitude(sign0 * sign1, q),
            IBig::from_sign_magnitude(sign0, r),
        )
    }
}

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
                self.div_unsigned(rhs)
            }
        }

        impl Div<$t> for &UBig {
            type Output = UBig;

            #[inline]
            fn div(self, rhs: $t) -> UBig {
                self.div_ref_unsigned(rhs)
            }
        }

        helper_macros::forward_binop_second_arg_by_value!(impl Div<$t> for UBig, div);

        impl DivAssign<$t> for UBig {
            #[inline]
            fn div_assign(&mut self, rhs: $t) {
                self.div_assign_unsigned(rhs)
            }
        }

        helper_macros::forward_binop_assign_arg_by_value!(impl DivAssign<$t> for UBig, div_assign);

        impl Rem<$t> for UBig {
            type Output = $t;

            #[inline]
            fn rem(self, rhs: $t) -> $t {
                self.rem_unsigned(rhs)
            }
        }

        impl Rem<$t> for &UBig {
            type Output = $t;

            #[inline]
            fn rem(self, rhs: $t) -> $t {
                self.rem_ref_unsigned(rhs)
            }
        }

        helper_macros::forward_binop_second_arg_by_value!(impl Rem<$t> for UBig, rem);

        impl RemAssign<$t> for UBig {
            #[inline]
            fn rem_assign(&mut self, rhs: $t) {
                self.rem_assign_unsigned(rhs)
            }
        }

        helper_macros::forward_binop_assign_arg_by_value!(impl RemAssign<$t> for UBig, rem_assign);

        impl DivRem<$t> for UBig {
            type OutputDiv = UBig;
            type OutputRem = $t;

            #[inline]
            fn div_rem(self, rhs: $t) -> (UBig, $t) {
                self.div_rem_unsigned(rhs)
            }
        }

        impl DivRem<$t> for &UBig {
            type OutputDiv = UBig;
            type OutputRem = $t;

            #[inline]
            fn div_rem(self, rhs: $t) -> (UBig, $t) {
                self.div_rem_ref_unsigned(rhs)
            }
        }

        helper_macros::forward_div_rem_second_arg_by_value!(impl DivRem<$t> for UBig, div_rem);

         impl DivEuclid<$t> for UBig {
            type Output = UBig;

            #[inline]
            fn div_euclid(self, rhs: $t) -> UBig {
                self.div_unsigned(rhs)
            }
        }

        impl DivEuclid<$t> for &UBig {
            type Output = UBig;

            #[inline]
            fn div_euclid(self, rhs: $t) -> UBig {
                self.div_ref_unsigned(rhs)
            }
        }
        helper_macros::forward_binop_second_arg_by_value!(impl DivEuclid<$t> for UBig, div_euclid);

        impl RemEuclid<$t> for UBig {
            type Output = $t;

            #[inline]
            fn rem_euclid(self, rhs: $t) -> $t {
                self.rem_unsigned(rhs)
            }
        }

        impl RemEuclid<$t> for &UBig {
            type Output = $t;

            #[inline]
            fn rem_euclid(self, rhs: $t) -> $t {
                self.rem_ref_unsigned(rhs)
            }
        }

        helper_macros::forward_binop_second_arg_by_value!(impl RemEuclid<$t> for UBig, rem_euclid);

        impl DivRemEuclid<$t> for UBig {
            type OutputDiv = UBig;
            type OutputRem = $t;

            #[inline]
            fn div_rem_euclid(self, rhs: $t) -> (UBig, $t) {
                self.div_rem_unsigned(rhs)
            }
        }

        impl DivRemEuclid<$t> for &UBig {
            type OutputDiv = UBig;
            type OutputRem = $t;

            #[inline]
            fn div_rem_euclid(self, rhs: $t) -> (UBig, $t) {
                self.div_rem_ref_unsigned(rhs)
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

macro_rules! impl_div_ubig_signed {
    ($t:ty) => {
        impl Div<$t> for UBig {
            type Output = UBig;

            #[inline]
            fn div(self, rhs: $t) -> UBig {
                self.div_signed(rhs)
            }
        }

        impl Div<$t> for &UBig {
            type Output = UBig;

            #[inline]
            fn div(self, rhs: $t) -> UBig {
                self.div_ref_signed(rhs)
            }
        }

        helper_macros::forward_binop_second_arg_by_value!(impl Div<$t> for UBig, div);

        impl DivAssign<$t> for UBig {
            #[inline]
            fn div_assign(&mut self, rhs: $t) {
                self.div_assign_signed(rhs)
            }
        }

        helper_macros::forward_binop_assign_arg_by_value!(impl DivAssign<$t> for UBig, div_assign);

        impl Rem<$t> for UBig {
            type Output = $t;

            #[inline]
            fn rem(self, rhs: $t) -> $t {
                self.rem_signed(rhs)
            }
        }

        impl Rem<$t> for &UBig {
            type Output = $t;

            #[inline]
            fn rem(self, rhs: $t) -> $t {
                self.rem_ref_signed(rhs)
            }
        }

        helper_macros::forward_binop_second_arg_by_value!(impl Rem<$t> for UBig, rem);

        impl RemAssign<$t> for UBig {
            #[inline]
            fn rem_assign(&mut self, rhs: $t) {
                self.rem_assign_signed(rhs)
            }
        }

        helper_macros::forward_binop_assign_arg_by_value!(impl RemAssign<$t> for UBig, rem_assign);

        impl DivRem<$t> for UBig {
            type OutputDiv = UBig;
            type OutputRem = $t;

            #[inline]
            fn div_rem(self, rhs: $t) -> (UBig, $t) {
                self.div_rem_signed(rhs)
            }
        }

        impl DivRem<$t> for &UBig {
            type OutputDiv = UBig;
            type OutputRem = $t;

            #[inline]
            fn div_rem(self, rhs: $t) -> (UBig, $t) {
                self.div_rem_ref_signed(rhs)
            }
        }

        helper_macros::forward_div_rem_second_arg_by_value!(impl DivRem<$t> for UBig, div_rem);

        impl DivEuclid<$t> for UBig {
            type Output = UBig;

            #[inline]
            fn div_euclid(self, rhs: $t) -> UBig {
                self.div_euclid_signed(rhs)
            }
        }

        impl DivEuclid<$t> for &UBig {
            type Output = UBig;

            #[inline]
            fn div_euclid(self, rhs: $t) -> UBig {
                self.div_euclid_ref_signed(rhs)
            }
        }

        helper_macros::forward_binop_second_arg_by_value!(impl DivEuclid<$t> for UBig, div_euclid);

        impl RemEuclid<$t> for UBig {
            type Output = $t;

            #[inline]
            fn rem_euclid(self, rhs: $t) -> $t {
                self.rem_euclid_signed(rhs)
            }
        }

        impl RemEuclid<$t> for &UBig {
            type Output = $t;

            #[inline]
            fn rem_euclid(self, rhs: $t) -> $t {
                self.rem_euclid_ref_signed(rhs)
            }
        }

        helper_macros::forward_binop_second_arg_by_value!(impl RemEuclid<$t> for UBig, rem_euclid);

        impl DivRemEuclid<$t> for UBig {
            type OutputDiv = UBig;
            type OutputRem = $t;

            #[inline]
            fn div_rem_euclid(self, rhs: $t) -> (UBig, $t) {
                self.div_rem_euclid_signed(rhs)
            }
        }

        impl DivRemEuclid<$t> for &UBig {
            type OutputDiv = UBig;
            type OutputRem = $t;

            #[inline]
            fn div_rem_euclid(self, rhs: $t) -> (UBig, $t) {
                self.div_rem_euclid_ref_signed(rhs)
            }
        }

        helper_macros::forward_div_rem_second_arg_by_value!(impl DivRemEuclid<$t> for UBig, div_rem_euclid);
    };
}

impl_div_ubig_signed!(i8);
impl_div_ubig_signed!(i16);
impl_div_ubig_signed!(i32);
impl_div_ubig_signed!(i64);
impl_div_ubig_signed!(i128);
impl_div_ubig_signed!(isize);

macro_rules! impl_div_ibig_unsigned {
    ($t:ty) => {
        impl Rem<$t> for IBig {
            // Can be negative, so does not fit in $t.
            type Output = IBig;

            #[inline]
            fn rem(self, rhs: $t) -> IBig {
                self.rem_unsigned(rhs)
            }
        }

        impl Rem<$t> for &IBig {
            type Output = IBig;

            #[inline]
            fn rem(self, rhs: $t) -> IBig {
                self.rem_ref_unsigned(rhs)
            }
        }

        impl DivRem<$t> for IBig {
            type OutputDiv = IBig;
            // Can be negative, so does not fit in $t.
            type OutputRem = IBig;

            #[inline]
            fn div_rem(self, rhs: $t) -> (IBig, IBig) {
                self.div_rem_unsigned(rhs)
            }
        }

        impl DivRem<$t> for &IBig {
            type OutputDiv = IBig;
            type OutputRem = IBig;

            #[inline]
            fn div_rem(self, rhs: $t) -> (IBig, IBig) {
                self.div_rem_ref_unsigned(rhs)
            }
        }

        impl_div_ibig_primitive!($t);
    };
}

macro_rules! impl_div_ibig_signed {
    ($t:ty) => {
        impl Rem<$t> for IBig {
            type Output = $t;

            #[inline]
            fn rem(self, rhs: $t) -> $t {
                self.rem_signed(rhs)
            }
        }

        impl Rem<$t> for &IBig {
            type Output = $t;

            #[inline]
            fn rem(self, rhs: $t) -> $t {
                self.rem_ref_signed(rhs)
            }
        }

        impl DivRem<$t> for IBig {
            type OutputDiv = IBig;
            type OutputRem = $t;

            #[inline]
            fn div_rem(self, rhs: $t) -> (IBig, $t) {
                self.div_rem_signed(rhs)
            }
        }

        impl DivRem<$t> for &IBig {
            type OutputDiv = IBig;
            type OutputRem = $t;

            #[inline]
            fn div_rem(self, rhs: $t) -> (IBig, $t) {
                self.div_rem_ref_signed(rhs)
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
                self.div_primitive(rhs)
            }
        }

        impl Div<$t> for &IBig {
            type Output = IBig;

            #[inline]
            fn div(self, rhs: $t) -> IBig {
                self.div_ref_primitive(rhs)
            }
        }

        helper_macros::forward_binop_second_arg_by_value!(impl Div<$t> for IBig, div);

        impl DivAssign<$t> for IBig {
            #[inline]
            fn div_assign(&mut self, rhs: $t) {
                self.div_assign_primitive(rhs)
            }
        }

        helper_macros::forward_binop_assign_arg_by_value!(impl DivAssign<$t> for IBig, div_assign);

        helper_macros::forward_binop_second_arg_by_value!(impl Rem<$t> for IBig, rem);

        impl RemAssign<$t> for IBig {
            #[inline]
            fn rem_assign(&mut self, rhs: $t) {
                self.rem_assign_primitive(rhs)
            }
        }

        helper_macros::forward_binop_assign_arg_by_value!(impl RemAssign<$t> for IBig, rem_assign);

        helper_macros::forward_div_rem_second_arg_by_value!(impl DivRem<$t> for IBig, div_rem);

        impl DivEuclid<$t> for IBig {
            type Output = IBig;

            #[inline]
            fn div_euclid(self, rhs: $t) -> IBig {
                self.div_euclid_primitive(rhs)
            }
        }

        impl DivEuclid<$t> for &IBig {
            type Output = IBig;

            #[inline]
            fn div_euclid(self, rhs: $t) -> IBig {
                self.div_euclid_ref_primitive(rhs)
            }
        }

        helper_macros::forward_binop_second_arg_by_value!(impl DivEuclid<$t> for IBig, div_euclid);

        impl RemEuclid<$t> for IBig {
            type Output = $t;

            #[inline]
            fn rem_euclid(self, rhs: $t) -> $t {
                self.rem_euclid_primitive(rhs)
            }
        }

        impl RemEuclid<$t> for &IBig {
            type Output = $t;

            #[inline]
            fn rem_euclid(self, rhs: $t) -> $t {
                self.rem_euclid_ref_primitive(rhs)
            }
        }

        helper_macros::forward_binop_second_arg_by_value!(impl RemEuclid<$t> for IBig, rem_euclid);

        impl DivRemEuclid<$t> for IBig {
            type OutputDiv = IBig;
            type OutputRem = $t;

            #[inline]
            fn div_rem_euclid(self, rhs: $t) -> (IBig, $t) {
                self.div_rem_euclid_primitive(rhs)
            }
        }

        impl DivRemEuclid<$t> for &IBig {
            type OutputDiv = IBig;
            type OutputRem = $t;

            #[inline]
            fn div_rem_euclid(self, rhs: $t) -> (IBig, $t) {
                self.div_rem_euclid_ref_primitive(rhs)
            }
        }

        helper_macros::forward_div_rem_second_arg_by_value!(impl DivRemEuclid<$t> for IBig, div_rem_euclid);
    };
}

impl_div_ibig_unsigned!(u8);
impl_div_ibig_unsigned!(u16);
impl_div_ibig_unsigned!(u32);
impl_div_ibig_unsigned!(u64);
impl_div_ibig_unsigned!(u128);
impl_div_ibig_unsigned!(usize);
impl_div_ibig_signed!(i8);
impl_div_ibig_signed!(i16);
impl_div_ibig_signed!(i32);
impl_div_ibig_signed!(i64);
impl_div_ibig_signed!(i128);
impl_div_ibig_signed!(isize);

mod ubig {
    use crate::buffer::{TypedRepr, TypedReprRef};

    use super::*;

    #[inline]
    pub fn div_repr_val_val(lhs: TypedRepr, rhs: TypedRepr) -> UBig {
        match (lhs, rhs) {
            (Small(dword0), Small(dword1)) => div_dword(dword0, dword1),
            (Small(_), Large(_)) => UBig::zero(),
            (Large(buffer0), Small(dword1)) => div_large_dword(buffer0, dword1),
            (Large(buffer0), Large(buffer1)) => {
                if buffer0.len() >= buffer1.len() {
                    div_large(buffer0, buffer1)
                } else {
                    UBig::zero()
                }
            }
        }
    }

    #[inline]
    pub fn div_repr_val_ref(lhs: TypedRepr, rhs: TypedReprRef) -> UBig {
        match (lhs, rhs) {
            (Small(dword0), RefSmall(dword1)) => div_dword(dword0, dword1),
            (Small(_), RefLarge(_)) => UBig::zero(),
            (Large(buffer0), RefSmall(dword1)) => div_large_dword(buffer0, dword1),
            (Large(buffer0), RefLarge(buffer1)) => {
                if buffer0.len() >= buffer1.len() {
                    div_large(buffer0, buffer1.into())
                } else {
                    UBig::zero()
                }
            }
        }
    }

    #[inline]
    pub fn div_repr_ref_val(lhs: TypedReprRef, rhs: TypedRepr) -> UBig {
        match (lhs, rhs) {
            (RefSmall(dword0), Small(dword1)) => div_dword(dword0, dword1),
            (RefSmall(_), Large(_)) => UBig::zero(),
            (RefLarge(buffer0), Small(dword1)) => div_large_dword(buffer0.into(), dword1),
            (RefLarge(buffer0), Large(buffer1)) => {
                if buffer0.len() >= buffer1.len() {
                    div_large(buffer0.into(), buffer1)
                } else {
                    UBig::zero()
                }
            }
        }
    }

    #[inline]
    pub fn div_repr_ref_ref(lhs: TypedReprRef, rhs: TypedReprRef) -> UBig {
        match (lhs, rhs) {
            (RefSmall(dword0), RefSmall(dword1)) => div_dword(dword0, dword1),
            (RefSmall(_), RefLarge(_)) => UBig::zero(),
            (RefLarge(buffer0), RefSmall(dword1)) => div_large_dword(buffer0.into(), dword1),
            (RefLarge(buffer0), RefLarge(buffer1)) => {
                if buffer0.len() >= buffer1.len() {
                    div_large(buffer0.into(), buffer1.into())
                } else {
                    UBig::zero()
                }
            }
        }
    }

    #[inline]
    pub fn rem_repr_val_val(lhs: TypedRepr, rhs: TypedRepr) -> UBig {
        match (lhs, rhs) {
            (Small(dword0), Small(dword1)) => rem_dword(dword0, dword1),
            (Small(dword0), Large(_)) => dword0.into(),
            (Large(buffer0), Small(dword1)) => rem_large_dword(&buffer0, dword1),
            (Large(buffer0), Large(buffer1)) => {
                if buffer0.len() >= buffer1.len() {
                    rem_large(buffer0, buffer1)
                } else {
                    buffer0.into()
                }
            }
        }
    }

    #[inline]
    pub fn rem_repr_val_ref(lhs: TypedRepr, rhs: TypedReprRef) -> UBig {
        match (lhs, rhs) {
            (Small(dword0), RefSmall(dword1)) => rem_dword(dword0, dword1),
            (Small(dword0), RefLarge(_)) => dword0.into(),
            (Large(buffer0), RefSmall(dword1)) => rem_large_dword(&buffer0, dword1),
            (Large(buffer0), RefLarge(buffer1)) => {
                if buffer0.len() >= buffer1.len() {
                    rem_large(buffer0, buffer1.into())
                } else {
                    buffer0.into()
                }
            }
        }
    }

    #[inline]
    pub fn rem_repr_ref_val(lhs: TypedReprRef, rhs: TypedRepr) -> UBig {
        match (lhs, rhs) {
            (RefSmall(dword0), Small(dword1)) => rem_dword(dword0, dword1),
            (RefSmall(dword0), Large(_)) => dword0.into(),
            (RefLarge(buffer0), Small(dword1)) => rem_large_dword(buffer0, dword1),
            (RefLarge(buffer0), Large(mut buffer1)) => {
                if buffer0.len() >= buffer1.len() {
                    rem_large(buffer0.into(), buffer1)
                } else {
                    // Reuse buffer1 for the remainder.
                    buffer1.clone_from_slice(buffer0);
                    buffer1.into()
                }
            }
        }
    }

    #[inline]
    pub fn rem_repr_ref_ref(lhs: TypedReprRef, rhs: TypedReprRef) -> UBig {
        match (lhs, rhs) {
            (RefSmall(dword0), RefSmall(dword1)) => rem_dword(dword0, dword1),
            (RefSmall(dword0), RefLarge(_)) => dword0.into(),
            (RefLarge(buffer0), RefSmall(dword1)) => rem_large_dword(buffer0, dword1),
            (RefLarge(buffer0), RefLarge(buffer1)) => {
                if buffer0.len() >= buffer1.len() {
                    rem_large(buffer0.into(), buffer1.into())
                } else {
                    Buffer::from(buffer0).into()
                }
            }
        }
    }

    #[inline]
    pub fn div_rem_repr_val_val(lhs: TypedRepr, rhs: TypedRepr) -> (UBig, UBig) {
        match (lhs, rhs) {
            (Small(dword0), Small(dword1)) => div_rem_dword(dword0, dword1),
            (Small(dword0), Large(_)) => (UBig::zero(), dword0.into()),
            (Large(buffer0), Small(dword1)) => div_rem_large_dword(buffer0, dword1),
            (Large(buffer0), Large(buffer1)) => {
                if buffer0.len() >= buffer1.len() {
                    div_rem_large(buffer0, buffer1)
                } else {
                    (UBig::zero(), buffer0.into())
                }
            }
        }
    }

    #[inline]
    pub fn div_rem_repr_val_ref(lhs: TypedRepr, rhs: TypedReprRef) -> (UBig, UBig) {
        match (lhs, rhs) {
            (Small(dword0), RefSmall(dword1)) => div_rem_dword(dword0, dword1),
            (Small(dword0), RefLarge(_)) => (UBig::zero(), dword0.into()),
            (Large(buffer0), RefSmall(dword1)) => div_rem_large_dword(buffer0, dword1),
            (Large(buffer0), RefLarge(buffer1)) => {
                if buffer0.len() >= buffer1.len() {
                    div_rem_large(buffer0, buffer1.into())
                } else {
                    (UBig::zero(), buffer0.into())
                }
            }
        }
    }

    #[inline]
    pub fn div_rem_repr_ref_val(lhs: TypedReprRef, rhs: TypedRepr) -> (UBig, UBig) {
        match (lhs, rhs) {
            (RefSmall(dword0), Small(dword1)) => div_rem_dword(dword0, dword1),
            (RefSmall(dword0), Large(_)) => (UBig::zero(), dword0.into()),
            (RefLarge(buffer0), Small(dword1)) => div_rem_large_dword(buffer0.into(), dword1),
            (RefLarge(buffer0), Large(mut buffer1)) => {
                if buffer0.len() >= buffer1.len() {
                    div_rem_large(buffer0.into(), buffer1)
                } else {
                    // Reuse buffer1 for the remainder.
                    buffer1.clone_from_slice(buffer0);
                    (UBig::zero(), buffer1.into())
                }
            }
        }
    }

    #[inline]
    pub fn div_rem_repr_ref_ref(lhs: TypedReprRef, rhs: TypedReprRef) -> (UBig, UBig) {
        match (lhs, rhs) {
            (RefSmall(dword0), RefSmall(dword1)) => div_rem_dword(dword0, dword1),
            (RefSmall(dword0), RefLarge(_)) => (UBig::zero(), dword0.into()),
            (RefLarge(buffer0), RefSmall(dword1)) => div_rem_large_dword(buffer0.into(), dword1),
            (RefLarge(buffer0), RefLarge(buffer1)) => {
                if buffer0.len() >= buffer1.len() {
                    div_rem_large(buffer0.into(), buffer1.into())
                } else {
                    (UBig::zero(), Buffer::from(buffer0).into())
                }
            }
        }
    }

    #[inline]
    pub fn div_dword(lhs: DoubleWord, rhs: DoubleWord) -> UBig {
        match lhs.checked_div(rhs) {
            Some(res) => UBig::from(res),
            None => panic_divide_by_0(),
        }
    }

    #[inline]
    pub fn rem_dword(lhs: DoubleWord, rhs: DoubleWord) -> UBig {
        match lhs.checked_rem(rhs) {
            Some(res) => UBig::from(res),
            None => panic_divide_by_0(),
        }
    }

    #[inline]
    pub fn div_rem_dword(lhs: DoubleWord, rhs: DoubleWord) -> (UBig, UBig) {
        // If division works, remainder also works.
        match lhs.checked_div(rhs) {
            Some(res) => (UBig::from(res), UBig::from(lhs % rhs)),
            None => panic_divide_by_0(),
        }
    }

    #[inline]
    pub fn div_large_dword(lhs: Buffer, rhs: DoubleWord) -> UBig {
        let (q, _) = div_rem_large_dword(lhs, rhs);
        q
    }

    #[inline]
    pub fn rem_large_dword(lhs: &[Word], rhs: DoubleWord) -> UBig {
        if rhs == 0 {
            panic_divide_by_0();
        }
        div::rem_by_dword(lhs, rhs).into()
    }

    pub fn div_rem_large_dword(mut buffer: Buffer, rhs: DoubleWord) -> (UBig, UBig) {
        if rhs == 0 {
            panic_divide_by_0();
        }
        let rem = div::div_by_dword_in_place(&mut buffer, rhs);
        (buffer.into(), rem.into())
    }

    pub fn div_large(mut lhs: Buffer, mut rhs: Buffer) -> UBig {
        let _shift = div_rem_in_lhs(&mut lhs, &mut rhs);
        lhs.erase_front(rhs.len());
        lhs.into()
    }

    pub fn rem_large(mut lhs: Buffer, mut rhs: Buffer) -> UBig {
        let shift = div_rem_in_lhs(&mut lhs, &mut rhs);
        let n = rhs.len();
        rhs.copy_from_slice(&lhs[..n]);
        let low_bits = shift::shr_in_place(&mut rhs, shift);
        debug_assert!(low_bits == 0);
        rhs.into()
    }

    pub fn div_rem_large(mut lhs: Buffer, mut rhs: Buffer) -> (UBig, UBig) {
        let shift = div_rem_in_lhs(&mut lhs, &mut rhs);
        let n = rhs.len();
        rhs.copy_from_slice(&lhs[..n]);
        let low_bits = shift::shr_in_place(&mut rhs, shift);
        debug_assert!(low_bits == 0);
        lhs.erase_front(n);
        (lhs.into(), rhs.into())
    }

    /// lhs = (lhs / rhs, lhs % rhs)
    ///
    /// Returns shift.
    pub fn div_rem_in_lhs(lhs: &mut Buffer, rhs: &mut Buffer) -> u32 {
        let (shift, fast_div_rhs_top) = div::normalize_large(rhs);
        let lhs_carry = shift::shl_in_place(lhs, shift);
        if lhs_carry != 0 {
            lhs.push_may_reallocate(lhs_carry);
        }
        let mut allocation =
            MemoryAllocation::new(div::memory_requirement_exact(lhs.len(), rhs.len()));
        let mut memory = allocation.memory();
        let overflow = div::div_rem_in_place(lhs, rhs, fast_div_rhs_top, &mut memory);
        if overflow {
            lhs.push_may_reallocate(1);
        }
        shift
    }
}

impl UBig {
    #[inline]
    fn div_unsigned<T: PrimitiveUnsigned>(self, rhs: T) -> UBig {
        self / UBig::from_unsigned(rhs)
    }

    #[inline]
    fn div_ref_unsigned<T: PrimitiveUnsigned>(&self, rhs: T) -> UBig {
        self / UBig::from_unsigned(rhs)
    }

    #[inline]
    fn div_assign_unsigned<T: PrimitiveUnsigned>(&mut self, rhs: T) {
        self.div_assign(UBig::from_unsigned(rhs))
    }

    #[inline]
    fn rem_unsigned<T: PrimitiveUnsigned>(self, rhs: T) -> T {
        (self % UBig::from_unsigned(rhs)).try_to_unsigned().unwrap()
    }

    #[inline]
    fn rem_ref_unsigned<T: PrimitiveUnsigned>(&self, rhs: T) -> T {
        (self % UBig::from_unsigned(rhs)).try_to_unsigned().unwrap()
    }

    #[inline]
    fn rem_assign_unsigned<T: PrimitiveUnsigned>(&mut self, rhs: T) {
        self.rem_assign(UBig::from_unsigned(rhs))
    }

    #[inline]
    fn div_rem_unsigned<T: PrimitiveUnsigned>(self, rhs: T) -> (UBig, T) {
        let (q, r) = self.div_rem(UBig::from_unsigned(rhs));
        (q, r.try_to_unsigned().unwrap())
    }

    #[inline]
    fn div_rem_ref_unsigned<T: PrimitiveUnsigned>(&self, rhs: T) -> (UBig, T) {
        let (q, r) = self.div_rem(UBig::from_unsigned(rhs));
        (q, r.try_to_unsigned().unwrap())
    }

    #[inline]
    fn div_signed<T: PrimitiveSigned>(self, rhs: T) -> UBig {
        UBig::from_ibig(IBig::from(self) / IBig::from_signed(rhs))
    }

    #[inline]
    fn div_ref_signed<T: PrimitiveSigned>(&self, rhs: T) -> UBig {
        UBig::from_ibig(IBig::from(self) / IBig::from_signed(rhs))
    }

    #[inline]
    fn div_assign_signed<T: PrimitiveSigned>(&mut self, rhs: T) {
        *self = mem::take(self).div_signed(rhs)
    }

    #[inline]
    fn rem_signed<T: PrimitiveSigned>(self, rhs: T) -> T {
        let (_, rhs_unsigned) = rhs.to_sign_magnitude();
        let res = self.rem_unsigned(rhs_unsigned);
        T::try_from_sign_magnitude(Positive, res).unwrap()
    }

    #[inline]
    fn rem_ref_signed<T: PrimitiveSigned>(&self, rhs: T) -> T {
        let (_, rhs_unsigned) = rhs.to_sign_magnitude();
        let res = self.rem_ref_unsigned(rhs_unsigned);
        T::try_from_sign_magnitude(Positive, res).unwrap()
    }

    #[inline]
    fn rem_assign_signed<T: PrimitiveSigned>(&mut self, rhs: T) {
        let res = IBig::from(mem::take(self)) % IBig::from_signed(rhs);
        *self = UBig::from_ibig(res);
    }

    #[inline]
    fn div_rem_signed<T: PrimitiveSigned>(self, rhs: T) -> (UBig, T) {
        let (q, r) = IBig::from(self).div_rem(IBig::from_signed(rhs));
        (
            UBig::from_ibig(q),
            r.try_to_signed().unwrap(),
        )
    }

    #[inline]
    fn div_rem_ref_signed<T: PrimitiveSigned>(&self, rhs: T) -> (UBig, T) {
        let (q, r) = IBig::from(self).div_rem(IBig::from_signed(rhs));
        (
            UBig::from_ibig(q),
            r.try_to_signed().unwrap(),
        )
    }

    #[inline]
    fn div_euclid_signed<T: PrimitiveSigned>(self, rhs: T) -> UBig {
        UBig::from_ibig(IBig::from(self).div_euclid(IBig::from_signed(rhs)))
    }

    #[inline]
    fn div_euclid_ref_signed<T: PrimitiveSigned>(&self, rhs: T) -> UBig {
        UBig::from_ibig(IBig::from(self).div_euclid(IBig::from_signed(rhs)))
    }

    #[inline]
    fn rem_euclid_signed<T: PrimitiveSigned>(self, rhs: T) -> T {
        self.rem_signed(rhs)
    }

    #[inline]
    fn rem_euclid_ref_signed<T: PrimitiveSigned>(&self, rhs: T) -> T {
        self.rem_ref_signed(rhs)
    }

    #[inline]
    fn div_rem_euclid_signed<T: PrimitiveSigned>(self, rhs: T) -> (UBig, T) {
        let (q, r) = IBig::from(self).div_rem_euclid(IBig::from_signed(rhs));
        (
            UBig::from_ibig(q),
            r.try_to_signed().unwrap(),
        )
    }

    #[inline]
    fn div_rem_euclid_ref_signed<T: PrimitiveSigned>(&self, rhs: T) -> (UBig, T) {
        let (q, r) = IBig::from(self).div_rem_euclid(IBig::from_signed(rhs));
        (
            UBig::from_ibig(q),
            r.try_to_signed().unwrap(),
        )
    }
}

impl IBig {
    #[inline]
    fn div_primitive<T>(self, rhs: T) -> IBig
    where
        IBig: From<T>,
    {
        self.div(IBig::from(rhs))
    }

    #[inline]
    fn div_ref_primitive<T>(&self, rhs: T) -> IBig
    where
        IBig: From<T>,
    {
        self.div(IBig::from(rhs))
    }

    #[inline]
    fn div_assign_primitive<T>(&mut self, rhs: T)
    where
        IBig: From<T>,
    {
        self.div_assign(IBig::from(rhs))
    }

    #[inline]
    fn rem_unsigned<T: PrimitiveUnsigned>(self, rhs: T) -> IBig {
        self % IBig::from_unsigned(rhs)
    }

    #[inline]
    fn rem_ref_unsigned<T: PrimitiveUnsigned>(&self, rhs: T) -> IBig {
        self % IBig::from_unsigned(rhs)
    }

    #[inline]
    fn rem_signed<T: PrimitiveSigned>(self, rhs: T) -> T {
        (self % IBig::from_signed(rhs)).try_to_signed().unwrap()
    }

    #[inline]
    fn rem_ref_signed<T: PrimitiveSigned>(&self, rhs: T) -> T {
        (self % IBig::from_signed(rhs)).try_to_signed().unwrap()
    }

    #[inline]
    fn rem_assign_primitive<T>(&mut self, rhs: T)
    where
        IBig: From<T>,
    {
        self.rem_assign(IBig::from(rhs))
    }

    #[inline]
    fn div_rem_unsigned<T: PrimitiveUnsigned>(self, rhs: T) -> (IBig, IBig) {
        self.div_rem(IBig::from_unsigned(rhs))
    }

    #[inline]
    fn div_rem_ref_unsigned<T: PrimitiveUnsigned>(&self, rhs: T) -> (IBig, IBig) {
        self.div_rem(IBig::from_unsigned(rhs))
    }

    #[inline]
    fn div_rem_signed<T: PrimitiveSigned>(self, rhs: T) -> (IBig, T) {
        let (q, r) = self.div_rem(IBig::from_signed(rhs));
        (q, r.try_to_signed().unwrap())
    }

    #[inline]
    fn div_rem_ref_signed<T: PrimitiveSigned>(&self, rhs: T) -> (IBig, T) {
        let (q, r) = self.div_rem(IBig::from_signed(rhs));
        (q, r.try_to_signed().unwrap())
    }

    #[inline]
    fn div_euclid_primitive<T>(self, rhs: T) -> IBig
    where
        IBig: From<T>,
    {
        self.div_euclid(IBig::from(rhs))
    }

    #[inline]
    fn div_euclid_ref_primitive<T>(&self, rhs: T) -> IBig
    where
        IBig: From<T>,
    {
        self.div_euclid(IBig::from(rhs))
    }

    #[inline]
    fn rem_euclid_primitive<T>(self, rhs: T) -> T
    where
        IBig: From<T>,
        T: TryFrom<IBig>,
        <T as TryFrom<IBig>>::Error: Debug,
    {
        T::try_from(self.rem_euclid(IBig::from(rhs))).unwrap()
    }

    #[inline]
    fn rem_euclid_ref_primitive<T>(&self, rhs: T) -> T
    where
        IBig: From<T>,
        T: TryFrom<IBig>,
        <T as TryFrom<IBig>>::Error: Debug,
    {
        T::try_from(self.rem_euclid(IBig::from(rhs))).unwrap()
    }

    #[inline]
    fn div_rem_euclid_primitive<T>(self, rhs: T) -> (IBig, T)
    where
        IBig: From<T>,
        T: TryFrom<IBig>,
        <T as TryFrom<IBig>>::Error: Debug,
    {
        let (q, r) = self.div_rem_euclid(IBig::from(rhs));
        (q, T::try_from(r).unwrap())
    }

    #[inline]
    fn div_rem_euclid_ref_primitive<T>(&self, rhs: T) -> (IBig, T)
    where
        IBig: From<T>,
        T: TryFrom<IBig>,
        <T as TryFrom<IBig>>::Error: Debug,
    {
        let (q, r) = self.div_rem_euclid(IBig::from(rhs));
        (q, T::try_from(r).unwrap())
    }
}

// TODO: organize all panic into error.rs
fn panic_divide_by_0() -> ! {
    panic!("divide by 0")
}
