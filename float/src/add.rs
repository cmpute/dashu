use core::ops::{Add, Sub};
use crate::utils::{shl_radix, round_with_fract, shr_rem_radix_in_place};

use crate::repr::FloatRepr;

impl<const X: usize, const R: u8> Add for FloatRepr<X, R> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
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
            let adjust = round_with_fract::<X, R>(&lhs.mantissa, rhs.mantissa, ediff);
            lhs.mantissa += adjust;
            return lhs;
        }

        // align the exponent
        let lhs_prec = lhs.actual_precision();
        let (exponent, rem, rem_prec) = if ediff + lhs_prec > precision {
            // if the shifted lhs exceeds the desired precision, normalize lhs and shift rhs
            let shift = precision - lhs_prec;
            let low_digits = shr_rem_radix_in_place::<X>(&mut rhs.mantissa, shift);
            shl_radix::<X>(&mut lhs.mantissa, ediff - shift);
            (lhs.exponent - (ediff - shift) as isize, low_digits, shift)
        } else {
            let low_digits = shr_rem_radix_in_place::<X>(&mut rhs.mantissa, ediff);
            (lhs.exponent, low_digits, ediff)
        };

        // actuall adding
        let mantissa = lhs.mantissa + rhs.mantissa;
        let adjust = round_with_fract::<X, R>(&mantissa, rem, rem_prec);
        Self::from_parts(mantissa + adjust, exponent)
    }
}

impl<const X: usize, const R: u8> Sub for FloatRepr<X, R> {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        return self.add(-rhs);
    }
}

// TODO: carefully determine whether the opperations take reference or value
impl<const X: usize, const R: u8> Add for &FloatRepr<X, R> {
    type Output = FloatRepr<X, R>;
    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        self.clone().add(rhs.clone())
    }
}
impl<const X: usize, const R: u8> Sub for &FloatRepr<X, R> {
    type Output = FloatRepr<X, R>;
    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        self.add(&(-rhs))
    }
}
impl<const X: usize, const R: u8> Sub<FloatRepr<X, R>> for &FloatRepr<X, R> {
    type Output = FloatRepr<X, R>;
    #[inline]
    fn sub(self, rhs: FloatRepr<X, R>) -> Self::Output {
        self.add(&(-rhs))
    }
}
