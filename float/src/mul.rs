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

impl<R: Round, const B: Word> MulAssign for FBig<R, B> {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        *self = core::mem::take(self) * rhs
    }
}
impl<R: Round, const B: Word> MulAssign<&FBig<R, B>> for FBig<R, B> {
    #[inline]
    fn mul_assign(&mut self, rhs: &FBig<R, B>) {
        *self = core::mem::take(self) * rhs
    }
}

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
        if self.repr.is_infinite() {
            panic_operate_with_inf();
        }

        let repr = Repr::new(self.repr.significand.square(), 2 * self.repr.exponent);
        FBig::new_raw(self.context.repr_round(repr).value(), self.context)
    }
}

impl<R: Round> Context<R> {
    pub fn mul<const B: Word>(&self, lhs: &FBig<R, B>, rhs: &FBig<R, B>) -> Rounded<FBig<R, B>> {
        check_inf_operands(&lhs.repr, &rhs.repr);

        // TODO: shrink lhs and rhs to at most double the precision before mul
        let repr = Repr::new(
            &lhs.repr.significand * &rhs.repr.significand,
            lhs.repr.exponent + rhs.repr.exponent,
        );
        self.repr_round(repr).map(|v| FBig::new_raw(v, *self))
    }
}
