/// Assert the expression value is zero in debug mode, but the expression will
/// still run in release mode. It's usually used for assertions on overflow.
macro_rules! debug_assert_zero {
    ($($arg:tt)*) => {{
        let __check__ = $($arg)*;
        debug_assert_eq!(__check__ as $crate::arch::word::DoubleWord, 0);
    }};
}

/// Implement `impl Op<B> for &A` by forwarding to `impl Op<B> for A`, including &B.
macro_rules! forward_binop_first_arg_by_value {
    (impl $trait:ident<$t2:ty> for $t1:ty, $method:ident) => {
        impl $trait<$t2> for &$t1 {
            type Output = <$t1 as $trait<$t2>>::Output;

            #[inline]
            fn $method(self, rhs: $t2) -> Self::Output {
                (*self).$method(rhs)
            }
        }

        impl<'a> $trait<&'a $t2> for &$t1 {
            type Output = <$t1 as $trait<&'a $t2>>::Output;

            #[inline]
            fn $method(self, rhs: &$t2) -> Self::Output {
                (*self).$method(rhs)
            }
        }
    };
}

/// Implement `impl Op<&B> for A` by forwarding to `impl Op<B> for A`, including &A.
macro_rules! forward_binop_second_arg_by_value {
    (impl $trait:ident<$t2:ty> for $t1:ty, $method:ident) => {
        impl $trait<&$t2> for $t1 {
            type Output = <$t1 as $trait<$t2>>::Output;

            #[inline]
            fn $method(self, rhs: &$t2) -> Self::Output {
                self.$method(*rhs)
            }
        }

        impl<'a> $trait<&$t2> for &'a $t1 {
            type Output = <&'a $t1 as $trait<$t2>>::Output;

            #[inline]
            fn $method(self, rhs: &$t2) -> Self::Output {
                self.$method(*rhs)
            }
        }
    };
}

/// Implement `impl Op<&B> for A` by forwarding to `impl Op<B> for A`, including &A.
/// Here Op has OutputDiv and OutputRem, rather than just Output.
macro_rules! forward_div_rem_second_arg_by_value {
    (impl $trait:ident<$t2:ty> for $t1:ty, $method:ident) => {
        impl $trait<&$t2> for $t1 {
            type OutputDiv = <$t1 as $trait<$t2>>::OutputDiv;
            type OutputRem = <$t1 as $trait<$t2>>::OutputRem;

            #[inline]
            fn $method(self, rhs: &$t2) -> (Self::OutputDiv, Self::OutputRem) {
                self.$method(*rhs)
            }
        }

        impl<'a> $trait<&$t2> for &'a $t1 {
            type OutputDiv = <&'a $t1 as $trait<$t2>>::OutputDiv;
            type OutputRem = <&'a $t1 as $trait<$t2>>::OutputRem;

            #[inline]
            fn $method(self, rhs: &$t2) -> (Self::OutputDiv, Self::OutputRem) {
                self.$method(*rhs)
            }
        }
    };
}

/// Implement `impl Op<B> for A` by forwarding to `impl Op<A> for B`, including &A and &B.
macro_rules! forward_binop_swap_args {
    (impl $trait:ident<$t2:ty> for $t1:ty, $method:ident) => {
        impl $trait<$t2> for $t1 {
            type Output = <$t2 as $trait<$t1>>::Output;

            #[inline]
            fn $method(self, rhs: $t2) -> Self::Output {
                rhs.$method(self)
            }
        }

        impl<'a> $trait<&'a $t2> for $t1 {
            type Output = <&'a $t2 as $trait<$t1>>::Output;

            #[inline]
            fn $method(self, rhs: &$t2) -> Self::Output {
                rhs.$method(self)
            }
        }

        impl<'a> $trait<$t2> for &'a $t1 {
            type Output = <$t2 as $trait<&'a $t1>>::Output;

            #[inline]
            fn $method(self, rhs: $t2) -> Self::Output {
                rhs.$method(self)
            }
        }

        impl<'a, 'b> $trait<&'a $t2> for &'b $t1 {
            type Output = <&'a $t2 as $trait<&'b $t1>>::Output;

            #[inline]
            fn $method(self, rhs: &$t2) -> Self::Output {
                rhs.$method(self)
            }
        }
    };
}

/// Implement `impl OpAssign<&B> for A` by forwarding to `impl OpAssign<B> for A`.
macro_rules! forward_binop_assign_arg_by_value {
    (impl $trait:ident<$t2:ty> for $t1:ty, $method:ident) => {
        impl $trait<&$t2> for $t1 {
            #[inline]
            fn $method(&mut self, rhs: &$t2) {
                self.$method(*rhs)
            }
        }
    };
}

/// Implement `impl Op<UBig> for UBig` by forwarding to `lhs.repr().op(rhs.repr())`, including &UBig.
/// The output type is UBig.
macro_rules! forward_ubig_binop_to_repr {
    // normal operator
    (impl $trait:ident, $method:ident) => {
        crate::helper_macros::forward_ubig_binop_to_repr!(impl $trait, $method, $method);
    };
    // operator with different forwarded function
    (impl $trait:ident, $method:ident, $forward:ident) => {
        impl $trait<UBig> for UBig {
            type Output = UBig;

            #[inline]
            fn $method(self, rhs: UBig) -> UBig {
                UBig(self.into_repr().$forward(rhs.into_repr()))
            }
        }

        impl<'r> $trait<&'r UBig> for UBig {
            type Output = UBig;

            #[inline]
            fn $method(self, rhs: &UBig) -> UBig {
                UBig(self.into_repr().$forward(rhs.repr()))
            }
        }

        impl<'l> $trait<UBig> for &'l UBig {
            type Output = UBig;

            #[inline]
            fn $method(self, rhs: UBig) -> UBig {
                UBig(self.repr().$forward(rhs.into_repr()))
            }
        }

        impl<'l, 'r> $trait<&'r UBig> for &'l UBig {
            type Output = UBig;

            #[inline]
            fn $method(self, rhs: &UBig) -> UBig {
                UBig(self.repr().$forward(rhs.repr()))
            }
        }
    };
    (impl $trait:ident as divrem, $method:ident) => {
        crate::helper_macros::forward_ubig_binop_to_repr!(impl $trait as divrem, $method, $method);
    };
    // special case for div_rem related ops, with two output types
    (impl $trait:ident as divrem, $method:ident, $forward:ident) => {
        impl $trait<UBig> for UBig {
            type OutputDiv = UBig;
            type OutputRem = UBig;

            #[inline]
            fn $method(self, rhs: UBig) -> (UBig, UBig) {
                let (q, r) = self.into_repr().$forward(rhs.into_repr());
                (UBig(q), UBig(r))
            }
        }

        impl<'r> $trait<&'r UBig> for UBig {
            type OutputDiv = UBig;
            type OutputRem = UBig;

            #[inline]
            fn $method(self, rhs: &UBig) -> (UBig, UBig) {
                let (q, r) = self.into_repr().$forward(rhs.repr());
                (UBig(q), UBig(r))
            }
        }

        impl<'l> $trait<UBig> for &'l UBig {
            type OutputDiv = UBig;
            type OutputRem = UBig;

            #[inline]
            fn $method(self, rhs: UBig) -> (UBig, UBig) {
                let (q, r) = self.repr().$forward(rhs.into_repr());
                (UBig(q), UBig(r))
            }
        }

        impl<'l, 'r> $trait<&'r UBig> for &'l UBig {
            type OutputDiv = UBig;
            type OutputRem = UBig;

            #[inline]
            fn $method(self, rhs: &UBig) -> (UBig, UBig) {
                let (q, r) = self.repr().$forward(rhs.repr());
                (UBig(q), UBig(r))
            }
        }
    };
}

/// Implement `impl Op<IBig> for IBig` by forwarding to the macro `$impl` with arguments
/// `(lhs_sign, lhs_repr, rhs_sign, rhs_repr)`, including &IBig.
/// The output type is IBig.
macro_rules! forward_ibig_binop_to_repr {
    (impl $trait:ident, $method:ident, $impl:ident) => {
        impl $trait<IBig> for IBig {
            type Output = IBig;

            #[inline]
            fn $method(self, rhs: IBig) -> IBig {
                let (sign0, mag0) = self.into_sign_repr();
                let (sign1, mag1) = rhs.into_sign_repr();
                $impl!(sign0, mag0, sign1, mag1)
            }
        }

        impl<'r> $trait<&'r IBig> for IBig {
            type Output = IBig;

            #[inline]
            fn $method(self, rhs: &IBig) -> IBig {
                let (sign0, mag0) = self.into_sign_repr();
                let (sign1, mag1) = rhs.as_sign_repr();
                $impl!(sign0, mag0, sign1, mag1)
            }
        }

        impl<'l> $trait<IBig> for &'l IBig {
            type Output = IBig;

            #[inline]
            fn $method(self, rhs: IBig) -> IBig {
                let (sign0, mag0) = self.as_sign_repr();
                let (sign1, mag1) = rhs.into_sign_repr();
                $impl!(sign0, mag0, sign1, mag1)
            }
        }

        impl<'l, 'r> $trait<&'r IBig> for &'l IBig {
            type Output = IBig;

            #[inline]
            fn $method(self, rhs: &IBig) -> IBig {
                let (sign0, mag0) = self.as_sign_repr();
                let (sign1, mag1) = rhs.as_sign_repr();
                $impl!(sign0, mag0, sign1, mag1)
            }
        }
    };
    (impl $trait:ident as divrem, $method:ident, $impl:ident) => {
        impl $trait<IBig> for IBig {
            type OutputDiv = IBig;
            type OutputRem = IBig;

            #[inline]
            fn $method(self, rhs: IBig) -> (IBig, IBig) {
                let (sign0, mag0) = self.into_sign_repr();
                let (sign1, mag1) = rhs.into_sign_repr();
                $impl!(sign0, mag0, sign1, mag1)
            }
        }

        impl<'r> $trait<&'r IBig> for IBig {
            type OutputDiv = IBig;
            type OutputRem = IBig;

            #[inline]
            fn $method(self, rhs: &IBig) -> (IBig, IBig) {
                let (sign0, mag0) = self.into_sign_repr();
                let (sign1, mag1) = rhs.as_sign_repr();
                $impl!(sign0, mag0, sign1, mag1)
            }
        }

        impl<'l> $trait<IBig> for &'l IBig {
            type OutputDiv = IBig;
            type OutputRem = IBig;

            #[inline]
            fn $method(self, rhs: IBig) -> (IBig, IBig) {
                let (sign0, mag0) = self.as_sign_repr();
                let (sign1, mag1) = rhs.into_sign_repr();
                $impl!(sign0, mag0, sign1, mag1)
            }
        }

        impl<'l, 'r> $trait<&'r IBig> for &'l IBig {
            type OutputDiv = IBig;
            type OutputRem = IBig;

            #[inline]
            fn $method(self, rhs: &IBig) -> (IBig, IBig) {
                let (sign0, mag0) = self.as_sign_repr();
                let (sign1, mag1) = rhs.as_sign_repr();
                $impl!(sign0, mag0, sign1, mag1)
            }
        }
    };
}

/// Implement `impl OpAssign<B> for A` by forwarding to `*A = mem::take(A).op(B)`, including &B.
macro_rules! forward_binop_assign_by_taking {
    (impl $trait:ident<$t2:ty> for $t1:ty, $methodassign:ident, $method:ident) => {
        impl $trait<$t2> for $t1 {
            #[inline]
            fn $methodassign(&mut self, rhs: $t2) {
                *self = core::mem::take(self).$method(rhs);
            }
        }
        impl $trait<&$t2> for $t1 {
            #[inline]
            fn $methodassign(&mut self, rhs: &$t2) {
                *self = core::mem::take(self).$method(rhs);
            }
        }
    };
}

/// Implement `impl Op<IBig> for UBig` by forwarding to the macro `$impl` with arguments
/// `(self_repr, rhs_sign, rhs_repr)`
macro_rules! forward_ubig_ibig_binop_to_repr {
    (impl $trait:ident, $method:ident, $impl:ident) => {
        impl $trait<IBig> for UBig {
            type Output = IBig;

            #[inline]
            fn $method(self, rhs: IBig) -> IBig {
                let lhs_mag = self.into_repr();
                let (rhs_sign, rhs_mag) = rhs.into_sign_repr();
                $impl!(lhs_mag, rhs_sign, rhs_mag)
            }
        }

        impl<'r> $trait<&'r IBig> for UBig {
            type Output = IBig;

            #[inline]
            fn $method(self, rhs: &IBig) -> IBig {
                let lhs_mag = self.into_repr();
                let (rhs_sign, rhs_mag) = rhs.as_sign_repr();
                $impl!(lhs_mag, rhs_sign, rhs_mag)
            }
        }

        impl<'l> $trait<IBig> for &'l UBig {
            type Output = IBig;

            #[inline]
            fn $method(self, rhs: IBig) -> IBig {
                let lhs_mag = self.repr();
                let (rhs_sign, rhs_mag) = rhs.into_sign_repr();
                $impl!(lhs_mag, rhs_sign, rhs_mag)
            }
        }

        impl<'l, 'r> $trait<&'r IBig> for &'l UBig {
            type Output = IBig;

            #[inline]
            fn $method(self, rhs: &IBig) -> IBig {
                let lhs_mag = self.repr();
                let (rhs_sign, rhs_mag) = rhs.as_sign_repr();
                $impl!(lhs_mag, rhs_sign, rhs_mag)
            }
        }
    };
}

/// Implement `impl Op<UBig> for IBig` by forwarding to the macro `$impl` with arguments
/// `(self_sign, self_repr, rhs_repr)`
macro_rules! forward_ibig_ubig_binop_to_repr {
    (impl $trait:ident, $method:ident, $impl:ident) => {
        impl $trait<UBig> for IBig {
            type Output = IBig;

            #[inline]
            fn $method(self, rhs: UBig) -> IBig {
                let (lhs_sign, lhs_mag) = self.into_sign_repr();
                let rhs_mag = rhs.into_repr();
                $impl!(lhs_sign, lhs_mag, rhs_mag)
            }
        }

        impl<'r> $trait<&'r UBig> for IBig {
            type Output = IBig;

            #[inline]
            fn $method(self, rhs: &UBig) -> IBig {
                let (lhs_sign, lhs_mag) = self.into_sign_repr();
                let rhs_mag = rhs.repr();
                $impl!(lhs_sign, lhs_mag, rhs_mag)
            }
        }

        impl<'l> $trait<UBig> for &'l IBig {
            type Output = IBig;

            #[inline]
            fn $method(self, rhs: UBig) -> IBig {
                let (lhs_sign, lhs_mag) = self.as_sign_repr();
                let rhs_mag = rhs.into_repr();
                $impl!(lhs_sign, lhs_mag, rhs_mag)
            }
        }

        impl<'l, 'r> $trait<&'r UBig> for &'l IBig {
            type Output = IBig;

            #[inline]
            fn $method(self, rhs: &UBig) -> IBig {
                let (lhs_sign, lhs_mag) = self.as_sign_repr();
                let rhs_mag = rhs.repr();
                $impl!(lhs_sign, lhs_mag, rhs_mag)
            }
        }
    };
}

pub(crate) use debug_assert_zero;
pub(crate) use forward_binop_assign_arg_by_value;
pub(crate) use forward_binop_assign_by_taking;
pub(crate) use forward_binop_first_arg_by_value;
pub(crate) use forward_binop_second_arg_by_value;
pub(crate) use forward_binop_swap_args;
pub(crate) use forward_div_rem_second_arg_by_value;
pub(crate) use forward_ibig_binop_to_repr;
pub(crate) use forward_ibig_ubig_binop_to_repr;
pub(crate) use forward_ubig_binop_to_repr;
pub(crate) use forward_ubig_ibig_binop_to_repr;
