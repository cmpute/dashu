use crate::{
    Word,
    round::{Rounding, Round},
    repr::{Context, Repr},
    fbig::FBig,
    utils::{shl_radix, shr_rem_radix_in_place},
};
use core::ops::{Add, Sub};

use dashu_base::Approximation;

impl<const B: Word, R: Round> Add for FBig<B, R> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        let context = Context::max(self.context, rhs.context);
        FBig {
            repr: context.add(self.repr, rhs.repr).value(),
            context
        }
    }
}

impl<const B: Word, R: Round> Sub for FBig<B, R> {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        let context = Context::max(self.context, rhs.context);
        FBig {
            repr: context.add(self.repr, -rhs.repr).value(),
            context
        }
    }
}

impl<const B: Word, R: Round> Add for &FBig<B, R> {
    type Output = FBig<B, R>;
    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        unimplemented!()
    }
}
impl<const B: Word, R: Round> Sub for &FBig<B, R> {
    type Output = FBig<B, R>;
    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        self.add(&(-rhs))
    }
}
impl<const B: Word, R: Round> Sub<FBig<B, R>> for &FBig<B, R> {
    type Output = FBig<B, R>;
    #[inline]
    fn sub(self, rhs: FBig<B, R>) -> Self::Output {
        self.add(&(-rhs))
    }
}

impl<R: Round> Context<R> {
    // TODO: let add take reference
    pub fn add<const B: Word>(&self, lhs: Repr<B>, rhs: Repr<B>) -> Approximation<Repr<B>, Rounding> {
        // put the oprand of lower exponent on the right
        let (mut lhs, mut rhs) = if lhs.exponent >= rhs.exponent {
            (lhs, rhs)
        } else {
            (rhs, lhs)
        };

        // shortcut if lhs is too small
        let ediff = (lhs.exponent - rhs.exponent) as usize;
        if ediff > self.precision {
            let adjust = R::round_fract::<B>(&lhs.significand, rhs.significand, ediff);
            lhs.significand += adjust;
            return Approximation::InExact(lhs, adjust);
        }

        // align the exponent
        let lhs_prec = lhs.digits();
        if ediff + lhs_prec > self.precision {
            // if the shifted lhs exceeds the desired precision, normalize lhs and shift rhs
            let shift = self.precision - lhs_prec;
            let low_digits = shr_rem_radix_in_place::<B>(&mut rhs.significand, shift);
            shl_radix::<B>(&mut lhs.significand, ediff - shift);

            // do addition
            let significand = lhs.significand + rhs.significand;
            let exponent = lhs.exponent - (ediff - shift) as isize;
            let adjust = R::round_fract::<B>(&significand, low_digits, shift);
            Approximation::InExact(Repr::new(significand + adjust, exponent), adjust)
        } else {
            // otherwise directly shift lhs to required position
            shl_radix::<B>(&mut lhs.significand, ediff);
            Approximation::Exact(Repr::new(lhs.significand + rhs.significand, rhs.exponent))
        }
    }

    pub fn add_assign<const B: Word>(&self, lhs: &mut Repr<B>, rhs: &Repr<B>) -> Approximation<(), Rounding> {
        unimplemented!()
    }
}