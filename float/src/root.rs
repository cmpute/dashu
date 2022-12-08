use dashu_base::{Approximation, Sign, SquareRootRem, UnsignedAbs};
use dashu_int::IBig;

use crate::{
    error::{assert_finite, assert_limited_precision, panic_root_negative},
    fbig::FBig,
    repr::{Context, Repr, Word},
    round::{Round, Rounded},
    utils::{shl_digits, split_digits_ref},
};

impl<R: Round, const B: Word> FBig<R, B> {
    /// Calculate the square root of the floating point number.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str_native("1.23")?;
    /// assert_eq!(a.sqrt(), DBig::from_str_native("1.11")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn sqrt(&self) -> Self {
        self.context.sqrt(self.repr()).value()
    }
}

impl<R: Round> Context<R> {
    /// Calculate the square root of the floating point number.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// use dashu_base::Approximation::*;
    /// use dashu_float::{Context, round::{mode::HalfAway, Rounding::*}};
    ///
    /// let context = Context::<HalfAway>::new(2);
    /// let a = DBig::from_str_native("1.23")?;
    /// assert_eq!(context.sqrt(&a.repr()), Inexact(DBig::from_str_native("1.1")?, NoOp));
    /// # Ok::<(), ParseError>(())
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the precision is unlimited.
    pub fn sqrt<const B: Word>(&self, x: &Repr<B>) -> Rounded<FBig<R, B>> {
        assert_finite(x);
        assert_limited_precision(self.precision);
        if x.sign() == Sign::Negative {
            panic_root_negative()
        }

        // adjust the signifcand so that the exponent is even
        let digits = x.digits() as isize;
        let shift = self.precision as isize * 2 - (digits & 1) + (x.exponent & 1) - digits;
        let (signif, low, low_digits) = if shift > 0 {
            (shl_digits::<B>(&x.significand, shift as usize), IBig::ZERO, 0)
        } else {
            let shift = (-shift) as usize;
            let (hi, lo) = split_digits_ref::<B>(&x.significand, shift);
            (hi, lo, shift)
        };

        let (root, rem) = signif.unsigned_abs().sqrt_rem();
        let root = Sign::Positive * root;
        let exp = (x.exponent - shift) / 2;

        let res = if rem.is_zero() {
            Approximation::Exact(root)
        } else {
            let adjust = R::round_low_part(&root, Sign::Positive, || {
                (Sign::Positive * rem)
                    .cmp(&root)
                    .then_with(|| (low * 4u8).cmp(&Repr::<B>::BASE.pow(low_digits)))
            });
            Approximation::Inexact(root + adjust, adjust)
        };
        res.map(|signif| Repr::new(signif, exp))
            .and_then(|v| self.repr_round(v))
            .map(|v| FBig::new(v, *self))
    }
}
