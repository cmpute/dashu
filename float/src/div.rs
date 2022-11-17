use crate::{
    error::{check_inf_operands, check_precision_limited},
    fbig::FBig,
    helper_macros,
    repr::{Context, Repr, Word},
    round::{Round, Rounded},
    utils::{digit_len, shl_digits_in_place},
};
use core::ops::{Div, DivAssign};
use dashu_base::{Approximation, DivEuclid, DivRem, DivRemEuclid, Inverse, RemEuclid};
use dashu_int::{IBig, UBig};

impl<R: Round, const B: Word> Div<FBig<R, B>> for FBig<R, B> {
    type Output = FBig<R, B>;
    fn div(self, rhs: FBig<R, B>) -> Self::Output {
        let context = Context::max(self.context, rhs.context);
        FBig::new(context.repr_div(self.repr, &rhs.repr).value(), context)
    }
}

impl<'l, R: Round, const B: Word> Div<FBig<R, B>> for &'l FBig<R, B> {
    type Output = FBig<R, B>;
    fn div(self, rhs: FBig<R, B>) -> Self::Output {
        let context = Context::max(self.context, rhs.context);
        FBig::new(context.repr_div(self.repr.clone(), &rhs.repr).value(), context)
    }
}

impl<'r, R: Round, const B: Word> Div<&'r FBig<R, B>> for FBig<R, B> {
    type Output = FBig<R, B>;
    fn div(self, rhs: &FBig<R, B>) -> Self::Output {
        let context = Context::max(self.context, rhs.context);
        FBig::new(context.repr_div(self.repr, &rhs.repr).value(), context)
    }
}

impl<'l, 'r, R: Round, const B: Word> Div<&'r FBig<R, B>> for &'l FBig<R, B> {
    type Output = FBig<R, B>;
    fn div(self, rhs: &FBig<R, B>) -> Self::Output {
        let context = Context::max(self.context, rhs.context);
        FBig::new(context.repr_div(self.repr.clone(), &rhs.repr).value(), context)
    }
}

impl<R: Round, const B: Word> DivAssign for FBig<R, B> {
    #[inline]
    fn div_assign(&mut self, rhs: Self) {
        *self = core::mem::take(self) / rhs
    }
}
impl<R: Round, const B: Word> DivAssign<&FBig<R, B>> for FBig<R, B> {
    #[inline]
    fn div_assign(&mut self, rhs: &FBig<R, B>) {
        *self = core::mem::take(self) / rhs
    }
}

impl<R: Round, const B: Word> DivEuclid<FBig<R, B>> for FBig<R, B> {
    type Output = IBig;
    #[inline]
    fn div_euclid(self, rhs: FBig<R, B>) -> Self::Output {
        let (num, den) = align_as_int(self, rhs);
        num.div_euclid(den)
    }
}

impl<'l, R: Round, const B: Word> DivEuclid<FBig<R, B>> for &'l FBig<R, B> {
    type Output = IBig;
    #[inline]
    fn div_euclid(self, rhs: FBig<R, B>) -> Self::Output {
        self.clone().div_euclid(rhs)
    }
}

impl<'r, R: Round, const B: Word> DivEuclid<&'r FBig<R, B>> for FBig<R, B> {
    type Output = IBig;
    #[inline]
    fn div_euclid(self, rhs: &FBig<R, B>) -> Self::Output {
        self.div_euclid(rhs.clone())
    }
}

impl<'l, 'r, R: Round, const B: Word> DivEuclid<&'r FBig<R, B>> for &'l FBig<R, B> {
    type Output = IBig;
    #[inline]
    fn div_euclid(self, rhs: &FBig<R, B>) -> Self::Output {
        self.clone().div_euclid(rhs.clone())
    }
}

impl<R: Round, const B: Word> RemEuclid<FBig<R, B>> for FBig<R, B> {
    type Output = FBig<R, B>;
    #[inline]
    fn rem_euclid(self, rhs: FBig<R, B>) -> Self::Output {
        let r_exponent = self.repr.exponent.min(rhs.repr.exponent);
        let context = Context::max(self.context, rhs.context);

        let (num, den) = align_as_int(self, rhs);
        let r = num.rem_euclid(den);
        let mut r = context.convert_int(r.into()).value();
        if !r.repr.significand.is_zero() {
            r.repr.exponent += r_exponent;
        }
        r
    }
}

impl<'l, R: Round, const B: Word> RemEuclid<FBig<R, B>> for &'l FBig<R, B> {
    type Output = FBig<R, B>;
    #[inline]
    fn rem_euclid(self, rhs: FBig<R, B>) -> Self::Output {
        self.clone().rem_euclid(rhs)
    }
}

impl<'r, R: Round, const B: Word> RemEuclid<&'r FBig<R, B>> for FBig<R, B> {
    type Output = FBig<R, B>;
    #[inline]
    fn rem_euclid(self, rhs: &FBig<R, B>) -> Self::Output {
        self.rem_euclid(rhs.clone())
    }
}

impl<'l, 'r, R: Round, const B: Word> RemEuclid<&'r FBig<R, B>> for &'l FBig<R, B> {
    type Output = FBig<R, B>;
    #[inline]
    fn rem_euclid(self, rhs: &FBig<R, B>) -> Self::Output {
        self.clone().rem_euclid(rhs.clone())
    }
}

impl<R: Round, const B: Word> DivRemEuclid<FBig<R, B>> for FBig<R, B> {
    type OutputDiv = IBig;
    type OutputRem = FBig<R, B>;
    #[inline]
    fn div_rem_euclid(self, rhs: FBig<R, B>) -> (IBig, FBig<R, B>) {
        let r_exponent = self.repr.exponent.min(rhs.repr.exponent);
        let context = Context::max(self.context, rhs.context);

        let (num, den) = align_as_int(self, rhs);
        let (q, r) = num.div_rem_euclid(den);
        let mut r = context.convert_int(r.into()).value();
        if !r.repr.significand.is_zero() {
            r.repr.exponent += r_exponent;
        }
        (q, r)
    }
}

impl<'l, R: Round, const B: Word> DivRemEuclid<FBig<R, B>> for &'l FBig<R, B> {
    type OutputDiv = IBig;
    type OutputRem = FBig<R, B>;
    #[inline]
    fn div_rem_euclid(self, rhs: FBig<R, B>) -> (IBig, FBig<R, B>) {
        self.clone().div_rem_euclid(rhs)
    }
}

impl<'r, R: Round, const B: Word> DivRemEuclid<&'r FBig<R, B>> for FBig<R, B> {
    type OutputDiv = IBig;
    type OutputRem = FBig<R, B>;
    #[inline]
    fn div_rem_euclid(self, rhs: &FBig<R, B>) -> (IBig, FBig<R, B>) {
        self.div_rem_euclid(rhs.clone())
    }
}

impl<'l, 'r, R: Round, const B: Word> DivRemEuclid<&'r FBig<R, B>> for &'l FBig<R, B> {
    type OutputDiv = IBig;
    type OutputRem = FBig<R, B>;
    #[inline]
    fn div_rem_euclid(self, rhs: &FBig<R, B>) -> (IBig, FBig<R, B>) {
        self.clone().div_rem_euclid(rhs.clone())
    }
}

macro_rules! impl_add_sub_primitive_with_fbig {
    ($($t:ty)*) => {$(
        helper_macros::impl_commutative_binop_with_primitive!(impl Div<$t>, div);
        helper_macros::impl_binop_assign_with_primitive!(impl DivAssign<$t>, div_assign);
    )*};
}
impl_add_sub_primitive_with_fbig!(u8 u16 u32 u64 u128 usize UBig i8 i16 i32 i64 i128 isize IBig);

impl<R: Round, const B: Word> Inverse for FBig<R, B> {
    type Output = FBig<R, B>;

    #[inline]
    fn inv(self) -> Self::Output {
        self.context.inv(&self.repr).value()
    }
}

impl<R: Round, const B: Word> Inverse for &FBig<R, B> {
    type Output = FBig<R, B>;

    #[inline]
    fn inv(self) -> Self::Output {
        self.context.inv(&self.repr).value()
    }
}

// Align two float by exponent such that they are both turned into integers
fn align_as_int<R: Round, const B: Word>(lhs: FBig<R, B>, rhs: FBig<R, B>) -> (IBig, IBig) {
    let ediff = lhs.repr.exponent - rhs.repr.exponent;
    let (mut num, mut den) = (lhs.repr.significand, rhs.repr.significand);
    if ediff >= 0 {
        shl_digits_in_place::<B>(&mut num, ediff as _);
    } else {
        shl_digits_in_place::<B>(&mut den, (-ediff) as _);
    }
    (num, den)
}

impl<R: Round> Context<R> {
    pub(crate) fn repr_div<const B: Word>(&self, lhs: Repr<B>, rhs: &Repr<B>) -> Rounded<Repr<B>> {
        check_inf_operands(&lhs, rhs);
        check_precision_limited(self.precision);

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

    /// Divide two floating point numbers under this context.
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
    /// assert_eq!(context.div(&a.repr(), &b.repr()), Inexact(DBig::from_str_native("-0.18")?, NoOp));
    /// # Ok::<(), ParseError>(())
    /// ```
    ///
    /// # Euclidean Division
    ///
    /// To do euclidean division on the float numbers (get an integer quotient and remainder, equivalent to C99's
    /// `fmod` and `remquo`), please use the methods provided by traits [DivEuclid], [RemEuclid] and [DivRemEuclid].
    ///
    pub fn div<const B: Word>(&self, lhs: &Repr<B>, rhs: &Repr<B>) -> Rounded<FBig<R, B>> {
        check_inf_operands(lhs, rhs);

        let lhs_repr = if !lhs.is_zero() && lhs.digits_ub() > rhs.digits_lb() + self.precision {
            // shrink lhs if it's larger than necessary
            Self::new(rhs.digits() + self.precision)
                .repr_round_ref(lhs)
                .value()
        } else {
            lhs.clone()
        };
        self.repr_div(lhs_repr, rhs).map(|v| FBig::new(v, *self))
    }

    /// Compute the multiplicative inverse of an `FBig`
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// use dashu_base::Approximation::*;
    /// use dashu_float::{Context, round::{mode::HalfAway, Rounding::*}};
    ///
    /// let context = Context::<HalfAway>::new(2);
    /// let a = DBig::from_str_native("-1.234")?;
    /// assert_eq!(context.inv(&a.repr()), Inexact(DBig::from_str_native("-0.81")?, NoOp));
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn inv<const B: Word>(&self, f: &Repr<B>) -> Rounded<FBig<R, B>> {
        self.repr_div(Repr::one(), f).map(|v| FBig::new(v, *self))
    }
}
