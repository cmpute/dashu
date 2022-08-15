use dashu_base::Approximation;
use dashu_int::Word;

use crate::{
    repr::{Context, Repr},
    fbig::FBig,
    round::{Round, Rounding},
    utils::{digit_len, shr_rem_radix_in_place},
};
use core::ops::Mul;

impl<const B: Word, R: Round> Mul for &FBig<B, R> {
    type Output = FBig<B, R>;

    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        let context = Context::max(self.context, rhs.context);
        FBig {
            repr: context.mul(&self.repr, &rhs.repr).value(),
            context
        }
    }
}

impl<const B: Word, R: Round> Mul for FBig<B, R> {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        (&self).mul(&rhs)
    }
}
impl<const B: Word, R: Round> Mul<FBig<B, R>> for &FBig<B, R> {
    type Output = FBig<B, R>;
    #[inline]
    fn mul(self, rhs: FBig<B, R>) -> Self::Output {
        self.mul(&rhs)
    }
}

impl<R: Round> Context<R> {
    pub fn mul<const B: Word>(&self, lhs: &Repr<B>, rhs: &Repr<B>) -> Approximation<Repr<B>, Rounding> {
        let exponent = lhs.exponent + rhs.exponent;
        let mut significand = &lhs.significand * &rhs.significand;
        let actual_prec = digit_len::<B>(&significand);
        if actual_prec > self.precision {
            let shift = actual_prec - self.precision;
            let low_digits = shr_rem_radix_in_place::<B>(&mut significand, shift);
            let adjust = R::round_fract::<B>(&significand, low_digits, shift);
            Approximation::InExact(Repr::new(significand + adjust, exponent), adjust)
        } else {
            Approximation::Exact(Repr::new(significand, exponent))
        }
    }

    pub fn mul_assign<const B: Word>(&self, lhs: &mut Repr<B>, rhs: &Repr<B>) -> Approximation<(), Rounding> {
        unimplemented!()
    }
}
