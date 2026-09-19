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
    ball::Ball,
    error::{assert_limited_precision, FpError},
    fbig::FBig,
    math::{
        cache::{reborrow_cache, ConstCache},
        FpResult,
    },
    repr::{Context, Repr, Word},
    round::{mode, ErrorBounds},
};
use dashu_base::{Abs, AbsOrd, Approximation::Exact, BitTest, Sign};
use dashu_int::IBig;

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
        // sinh(x) = (exp_m1(x) - exp_m1(-x)) / 2  (cancellation-free). Both `exp_m1` come from the
        // Ball-based `exp_compute`; the subtraction/division roundings, the `exp_m1` errors *and
        // the input's own rounding* are tracked mechanically by the Ball propagation (the raw `x`
        // is passed in — `exp_compute` rounds it to the working precision and carries the error).
        // For huge |x|, `exp_m1` overflows inside the closure and propagates; sinh(±huge) = ±inf,
        // so the sign follows `x` (the propagated error carries an intermediate sign, remapped
        // below).
        let initial_guard = self.base_guard_digits::<B>() + 10;
        self.ziv(initial_guard, |guard| {
            let work = Context::<mode::HalfEven>::new(self.precision + guard);
            let n = 1usize << (work.precision.bit_len() / 2);
            let neg_x = -x.clone();
            let ep =
                work.exp_compute::<B>(x, work.precision, true, n, reborrow_cache(&mut cache))?;
            let em =
                work.exp_compute::<B>(&neg_x, work.precision, true, n, reborrow_cache(&mut cache))?;
            let wp = work.precision;
            Ok(ep
                .sub(&em, wp)?
                .div_int(2, wp)?
                .to_value_radius::<R>(&Context::<R>::new(wp)))
        })
        .map_err(|_| FpError::Overflow(x.sign()))
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

        // cosh(x) = (exp_m1(x) + exp_m1(-x)) / 2 + 1 (no cancellation: same-sign sum). Both
        // `exp_m1` come from the Ball-based `exp_compute`; the sum/divide/+1 roundings and the
        // input's own rounding (folded by `exp_compute` from the raw `x`) are tracked
        // mechanically. For huge |x|, `exp_m1` overflows inside the closure and propagates;
        // cosh(±huge) = +inf (always positive).
        let initial_guard = self.base_guard_digits::<B>() + 10;
        self.ziv(initial_guard, |guard| {
            let work = Context::<mode::HalfEven>::new(self.precision + guard);
            let n = 1usize << (work.precision.bit_len() / 2);
            let neg_x = -x.clone();
            let ep =
                work.exp_compute::<B>(x, work.precision, true, n, reborrow_cache(&mut cache))?;
            let em =
                work.exp_compute::<B>(&neg_x, work.precision, true, n, reborrow_cache(&mut cache))?;
            let wp = work.precision;
            let one = Ball::exact_int(IBig::ONE, wp);
            Ok(ep
                .add(&em, wp)?
                .div_int(2, wp)?
                .add(&one, wp)?
                .to_value_radius::<R>(&Context::<R>::new(wp)))
        })
        .map_err(|_| FpError::Overflow(Sign::Positive))
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

        // sinh = (ep - em)/2; cosh = (ep + em)/2 + 1, sharing the two `exp_m1` calls. Certified as a
        // pair via `ziv_pair` (retry while either endpoint straddles a boundary); the input's own
        // rounding is folded by `exp_compute` from the raw `x`. For huge |x|, `exp_m1` overflows
        // inside the closure and propagates to both slots; sinh(±huge) = ±inf, cosh(±huge) = +inf,
        // so each slot's overflow sign is remapped below.
        let initial_guard = self.base_guard_digits::<B>() + 10;
        let (sinh_r, cosh_r) = self.ziv_pair(initial_guard, |guard| {
            let work = Context::<mode::HalfEven>::new(self.precision + guard);
            let n = 1usize << (work.precision.bit_len() / 2);
            let neg_x = -x.clone();
            let ep =
                work.exp_compute::<B>(x, work.precision, true, n, reborrow_cache(&mut cache))?;
            let em =
                work.exp_compute::<B>(&neg_x, work.precision, true, n, reborrow_cache(&mut cache))?;
            let wp = work.precision;
            let one = Ball::exact_int(IBig::ONE, wp);
            let sinh_ball = ep.sub(&em, wp)?.div_int(2, wp)?;
            let cosh_ball = ep.add(&em, wp)?.div_int(2, wp)?.add(&one, wp)?;
            let ctx = Context::<R>::new(wp);
            Ok((sinh_ball.to_value_radius::<R>(&ctx), cosh_ball.to_value_radius::<R>(&ctx)))
        });
        (
            sinh_r.map_err(|_| FpError::Overflow(x.sign())),
            cosh_r.map_err(|_| FpError::Overflow(Sign::Positive)),
        )
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

        // tanh(x) = exp_m1(2x) / (exp_m1(2x) + 2). `exp_m1(2x)` comes from the Ball-based
        // `exp_compute` on the *exact* `2x = x + x` (so `exp_compute` folds the input's own
        // rounding into the radius); the division's rounding is tracked mechanically. For large
        // positive x it overflows → tanh = +1 (returned inline as an exact value); for large
        // negative x, exp_m1(2x) → -1 (finite), so tanh → -1 naturally.
        let initial_guard = self.base_guard_digits::<B>() + 10;
        self.ziv(initial_guard, |guard| {
            let work = Context::<mode::HalfEven>::new(self.precision + guard);
            let n = 1usize << (work.precision.bit_len() / 2);
            let two_x = x + x;
            match work.exp_compute::<B>(&two_x, work.precision, true, n, reborrow_cache(&mut cache))
            {
                Err(FpError::Overflow(_)) => Ok((FBig::<R, B>::ONE, FBig::<R, B>::ZERO)), // exact +1
                Ok(e) => {
                    let wp = work.precision;
                    let two = Ball::exact_int(IBig::from(2), wp);
                    Ok(e.div(&e.add(&two, wp)?, wp)?
                        .to_value_radius::<R>(&Context::<R>::new(wp)))
                }
                Err(other) => unreachable!("exp_m1 on finite input: {other:?}"),
            }
        })
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
        // `sqrt(x²+1) − 1` cancellation near 0. The composition is tracked as a [`Ball`] from the
        // rounded input up: the input's own rounding, the sqr, sqrt, division and the `ln_1p`
        // input error all propagate mechanically (a hand-picked per-op ulp count cannot see the
        // operand errors inherited through the chain). The `|x|` so large that `x²` overflows arm
        // falls back to the asymptotic `sign·ln(2|x|)` on the exact `2|x|` (whose input rounding
        // `ln_compute` folds itself).
        let initial_guard = self.base_guard_digits::<B>() + 10;
        self.ziv(initial_guard, |guard| {
            let work = Context::<mode::HalfEven>::new(self.precision + guard);
            let sign = x.sign();
            let wp = work.precision;
            let x_ball = Ball::from_rounded(work.repr_round_ref(x), wp);
            let abs_x_ball = if sign == Sign::Negative {
                x_ball.neg()
            } else {
                x_ball
            };
            let one = Ball::exact_int(IBig::ONE, wp);
            let res = match abs_x_ball.mul(&abs_x_ball, wp) {
                Ok(x_sq_ball) => {
                    let sqrt_plus_one = x_sq_ball.add(&one, wp)?.sqrt(wp)?.add(&one, wp)?;
                    let arg = abs_x_ball.add(&x_sq_ball.div(&sqrt_plus_one, wp)?, wp)?;
                    work.ln_1p_ball::<B>(&arg, reborrow_cache(&mut cache))
                }
                // |x| so large that x² overflows: asinh(x) ≈ sign·ln(2|x|).
                Err(FpError::Overflow(_)) => {
                    let two_abs = Repr::new((x.significand() * IBig::from(2)).abs(), x.exponent());
                    work.ln_compute::<B>(
                        &two_abs,
                        work.precision,
                        false,
                        reborrow_cache(&mut cache),
                    )
                }
                Err(other) => unreachable!("sqr: {other:?}"),
            }?;
            let result = if sign == Sign::Negative {
                res.neg()
            } else {
                res
            };
            Ok(result.to_value_radius::<R>(&Context::<R>::new(wp)))
        })
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
        // cancellation near x = 1. The composition is tracked as a [`Ball`] from the rounded input
        // up: the input's own rounding flows into `x−1`/`x+1` and through the product, sqrt,
        // addition and the `ln_1p` input error mechanically. (The input rounding is what a
        // per-op ulp count on the *result* of `x−1` misses: near x = 1 the subtraction cancels
        // the value — and its ulp — down by |x−1|, while the inherited operand error stays at
        // ulp(x); an under-estimated radius then lets Ziv certify the wrong neighbour of a tie.)
        // The `(x-1)(x+1)` overflow arm falls back to the asymptotic `ln(2x)` on the exact `2x`
        // (whose input rounding `ln_compute` folds itself).
        let initial_guard = self.base_guard_digits::<B>() + 10;
        self.ziv(initial_guard, |guard| {
            let work = Context::<mode::HalfEven>::new(self.precision + guard);
            let wp = work.precision;
            let x_ball = Ball::from_rounded(work.repr_round_ref(x), wp);
            let one = Ball::exact_int(IBig::ONE, wp);
            let xm1_ball = x_ball.sub(&one, wp)?;
            let xp1_ball = x_ball.add(&one, wp)?;
            let res = match xm1_ball.mul(&xp1_ball, wp) {
                Ok(prod_ball) => {
                    let arg = xm1_ball.add(&prod_ball.sqrt(wp)?, wp)?;
                    work.ln_1p_ball::<B>(&arg, reborrow_cache(&mut cache))
                }
                // (x-1)(x+1) overflowed: acosh(x) ≈ ln(2x).
                Err(FpError::Overflow(_)) => {
                    let two_x = Repr::new(x.significand() * IBig::from(2), x.exponent());
                    work.ln_compute::<B>(&two_x, work.precision, false, reborrow_cache(&mut cache))
                }
                Err(other) => unreachable!("mul: {other:?}"),
            }?;
            Ok(res.to_value_radius::<R>(&Context::<R>::new(wp)))
        })
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

        // atanh(x) = ln_1p(2x/(1-x)) / 2. The ratio and the `ln_1p` input error are tracked as a
        // [`Ball`]; near |x| = 1 the `2x/(1-x)` division amplifies, but the Ball tracks it (Ziv
        // retries there).
        let initial_guard = self.base_guard_digits::<B>() + 10;
        self.ziv(initial_guard, |guard| {
            let work = Context::<mode::HalfEven>::new(self.precision + guard);
            let wp = work.precision;
            let x_ball = Ball::from_rounded(work.repr_round_ref(x), wp);
            let one = Ball::exact_int(IBig::ONE, wp);
            let ratio = x_ball
                .scale_int(&IBig::from(2), wp)?
                .div(&one.sub(&x_ball, wp)?, wp)?;
            let res = work.ln_1p_ball::<B>(&ratio, reborrow_cache(&mut cache))?;
            Ok(res
                .div_int(2, wp)?
                .to_value_radius::<R>(&Context::<R>::new(wp)))
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::round::mode;
    use dashu_int::IBig;

    // `sinh`/`cosh` go through `unwrap_fp`, so a huge-|x| overflow saturates to the directed
    // endpoint: outward (Up) → ±∞ (cosh) / sign·∞ (sinh), inward (Zero) → the largest finite.
    #[test]
    fn test_sinh_cosh_directed_overflow() {
        let p = 53;
        let max_sig = (IBig::ONE << p) - IBig::ONE;
        let huge = FBig::<mode::HalfEven, 2>::from_parts(IBig::ONE << 63, 0)
            .with_precision(p)
            .value();

        let sinh_up = huge.clone().with_rounding::<mode::Up>().sinh();
        let sinh_zero = huge.clone().with_rounding::<mode::Zero>().sinh();
        assert!(
            sinh_up.repr().is_infinite() && sinh_up.repr().sign() == Sign::Positive,
            "sinh Up -> +∞"
        );
        assert_eq!(sinh_zero.repr().significand(), &max_sig, "sinh Zero -> largest finite");
        assert_eq!(sinh_zero.repr().exponent(), isize::MAX);

        let cosh_up = huge.clone().with_rounding::<mode::Up>().cosh();
        let cosh_zero = huge.clone().with_rounding::<mode::Zero>().cosh();
        assert!(
            cosh_up.repr().is_infinite() && cosh_up.repr().sign() == Sign::Positive,
            "cosh Up -> +∞"
        );
        assert_eq!(cosh_zero.repr().significand(), &max_sig, "cosh Zero -> largest finite");
        assert_eq!(cosh_zero.repr().exponent(), isize::MAX);

        // Negative huge: this is the case the closure's sign remap exists for — `exp_m1(−x)`
        // overflows carrying a positive sign that sinh must flip to negative (and cosh must leave
        // positive). Under `Up`, a negative overflow rounds inward (largest finite negative) while a
        // positive overflow reaches +∞.
        let neg_max_sig = -max_sig.clone();
        let sinh_neg_up = (-huge.clone()).with_rounding::<mode::Up>().sinh();
        assert_eq!(sinh_neg_up.repr().sign(), Sign::Negative, "sinh(−huge) sign");
        assert_eq!(
            sinh_neg_up.repr().significand(),
            &neg_max_sig,
            "sinh(−huge) Up -> largest finite"
        );
        assert_eq!(sinh_neg_up.repr().exponent(), isize::MAX);
        let cosh_neg_up = (-huge.clone()).with_rounding::<mode::Up>().cosh();
        assert!(
            cosh_neg_up.repr().is_infinite() && cosh_neg_up.repr().sign() == Sign::Positive,
            "cosh(−huge) Up -> +∞"
        );

        // sinh_cosh(−huge) = (largest finite negative, +∞) under Up — per-slot sign remap.
        let (sh, ch) = (-huge.clone()).with_rounding::<mode::Up>().sinh_cosh();
        assert_eq!(sh.repr().sign(), Sign::Negative, "sinh_cosh[0](−huge) sign");
        assert_eq!(
            sh.repr().significand(),
            &neg_max_sig,
            "sinh_cosh[0](−huge) Up -> largest finite"
        );
        assert!(
            ch.repr().is_infinite() && ch.repr().sign() == Sign::Positive,
            "sinh_cosh[1](−huge) -> +∞"
        );
    }

    /// A near-tie argument with more digits than the working precision: the input's own
    /// rounding (½ ulp of `x`, which survives the `x−1` cancellation untouched) must be part
    /// of the certified radius. Regression for the mis-rounding this input exhibited (issue
    /// #102): the true acosh sits 1.18e-133 above the 50-digit midpoint, and the dropped
    /// input error (~1e-62) let Ziv certify the lower neighbour.
    fn acosh_near_tie_arg() -> FBig<mode::HalfEven, 10> {
        use core::str::FromStr;
        FBig::<mode::HalfEven, 10>::from_str(
            "1.000008608520579272799119633794615751764072075741189452445358800365413956109281413683821004335915587724859246904",
        )
        .unwrap()
    }

    #[test]
    fn acosh_rounds_up_above_decimal_tie() {
        use core::str::FromStr;
        let x = acosh_near_tie_arg();
        let y = Context::<mode::HalfEven>::new(50)
            .acosh(x.repr(), None)
            .unwrap()
            .value();
        assert_eq!(
            y,
            FBig::<mode::HalfEven, 10>::from_str(
                "0.0041493392794990204344718343065778753953054547309876"
            )
            .unwrap()
        );
    }

    /// The near-tie argument under directed modes must match a high-precision oracle re-rounded
    /// under the same mode (the definition of correct rounding).
    #[test]
    fn acosh_asinh_directed_match_oracle_near_tie() {
        use core::str::FromStr;
        let x = acosh_near_tie_arg();
        let asinh_arg = FBig::<mode::HalfEven, 10>::from_str(
            "0.0041493392794990204344718343065778753953054547309875500000000000000000000000000000000000000000000000000000001179",
        )
        .unwrap();
        for p in [16usize, 34, 50] {
            let oracle_acosh = Context::<mode::HalfEven>::new(p + 60)
                .acosh::<10>(x.repr(), None)
                .unwrap()
                .value();
            let oracle_asinh = Context::<mode::HalfEven>::new(p + 60)
                .asinh::<10>(asinh_arg.repr(), None)
                .unwrap()
                .value();
            macro_rules! check {
                ($mode:ty, $name:expr) => {{
                    let want_acosh = Context::<$mode>::new(p)
                        .repr_round_ref(&oracle_acosh.repr)
                        .value();
                    let got_acosh = Context::<$mode>::new(p)
                        .acosh::<10>(x.repr(), None)
                        .unwrap()
                        .value();
                    assert_eq!(got_acosh.repr, want_acosh, "{} acosh p={p}", $name);
                    let want_asinh = Context::<$mode>::new(p)
                        .repr_round_ref(&oracle_asinh.repr)
                        .value();
                    let got_asinh = Context::<$mode>::new(p)
                        .asinh::<10>(asinh_arg.repr(), None)
                        .unwrap()
                        .value();
                    assert_eq!(got_asinh.repr, want_asinh, "{} asinh p={p}", $name);
                }};
            }
            check!(mode::Down, "Down");
            check!(mode::Up, "Up");
            check!(mode::Zero, "Zero");
            check!(mode::HalfEven, "HalfEven");
        }
    }

    /// Long-digit inputs through every hyperbolic inverse: the result must match a
    /// high-precision oracle under each rounding mode (the input's rounding is folded into the
    /// radius on every path — sinh/cosh/tanh through `exp_compute`, asinh/acosh/atanh through
    /// the ball compositions).
    #[test]
    fn hyper_directed_match_oracle_long_input() {
        use core::str::FromStr;
        let args = [
            "1.000008608520579272799119633794615751764072075741189452445358800365413956109281413683821004335915587724859246904",
            "12.345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456",
            "-0.8608520579272799119633794615751764072075741189452445358800365413956109281413683821004335915587724859246904",
        ];
        for a in args {
            let x = FBig::<mode::HalfEven, 10>::from_str(a).unwrap();
            let p = 34;
            let ctx_h = Context::<mode::HalfEven>::new(p + 60);
            let oracle_asinh = ctx_h.asinh::<10>(x.repr(), None).unwrap().value().repr;
            let oracle_tanh = ctx_h.tanh::<10>(x.repr(), None).unwrap().value().repr;
            let oracle_sinh = ctx_h.sinh::<10>(x.repr(), None).unwrap().value().repr;
            let oracle_cosh = ctx_h.cosh::<10>(x.repr(), None).unwrap().value().repr;
            let oracle_atanh = if x.abs_cmp(&FBig::ONE).is_lt() {
                Some(ctx_h.atanh::<10>(x.repr(), None).unwrap().value().repr)
            } else {
                None
            };
            let oracle_acosh =
                if !x.repr().significand().is_zero() && x.repr().sign() == Sign::Positive {
                    Some(ctx_h.acosh::<10>(x.repr(), None).unwrap().value().repr)
                } else {
                    None
                };
            macro_rules! check {
                ($mode:ty, $name:expr, $f:ident, $oracle:expr) => {{
                    let want = Context::<$mode>::new(p).repr_round_ref(&$oracle).value();
                    let got = Context::<$mode>::new(p)
                        .$f::<10>(x.repr(), None)
                        .unwrap()
                        .value();
                    assert_eq!(got.repr, want, "{} {} p={p} x={a}", $name, stringify!($f));
                }};
            }
            check!(mode::Down, "Down", asinh, oracle_asinh);
            check!(mode::Up, "Up", asinh, oracle_asinh);
            check!(mode::Down, "Down", tanh, oracle_tanh);
            check!(mode::Up, "Up", tanh, oracle_tanh);
            check!(mode::Down, "Down", sinh, oracle_sinh);
            check!(mode::Up, "Up", sinh, oracle_sinh);
            check!(mode::Down, "Down", cosh, oracle_cosh);
            check!(mode::Up, "Up", cosh, oracle_cosh);
            if let Some(o) = &oracle_atanh {
                check!(mode::Down, "Down", atanh, *o);
                check!(mode::Up, "Up", atanh, *o);
            }
            if let Some(o) = &oracle_acosh {
                check!(mode::Down, "Down", acosh, *o);
                check!(mode::Up, "Up", acosh, *o);
            }
        }
    }
}
