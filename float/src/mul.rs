use crate::{
    fbig::FBig,
    error::{check_inf_operands, panic_operate_with_inf},
    repr::{Context, Repr, Word},
    round::{Round, Rounded},
};
use core::ops::{Mul, MulAssign};

impl<'l, 'r, const B: Word, R: Round> Mul<&'r FBig<B, R>> for &'l FBig<B, R> {
    type Output = FBig<B, R>;

    #[inline]
    fn mul(self, rhs: &FBig<B, R>) -> Self::Output {
        check_inf_operands(&self.repr, &rhs.repr);

        let context = Context::max(self.context, rhs.context);
        let repr = Repr::new(
            &self.repr.significand * &rhs.repr.significand,
            self.repr.exponent + rhs.repr.exponent,
        );
        FBig::new_raw(context.repr_round(repr).value(), context)
    }
}

impl<'r, const B: Word, R: Round> Mul<&'r FBig<B, R>> for FBig<B, R> {
    type Output = FBig<B, R>;

    #[inline]
    fn mul(self, rhs: &FBig<B, R>) -> Self::Output {
        check_inf_operands(&self.repr, &rhs.repr);

        let context = Context::max(self.context, rhs.context);
        let repr = Repr::new(
            self.repr.significand * &rhs.repr.significand,
            self.repr.exponent + rhs.repr.exponent,
        );
        FBig::new_raw(context.repr_round(repr).value(), context)
    }
}

impl<'l, const B: Word, R: Round> Mul<FBig<B, R>> for &'l FBig<B, R> {
    type Output = FBig<B, R>;

    #[inline]
    fn mul(self, rhs: FBig<B, R>) -> Self::Output {
        check_inf_operands(&self.repr, &rhs.repr);

        let context = Context::max(self.context, rhs.context);
        let repr = Repr::new(
            &self.repr.significand * rhs.repr.significand,
            self.repr.exponent + rhs.repr.exponent,
        );
        FBig::new_raw(context.repr_round(repr).value(), context)
    }
}

impl<const B: Word, R: Round> Mul<FBig<B, R>> for FBig<B, R> {
    type Output = FBig<B, R>;

    #[inline]
    fn mul(self, rhs: FBig<B, R>) -> Self::Output {
        check_inf_operands(&self.repr, &rhs.repr);

        let context = Context::max(self.context, rhs.context);
        let repr = Repr::new(
            self.repr.significand * rhs.repr.significand,
            self.repr.exponent + rhs.repr.exponent,
        );
        FBig::new_raw(context.repr_round(repr).value(), context)
    }
}

impl<const B: Word, R: Round> MulAssign for FBig<B, R> {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        *self = core::mem::take(self) * rhs
    }
}
impl<const B: Word, R: Round> MulAssign<&FBig<B, R>> for FBig<B, R> {
    #[inline]
    fn mul_assign(&mut self, rhs: &FBig<B, R>) {
        *self = core::mem::take(self) * rhs
    }
}

impl<const B: Word, R: Round> FBig<B, R> {
    #[inline]
    pub fn square(&self) -> Self {
        if self.repr.is_infinite() {
            panic_operate_with_inf();
        }

        let repr = Repr::new(
            self.repr.significand.square(),
            2 * self.repr.exponent,
        );
        FBig::new_raw(self.context.repr_round(repr).value(), self.context)
    }
}

impl<R: Round> Context<R> {
    pub fn mul<const B: Word>(
        &self,
        lhs: &FBig<B, R>,
        rhs: &FBig<B, R>,
    ) -> Rounded<FBig<B, R>> {
        check_inf_operands(&lhs.repr, &rhs.repr);

        // TODO: shrink lhs and rhs to at most double the precision before mul
        let repr = Repr::new(
            &lhs.repr.significand * &rhs.repr.significand,
            lhs.repr.exponent + rhs.repr.exponent,
        );
        self.repr_round(repr).map(|v| FBig::new_raw(v, *self))
    }
}

// TODO(next): implement more variants with macros, after implementing From<primitive> for FBig
impl<const B: Word, R: Round> Mul<FBig<B, R>> for i32 {
    type Output = FBig<B, R>;

    #[inline]
    fn mul(self, rhs: FBig<B, R>) -> Self::Output {
        FBig::from(dashu_int::IBig::from(self)) * rhs
    }
}

impl<const B: Word, R: Round> Mul<dashu_int::IBig> for &FBig<B, R> {
    type Output = FBig<B, R>;

    #[inline]
    fn mul(self, rhs: dashu_int::IBig) -> Self::Output {
        self * FBig::from(rhs)
    }
}

impl<const B: Word, R: Round> Mul<dashu_int::IBig> for FBig<B, R> {
    type Output = FBig<B, R>;

    #[inline]
    fn mul(self, rhs: dashu_int::IBig) -> Self::Output {
        self * FBig::from(rhs)
    }
}

impl<const B: Word, R: Round> Mul<u32> for FBig<B, R> {
    type Output = FBig<B, R>;

    #[inline]
    fn mul(self, rhs: u32) -> Self::Output {
        self * FBig::from(dashu_int::IBig::from(rhs))
    }
}
