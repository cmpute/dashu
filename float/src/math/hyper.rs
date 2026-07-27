//! Hyperbolic functions, built from the cancellation-free `exp_m1` / `ln_1p` primitives:
//!
//! - `sinh(x) = (exp_m1(x) - exp_m1(-x)) / 2`
//! - `cosh(x) = (exp_m1(x) + exp_m1(-x)) / 2 + 1`
//! - `tanh(x) = exp_m1(2x) / (exp_m1(2x) + 2)`
//! - `asinh(x) = sign(x) · ln_1p(|x| + x²/(sqrt(x²+1)+1))`
//! - `acosh(x) = ln_1p((x-1) + sqrt((x-1)(x+1)))`  (x ≥ 1)
//! - `atanh(x) = ln_1p(2x/(1-x)) / 2`  (|x| < 1)
//!
//! The `exp_m1` / `ln_1p` forms avoid the catastrophic cancellation that the naive
//! `(exp(x)-exp(-x))/2` and `ln(1+…)` formulas suffer for small arguments. Special
//! values follow IEEE 754: infinities are values (not errors) for the forward functions
//! and `asinh`; `acosh(x<1)` and `atanh(|x|>1)` are domain errors.

use crate::{
    error::{assert_limited_precision, FpError},
    exp::exp_overflows,
    fbig::FBig,
    math::{
        cache::{reborrow_cache, ConstCache},
        FpResult,
    },
    repr::{Context, Repr, Word},
    round::{ErrorBounds, Round},
};
use dashu_base::{Abs, AbsOrd, Approximation::Exact, Sign};

impl<R: ErrorBounds> Context<R> {
    /// Hyperbolic sine.
    pub fn sinh<const B: Word>(
        &self,
        x: &Repr<B>,
        mut cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        if x.is_infinite() {
            return Ok(Exact(FBig::new(Repr::infinity_with_sign(x.sign()), *self)));
        }
        assert_limited_precision(self.precision);
        if x.significand.is_zero() {
            // sinh(±0) = ±0
            return Ok(Exact(FBig::new(signed_zero_repr(x), *self)));
        }
        // Hoist the exp overflow out of the Ziv closure (it can't return Err): sinh(±huge) = ±inf.
        if exp_overflows::<R, B>(self, x, &mut cache) {
            return Err(FpError::Overflow(x.sign()));
        }

        // sinh(x) = (exp_m1(x) - exp_m1(-x)) / 2  (cancellation-free). `exp_m1` is itself Ziv-correct
        // at the working precision, so only the subtraction/divide rounding contributes to the
        // radius (a few working-ULPs, scaled by the `exp_m1(x) ≈ 2·sinh(x)` magnitude ratio).
        let initial_guard = self.base_guard_digits::<B>() + 10;
        Ok(self.ziv(initial_guard, |guard| {
            let work = Context::<R>::new(self.precision + guard);
            let x_f = FBig::<R, B>::new(work.repr_round_ref(x).value(), work);
            let ep = work
                .exp_m1(&x_f.repr, reborrow_cache(&mut cache))
                .unwrap()
                .value();
            let em = work
                .exp_m1(&(-x_f.clone()).repr, reborrow_cache(&mut cache))
                .unwrap()
                .value();
            let result = (ep - em) / 2i32;
            let radius = result.ulp() * 12;
            (result, radius)
        }))
    }

    /// Hyperbolic cosine.
    pub fn cosh<const B: Word>(
        &self,
        x: &Repr<B>,
        mut cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        if x.is_infinite() {
            // cosh(±inf) = +inf
            return Ok(Exact(FBig::new(Repr::infinity(), *self)));
        }
        assert_limited_precision(self.precision);
        if x.significand.is_zero() {
            // cosh(±0) = 1
            return Ok(Exact(FBig::new(Repr::one(), *self)));
        }

        // Hoist the exp overflow out of the Ziv closure: cosh(±huge) = +inf (always positive).
        if exp_overflows::<R, B>(self, x, &mut cache) {
            return Err(FpError::Overflow(Sign::Positive));
        }
        // cosh(x) = (exp_m1(x) + exp_m1(-x)) / 2 + 1 (no cancellation: same-sign sum). `exp_m1` is
        // Ziv-correct at the working precision; the radius is a few working-ULPs (the `exp_m1(x) ≈
        // 2·cosh(x)` magnitude ratio, plus the trailing +1).
        let initial_guard = self.base_guard_digits::<B>() + 10;
        Ok(self.ziv(initial_guard, |guard| {
            let work = Context::<R>::new(self.precision + guard);
            let x_f = FBig::<R, B>::new(work.repr_round_ref(x).value(), work);
            let ep = work
                .exp_m1(&x_f.repr, reborrow_cache(&mut cache))
                .unwrap()
                .value();
            let em = work
                .exp_m1(&(-x_f.clone()).repr, reborrow_cache(&mut cache))
                .unwrap()
                .value();
            let result = (ep + em) / 2i32 + FBig::<R, B>::ONE;
            let radius = result.ulp() * 14;
            (result, radius)
        }))
    }

    /// Simultaneously compute `sinh(x)` and `cosh(x)` (context layer). Returns
    /// `(sinh_result, cosh_result)` where each is a [`FpResult`].
    ///
    /// This is more efficient than calling [`sinh`](Context::sinh) and [`cosh`](Context::cosh)
    /// separately, since the two share the `exp_m1(±x)` sub-computations.
    pub fn sinh_cosh<const B: Word>(
        &self,
        x: &Repr<B>,
        mut cache: Option<&mut ConstCache>,
    ) -> (FpResult<FBig<R, B>>, FpResult<FBig<R, B>>) {
        if x.is_infinite() {
            return (
                Ok(Exact(FBig::new(Repr::infinity_with_sign(x.sign()), *self))),
                Ok(Exact(FBig::new(Repr::infinity(), *self))),
            );
        }
        assert_limited_precision(self.precision);
        if x.significand.is_zero() {
            return (
                Ok(Exact(FBig::new(signed_zero_repr(x), *self))),
                Ok(Exact(FBig::new(Repr::one(), *self))),
            );
        }

        // Hoist the exp overflow out of the Ziv closure: sinh(±huge) = ±inf, cosh(±huge) = +inf.
        if exp_overflows::<R, B>(self, x, &mut cache) {
            return (Err(FpError::Overflow(x.sign())), Err(FpError::Overflow(Sign::Positive)));
        }
        // sinh = (ep - em)/2; cosh = (ep + em)/2 + 1, sharing the two `exp_m1` calls. Certified as a
        // pair via `ziv_pair` (retry while either endpoint straddles a boundary).
        let initial_guard = self.base_guard_digits::<B>() + 10;
        let (sinh_r, cosh_r) = self.ziv_pair(initial_guard, |guard| {
            let work = Context::<R>::new(self.precision + guard);
            let x_f = FBig::<R, B>::new(work.repr_round_ref(x).value(), work);
            let ep = work
                .exp_m1(&x_f.repr, reborrow_cache(&mut cache))
                .unwrap()
                .value();
            let em = work
                .exp_m1(&(-x_f.clone()).repr, reborrow_cache(&mut cache))
                .unwrap()
                .value();
            let sinh_val = (ep.clone() - em.clone()) / 2i32;
            let cosh_val = (ep + em) / 2i32 + FBig::<R, B>::ONE;
            let sinh_radius = sinh_val.ulp() * 12;
            let cosh_radius = cosh_val.ulp() * 14;
            ((sinh_val, sinh_radius), (cosh_val, cosh_radius))
        });
        (Ok(sinh_r), Ok(cosh_r))
    }

    /// Hyperbolic tangent.
    pub fn tanh<const B: Word>(
        &self,
        x: &Repr<B>,
        mut cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        if x.is_infinite() {
            // tanh(±inf) = ±1
            let one = FBig::new(Repr::one(), *self);
            return Ok(Exact(if x.sign() == Sign::Negative {
                -one
            } else {
                one
            }));
        }
        assert_limited_precision(self.precision);
        if x.significand.is_zero() {
            // tanh(±0) = ±0
            return Ok(Exact(FBig::new(signed_zero_repr(x), *self)));
        }

        // tanh(x) = exp_m1(2x) / (exp_m1(2x) + 2). `exp_m1(2x)` is Ziv-correct at the working
        // precision. For large positive x it overflows → tanh = +1 (returned inline as an exact
        // value); for large negative x, exp_m1(2x) → -1 (finite), so tanh → -1 naturally.
        let initial_guard = self.base_guard_digits::<B>() + 10;
        Ok(self.ziv(initial_guard, |guard| {
            let work = Context::<R>::new(self.precision + guard);
            let x_f = FBig::<R, B>::new(work.repr_round_ref(x).value(), work);
            let two_x = x_f * 2i32;
            match work.exp_m1(&two_x.repr, reborrow_cache(&mut cache)) {
                Err(FpError::Overflow(_)) => (FBig::<R, B>::ONE, FBig::<R, B>::ZERO), // exact +1
                Ok(e) => {
                    let e = e.value();
                    let result = e.clone() / (e + 2i32);
                    let radius = result.ulp() * 12;
                    (result, radius)
                }
                Err(other) => unreachable!("exp_m1 on finite input: {other:?}"),
            }
        }))
    }

    /// Inverse hyperbolic sine.
    pub fn asinh<const B: Word>(
        &self,
        x: &Repr<B>,
        mut cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        if x.is_infinite() {
            return Ok(Exact(FBig::new(Repr::infinity_with_sign(x.sign()), *self)));
        }
        assert_limited_precision(self.precision);
        if x.significand.is_zero() {
            // asinh(±0) = ±0
            return Ok(Exact(FBig::new(signed_zero_repr(x), *self)));
        }

        // asinh(x) = sign(x) · ln_1p(|x| + x²/(sqrt(x²+1)+1)) — the x²/(sqrt+1) form avoids the
        // `sqrt(x²+1) − 1` cancellation near 0. `ln_1p`/`ln`/`sqrt` are Ziv-correct at the working
        // precision, so the radius is a few working-ULPs of accumulated arithmetic. The `|x|` so
        // large that `x²` overflows arm falls back to the asymptotic `sign·ln(2|x|)`.
        let initial_guard = self.base_guard_digits::<B>() + 10;
        Ok(self.ziv(initial_guard, |guard| {
            let work = Context::<R>::new(self.precision + guard);
            let x_f = FBig::<R, B>::new(work.repr_round_ref(x).value(), work);
            let sign = x_f.sign();
            let abs_x = x_f.abs();
            let res = match work.sqr(&abs_x.repr) {
                Ok(x_sq) => {
                    let x_sq = x_sq.value();
                    let sqrt_plus_one = work
                        .sqrt(&(x_sq.clone() + FBig::<R, B>::ONE).repr)
                        .unwrap()
                        .value()
                        + FBig::<R, B>::ONE;
                    let arg = abs_x.clone() + x_sq / sqrt_plus_one;
                    work.ln_1p(&arg.repr, reborrow_cache(&mut cache))
                        .unwrap()
                        .value()
                }
                // |x| so large that x² overflows: asinh(x) ≈ sign·ln(2|x|).
                Err(FpError::Overflow(_)) => work
                    .ln(&(abs_x.clone() * 2i32).repr, reborrow_cache(&mut cache))
                    .unwrap()
                    .value(),
                Err(other) => unreachable!("sqr: {other:?}"),
            };
            let result = apply_sign(res, sign);
            let radius = result.ulp() * 14;
            (result, radius)
        }))
    }

    /// Inverse hyperbolic cosine. Domain: `x ≥ 1`.
    pub fn acosh<const B: Word>(
        &self,
        x: &Repr<B>,
        mut cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        if x.is_infinite() {
            if x.sign() == Sign::Negative {
                return Err(FpError::OutOfDomain);
            }
            return Ok(Exact(FBig::new(Repr::infinity(), *self)));
        }
        assert_limited_precision(self.precision);
        // domain x ≥ 1 (acosh(1) = 0 is handled below; x < 1 is an error)
        if x.sign() == Sign::Negative
            || FBig::<R, B>::new(x.clone(), *self)
                .abs_cmp(&FBig::ONE)
                .is_lt()
        {
            return Err(FpError::OutOfDomain);
        }
        if x.is_one() {
            return Ok(Exact(FBig::new(Repr::zero(), *self)));
        }

        // acosh(x) = ln_1p((x-1) + sqrt((x-1)(x+1))) — the (x-1)(x+1) form avoids the `x²−1`
        // cancellation near x = 1. `ln_1p`/`ln`/`sqrt` are Ziv-correct at the working precision;
        // the radius is a few working-ULPs (generous for the near-x=1 cancellation). The `(x-1)(x+1)`
        // overflow arm falls back to the asymptotic `ln(2x)`.
        let initial_guard = self.base_guard_digits::<B>() + 10;
        Ok(self.ziv(initial_guard, |guard| {
            let work = Context::<R>::new(self.precision + guard);
            let x_f = FBig::<R, B>::new(work.repr_round_ref(x).value(), work);
            let xm1 = &x_f - FBig::<R, B>::ONE;
            let xp1 = &x_f + FBig::<R, B>::ONE;
            let res = match work.mul(&xm1.repr, &xp1.repr) {
                Ok(prod) => {
                    let arg = xm1.clone() + work.sqrt(&prod.value().repr).unwrap().value();
                    work.ln_1p(&arg.repr, reborrow_cache(&mut cache))
                        .unwrap()
                        .value()
                }
                // (x-1)(x+1) overflowed: acosh(x) ≈ ln(2x).
                Err(FpError::Overflow(_)) => work
                    .ln(&(x_f.clone() * 2i32).repr, reborrow_cache(&mut cache))
                    .unwrap()
                    .value(),
                Err(other) => unreachable!("mul: {other:?}"),
            };
            let radius = res.ulp() * 16;
            (res, radius)
        }))
    }

    /// Inverse hyperbolic tangent. Domain: `-1 < x < 1` (`x = ±1` → ±∞, `|x| > 1` is an error).
    pub fn atanh<const B: Word>(
        &self,
        x: &Repr<B>,
        mut cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        if x.is_infinite() {
            return Err(FpError::OutOfDomain);
        }
        assert_limited_precision(self.precision);
        if x.significand.is_zero() {
            // atanh(±0) = ±0
            return Ok(Exact(FBig::new(signed_zero_repr(x), *self)));
        }
        // domain |x| < 1: |x| = 1 → ±∞ (value), |x| > 1 → error
        match FBig::<R, B>::new(x.clone(), *self).abs_cmp(&FBig::ONE) {
            core::cmp::Ordering::Greater => return Err(FpError::OutOfDomain),
            core::cmp::Ordering::Equal => {
                return Ok(Exact(FBig::new(Repr::infinity_with_sign(x.sign()), *self)));
            }
            _ => {}
        }

        // atanh(x) = ln_1p(2x/(1-x)) / 2. `ln_1p` is Ziv-correct at the working precision; the
        // radius is a few working-ULPs (generous: the `2x/(1-x)` division amplifies as |x| → 1, but
        // the result grows there too, so its ULP keeps the bound sound — Ziv retries near |x|=1).
        let initial_guard = self.base_guard_digits::<B>() + 10;
        Ok(self.ziv(initial_guard, |guard| {
            let work = Context::<R>::new(self.precision + guard);
            let x_f = FBig::<R, B>::new(work.repr_round_ref(x).value(), work);
            let ratio = (x_f.clone() * 2i32) / (FBig::<R, B>::ONE - &x_f);
            let res = work
                .ln_1p(&ratio.repr, reborrow_cache(&mut cache))
                .unwrap()
                .value();
            let result = res / 2i32;
            let radius = result.ulp() * 16;
            (result, radius)
        }))
    }
}

impl<R: ErrorBounds, const B: Word> FBig<R, B> {
    /// Calculate the hyperbolic sine of the floating point number.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str("0.5000000")?;
    /// assert_eq!(a.sinh(), DBig::from_str("0.52109531")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn sinh(&self) -> Self {
        self.context.unwrap_fp(self.context.sinh(&self.repr, None))
    }

    /// Calculate the hyperbolic cosine of the floating point number.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str("0.5000000")?;
    /// assert_eq!(a.cosh(), DBig::from_str("1.127626")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn cosh(&self) -> Self {
        self.context.unwrap_fp(self.context.cosh(&self.repr, None))
    }

    /// Simultaneously calculate the hyperbolic sine and cosine of the number.
    ///
    /// This is more efficient than calling [`sinh`](FBig::sinh) and [`cosh`](FBig::cosh)
    /// separately, since the two share the `exp_m1(±x)` sub-computations.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str("0.5000000")?;
    /// let (s, c) = a.sinh_cosh();
    /// assert_eq!(s, DBig::from_str("0.52109531")?);
    /// assert_eq!(c, DBig::from_str("1.127626")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn sinh_cosh(&self) -> (Self, Self) {
        let (s, c) = self.context.sinh_cosh(&self.repr, None);
        (self.context.unwrap_fp(s), self.context.unwrap_fp(c))
    }

    /// Calculate the hyperbolic tangent of the floating point number.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str("0.5000000")?;
    /// assert_eq!(a.tanh(), DBig::from_str("0.46211716")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn tanh(&self) -> Self {
        self.context.unwrap_fp(self.context.tanh(&self.repr, None))
    }

    /// Calculate the inverse hyperbolic sine of the floating point number.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str("0.5000000")?;
    /// assert_eq!(a.asinh(), DBig::from_str("0.48121183")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn asinh(&self) -> Self {
        self.context.unwrap_fp(self.context.asinh(&self.repr, None))
    }

    /// Calculate the inverse hyperbolic cosine of the floating point number.
    ///
    /// # Panics
    ///
    /// Panics if the number is less than 1 (out of domain).
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str("2.000000")?;
    /// assert_eq!(a.acosh(), DBig::from_str("1.316958")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn acosh(&self) -> Self {
        self.context.unwrap_fp(self.context.acosh(&self.repr, None))
    }

    /// Calculate the inverse hyperbolic tangent of the floating point number.
    ///
    /// # Panics
    ///
    /// Panics if the absolute value is greater than or equal to 1 (out of domain;
    /// `|x| = 1` is infinite and `|x| > 1` is not real).
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str("0.5000000")?;
    /// assert_eq!(a.atanh(), DBig::from_str("0.54930614")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn atanh(&self) -> Self {
        self.context.unwrap_fp(self.context.atanh(&self.repr, None))
    }
}

/// `±0` `Repr` carrying the sign of `x` (used by the odd hyperbolics at zero input).
fn signed_zero_repr<const B: Word>(x: &Repr<B>) -> Repr<B> {
    if x.is_neg_zero() {
        Repr::neg_zero()
    } else {
        Repr::zero()
    }
}

/// Negate `v` when `sign` is `Negative` (used to apply `sign(x)` in `asinh`).
fn apply_sign<R: Round, const B: Word>(v: FBig<R, B>, sign: Sign) -> FBig<R, B> {
    if sign == Sign::Negative {
        -v
    } else {
        v
    }
}
