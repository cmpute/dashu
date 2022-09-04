use dashu_int::{UBig, IBig};

use crate::{
    error::{check_inf_operands, panic_operate_with_inf},
    fbig::FBig,
    helper_macros,
    repr::{Context, Repr, Word},
    round::{Round, Rounded},
};
use core::ops::{Mul, MulAssign};

impl<'l, 'r, const B: Word, R: Round> Mul<&'r FBig<R, B>> for &'l FBig<R, B> {
    type Output = FBig<R, B>;

    #[inline]
    fn mul(self, rhs: &FBig<R, B>) -> Self::Output {
        check_inf_operands(&self.repr, &rhs.repr);

        let context = Context::max(self.context, rhs.context);
        let repr = Repr::new(
            &self.repr.significand * &rhs.repr.significand,
            self.repr.exponent + rhs.repr.exponent,
        );
        FBig::new_raw(context.repr_round(repr).value(), context)
    }
}

impl<'r, const B: Word, R: Round> Mul<&'r FBig<R, B>> for FBig<R, B> {
    type Output = FBig<R, B>;

    #[inline]
    fn mul(self, rhs: &FBig<R, B>) -> Self::Output {
        check_inf_operands(&self.repr, &rhs.repr);

        let context = Context::max(self.context, rhs.context);
        let repr = Repr::new(
            self.repr.significand * &rhs.repr.significand,
            self.repr.exponent + rhs.repr.exponent,
        );
        FBig::new_raw(context.repr_round(repr).value(), context)
    }
}

impl<'l, const B: Word, R: Round> Mul<FBig<R, B>> for &'l FBig<R, B> {
    type Output = FBig<R, B>;

    #[inline]
    fn mul(self, rhs: FBig<R, B>) -> Self::Output {
        check_inf_operands(&self.repr, &rhs.repr);

        let context = Context::max(self.context, rhs.context);
        let repr = Repr::new(
            &self.repr.significand * rhs.repr.significand,
            self.repr.exponent + rhs.repr.exponent,
        );
        FBig::new_raw(context.repr_round(repr).value(), context)
    }
}

impl<R: Round, const B: Word> Mul<FBig<R, B>> for FBig<R, B> {
    type Output = FBig<R, B>;

    #[inline]
    fn mul(self, rhs: FBig<R, B>) -> Self::Output {
        check_inf_operands(&self.repr, &rhs.repr);

        let context = Context::max(self.context, rhs.context);
        let repr = Repr::new(
            self.repr.significand * rhs.repr.significand,
            self.repr.exponent + rhs.repr.exponent,
        );
        FBig::new_raw(context.repr_round(repr).value(), context)
    }
}

helper_macros::impl_binop_assign_by_taking!(impl MulAssign<Self>, mul_assign, mul);

macro_rules! impl_add_sub_primitive_with_fbig {
    ($($t:ty)*) => {$(
        helper_macros::impl_commutative_binop_with_primitive!(impl Mul<$t>, mul);
        helper_macros::impl_binop_assign_with_primitive!(impl MulAssign<$t>, mul_assign);
    )*};
}
impl_add_sub_primitive_with_fbig!(u8 u16 u32 u64 u128 usize UBig i8 i16 i32 i64 i128 isize IBig);

impl<R: Round, const B: Word> FBig<R, B> {
    #[inline]
    pub fn square(&self) -> Self {
        self.context.square(self).value()
    }
}

impl<R: Round> Context<R> {
    pub fn mul<_R1: Round, _R2: Round, const B: Word>(&self, lhs: &FBig<_R1, B>, rhs: &FBig<_R2, B>) -> Rounded<FBig<R, B>> {
        check_inf_operands(&lhs.repr, &rhs.repr);

        // at most double the precision is required to get a correct result
        // shrink the input operands if necessary
        let max_precision = self.precision * 2;

        let lhs_shrink;
        let lhs_repr = if lhs.digits() > max_precision {
            lhs_shrink = Context::<R>::new(max_precision).repr_round_ref(&lhs.repr).value();
            &lhs_shrink
        } else {
            &lhs.repr
        };

        let rhs_shrink;
        let rhs_repr = if rhs.digits() > max_precision {
            rhs_shrink = Context::<R>::new(max_precision).repr_round_ref(&rhs.repr).value();
            &rhs_shrink
        } else {
            &rhs.repr
        };

        let repr = Repr::new(
            &lhs_repr.significand * &rhs_repr.significand,
            lhs_repr.exponent + rhs_repr.exponent,
        );
        self.repr_round(repr).map(|v| FBig::new_raw(v, *self))
    }

    pub fn square<_R: Round, const B: Word>(&self, f: &FBig<_R, B>) -> Rounded<FBig<R, B>> {
        if f.repr.is_infinite() {
            panic_operate_with_inf();
        }

        // shrink the input operands if necessary
        let max_precision = self.precision * 2;
        let f_shrink;
        let f_repr = if f.repr.digits() > max_precision {
            f_shrink = Context::<R>::new(max_precision).repr_round_ref(&f.repr).value();
            &f_shrink
        } else {
            &f.repr
        };

        let repr = Repr::new(f_repr.significand.square(), 2 * f_repr.exponent);
        self.repr_round(repr).map(|v| FBig::new_raw(v, *self))
    }
}
