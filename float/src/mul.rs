use dashu_int::{IBig, UBig};

use crate::{
    error::{check_inf, check_inf_operands},
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
        check_inf_operands(&self.repr, &rhs.repr);

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
        check_inf_operands(&self.repr, &rhs.repr);

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
        check_inf_operands(&self.repr, &rhs.repr);

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
        check_inf_operands(&self.repr, &rhs.repr);

        let context = Context::max(self.context, rhs.context);
        let repr = Repr::new(
            self.repr.significand * rhs.repr.significand,
            self.repr.exponent + rhs.repr.exponent,
        );
        FBig::new(context.repr_round(repr).value(), context)
    }
}

helper_macros::impl_binop_assign_by_taking!(impl MulAssign<Self>, mul_assign, mul);

macro_rules! impl_add_sub_primitive_with_fbig {
    ($($t:ty)*) => {$(
        helper_macros::impl_commutative_binop_with_primitive!(impl Mul<$t>, mul);
        helper_macros::impl_binop_assign_with_primitive!(impl MulAssign<$t>, mul_assign);
    )*};
}
impl_add_sub_primitive_with_fbig!(u8 u16 u32 u64 u128 usize UBig i8 i16 i32 i64 i128 isize IBig);

impl<R: Round, const B: Word> FBig<R, B> {
    #[inline]
    pub fn square(&self) -> Self {
        self.context.square(&self.repr).value()
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
        check_inf_operands(lhs, rhs);

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
    /// assert_eq!(context.square(&a.repr()), Inexact(DBig::from_str_native("1.5")?, NoOp));
    /// # Ok::<(), ParseError>(())
    /// ```
    pub fn square<const B: Word>(&self, f: &Repr<B>) -> Rounded<FBig<R, B>> {
        check_inf(f);

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

        let repr = Repr::new(f_repr.significand.square(), 2 * f_repr.exponent);
        self.repr_round(repr).map(|v| FBig::new(v, *self))
    }
}
