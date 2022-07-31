use crate::{
    round::{Rounding, Round},
    repr::FloatRepr,
    utils::{shl_radix, shr_rem_radix_in_place},
};
use core::ops::{Add, Sub};

use dashu_base::Approximation;

impl<const X: usize, R: Round> Add for FloatRepr<X, R> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        self.add(rhs).value()
    }
}

impl<const X: usize, R: Round> Sub for FloatRepr<X, R> {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        return self.add(-rhs).value();
    }
}

impl<const X: usize, R: Round> Add for &FloatRepr<X, R> {
    type Output = FloatRepr<X, R>;
    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        self.clone().add(rhs.clone()).value()
    }
}
impl<const X: usize, R: Round> Sub for &FloatRepr<X, R> {
    type Output = FloatRepr<X, R>;
    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        self.add(&(-rhs))
    }
}
impl<const X: usize, R: Round> Sub<FloatRepr<X, R>> for &FloatRepr<X, R> {
    type Output = FloatRepr<X, R>;
    #[inline]
    fn sub(self, rhs: FloatRepr<X, R>) -> Self::Output {
        self.add(&(-rhs))
    }
}

// TODO: rename the add function returning approximation to something else

impl<const X: usize, R: Round> FloatRepr<X, R> {
    fn add(self, rhs: Self) -> Approximation<Self, Rounding> {
        // put the oprand of lower exponent on the right
        let (mut lhs, mut rhs) = if self.exponent >= rhs.exponent {
            (self, rhs)
        } else {
            (rhs, self)
        };

        // shortcut if lhs is too small
        let ediff = (lhs.exponent - rhs.exponent) as usize;
        let precision = lhs.precision.max(rhs.precision);
        if ediff > precision {
            let adjust = R::round_fract::<X>(&lhs.mantissa, rhs.mantissa, ediff);
            lhs.mantissa += adjust;
            return Approximation::InExact(lhs, adjust);
        }

        // align the exponent
        let lhs_prec = lhs.actual_precision();
        if ediff + lhs_prec > precision {
            // if the shifted lhs exceeds the desired precision, normalize lhs and shift rhs
            let shift = precision - lhs_prec;
            let low_digits = shr_rem_radix_in_place::<X>(&mut rhs.mantissa, shift);
            shl_radix::<X>(&mut lhs.mantissa, ediff - shift);

            // do addition
            let mantissa = lhs.mantissa + rhs.mantissa;
            let exponent = lhs.exponent - (ediff - shift) as isize;
            let adjust = R::round_fract::<X>(&mantissa, low_digits, shift);
            Approximation::InExact(Self::from_parts(mantissa + adjust, exponent), adjust)
        } else {
            // otherwise directly shift lhs to required position
            shl_radix::<X>(&mut lhs.mantissa, ediff);
            Approximation::Exact(Self::from_parts(lhs.mantissa + rhs.mantissa, rhs.exponent))
        }
    }
}
