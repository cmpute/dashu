use core::ops::Div;
use std::convert::TryInto;
use dashu_base::Abs;
use dashu_int::{IBig, ibig};
use crate::{repr::FloatRepr, utils::{shr_rem_radix, get_precision}, ibig_ext::log_rem};

impl<const X: usize, const R: u8> Div for FloatRepr<X, R> {
    type Output = Self;
    fn div(self, rhs: Self) -> Self::Output {
        Self::from_ratio_exponent(
            self.mantissa, rhs.mantissa,
            self.exponent - rhs.exponent,
            self.precision.max(rhs.precision)
        )
    }
}
