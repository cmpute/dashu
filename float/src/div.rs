use crate::{
    repr::FloatRepr,
    round::Round,
    utils::{get_precision, shr_rem_radix_in_place},
};
use core::ops::Div;
use core::{cmp::Ordering, marker::PhantomData};
use dashu_base::DivRem;
use dashu_int::IBig;

impl<const X: usize, R: Round> FloatRepr<X, R> {
    /// Create a floating number expressed as `(numerator / denominator) * radix ^ exponent` with given precision.
    pub fn from_ratio_exponent(
        numerator: IBig,
        denominator: IBig,
        mut exponent: isize,
        precision: usize,
    ) -> Self {
        // FIXME: use the fast div support from ibig
        // FIXME: and also use the max number of exponent in a word to do shifting
        let (mut mantissa, mut rem) = numerator.div_rem(&denominator);
        let mut digits = get_precision::<X>(&mantissa);
        match digits.cmp(&precision) {
            Ordering::Equal => {
                mantissa += R::round_ratio(&mantissa, rem, &denominator);
            }
            Ordering::Greater => {
                let shift = digits - precision;
                let low_digits = shr_rem_radix_in_place::<X>(&mut mantissa, shift);
                mantissa += R::round_fract::<X>(&mantissa, low_digits, precision);
                exponent = shift as isize;
            }
            Ordering::Less => {
                while digits < precision && !rem.is_zero() {
                    let (d, r) = (rem * IBig::from(X)).div_rem(&denominator);
                    rem = r;
                    mantissa *= IBig::from(X);
                    mantissa += d;
                    digits += 1;
                    exponent -= 1;
                }
                mantissa += R::round_fract::<X>(&mantissa, rem, 1);
            }
        }

        let (mantissa, exponent) = Self::normalize(mantissa, exponent);
        FloatRepr {
            mantissa,
            exponent,
            precision,
            _marker: PhantomData,
        }
    }

    /// Create a floating number by dividing two integers with given precision
    #[inline]
    pub fn from_ratio(numerator: IBig, denominator: IBig, precision: usize) -> Self {
        Self::from_ratio_exponent(numerator, denominator, 0, precision)
    }
}

impl<const X: usize, R: Round> Div for FloatRepr<X, R> {
    type Output = Self;
    fn div(self, rhs: Self) -> Self::Output {
        Self::from_ratio_exponent(
            self.mantissa,
            rhs.mantissa,
            self.exponent - rhs.exponent,
            self.precision.max(rhs.precision),
        )
    }
}
