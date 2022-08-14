use crate::{
    repr::{Repr, Context},
    fbig::FBig,
    round::Round,
    utils::{get_precision, shr_rem_radix_in_place},
};
use core::{ops::Div, cmp::Ordering};
use dashu_base::DivRem;
use dashu_int::{IBig, Sign, DoubleWord, Word};

impl<const B: Word, R: Round> FBig<B, R> {
    /// Create a floating number expressed as `(numerator / denominator) * radix ^ exponent` with given precision.
    pub fn from_ratio_exponent(
        numerator: IBig,
        denominator: IBig,
        mut exponent: isize,
        precision: usize,
    ) -> Self {
        // FIXME: change to first align the operands to 2n/n (n is working precision), then do the integer division
        let (mut significand, mut rem) = numerator.div_rem(&denominator);
        let mut digits = get_precision::<B>(&significand);
        match digits.cmp(&precision) {
            Ordering::Equal => {
                significand += R::round_ratio(&significand, rem, &denominator);
            }
            Ordering::Greater => {
                let shift = digits - precision;
                let low_digits = shr_rem_radix_in_place::<B>(&mut significand, shift);
                significand += R::round_fract::<B>(&significand, low_digits, precision);
                exponent = shift as isize;
            }
            Ordering::Less => {
                // TODO: create an associated const
                while digits < precision && !rem.is_zero() {
                    let (d, r) = (rem * Repr::<B>::B_IBIG).div_rem(&denominator);
                    rem = r;
                    significand *= Repr::<B>::B_IBIG;
                    significand += d;
                    digits += 1;
                    exponent -= 1;
                }
                significand += R::round_fract::<B>(&significand, rem, 1);
            }
        }

        FBig {
            repr: Repr::new(significand, exponent),
            context: Context::new(precision)
        }
    }

    /// Create a floating number by dividing two integers with given precision
    #[inline]
    pub fn from_ratio(numerator: IBig, denominator: IBig, precision: usize) -> Self {
        Self::from_ratio_exponent(numerator, denominator, 0, precision)
    }
}

impl<const B: Word, R: Round> Div for FBig<B, R> {
    type Output = Self;
    fn div(self, rhs: Self) -> Self::Output {
        Self::from_ratio_exponent(
            self.repr.significand,
            rhs.repr.significand,
            self.repr.exponent - rhs.repr.exponent,
            self.context.precision.max(rhs.context.precision),
        )
    }
}
