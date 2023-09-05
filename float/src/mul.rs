use dashu_int::{IBig, UBig};

use crate::{
    error::{assert_finite, assert_finite_operands},
    fbig::FBig,
    helper_macros,
    repr::{Context, Repr, Word},
    round::{Round, Rounded},
};
use core::ops::{Mul, MulAssign};

impl<'l, 'r, R: Round, const B: Word> Mul<&'r FBig<R, B>> for &'l FBig<R, B> {
    type Output = FBig<R, B>;

    #[inline]
    fn mul(self, rhs: &FBig<R, B>) -> Self::Output {
        assert_finite_operands(&self.repr, &rhs.repr);

        let context = Context::max(self.context, rhs.context);
        let repr = Repr::new(
            &self.repr.significand * &rhs.repr.significand,
            self.repr.exponent + rhs.repr.exponent,
        );
        FBig::new(context.repr_round(repr).value(), context)
    }
}

impl<'r, R: Round, const B: Word> Mul<&'r FBig<R, B>> for FBig<R, B> {
    type Output = FBig<R, B>;

    #[inline]
    fn mul(self, rhs: &FBig<R, B>) -> Self::Output {
        assert_finite_operands(&self.repr, &rhs.repr);

        let context = Context::max(self.context, rhs.context);
        let repr = Repr::new(
            self.repr.significand * &rhs.repr.significand,
            self.repr.exponent + rhs.repr.exponent,
        );
        FBig::new(context.repr_round(repr).value(), context)
    }
}

impl<'l, R: Round, const B: Word> Mul<FBig<R, B>> for &'l FBig<R, B> {
    type Output = FBig<R, B>;

    #[inline]
    fn mul(self, rhs: FBig<R, B>) -> Self::Output {
        assert_finite_operands(&self.repr, &rhs.repr);

        let context = Context::max(self.context, rhs.context);
        let repr = Repr::new(
            &self.repr.significand * rhs.repr.significand,
            self.repr.exponent + rhs.repr.exponent,
        );
        FBig::new(context.repr_round(repr).value(), context)
    }
}

impl<R: Round, const B: Word> Mul<FBig<R, B>> for FBig<R, B> {
    type Output = FBig<R, B>;

    #[inline]
    fn mul(self, rhs: FBig<R, B>) -> Self::Output {
        assert_finite_operands(&self.repr, &rhs.repr);

        let context = Context::max(self.context, rhs.context);
        let repr = Repr::new(
            self.repr.significand * rhs.repr.significand,
            self.repr.exponent + rhs.repr.exponent,
        );
        FBig::new(context.repr_round(repr).value(), context)
    }
}

helper_macros::impl_binop_assign_by_taking!(impl MulAssign<Self>, mul_assign, mul);

macro_rules! impl_mul_primitive_with_fbig {
    ($($t:ty)*) => {$(
        helper_macros::impl_binop_with_primitive!(impl Mul<$t>, mul);
        helper_macros::impl_binop_assign_with_primitive!(impl MulAssign<$t>, mul_assign);
    )*};
}
impl_mul_primitive_with_fbig!(u8 u16 u32 u64 u128 usize UBig i8 i16 i32 i64 i128 isize IBig);

impl<R: Round, const B: Word> FBig<R, B> {
    /// Compute the square of this number (`self * self`)
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str_native("-1.234")?;
    /// assert_eq!(a.sqr(), DBig::from_str_native("1.523")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn sqr(&self) -> Self {
        self.context.sqr(&self.repr).value()
    }

    /// Compute the square of this number (`self * self`)
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str_native("-1.234")?;
    /// assert_eq!(a.cubic(), DBig::from_str_native("-1.879")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn cubic(&self) -> Self {
        self.context.cubic(&self.repr).value()
    }
}

impl<R: Round> Context<R> {
    /// Multiply two floating point numbers under this context.
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
    /// let a = DBig::from_str_native("-1.234")?;
    /// let b = DBig::from_str_native("6.789")?;
    /// assert_eq!(
    ///     context.mul(&a.repr(), &b.repr()),
    ///     Inexact(DBig::from_str_native("-8.4")?, SubOne)
    /// );
    /// # Ok::<(), ParseError>(())
    /// ```
    pub fn mul<const B: Word>(&self, lhs: &Repr<B>, rhs: &Repr<B>) -> Rounded<FBig<R, B>> {
        assert_finite_operands(lhs, rhs);

        // at most double the precision is required to get a correct result
        // shrink the input operands if necessary
        let max_precision = if self.is_limited() {
            self.precision * 2
        } else {
            usize::MAX
        };

        let lhs_shrink;
        let lhs_repr = if lhs.digits() > max_precision {
            lhs_shrink = Context::<R>::new(max_precision).repr_round_ref(lhs).value();
            &lhs_shrink
        } else {
            lhs
        };

        let rhs_shrink;
        let rhs_repr = if rhs.digits() > max_precision {
            rhs_shrink = Context::<R>::new(max_precision).repr_round_ref(rhs).value();
            &rhs_shrink
        } else {
            rhs
        };

        let repr = Repr::new(
            &lhs_repr.significand * &rhs_repr.significand,
            lhs_repr.exponent + rhs_repr.exponent,
        );
        self.repr_round(repr).map(|v| FBig::new(v, *self))
    }

    /// Calculate the square of the floating point number under this context.
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
    /// let a = DBig::from_str_native("-1.234")?;
    /// assert_eq!(context.sqr(&a.repr()), Inexact(DBig::from_str_native("1.5")?, NoOp));
    /// # Ok::<(), ParseError>(())
    /// ```
    pub fn sqr<const B: Word>(&self, f: &Repr<B>) -> Rounded<FBig<R, B>> {
        assert_finite(f);

        // shrink the input operands if necessary
        let max_precision = if self.is_limited() {
            self.precision * 2
        } else {
            usize::MAX
        };

        let f_shrink;
        let f_repr = if f.digits() > max_precision {
            f_shrink = Context::<R>::new(max_precision).repr_round_ref(f).value();
            &f_shrink
        } else {
            f
        };

        let repr = Repr::new(f_repr.significand.sqr().into(), 2 * f_repr.exponent);
        self.repr_round(repr).map(|v| FBig::new(v, *self))
    }

    /// Calculate the square of the floating point number under this context.
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
    /// let a = DBig::from_str_native("-1.234")?;
    /// assert_eq!(context.cubic(&a.repr()), Inexact(DBig::from_str_native("-1.9")?, SubOne));
    /// # Ok::<(), ParseError>(())
    /// ```
    pub fn cubic<const B: Word>(&self, f: &Repr<B>) -> Rounded<FBig<R, B>> {
        assert_finite(f);

        // shrink the input operands if necessary
        let max_precision = if self.is_limited() {
            self.precision * 3
        } else {
            usize::MAX
        };

        let f_shrink;
        let f_repr = if f.digits() > max_precision {
            f_shrink = Context::<R>::new(max_precision).repr_round_ref(f).value();
            &f_shrink
        } else {
            f
        };

        // TODO(next): increase dependency version on dashu_int because we use the new function cubic()
        let repr = Repr::new(f_repr.significand.cubic(), 3 * f_repr.exponent);
        self.repr_round(repr).map(|v| FBig::new(v, *self))
    }
}
