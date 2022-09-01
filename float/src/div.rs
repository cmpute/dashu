use crate::{
    error::check_inf_operands,
    fbig::FBig,
    repr::{Context, Repr, Word},
    round::{Round, Rounded},
    utils::{digit_len, shl_digits_in_place},
};
use core::ops::{Div, DivAssign};
use dashu_base::{Approximation, DivRem};
use dashu_int::IBig;

impl<const B: Word, R: Round> Div<FBig<B, R>> for FBig<B, R> {
    type Output = FBig<B, R>;
    fn div(self, rhs: FBig<B, R>) -> Self::Output {
        let context = Context::max(self.context, rhs.context);
        FBig::new_raw(context.repr_div(self.repr, &rhs.repr).value(), context)
    }
}

impl<'l, const B: Word, R: Round> Div<FBig<B, R>> for &'l FBig<B, R> {
    type Output = FBig<B, R>;
    fn div(self, rhs: FBig<B, R>) -> Self::Output {
        let context = Context::max(self.context, rhs.context);
        FBig::new_raw(context.repr_div(self.repr.clone(), &rhs.repr).value(), context)
    }
}

impl<'r, const B: Word, R: Round> Div<&'r FBig<B, R>> for FBig<B, R> {
    type Output = Self;
    fn div(self, rhs: &FBig<B, R>) -> Self::Output {
        let context = Context::max(self.context, rhs.context);
        FBig::new_raw(context.repr_div(self.repr, &rhs.repr).value(), context)
    }
}

impl<'l, 'r, const B: Word, R: Round> Div<&'r FBig<B, R>> for &'l FBig<B, R> {
    type Output = FBig<B, R>;
    fn div(self, rhs: &FBig<B, R>) -> Self::Output {
        let context = Context::max(self.context, rhs.context);
        FBig::new_raw(context.repr_div(self.repr.clone(), &rhs.repr).value(), context)
    }
}

impl<const B: Word, R: Round> DivAssign for FBig<B, R> {
    #[inline]
    fn div_assign(&mut self, rhs: Self) {
        *self = core::mem::take(self) / rhs
    }
}
impl<const B: Word, R: Round> DivAssign<&FBig<B, R>> for FBig<B, R> {
    #[inline]
    fn div_assign(&mut self, rhs: &FBig<B, R>) {
        *self = core::mem::take(self) / rhs
    }
}

impl<const B: Word, R: Round> FBig<B, R> {
    /// Create a floating number by dividing two integers with given precision
    #[inline]
    #[deprecated] // TODO: remove this, implement as From<RBig> in future
    pub fn from_ratio(numerator: IBig, denominator: IBig, precision: usize) -> Self {
        let context = Context::new(precision);
        let dummy = Context::new(0);
        let n = FBig::new_raw(Repr::new(numerator, 0), dummy);
        let d = FBig::new_raw(Repr::new(denominator, 0), dummy);
        context.div(&n, &d).value()
    }
}

impl<R: Round> Context<R> {
    pub(crate) fn repr_div<const B: Word>(&self, lhs: Repr<B>, rhs: &Repr<B>) -> Rounded<Repr<B>> {
        check_inf_operands(&lhs, &rhs);

        // this method don't deal with the case where lhs significand is too large
        debug_assert!(lhs.digits() <= self.precision + rhs.digits());

        let (mut q, mut r) = lhs.significand.div_rem(&rhs.significand);
        let mut e = lhs.exponent - rhs.exponent;
        if r.is_zero() {
            return Approximation::Exact(Repr::new(q, e));
        }

        let ddigits = digit_len::<B>(&rhs.significand);
        if q.is_zero() {
            // lhs.significand < rhs.significand
            let rdigits = digit_len::<B>(&r); // rdigits <= ddigits
            let shift = ddigits + self.precision - rdigits;
            shl_digits_in_place::<B>(&mut r, shift);
            e -= shift as isize;
            let (q0, r0) = r.div_rem(&rhs.significand);
            q = q0;
            r = r0;
        } else {
            let ndigits = digit_len::<B>(&q) + ddigits;
            if ndigits < ddigits + self.precision {
                // TODO: here the operations can be optimized: 1. prevent double power, 2. q += q0 can be |= if B is power of 2
                let shift = ddigits + self.precision - ndigits;
                shl_digits_in_place::<B>(&mut q, shift);
                shl_digits_in_place::<B>(&mut r, shift);
                e -= shift as isize;

                let (q0, r0) = r.div_rem(&rhs.significand);
                q += q0;
                r = r0;
            }
        }

        if r.is_zero() {
            Approximation::Exact(Repr::new(q, e))
        } else {
            let adjust = R::round_ratio(&q, r, &rhs.significand);
            Approximation::Inexact(Repr::new(q + adjust, e), adjust)
        }
    }

    pub fn div<const B: Word>(&self, lhs: &FBig<B, R>, rhs: &FBig<B, R>) -> Rounded<FBig<B, R>> {
        let lhs_repr = if !lhs.repr.is_zero()
            && lhs.repr.digits_ub() > rhs.repr.digits_lb() + self.precision
        {
            // shrink lhs if it's larger than necessary
            Self::new(rhs.repr.digits() + self.precision)
                .repr_round(lhs.repr.clone())
                .value()
        } else {
            lhs.repr.clone()
        };
        self.repr_div(lhs_repr, &rhs.repr)
            .map(|v| FBig::new_raw(v, *self))
    }
}

// TODO: implement more variants with macros, after implementing From<primitive> for FBig
impl<const B: Word, R: Round> Div<dashu_int::IBig> for FBig<B, R> {
    type Output = FBig<B, R>;

    #[inline]
    fn div(self, rhs: dashu_int::IBig) -> Self::Output {
        self / FBig::from(rhs)
    }
}

// TODO: implement div_euclid, rem_euclid, div_rem_euclid for float, as it can be properly defined
