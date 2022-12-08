use crate::{
    error::{assert_finite_operands, assert_limited_precision},
    fbig::FBig,
    helper_macros::{self, impl_binop_assign_by_taking},
    repr::{Context, Repr, Word},
    round::{Round, Rounded},
    utils::{digit_len, shl_digits_in_place, split_digits},
};
use core::ops::{Div, DivAssign, Rem, RemAssign};
use dashu_base::{Approximation, DivEuclid, DivRem, DivRemEuclid, Inverse, RemEuclid, Sign};
use dashu_int::{IBig, UBig, modular::ModuloRing};

macro_rules! impl_div_rem_for_fbig {
    (impl $op:ident, $method:ident, $repr_method:ident) => {
        impl<R: Round, const B: Word> $op<FBig<R, B>> for FBig<R, B> {
            type Output = FBig<R, B>;
            fn $method(self, rhs: FBig<R, B>) -> Self::Output {
                let context = Context::max(self.context, rhs.context);
                FBig::new(context.$repr_method(self.repr, rhs.repr).value(), context)
            }
        }

        impl<'l, R: Round, const B: Word> $op<FBig<R, B>> for &'l FBig<R, B> {
            type Output = FBig<R, B>;
            fn $method(self, rhs: FBig<R, B>) -> Self::Output {
                let context = Context::max(self.context, rhs.context);
                FBig::new(context.$repr_method(self.repr.clone(), rhs.repr).value(), context)
            }
        }

        impl<'r, R: Round, const B: Word> $op<&'r FBig<R, B>> for FBig<R, B> {
            type Output = FBig<R, B>;
            fn $method(self, rhs: &FBig<R, B>) -> Self::Output {
                let context = Context::max(self.context, rhs.context);
                FBig::new(context.$repr_method(self.repr, rhs.repr.clone()).value(), context)
            }
        }

        impl<'l, 'r, R: Round, const B: Word> $op<&'r FBig<R, B>> for &'l FBig<R, B> {
            type Output = FBig<R, B>;
            fn $method(self, rhs: &FBig<R, B>) -> Self::Output {
                let context = Context::max(self.context, rhs.context);
                FBig::new(context.$repr_method(self.repr.clone(), rhs.repr.clone()).value(), context)
            }
        }
    };
}
impl_div_rem_for_fbig!(impl Div, div, repr_div);
impl_div_rem_for_fbig!(impl Rem, rem, repr_rem);

impl_binop_assign_by_taking!(impl DivAssign<Self>, div_assign, div);
impl_binop_assign_by_taking!(impl RemAssign<Self>, rem_assign, rem);

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

macro_rules! impl_div_primitive_with_fbig {
    ($($t:ty)*) => {$(
        helper_macros::impl_binop_with_primitive!(impl Div<$t>, div);
        helper_macros::impl_binop_assign_with_primitive!(impl DivAssign<$t>, div_assign);
    )*};
}
impl_div_primitive_with_fbig!(u8 u16 u32 u64 u128 usize UBig i8 i16 i32 i64 i128 isize IBig);
// TODO: we should specialize FBig / UBig or FBig / IBig for better efficiency

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
    pub(crate) fn repr_div<const B: Word>(&self, lhs: Repr<B>, rhs: Repr<B>) -> Rounded<Repr<B>> {
        assert_finite_operands(&lhs, &rhs);
        assert_limited_precision(self.precision);

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

    pub(crate) fn repr_rem<const B: Word>(&self, lhs: Repr<B>, rhs: Repr<B>) -> Rounded<Repr<B>> {
        assert_finite_operands(&lhs, &rhs);

        let (lhs_sign, lhs_signif) = lhs.significand.into_parts();
        let (_, rhs_signif) = rhs.significand.into_parts();

        use core::cmp::Ordering;
        let (rem, neg) = match lhs.exponent.cmp(&rhs.exponent) {
            Ordering::Equal => {
                let r1 = lhs_signif % &rhs_signif;
                let r2 = rhs_signif - &r1;
                if r1 <= r2 {
                    (r1, false)
                } else {
                    (r2, true)
                }
            },
            Ordering::Greater => {
                // if the least significant digit of lhs is higher than rhs, then we can
                // align lhs to rhs and do simple modulo operations
                let modulo = ModuloRing::new(rhs_signif);
                let shift = (lhs.exponent - rhs.exponent) as usize;
                let scaling = if B == 2 {
                    modulo.convert(UBig::ONE << shift)
                } else {
                    modulo.convert(UBig::from_word(B)).pow(&shift.into())
                };
                let r = modulo.convert(lhs_signif) * scaling;
                let r1 = r.residue();
                let r2 = (-r).residue();
                if r1 <= r2 {
                    (r1, false)
                } else {
                    (r2, true)
                }
            },
            Ordering::Less => {
                // otherwise we have to split lhs into two parts
                let shift = (rhs.exponent - lhs.exponent) as usize;
                let (hi, lo) = split_digits::<B>(lhs_signif.into(), shift);
                let mut r1 = hi % &rhs_signif;
                let mut r2 = rhs_signif - &r1; // note that r2 >= 1
                if r1 <= r2 {
                    shl_digits_in_place::<B>(&mut r1, shift);
                    r1 += lo;
                    (r1.try_into().unwrap(), false)
                } else {
                    r2 -= UBig::ONE;
                    shl_digits_in_place::<B>(&mut r2, shift);
                    r2 -= lo;
                    (r2.try_into().unwrap(), true)
                }
            }
        };

        let exponent = lhs.exponent.min(rhs.exponent);
        let significand = Sign::from(neg) * lhs_sign * rem;
        if significand.is_zero() {
            Approximation::Exact(Repr::zero())
        } else {
            self.repr_round(Repr::new(significand, exponent))
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
        assert_finite_operands(lhs, rhs);

        let lhs_repr = if !lhs.is_zero() && lhs.digits_ub() > rhs.digits_lb() + self.precision {
            // shrink lhs if it's larger than necessary
            Self::new(rhs.digits() + self.precision)
                .repr_round_ref(lhs)
                .value()
        } else {
            lhs.clone()
        };
        self.repr_div(lhs_repr, rhs.clone()).map(|v| FBig::new(v, *self))
    }

    /// Calculate the remainder of `⌈lhs / rhs⌋`.
    /// 
    /// The remainder is calculated as `r = lhs - ⌈lhs / rhs⌋ * rhs`, the division rounds to the nearest and ties to away.
    /// So if `n = (lhs / rhs).round()`, then `lhs == n * rhs + r` (given enough precision).
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// use dashu_base::Approximation::*;
    /// use dashu_float::{Context, round::{mode::HalfAway, Rounding::*}};
    ///
    /// let context = Context::<HalfAway>::new(3);
    /// let a = DBig::from_str_native("6.789")?;
    /// let b = DBig::from_str_native("-1.234")?;
    /// assert_eq!(context.rem(&a.repr(), &b.repr()), Exact(DBig::from_str_native("-0.615")?));
    /// # Ok::<(), ParseError>(())
    /// ```
    pub fn rem<const B: Word>(&self, lhs: &Repr<B>, rhs: &Repr<B>) -> Rounded<FBig<R, B>> {
        assert_finite_operands(lhs, rhs);
        self.repr_rem(lhs.clone(), rhs.clone()).map(|v| FBig::new(v, *self))
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
        self.repr_div(Repr::one(), f.clone()).map(|v| FBig::new(v, *self))
    }
}
