use crate::{
    ibig_ext::log_rem,
    repr::FloatRepr,
    utils::{get_precision, shr_rem_radix},
};
use core::ops::Div;
use dashu_base::Abs;
use dashu_int::{ibig, IBig};
use std::convert::TryInto;

impl<const X: usize, const R: u8> Div for FloatRepr<X, R> {
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
