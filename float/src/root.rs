use dashu_base::{Approximation, CubicRoot, Sign, SquareRoot, SquareRootRem, UnsignedAbs};
use dashu_int::{IBig, UBig};

use crate::{
    error::{assert_limited_precision, panic_root_zeroth, FpError, FpResult},
    fbig::FBig,
    repr::{Context, Repr, Word},
    round::Round,
    utils::{shl_digits, split_digits_ref},
};

impl<R: Round, const B: Word> SquareRoot for FBig<R, B> {
    type Output = Self;
    #[inline]
    fn sqrt(&self) -> Self {
        self.context.unwrap_fp(self.context.sqrt(self.repr()))
    }
}

impl<R: Round, const B: Word> CubicRoot for FBig<R, B> {
    type Output = Self;
    #[inline]
    fn cbrt(&self) -> Self {
        self.context.unwrap_fp(self.context.cbrt(self.repr()))
    }
}

impl<R: Round, const B: Word> FBig<R, B> {
    /// Calculate the square root of the floating point number.
    ///
    /// # Panics
    ///
    /// Panics if the precision is unlimited.
    #[inline]
    pub fn sqrt(&self) -> Self {
        self.context.unwrap_fp(self.context.sqrt(&self.repr))
    }

    /// Calculate the nth root of the floating point number.
    ///
    /// When `n` is large the computation can be expensive — the significand is
    /// padded to `n · precision` digits before the integer root is taken, and
    /// the integer Newton iteration works with numbers of that size. For large
    /// `n` consider [`powf`][`FBig::powf`] with a rational exponent `1 / n`
    /// as a faster approximate alternative.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str("16")?;
    /// assert_eq!(a.nth_root(4), DBig::from_str("2")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if `n` is zero, or if `n` is even and the number is negative.
    #[inline]
    pub fn nth_root(&self, n: usize) -> Self {
        self.context
            .unwrap_fp(self.context.nth_root(n, self.repr()))
    }
}

impl<R: Round> Context<R> {
    /// Calculate the square root of the floating point number.
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
    /// let a = DBig::from_str("1.23")?;
    /// assert_eq!(context.sqrt(&a.repr()), Ok(Inexact(DBig::from_str("1.1")?, NoOp)));
    /// # Ok::<(), ParseError>(())
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the precision is unlimited.
    pub fn sqrt<const B: Word>(&self, x: &Repr<B>) -> FpResult<FBig<R, B>> {
        if x.is_infinite() {
            return Err(FpError::InfiniteInput);
        }
        assert_limited_precision(self.precision);
        if x.significand.is_zero() {
            // sqrt(+0) = +0, sqrt(-0) = -0 (preserve the sign of zero)
            return Ok(Approximation::Exact(FBig::new(x.clone(), *self)));
        }
        if x.sign() == Sign::Negative {
            return Err(FpError::OutOfDomain);
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
                    .then_with(|| (low * 4u8).cmp(&Repr::<B>::BASE.pow(low_digits).into()))
            });
            Approximation::Inexact(root + adjust, adjust)
        };
        Ok(res
            .map(|signif| Repr::new(signif, exp))
            .and_then(|v| self.repr_round(v))
            .map(|v| FBig::new(v, *self)))
    }

    /// Calculate the cubic root of the floating point number.
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
    /// let a = DBig::from_str("8")?;
    /// assert_eq!(context.cbrt(&a.repr()), Ok(Exact(DBig::from_str("2")?)));
    /// # Ok::<(), ParseError>(())
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the precision is unlimited.
    #[inline]
    pub fn cbrt<const B: Word>(&self, x: &Repr<B>) -> FpResult<FBig<R, B>> {
        self.nth_root(3, x)
    }

    /// Calculate the nth root of the floating point number.
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
    /// let a = DBig::from_str("27")?;
    /// assert_eq!(context.nth_root(3, &a.repr()), Ok(Exact(DBig::from_str("3")?)));
    /// # Ok::<(), ParseError>(())
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if `n` is zero, if the precision is unlimited, or if `n` is even and `x` is negative.
    pub fn nth_root<const B: Word>(&self, n: usize, x: &Repr<B>) -> FpResult<FBig<R, B>> {
        if x.is_infinite() {
            return Err(FpError::InfiniteInput);
        }
        assert_limited_precision(self.precision);
        if n == 0 {
            panic_root_zeroth()
        }
        debug_assert!(n < isize::MAX as usize);
        let sign = x.sign();
        if sign == Sign::Negative && n % 2 == 0 {
            return Err(FpError::OutOfDomain);
        }
        if n == 1 {
            return Ok(self.repr_round_ref(x).map(|v| FBig::new(v, *self)));
        }
        if x.significand.is_zero() {
            // UBig::ZERO.nth_root(n) erroneously returns ONE, so short-circuit here.
            // An even root of -0 already errored above, so reaching here the sign is
            // preserved: odd root of ±0 is ±0.
            return Ok(Approximation::Exact(FBig::new(x.clone(), *self)));
        }

        // operate on the magnitude so that shifting/splitting keep a clean sign;
        // the original sign is re-applied to the result at the end.
        let xmag: IBig = if sign == Sign::Negative {
            -&x.significand
        } else {
            x.significand.clone()
        };

        // adjust the significand so that the exponent is divisible by n and the
        // significand carries at least n*precision digits (required for rounding)
        let digits = x.digits() as isize;
        let r = (x.exponent + digits).rem_euclid(n as isize);
        let shift = n as isize * self.precision as isize - digits + r;
        let (signif, low, low_digits) = if shift > 0 {
            (shl_digits::<B>(&xmag, shift as usize), IBig::ZERO, 0)
        } else {
            let shift = (-shift) as usize;
            let (hi, lo) = split_digits_ref::<B>(&xmag, shift);
            (hi, lo, shift)
        };

        let mag: UBig = signif.unsigned_abs();
        let root: UBig = mag.nth_root(n);
        let rem: UBig = &mag - root.clone().pow(n);
        let exp = (x.exponent - shift) / n as isize;

        let result_sign = if sign == Sign::Negative {
            Sign::Negative
        } else {
            Sign::Positive
        };
        let signed_root: IBig = result_sign * root.clone();

        let res = if rem.is_zero() && low.is_zero() {
            Approximation::Exact(signed_root)
        } else {
            let adjust = R::round_low_part(&signed_root, result_sign, || {
                // The true value is (mag + low / BASE^low_digits)^(1/n) and
                // root = floor(mag^(1/n)); its fractional part is compared to 1/2.
                // frac < 1/2  <=>  2^n * full < (2*root + 1)^n * BASE^low_digits,
                // where full = mag * BASE^low_digits + low (the full significand).
                let base_pow = Repr::<B>::BASE.pow(low_digits);
                let full = &mag * &base_pow + low.unsigned_abs();
                let lhs = full << n;
                let rhs = ((root.clone() << 1) + UBig::from_word(1)).pow(n) * base_pow;
                lhs.cmp(&rhs)
            });
            Approximation::Inexact(signed_root.clone() + adjust, adjust)
        };
        Ok(res
            .map(|signif| Repr::new(signif, exp))
            .and_then(|v| self.repr_round(v))
            .map(|v| FBig::new(v, *self)))
    }
}


impl<R: Round> Context<R> {
    /// Compute `sqrt(a² + b²)` without spurious overflow/underflow.
    ///
    /// This is the overflow-safe scaled sum-of-squares: the larger-magnitude operand is never
    /// squared. Writing `m = max(|a|, |b|)` and `r = min(|a|,|b|) / m` (so `|r| ≤ 1`), the result is
    /// `m · sqrt(1 + r²)`, where `1 + r² ∈ [1, 2]` cannot overflow. The final `m · sqrt(1 + r²)`
    /// overflows only when the true result genuinely exceeds the exponent range (reported as
    /// [`FpError::Overflow`]). `hypot(±inf, ·) = +inf`, `hypot(0, 0) = +0`.
    ///
    /// This is a field-arithmetic-class op (no constant cache), like `sqrt`/`atan2`.
    ///
    /// # Panics
    ///
    /// Panics if the precision is unlimited.
    pub fn hypot<const B: Word>(&self, a: &Repr<B>, b: &Repr<B>) -> FpResult<FBig<R, B>> {
        if a.is_infinite() || b.is_infinite() {
            return Ok(Approximation::Exact(FBig::new(Repr::infinity(), *self)));
        }
        assert_limited_precision(self.precision);
        if a.significand.is_zero() && b.significand.is_zero() {
            return Ok(Approximation::Exact(FBig::new(Repr::zero(), *self)));
        }

        let guard = crate::utils::ceil_usize(<usize as dashu_base::EstimatedLog2>::log2_est(
            &self.precision,
        )) + 10;
        let gctx = Context::<R>::new(self.precision + guard);

        // magnitudes, ordered large >= small (both finite, not both zero here)
        let a_mag = if a.sign() == Sign::Negative { -a.clone() } else { a.clone() };
        let b_mag = if b.sign() == Sign::Negative { -b.clone() } else { b.clone() };
        let (large, small) = if a_mag.cmp(&b_mag).is_ge() {
            (a_mag, b_mag)
        } else {
            (b_mag, a_mag)
        };

        if small.significand.is_zero() {
            // hypot(x, 0) = |x|; `large` is already a magnitude
            return Ok(gctx.repr_round_ref(&large).map(|v| FBig::new(v, *self)));
        }

        // r = small / large ∈ [0, 1]; 1 + r² ∈ [1, 2] (no overflow); result = large · sqrt(1+r²)
        let r = gctx.div(&small, &large)?.value();
        let r2 = gctx.sqr(r.repr())?.value();
        let sum = gctx.add(&Repr::one(), r2.repr())?.value();
        let root = gctx.sqrt(sum.repr())?.value();
        let result = gctx.mul(&large, root.repr())?.value();
        Ok(result.with_precision(self.precision))
    }
}

impl<R: Round, const B: Word> FBig<R, B> {
    /// Compute `sqrt(self² + other²)` without spurious overflow/underflow.
    ///
    /// The result precision is `max(self.precision(), other.precision())`. See
    /// [`Context::hypot`] for the overflow-safety strategy.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str("3")?;
    /// let b = DBig::from_str("4")?;
    /// assert_eq!(a.hypot(&b), DBig::from_str("5")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the precision is unlimited.
    #[inline]
    pub fn hypot(&self, other: &Self) -> Self {
        let context = Context::max(self.context, other.context);
        context.unwrap_fp(context.hypot(&self.repr, &other.repr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::round::mode;

    #[test]
    #[should_panic]
    fn test_fbig_sqrt_negative_panics() {
        // sqrt(-1) is out of domain; the FBig layer panics.
        let neg_one = FBig::<mode::HalfEven>::try_from(-1.0f64).unwrap();
        let _ = neg_one.sqrt();
    }

    #[test]
    fn test_hypot_pythagorean() {
        let ctx = Context::<mode::HalfEven>::new(53);
        let mk = |v: i32| Repr::<2>::new(v.into(), 0);
        // hypot(3, 4) = 5
        let r = ctx.hypot(&mk(3), &mk(4)).unwrap().value();
        assert_eq!(r.repr().significand(), &5.into());
        // hypot(5, 0) = 5
        let r = ctx.hypot(&mk(5), &mk(0)).unwrap().value();
        assert_eq!(r.repr().significand(), &5.into());
        // hypot(0, 0) = 0
        let r = ctx.hypot(&mk(0), &mk(0)).unwrap().value();
        assert!(r.repr().is_zero());
        // hypot(inf, x) = +inf
        let r = ctx.hypot(&Repr::infinity(), &mk(3)).unwrap().value();
        assert!(r.repr().is_infinite());
        assert_eq!(r.repr().sign(), Sign::Positive);
    }

    #[test]
    fn test_hypot_no_spurious_overflow() {
        // a value whose square would collide with the +inf sentinel exponent, but whose
        // hypot is itself representable: hypot(a, 0) = |a| must not overflow via a².
        let ctx = Context::<mode::HalfEven>::new(53);
        // exponent near isize::MAX/2 so that a² would overflow, but |a| is fine
        let a = Repr::<2>::new(IBig::from(3), isize::MAX / 2);
        let r = ctx.hypot(&a, &Repr::<2>::zero()).unwrap().value();
        assert_eq!(r.repr().exponent(), isize::MAX / 2);
    }
}
