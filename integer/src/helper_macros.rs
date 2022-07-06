/// Implement `impl Op<B> for &A` by forwarding to `impl Op<B> for A`, including &B.
macro_rules! forward_binop_first_arg_by_value {
    (impl $tr:ident<$t2:ty> for $t1:ty, $f:ident) => {
        impl $tr<$t2> for &$t1 {
            type Output = <$t1 as $tr<$t2>>::Output;

            #[inline]
            fn $f(self, rhs: $t2) -> Self::Output {
                (*self).$f(rhs)
            }
        }

        impl<'a> $tr<&'a $t2> for &$t1 {
            type Output = <$t1 as $tr<&'a $t2>>::Output;

            #[inline]
            fn $f(self, rhs: &$t2) -> Self::Output {
                (*self).$f(rhs)
            }
        }
    };
}

/// Implement `impl Op<&B> for A` by forwarding to `impl Op<B> for A`, including &A.
macro_rules! forward_binop_second_arg_by_value {
    (impl $tr:ident<$t2:ty> for $t1:ty, $f:ident) => {
        impl $tr<&$t2> for $t1 {
            type Output = <$t1 as $tr<$t2>>::Output;

            #[inline]
            fn $f(self, rhs: &$t2) -> Self::Output {
                self.$f(*rhs)
            }
        }

        impl<'a> $tr<&$t2> for &'a $t1 {
            type Output = <&'a $t1 as $tr<$t2>>::Output;

            #[inline]
            fn $f(self, rhs: &$t2) -> Self::Output {
                self.$f(*rhs)
            }
        }
    };
}

/// Implement `impl Op<&B> for A` by forwarding to `impl Op<B> for A`, including &A.
/// Here Op has OutputDiv and OutputRem, rather than just Output.
/// 
macro_rules! forward_div_rem_second_arg_by_value {
    (impl $tr:ident<$t2:ty> for $t1:ty, $f:ident) => {
        impl $tr<&$t2> for $t1 {
            type OutputDiv = <$t1 as $tr<$t2>>::OutputDiv;
            type OutputRem = <$t1 as $tr<$t2>>::OutputRem;

            #[inline]
            fn $f(self, rhs: &$t2) -> (Self::OutputDiv, Self::OutputRem) {
                self.$f(*rhs)
            }
        }

        impl<'a> $tr<&$t2> for &'a $t1 {
            type OutputDiv = <&'a $t1 as $tr<$t2>>::OutputDiv;
            type OutputRem = <&'a $t1 as $tr<$t2>>::OutputRem;

            #[inline]
            fn $f(self, rhs: &$t2) -> (Self::OutputDiv, Self::OutputRem) {
                self.$f(*rhs)
            }
        }
    };
}

/// Implement `impl Op<B> for A` by forwarding to `impl Op<A> for B`, including &A and &B.
macro_rules! forward_binop_swap_args {
    (impl $tr:ident<$t2:ty> for $t1:ty, $f:ident) => {
        impl $tr<$t2> for $t1 {
            type Output = <$t2 as $tr<$t1>>::Output;

            #[inline]
            fn $f(self, rhs: $t2) -> Self::Output {
                rhs.$f(self)
            }
        }

        impl<'a> $tr<&'a $t2> for $t1 {
            type Output = <&'a $t2 as $tr<$t1>>::Output;

            #[inline]
            fn $f(self, rhs: &$t2) -> Self::Output {
                rhs.$f(self)
            }
        }

        impl<'a> $tr<$t2> for &'a $t1 {
            type Output = <$t2 as $tr<&'a $t1>>::Output;

            #[inline]
            fn $f(self, rhs: $t2) -> Self::Output {
                rhs.$f(self)
            }
        }

        impl<'a, 'b> $tr<&'a $t2> for &'b $t1 {
            type Output = <&'a $t2 as $tr<&'b $t1>>::Output;

            #[inline]
            fn $f(self, rhs: &$t2) -> Self::Output {
                rhs.$f(self)
            }
        }
    };
}

/// Implement `impl OpAssign<&B> for A` by forwarding to `impl OpAssign<B> for A`.
macro_rules! forward_binop_assign_arg_by_value {
    (impl $tr:ident<$t2:ty> for $t1:ty, $f:ident) => {
        impl $tr<&$t2> for $t1 {
            #[inline]
            fn $f(&mut self, rhs: &$t2) {
                self.$f(*rhs)
            }
        }
    };
}

/// Implement `impl Op<UBig> for UBig` by forwarding to `lhs.repr().op(rhs.repr())`, including &UBig.
/// The output type is UBig.
macro_rules! forward_ubig_binop_to_repr {
    (impl $tr:ident, $f:ident) => {
        impl $tr<UBig> for UBig {
            type Output = UBig;

            #[inline]
            fn $f(self, rhs: UBig) -> UBig {
                UBig(self.into_repr().$f(rhs.into_repr()))
            }
        }

        impl<'r> $tr<&'r UBig> for UBig {
            type Output = UBig;

            #[inline]
            fn $f(self, rhs: &UBig) -> UBig {
                UBig(self.into_repr().$f(rhs.repr()))
            }
        }

        impl<'l> $tr<UBig> for &'l UBig {
            type Output = UBig;

            #[inline]
            fn $f(self, rhs: UBig) -> UBig {
                UBig(self.repr().$f(rhs.into_repr()))
            }
        }

        impl<'l, 'r> $tr<&'r UBig> for &'l UBig {
            type Output = UBig;

            #[inline]
            fn $f(self, rhs: &UBig) -> UBig {
                UBig(self.repr().$f(rhs.repr()))
            }
        }
    };
}

/// Implement `impl OpAssign<B> for A` by forwarding to `*A = mem::take(A).op(B)`, including &B.
macro_rules! forward_binop_assign_by_taking {
    (impl $tr:ident<$t2:ty> for $t1:ty, $fassign:ident, $f:ident) => {
        impl $tr<$t2> for $t1 {
            #[inline]
            fn $fassign(&mut self, rhs: $t2) {
                *self = core::mem::take(self).$f(rhs);
            }
        }
        impl $tr<&$t2> for $t1 {
            #[inline]
            fn $fassign(&mut self, rhs: &$t2) {
                *self = core::mem::take(self).$f(rhs);
            }
        }
    };
}

/// Implement `impl Add<IBig> for IBig` by forwarding to the Repr
macro_rules! forword_ibig_add_to_repr {
    ($lhs_sign:ident, $lhs_mag:ident, $rhs_sign:ident, $rhs_mag:ident) => {
        match ($lhs_sign, $rhs_sign) {
            (crate::sign::Sign::Positive, crate::sign::Sign::Positive) => IBig($lhs_mag.add($rhs_mag)),
            (crate::sign::Sign::Positive, crate::sign::Sign::Negative) => IBig($lhs_mag.sub_signed($rhs_mag)),
            (crate::sign::Sign::Negative, crate::sign::Sign::Positive) => IBig($rhs_mag.sub_signed($lhs_mag)),
            (crate::sign::Sign::Negative, crate::sign::Sign::Negative) => IBig($lhs_mag.add($rhs_mag).neg()),
        }
    };
}

/// Implement `impl BitAnd<IBig> for IBig` by forwarding to the Repr
macro_rules! forword_ibig_bitand_to_repr {
    ($lhs_sign:ident, $lhs_mag:ident, $rhs_sign:ident, $rhs_mag:ident) => {
        match ($lhs_sign, $rhs_sign) {
            (crate::sign::Sign::Positive, crate::sign::Sign::Positive) => IBig($lhs_mag.bitand($rhs_mag)),
            (crate::sign::Sign::Positive, crate::sign::Sign::Negative) => IBig($lhs_mag.and_not($rhs_mag.sub_one().into_typed())),
            (crate::sign::Sign::Negative, crate::sign::Sign::Positive) => IBig($rhs_mag.and_not($lhs_mag.sub_one().into_typed())),
            (crate::sign::Sign::Negative, crate::sign::Sign::Negative) => {
                IBig($lhs_mag.sub_one().into_typed().bitor($rhs_mag.sub_one().into_typed())).not()
            }
        }
    };
}

/// Implement `impl BitOr<IBig> for IBig` by forwarding to the Repr
macro_rules! forword_ibig_bitor_to_repr {
    ($lhs_sign:ident, $lhs_mag:ident, $rhs_sign:ident, $rhs_mag:ident) => {
        match ($lhs_sign, $rhs_sign) {
            (crate::sign::Sign::Positive, crate::sign::Sign::Positive) => IBig($lhs_mag.bitor($rhs_mag)),
            (crate::sign::Sign::Positive, crate::sign::Sign::Negative) => IBig($rhs_mag.sub_one().into_typed().and_not($lhs_mag)).not(),
            (crate::sign::Sign::Negative, crate::sign::Sign::Positive) => IBig($lhs_mag.sub_one().into_typed().and_not($rhs_mag)).not(),
            (crate::sign::Sign::Negative, crate::sign::Sign::Negative) => {
                IBig($lhs_mag.sub_one().into_typed().bitand($rhs_mag.sub_one().into_typed())).not()
            }
        }
    };
}

/// Implement `impl BitXor<IBig> for IBig` by forwarding to the Repr
macro_rules! forword_ibig_bitxor_to_repr {
    ($lhs_sign:ident, $lhs_mag:ident, $rhs_sign:ident, $rhs_mag:ident) => {
        match ($lhs_sign, $rhs_sign) {
            (crate::sign::Sign::Positive, crate::sign::Sign::Positive) => IBig($lhs_mag.bitxor($rhs_mag)),
            (crate::sign::Sign::Positive, crate::sign::Sign::Negative) => IBig($lhs_mag.bitxor($rhs_mag.sub_one().into_typed())).not(),
            (crate::sign::Sign::Negative, crate::sign::Sign::Positive) => IBig($lhs_mag.sub_one().into_typed().bitxor($rhs_mag)).not(),
            (crate::sign::Sign::Negative, crate::sign::Sign::Negative) => {
                IBig($lhs_mag.sub_one().into_typed().bitxor($rhs_mag.sub_one().into_typed()))
            }
        }
    };
}

pub(crate) use forward_binop_assign_arg_by_value;
pub(crate) use forward_binop_assign_by_taking;
pub(crate) use forward_binop_first_arg_by_value;
pub(crate) use forward_binop_second_arg_by_value;
pub(crate) use forward_binop_swap_args;
pub(crate) use forward_div_rem_second_arg_by_value;
pub(crate) use forward_ubig_binop_to_repr;
pub(crate) use forword_ibig_add_to_repr;
pub(crate) use forword_ibig_bitand_to_repr;
pub(crate) use forword_ibig_bitor_to_repr;
pub(crate) use forword_ibig_bitxor_to_repr;
