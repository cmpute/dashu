use dashu_base::Sign::{self, *};
use dashu_int::{IBig, UBig};

use crate::{
    add::cancel_zero,
    error::{assert_finite_operands, FpError, FpResult},
    fbig::FBig,
    helper_macros,
    repr::{Context, Repr, Word},
    round::Round,
};
use core::cmp::Ordering;
use core::ops::{Mul, MulAssign};

impl<R: Round, const B: Word> Mul<&FBig<R, B>> for &FBig<R, B> {
    type Output = FBig<R, B>;

    #[inline]
    fn mul(self, rhs: &FBig<R, B>) -> Self::Output {
        assert_finite_operands(&self.repr, &rhs.repr);

        let context = Context::max(self.context, rhs.context);
        let repr = &self.repr * &rhs.repr;
        if repr.is_infinite() {
            return FBig::new(repr, context);
        }
        FBig::new(context.repr_round(repr).value(), context)
    }
}

impl<R: Round, const B: Word> Mul<&FBig<R, B>> for FBig<R, B> {
    type Output = FBig<R, B>;

    #[inline]
    fn mul(self, rhs: &FBig<R, B>) -> Self::Output {
        assert_finite_operands(&self.repr, &rhs.repr);

        let context = Context::max(self.context, rhs.context);
        let repr = &self.repr * &rhs.repr;
        if repr.is_infinite() {
            return FBig::new(repr, context);
        }
        FBig::new(context.repr_round(repr).value(), context)
    }
}

impl<R: Round, const B: Word> Mul<FBig<R, B>> for &FBig<R, B> {
    type Output = FBig<R, B>;

    #[inline]
    fn mul(self, rhs: FBig<R, B>) -> Self::Output {
        assert_finite_operands(&self.repr, &rhs.repr);

        let context = Context::max(self.context, rhs.context);
        let repr = &self.repr * &rhs.repr;
        if repr.is_infinite() {
            return FBig::new(repr, context);
        }
        FBig::new(context.repr_round(repr).value(), context)
    }
}

impl<R: Round, const B: Word> Mul<FBig<R, B>> for FBig<R, B> {
    type Output = FBig<R, B>;

    #[inline]
    fn mul(self, rhs: FBig<R, B>) -> Self::Output {
        assert_finite_operands(&self.repr, &rhs.repr);

        let context = Context::max(self.context, rhs.context);
        let repr = &self.repr * &rhs.repr;
        if repr.is_infinite() {
            return FBig::new(repr, context);
        }
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
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str("-1.234")?;
    /// assert_eq!(a.sqr(), DBig::from_str("1.523")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn sqr(&self) -> Self {
        self.context.unwrap_fp(self.context.sqr(&self.repr))
    }

    /// Compute the cubic of this number (`self * self * self`)
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str("-1.234")?;
    /// assert_eq!(a.cubic(), DBig::from_str("-1.879")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn cubic(&self) -> Self {
        self.context.unwrap_fp(self.context.cubic(&self.repr))
    }

    /// Fused multiply–add with a single rounding: `c + sign·(self * b)`.
    ///
    /// Unlike `(self * b) + c`, which rounds twice, `fma` rounds the exact
    /// `self * b + c` once. `sign` scales the product: [`Sign::Positive`] gives
    /// `self*b + c`, [`Sign::Negative`] gives `c − self*b`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::{ParseError, Sign};
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str("1.5")?;
    /// let b = DBig::from_str("2.0")?;
    /// let c = DBig::from_str("0.1")?;
    /// // 1.5*2.0 + 0.1 = 3.1
    /// assert_eq!(a.fma(&b, &c, Sign::Positive), DBig::from_str("3.1")?);
    /// // 0.1 − 1.5*2.0 = −2.9
    /// assert_eq!(a.fma(&b, &c, Sign::Negative), DBig::from_str("-2.9")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn fma(&self, b: &Self, c: &Self, sign: Sign) -> Self {
        let context = Context::max(self.context, Context::max(b.context, c.context));
        context.unwrap_fp(context.fma(&self.repr, &b.repr, &c.repr, sign))
    }
}

impl<R: Round> Context<R> {
    /// Multiply two floating point numbers under this context.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// use dashu_base::Approximation::*;
    /// use dashu_float::{Context, round::{mode::HalfAway, Rounding::*}};
    ///
    /// let context = Context::<HalfAway>::new(2);
    /// let a = DBig::from_str("-1.234")?;
    /// let b = DBig::from_str("6.789")?;
    /// assert_eq!(
    ///     context.mul(&a.repr(), &b.repr()),
    ///     Ok(Inexact(DBig::from_str("-8.4")?, SubOne))
    /// );
    /// # Ok::<(), ParseError>(())
    /// ```
    pub fn mul<const B: Word>(&self, lhs: &Repr<B>, rhs: &Repr<B>) -> FpResult<FBig<R, B>> {
        if lhs.is_infinite() || rhs.is_infinite() {
            return Err(FpError::InfiniteInput);
        }

        // Exact product of the full operands, then round. (An earlier version shrank each operand
        // to 2*precision — via `repr_round_ref`, which rounds each operand *correctly* to 2p digits —
        // before multiplying. But rounding the operands *before* multiplying perturbs the product
        // by the accumulated operand-rounding error (~2^-2p relative), so rounding that perturbed
        // product to `precision` could land 1 ulp off the exact-product-rounded value when the true
        // product sat near a rounding boundary. The exact product is always correctly rounded; the
        // shrink only mattered for operands far larger than the target precision, which is uncommon.)
        let repr = lhs * rhs;
        let repr = if repr.is_infinite() {
            return Err(FpError::Overflow(repr.sign()));
        } else if repr.significand.is_zero()
            && !lhs.significand.is_zero()
            && !rhs.significand.is_zero()
        {
            return Err(FpError::Underflow(repr.sign()));
        } else {
            repr
        };
        Ok(self.repr_round(repr).map(|v| FBig::new(v, *self)))
    }

    /// Calculate the square of the floating point number under this context.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// use dashu_base::Approximation::*;
    /// use dashu_float::{Context, round::{mode::HalfAway, Rounding::*}};
    ///
    /// let context = Context::<HalfAway>::new(2);
    /// let a = DBig::from_str("-1.234")?;
    /// assert_eq!(context.sqr(&a.repr()), Ok(Inexact(DBig::from_str("1.5")?, NoOp)));
    /// # Ok::<(), ParseError>(())
    /// ```
    pub fn sqr<const B: Word>(&self, f: &Repr<B>) -> FpResult<FBig<R, B>> {
        if f.is_infinite() {
            return Err(FpError::InfiniteInput);
        }

        // Exact square of the full significand, then round. (An earlier version shrank the operand
        // to 2*precision before squaring, but that pre-rounding perturbs the square and could leave
        // the result 1 ulp off the correctly-rounded value near a rounding boundary — same issue
        // as `mul`. The dedicated `sqr` kernel is still used; it just gets the full significand.)
        let exponent = f.exponent.checked_mul(2).ok_or({
            // sqr always produces a non-negative result
            if f.exponent > 0 {
                FpError::Overflow(Positive)
            } else {
                FpError::Underflow(Positive)
            }
        })?;
        let repr = Repr::new(f.significand.sqr().into(), exponent);
        let repr = repr.check_finite_exponent()?;
        Ok(self.repr_round(repr).map(|v| FBig::new(v, *self)))
    }

    /// Calculate the cubic of the floating point number under this context.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// use dashu_base::Approximation::*;
    /// use dashu_float::{Context, round::{mode::HalfAway, Rounding::*}};
    ///
    /// let context = Context::<HalfAway>::new(2);
    /// let a = DBig::from_str("-1.234")?;
    /// assert_eq!(context.cubic(&a.repr()), Ok(Inexact(DBig::from_str("-1.9")?, SubOne)));
    /// # Ok::<(), ParseError>(())
    /// ```
    pub fn cubic<const B: Word>(&self, f: &Repr<B>) -> FpResult<FBig<R, B>> {
        if f.is_infinite() {
            return Err(FpError::InfiniteInput);
        }

        // Exact cube of the full significand, then round. (An earlier version shrank the operand
        // to 3*precision before cubing, but that pre-rounding perturbs the cube and could leave the
        // result 1 ulp off the correctly-rounded value near a rounding boundary — same issue as
        // `mul`. The dedicated `cubic` kernel is still used; it just gets the full significand.)
        let repr = if f.significand.is_zero() {
            // cubic(±0) = ±0 (odd power preserves sign)
            if f.is_neg_zero() {
                Repr::neg_zero()
            } else {
                Repr::zero()
            }
        } else {
            let sign = f.sign();
            let exponent = f.exponent.checked_mul(3).ok_or({
                if f.exponent > 0 {
                    FpError::Overflow(sign)
                } else {
                    FpError::Underflow(sign)
                }
            })?;
            let repr = Repr::new(f.significand.cubic(), exponent);
            repr.check_finite_exponent()?
        };
        Ok(self.repr_round(repr).map(|v| FBig::new(v, *self)))
    }

    /// Fused multiply–add under this context: `c + sign·(a·b)`, rounded once.
    ///
    /// The product `a·b` is formed exactly, then added to `c` with a single
    /// rounding (reusing the aligned-then-round path of [`add`](Self::add), so the
    /// severe-cancellation and sticky-tail handling is identical — including the
    /// single guard digit an effective subtraction may leave in the result).
    /// `sign` scales the product: [`Sign::Positive`] → `a·b + c`,
    /// [`Sign::Negative`] → `c − a·b`.
    ///
    /// Returns [`FpError::InfiniteInput`] if any operand is infinite (matching
    /// [`add`](Self::add)/[`mul`](Self::mul); dashu rejects infinite operands
    /// outright, so the IEEE-754 `inf·0` / `inf−inf` indeterminate forms do not
    /// arise). [`Overflow`](FpError::Overflow)/[`Underflow`](FpError::Underflow)
    /// propagate from the product's exponent.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::{Approximation::*, ParseError, Sign};
    /// # use dashu_float::{Context, DBig, round::{mode::HalfAway, Rounding::*}};
    /// let context = Context::<HalfAway>::new(2);
    /// let a = DBig::from_str("1.5")?;
    /// let b = DBig::from_str("2.0")?;
    /// let c = DBig::from_str("0.1")?;
    /// assert_eq!(
    ///     context.fma(&a.repr(), &b.repr(), &c.repr(), Sign::Positive),
    ///     Ok(Exact(DBig::from_str("3.1")?))
    /// );
    /// # Ok::<(), ParseError>(())
    /// ```
    pub fn fma<const B: Word>(
        &self,
        a: &Repr<B>,
        b: &Repr<B>,
        c: &Repr<B>,
        sign: Sign,
    ) -> FpResult<FBig<R, B>> {
        if a.is_infinite() || b.is_infinite() || c.is_infinite() {
            return Err(FpError::InfiniteInput);
        }

        // Exact product a·b. No operand shrinking (unlike Context::mul's 2p bound):
        // a cancellation between the product and c can expose arbitrarily low
        // product digits, so the full exact product is required for a correctly-
        // rounded result. The `Repr` product saturates exponent overflow/underflow
        // to the infinity/zero sentinels, so detect those as Context::mul does.
        let prod = a * b;
        let prod = if prod.is_infinite() {
            return Err(FpError::Overflow(prod.sign()));
        } else if prod.significand.is_zero() && !a.significand.is_zero() && !b.significand.is_zero()
        {
            return Err(FpError::Underflow(prod.sign()));
        } else {
            prod
        };

        // Add c to sign·(a·b) with a single rounding. The product is exact, so the
        // only rounding is in the add step — the same path as Context::add/sub.
        let sum = if prod.significand.is_zero() {
            // a·b == ±0: the signed zero product adds nothing to c.
            self.repr_round_ref(c)
        } else {
            let signed_prod = if sign == Negative { prod.neg() } else { prod };
            if c.significand.is_zero() {
                // c == ±0: the result is sign·(a·b), rounded once.
                self.repr_round(signed_prod)
            } else {
                match c.exponent.cmp(&signed_prod.exponent) {
                    Ordering::Equal => self.repr_round(cancel_zero::<R, B>(
                        &c.significand + signed_prod.significand,
                        c.exponent,
                    )),
                    Ordering::Greater => {
                        self.repr_add_large_small(c.clone(), &signed_prod, Positive)
                    }
                    Ordering::Less => self.repr_add_small_large(c.clone(), &signed_prod, Positive),
                }
            }
        };
        Ok(sum.map(|v| FBig::new(v, *self)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::round::mode;
    use dashu_int::IBig;

    /// Reference: `c + sign·(a·b)` computed exactly at `4p+32` digits then rounded
    /// down to `p`. A correctly-rounded `fma` must agree with this.
    fn oracle<const B: Word, R: Round>(
        a: &Repr<B>,
        b: &Repr<B>,
        c: &Repr<B>,
        sign: Sign,
        p: usize,
    ) -> FBig<R, B> {
        let hi = Context::<R>::new(p * 4 + 32);
        let prod = hi.mul(a, b).unwrap().value();
        let signed = if sign == Negative { -prod } else { prod };
        let sum = hi.add(c, signed.repr()).unwrap().value();
        sum.with_precision(p).value()
    }

    fn r<const B: Word>(sig: i128, exp: isize) -> Repr<B> {
        Repr::new(IBig::from(sig), exp)
    }

    /// Force-round `v`'s significand to exactly `p` digits. (`with_precision` is a
    /// no-op when the context precision already equals `p`; the guard digit an
    /// effective subtraction leaves lives in the significand, beyond the context
    /// precision, so it must be rounded away explicitly.)
    fn round_sig<R: Round, const B: Word>(v: &FBig<R, B>, p: usize) -> FBig<R, B> {
        let ctx = Context::<R>::new(p);
        FBig::new(ctx.repr_round_ref(v.repr()).value(), ctx)
    }

    /// `fma` matches the high-precision oracle across fixed inputs, precisions,
    /// both signs, base 10. (FMA reuses the add path, so on an effective
    /// subtraction it may carry one guard digit — like `Context::sub` — so we
    /// re-round to `p` before comparing to the exactly-`p` oracle.)
    #[test]
    fn test_fma_matches_oracle_decimal() {
        // (a sig, a exp, b sig, b exp, c sig, c exp)
        let cases: &[(i128, isize, i128, isize, i128, isize)] = &[
            (15, -1, 20, -1, 10, -1),     // 1.5·2.0 + 0.1
            (123, -2, 456, -2, 789, -2),  // 1.23·4.56 + 7.89
            (101, -2, 99, -2, -9999, -4), // 1.01·0.99 − 0.9999 ≈ 0 (cancellation, a≠b)
            (999, -2, 101, -1, -1, 2),    // 9.99·10.1 − 100 (mild cancel, diff exponents)
        ];
        for &(asg, ae, bsg, be, csg, ce) in cases {
            for &p in &[2usize, 5, 20] {
                let (a, b, c) = (r::<10>(asg, ae), r::<10>(bsg, be), r::<10>(csg, ce));
                let ctx = Context::<mode::HalfAway>::new(p);
                for sign in [Positive, Negative] {
                    let got = ctx.fma(&a, &b, &c, sign).unwrap().value();
                    let want = oracle::<10, mode::HalfAway>(&a, &b, &c, sign, p);
                    assert_eq!(
                        round_sig(&got, p),
                        want,
                        "fma mismatch p={p} sign={sign:?} a={asg}e{ae} b={bsg}e{be} c={csg}e{ce}"
                    );
                }
            }
        }
    }

    /// Base-2 spot check (HalfEven).
    #[test]
    fn test_fma_matches_oracle_binary() {
        let (a, b, c) = (r::<2>(5, -2), r::<2>(3, -1), r::<2>(7, -3)); // 1.25, 1.5, 0.875
        for &p in &[4usize, 10, 30] {
            let ctx = Context::<mode::HalfEven>::new(p);
            for sign in [Positive, Negative] {
                let got = ctx.fma(&a, &b, &c, sign).unwrap().value();
                let want = oracle::<2, mode::HalfEven>(&a, &b, &c, sign, p);
                assert_eq!(round_sig(&got, p), want, "base-2 fma mismatch p={p} sign={sign:?}");
            }
        }
    }

    /// A zero product ⇒ result is `c`; a zero `c` ⇒ result is `a·b`.
    #[test]
    fn test_fma_zero_operands() {
        let ctx = Context::<mode::HalfAway>::new(5);
        let (z, a, c) = (r::<10>(0, 0), r::<10>(3, 0), r::<10>(7, 0));
        // a·b == 0 (z·a): result is c.
        assert_eq!(ctx.fma(&z, &a, &c, Positive).unwrap().value().repr(), &c);
        // c == 0: result is a·b (3·3 = 9).
        assert_eq!(ctx.fma(&a, &a, &z, Positive).unwrap().value().repr(), &r::<10>(9, 0));
    }

    /// Any infinite operand ⇒ `InfiniteInput`.
    #[test]
    fn test_fma_infinity_is_error() {
        let ctx = Context::<mode::HalfAway>::new(5);
        let (inf, a) = (Repr::<10>::infinity(), r::<10>(3, 0));
        assert_eq!(ctx.fma(&inf, &a, &a, Positive), Err(FpError::InfiniteInput));
        assert_eq!(ctx.fma(&a, &a, &inf, Positive), Err(FpError::InfiniteInput));
    }

    /// An exact-zero result is `-0` under roundTowardNegative (Down), exercising
    /// the `cancel_zero` path (IEEE 754 §6.3).
    #[test]
    fn test_fma_exact_zero_is_neg_zero_under_down() {
        let ctx = Context::<mode::Down>::new(5);
        // 2·3 + (-6) = 0 exactly.
        let (a, b, c) = (r::<10>(2, 0), r::<10>(3, 0), r::<10>(-6, 0));
        let got = ctx.fma(&a, &b, &c, Positive).unwrap().value();
        assert!(got.repr().is_neg_zero(), "expected -0, got {:?}", got.repr());
    }
}
