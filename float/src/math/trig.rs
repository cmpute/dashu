use crate::{
    error::{assert_limited_precision, FpError},
    fbig::FBig,
    math::{
        cache::{reborrow_cache, ConstCache},
        FpResult,
    },
    repr::{Context, Repr, Word},
    round::{Round, Rounded},
};
use core::cmp::Ordering;
use core::convert::TryFrom;
use dashu_base::{AbsOrd, Approximation::Exact, RemEuclid, Sign};
use dashu_int::IBig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Quadrant {
    First,
    Second,
    Third,
    Fourth,
}

/// Build a `Normal` result equal to `±0`, preserving the sign of `x` (used by `sin`/`tan`/`sin_cos`
/// at zero input, where `sin(-0) = -0` and `tan(-0) = -0`).
fn signed_zero_normal<R: Round, const B: Word>(
    ctx: &Context<R>,
    x: &Repr<B>,
) -> FpResult<FBig<R, B>> {
    let zero = if x.is_neg_zero() {
        Repr::neg_zero()
    } else {
        Repr::zero()
    };
    Ok(Exact(FBig::<R, B>::new(zero, *ctx)))
}

impl<R: Round> Context<R> {
    /// Calculate the internal work context for trigonometric functions based on input magnitude.
    ///
    /// This ensures we have enough guard digits to prevent catastrophic cancellation
    /// during range reduction for large inputs.
    fn compute_work_context_trig<const B: Word>(self, x: &Repr<B>) -> Self {
        // x_mag estimates m = floor(log_BASE(|x|))
        let x_mag = (x.exponent.saturating_add(x.digits_ub() as isize)).max(0) as usize;

        // We need precision + log10(x) digits to maintain 'precision' digits after reduction.
        // We add a base of 50 guard digits, plus 10% of x_mag for very large arguments
        // to account for cumulative errors in division and multiplication during reduction.
        let extra_guards = 50 + x_mag / 10;
        let work_precision = self
            .precision
            .saturating_add(x_mag)
            .saturating_add(extra_guards);
        Self::new(work_precision)
    }

    /// Reduces the argument to the first quadrant for trigonometric evaluation.
    /// Returns the internal work context, the reduced argument `r`, and the quadrant `k % 4`.
    fn reduce_to_quadrant<const B: Word>(
        self,
        x: &Repr<B>,
        mut cache: Option<&mut ConstCache>,
    ) -> (Self, FBig<R, B>, Quadrant) {
        let work_context = self.compute_work_context_trig(x);
        let x_f = FBig::<R, B>::new(work_context.repr_round(x.clone()).value(), work_context);

        let pi = work_context.pi::<B>(reborrow_cache(&mut cache)).value();
        let half_pi = &pi / 2;
        let x_scaled: FBig<R, B> = &x_f / &half_pi;
        let k_f = x_scaled.round();
        let r = x_f - &k_f * half_pi;
        // `k_f` is the integer nearest `x_scaled`, so it's exact (or a signed zero
        // for a tiny argument in (-1, 0), which `IBig::try_from` treats as plain 0).
        let k = IBig::try_from(k_f).expect("k_f is an exact integer or signed zero");

        let k_mod_4_big = k.rem_euclid(IBig::from(4));
        let Ok(k_mod_4_int) = i8::try_from(k_mod_4_big) else {
            unreachable!("k % 4 is always in [0, 3]");
        };
        let quadrant = match k_mod_4_int {
            0 => Quadrant::First,
            1 => Quadrant::Second,
            2 => Quadrant::Third,
            3 => Quadrant::Fourth,
            _ => unreachable!(),
        };

        (work_context, r, quadrant)
    }

    /// Calculate the sine of the floating point representation.
    pub fn sin<const B: Word>(
        &self,
        x: &Repr<B>,
        mut cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        if x.is_infinite() {
            return Err(FpError::InfiniteInput);
        }
        assert_limited_precision(self.precision);

        if x.significand.is_zero() {
            // sin(±0) = ±0
            return signed_zero_normal(self, x);
        }

        let (work_context, r, quadrant) = self.reduce_to_quadrant(x, reborrow_cache(&mut cache));

        // 3. Evaluate the reduced series based on the quadrant
        let res = match quadrant {
            Quadrant::First => work_context.sin_internal(&r),
            Quadrant::Second => work_context.cos_internal(&r),
            Quadrant::Third => -work_context.sin_internal(&r),
            Quadrant::Fourth => -work_context.cos_internal(&r),
        };
        Ok(res.with_precision(self.precision))
    }

    /// Internal Taylor series for sine: S(x) = x - x^3/3! + x^5/5! - ...
    fn sin_internal<const B: Word>(self, x: &FBig<R, B>) -> FBig<R, B> {
        if x.repr.significand.is_zero() {
            return FBig::ZERO;
        }
        let x2 = x.sqr();
        let mut sum = x.clone();
        let mut term = x.clone();
        let mut k = 1usize;
        let threshold = sum.sub_ulp();
        loop {
            term *= &x2;
            term /= (2 * k) * (2 * k + 1);
            if term.abs_cmp(&threshold).is_le() {
                break;
            }
            if k % 2 == 1 {
                sum -= &term;
            } else {
                sum += &term;
            }
            k += 1;
        }
        sum
    }

    /// Calculate the cosine of the floating point representation.
    pub fn cos<const B: Word>(
        &self,
        x: &Repr<B>,
        mut cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        if x.is_infinite() {
            return Err(FpError::InfiniteInput);
        }
        assert_limited_precision(self.precision);

        if x.significand.is_zero() {
            // cos(±0) = 1
            return Ok(FBig::<R, B>::ONE.with_precision(self.precision));
        }

        let (work_context, r, quadrant) = self.reduce_to_quadrant(x, reborrow_cache(&mut cache));

        // 3. Evaluate the reduced series based on the quadrant
        let res = match quadrant {
            Quadrant::First => work_context.cos_internal(&r),
            Quadrant::Second => -work_context.sin_internal(&r),
            Quadrant::Third => -work_context.cos_internal(&r),
            Quadrant::Fourth => work_context.sin_internal(&r),
        };
        Ok(res.with_precision(self.precision))
    }

    /// Internal Taylor series for cosine: C(x) = 1 - x^2/2! + x^4/4! - ...
    fn cos_internal<const B: Word>(self, x: &FBig<R, B>) -> FBig<R, B> {
        if x.repr.significand.is_zero() {
            return FBig::ONE.with_precision(self.precision).value();
        }
        let x2 = x.sqr();
        let mut sum = FBig::<R, B>::ONE.with_precision(self.precision).value();
        let mut term = sum.clone();
        let mut k = 1usize;
        let threshold = sum.sub_ulp();
        loop {
            term *= &x2;
            term /= (2 * k) * (2 * k - 1);
            if term.abs_cmp(&threshold).is_le() {
                break;
            }
            if k % 2 == 1 {
                sum -= &term;
            } else {
                sum += &term;
            }
            k += 1;
        }
        sum
    }

    /// Calculate both the sine and cosine of the floating point representation.
    ///
    /// This is more efficient than calling `sin` and `cos` separately.
    pub fn sin_cos<const B: Word>(
        &self,
        x: &Repr<B>,
        mut cache: Option<&mut ConstCache>,
    ) -> (FpResult<FBig<R, B>>, FpResult<FBig<R, B>>) {
        if x.is_infinite() {
            return (Err(FpError::InfiniteInput), Err(FpError::InfiniteInput));
        }
        assert_limited_precision(self.precision);

        if x.significand.is_zero() {
            // sin(±0) = ±0, cos(±0) = 1
            let s = signed_zero_normal(self, x);
            let c = Ok(FBig::<R, B>::ONE.with_precision(self.precision));
            return (s, c);
        }

        let (work_context, r, quadrant) = self.reduce_to_quadrant(x, reborrow_cache(&mut cache));

        let (sin_r, cos_r) = work_context.sin_cos_internal(&r);

        let (s, c) = match quadrant {
            Quadrant::First => (sin_r, cos_r),
            Quadrant::Second => (cos_r, -sin_r),
            Quadrant::Third => (-sin_r, -cos_r),
            Quadrant::Fourth => (-cos_r, sin_r),
        };

        (Ok(s.with_precision(self.precision)), Ok(c.with_precision(self.precision)))
    }

    /// Simultaneously evaluate Taylor series for sine and cosine.
    pub(crate) fn sin_cos_internal<const B: Word>(
        self,
        x: &FBig<R, B>,
    ) -> (FBig<R, B>, FBig<R, B>) {
        if x.repr.significand.is_zero() {
            return (FBig::ZERO, FBig::ONE.with_precision(self.precision).value());
        }
        let x2 = x.sqr();
        let mut sin_sum = x.clone();
        let mut cos_sum = FBig::<R, B>::ONE.with_precision(self.precision).value();
        let mut sin_term = x.clone();
        let mut cos_term = cos_sum.clone();
        let mut k = 1usize;
        let sin_threshold = sin_sum.sub_ulp();
        let cos_threshold = cos_sum.sub_ulp();
        loop {
            cos_term *= &x2;
            cos_term /= (2 * k) * (2 * k - 1);
            sin_term *= &x2;
            sin_term /= (2 * k) * (2 * k + 1);

            if sin_term.abs_cmp(&sin_threshold).is_le() && cos_term.abs_cmp(&cos_threshold).is_le()
            {
                break;
            }

            if k % 2 == 1 {
                cos_sum -= &cos_term;
                sin_sum -= &sin_term;
            } else {
                cos_sum += &cos_term;
                sin_sum += &sin_term;
            }
            k += 1;
        }
        (sin_sum, cos_sum)
    }

    /// Calculate the tangent of the floating point representation.
    ///
    /// # Note
    /// Near odd multiples of π/2, the result is an infinity (returned as a value, not an error).
    pub fn tan<const B: Word>(
        &self,
        x: &Repr<B>,
        mut cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        if x.is_infinite() {
            return Err(FpError::InfiniteInput);
        }
        assert_limited_precision(self.precision);

        if x.significand.is_zero() {
            // tan(±0) = ±0
            return signed_zero_normal(self, x);
        }

        let (work_context, r, quadrant) = self.reduce_to_quadrant(x, reborrow_cache(&mut cache));
        let (sin_r, cos_r) = work_context.sin_cos_internal(&r);

        let (s_f, c_f) = match quadrant {
            Quadrant::First => (sin_r, cos_r),
            Quadrant::Second => (cos_r, -sin_r),
            Quadrant::Third => (-sin_r, -cos_r),
            Quadrant::Fourth => (-cos_r, sin_r),
        };

        if c_f.repr.is_zero() {
            // tan hits a pole: the result is an infinity with the sign of the numerator.
            let inf = if s_f.sign() == Sign::Negative {
                Repr::neg_infinity()
            } else {
                Repr::infinity()
            };
            return Ok(Rounded::Exact(FBig::new(inf, *self)));
        }
        self.div(&s_f.repr, &c_f.repr)
            .map(|r| r.and_then(|f| f.with_precision(self.precision)))
    }

    /// Calculate the arcsine of the floating point representation.
    ///
    /// # Methodology
    /// Uses the identity: `asin(x) = atan(x / sqrt(1 - x^2))`
    /// Returns `Err(OutOfDomain)` if `|x| > 1`.
    pub fn asin<const B: Word>(
        &self,
        x: &Repr<B>,
        mut cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        if x.is_infinite() {
            return Err(FpError::InfiniteInput);
        }
        assert_limited_precision(self.precision);

        let x_orig = FBig::<R, B>::new(x.clone(), *self);
        // Domain check: |x| must be <= 1
        if x_orig.abs_cmp(&FBig::ONE).is_gt() {
            return Err(FpError::OutOfDomain);
        }

        let guard_digits = 50;
        let work_precision = self.precision + guard_digits;
        let work_context = Self::new(work_precision);

        let x_f = FBig::<R, B>::new(work_context.repr_round(x.clone()).value(), work_context);

        let res = work_context.asin_internal(&x_f, reborrow_cache(&mut cache));
        Ok(res.with_precision(self.precision))
    }

    fn asin_internal<const B: Word>(
        self,
        x_f: &FBig<R, B>,
        mut cache: Option<&mut ConstCache>,
    ) -> FBig<R, B> {
        let one = FBig::<R, B>::ONE.with_precision(self.precision).value();
        let x2 = x_f.sqr();
        let d = self.unwrap_fp(self.sqrt(&(one - x2).repr));

        if d.repr.is_zero() {
            let pi = self.pi::<B>(reborrow_cache(&mut cache)).value();
            let half_pi: FBig<R, B> = pi / 2;
            if x_f.sign() == Sign::Positive {
                return half_pi;
            }
            return -half_pi;
        }

        self.atan_with_reduction(&(x_f / d), reborrow_cache(&mut cache))
    }

    /// Calculate the arccosine of the floating point representation.
    ///
    /// # Methodology
    /// Uses the identity: `acos(x) = pi/2 - asin(x)`.
    /// Higher precision is used internally to avoid catastrophic cancellation near x ≈ 1.
    pub fn acos<const B: Word>(
        &self,
        x: &Repr<B>,
        mut cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        if x.is_infinite() {
            return Err(FpError::InfiniteInput);
        }
        assert_limited_precision(self.precision);

        let x_orig = FBig::<R, B>::new(x.clone(), *self);
        // Domain check: |x| must be <= 1
        if x_orig.abs_cmp(&FBig::ONE).is_gt() {
            return Err(FpError::OutOfDomain);
        }

        let guard_digits = 50;
        let work_precision = self.precision + guard_digits;
        let work_context = Self::new(work_precision);

        let x_f = FBig::<R, B>::new(work_context.repr_round(x.clone()).value(), work_context);

        let asin_x = work_context.asin_internal(&x_f, reborrow_cache(&mut cache));
        let pi = work_context.pi::<B>(reborrow_cache(&mut cache)).value();
        let half_pi: FBig<R, B> = pi / 2;
        let res: FBig<R, B> = half_pi - asin_x;
        Ok(res.with_precision(self.precision))
    }

    /// Calculate the arctangent of the floating point representation.
    pub fn atan<const B: Word>(
        &self,
        x: &Repr<B>,
        mut cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        if x.is_infinite() {
            // atan(±inf) = ±π/2 — preserved (a well-defined finite result for an infinite input)
            let pi = self.pi::<B>(reborrow_cache(&mut cache)).value();
            let half_pi: FBig<R, B> = pi / 2;
            let res: FBig<R, B> = if x.sign() == Sign::Positive {
                half_pi
            } else {
                -half_pi
            };
            return Ok(res.with_precision(self.precision));
        }

        assert_limited_precision(self.precision);

        if x.significand.is_zero() {
            // atan(±0) = ±0
            return signed_zero_normal(self, x);
        }

        let guard_digits = 50;
        let work_precision = self.precision + guard_digits;
        let work_context = Self::new(work_precision);

        let x_f = FBig::<R, B>::new(work_context.repr_round(x.clone()).value(), work_context);
        let res = work_context.atan_with_reduction(&x_f, reborrow_cache(&mut cache));
        Ok(res.with_precision(self.precision))
    }

    /// Internal arctangent that includes range reduction but no guard digit allocation.
    fn atan_with_reduction<const B: Word>(
        self,
        x_f: &FBig<R, B>,
        mut cache: Option<&mut ConstCache>,
    ) -> FBig<R, B> {
        let sign = x_f.sign();
        let mut x_abs = x_f.clone();
        if sign == Sign::Negative {
            x_abs = -x_abs;
        }
        let mut res = if x_abs >= FBig::<R, B>::ONE.with_precision(self.precision).value() {
            let pi = self.pi::<B>(reborrow_cache(&mut cache)).value();
            let inv_x = FBig::<R, B>::ONE.with_precision(self.precision).value() / x_abs;
            (pi / 2) - self.atan_internal(&inv_x)
        } else {
            self.atan_internal(&x_abs)
        };
        if sign == Sign::Negative {
            res = -res;
        }
        res
    }

    /// Internal series for arctangent.
    /// Evaluates the Euler series for arctangent.
    fn atan_internal<const B: Word>(self, x: &FBig<R, B>) -> FBig<R, B> {
        // Euler's series for atan(x)
        let x2 = x.sqr();
        let one_plus_x2 = FBig::ONE + &x2;
        let mut term = x / &one_plus_x2;
        let mut sum = term.clone();
        let factor = (2 * &x2) / one_plus_x2;
        let mut n = 1usize;
        let threshold = sum.sub_ulp();
        loop {
            term *= &factor;
            term *= n;
            term /= 2 * n + 1;
            if term.abs_cmp(&threshold).is_le() {
                break;
            }
            sum += &term;
            n += 1;
        }
        sum
    }

    /// Calculate the arctangent of y / x.
    ///
    /// Handles signed infinities according to IEEE 754 standards.
    /// Returns `Err(OutOfDomain)` if both arguments are zero.
    pub fn atan2<const B: Word>(
        &self,
        y: &Repr<B>,
        x: &Repr<B>,
        mut cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        if y.is_finite() && x.is_finite() && y.significand.is_zero() && x.significand.is_zero() {
            return Err(FpError::OutOfDomain);
        }

        assert_limited_precision(self.precision);

        let guard_digits = 50;
        let work_precision = self.precision + guard_digits;
        let work_context = Self::new(work_precision);

        // Handle Infinities according to IEEE 754
        if y.is_infinite() || x.is_infinite() {
            let (sy, sx) = (y.sign() == Sign::Positive, x.sign() == Sign::Positive);
            let res: FBig<R, B> = match (y.is_infinite(), x.is_infinite(), sy, sx) {
                (true, true, true, true) => {
                    work_context.pi::<B>(reborrow_cache(&mut cache)).value() / 4
                }
                (true, true, true, false) => {
                    work_context.pi::<B>(reborrow_cache(&mut cache)).value() * 3 / 4
                }
                (true, true, false, true) => {
                    let pi4: FBig<R, B> =
                        work_context.pi::<B>(reborrow_cache(&mut cache)).value() / 4;
                    -pi4
                }
                (true, true, false, false) => {
                    let pi34: FBig<R, B> =
                        work_context.pi::<B>(reborrow_cache(&mut cache)).value() * 3 / 4;
                    -pi34
                }
                (true, false, true, _) => {
                    work_context.pi::<B>(reborrow_cache(&mut cache)).value() / 2
                }
                (true, false, false, _) => {
                    let half_pi: FBig<R, B> =
                        work_context.pi::<B>(reborrow_cache(&mut cache)).value() / 2;
                    -half_pi
                }
                (false, true, _, true) => {
                    // atan2(±finite, +inf) = ±0 (signed zero of y)
                    if sy {
                        FBig::<R, B>::ZERO.with_precision(work_precision).value()
                    } else {
                        FBig::<R, B>::new(Repr::neg_zero(), work_context)
                            .with_precision(work_precision)
                            .value()
                    }
                }
                (false, true, true, false) => {
                    work_context.pi::<B>(reborrow_cache(&mut cache)).value()
                }
                (false, true, false, false) => {
                    -work_context.pi::<B>(reborrow_cache(&mut cache)).value()
                }
                _ => unreachable!(),
            };
            return Ok(res.with_precision(self.precision));
        }

        let y_f = FBig::<R, B>::new(work_context.repr_round(y.clone()).value(), work_context);
        let x_f = FBig::<R, B>::new(work_context.repr_round(x.clone()).value(), work_context);

        match x_f.cmp(&FBig::<R, B>::ZERO) {
            Ordering::Greater => {
                let res =
                    work_context.atan_with_reduction(&(y_f / x_f), reborrow_cache(&mut cache));
                Ok(res.with_precision(self.precision))
            }
            Ordering::Less => {
                let pi = work_context.pi::<B>(reborrow_cache(&mut cache)).value();
                let y_sign = y_f.sign();
                let atan_yx =
                    work_context.atan_with_reduction(&(y_f / x_f), reborrow_cache(&mut cache));
                let res = if y_sign == Sign::Positive {
                    atan_yx + pi
                } else {
                    atan_yx - pi
                };
                Ok(res.with_precision(self.precision))
            }
            Ordering::Equal => {
                // x == 0 case
                let pi = work_context.pi::<B>(reborrow_cache(&mut cache)).value();
                let half_pi: FBig<R, B> = pi / 2;
                if y_f > FBig::<R, B>::ZERO {
                    Ok(half_pi.with_precision(self.precision))
                } else {
                    let res = -half_pi;
                    Ok(res.with_precision(self.precision))
                }
            }
        }
    }
}

impl<R: Round, const B: Word> FBig<R, B> {
    /// Calculate the sine of the floating point number.
    ///
    /// # Panics
    /// Panics if the input is infinite.
    #[inline]
    pub fn sin(&self) -> Self {
        self.context.unwrap_fp(self.context.sin(&self.repr, None))
    }

    /// Calculate the cosine of the floating point number.
    ///
    /// # Panics
    /// Panics if the input is infinite.
    #[inline]
    pub fn cos(&self) -> Self {
        self.context.unwrap_fp(self.context.cos(&self.repr, None))
    }

    /// Calculate both the sine and cosine of the floating point number.
    ///
    /// This is more efficient than calling `sin` and `cos` separately.
    ///
    /// # Panics
    /// Panics if the input is infinite.
    #[inline]
    pub fn sin_cos(&self) -> (Self, Self) {
        let (s, c) = self.context.sin_cos(&self.repr, None);
        (self.context.unwrap_fp(s), self.context.unwrap_fp(c))
    }

    /// Calculate the tangent of the floating point number.
    ///
    /// At odd multiples of π/2 the result is an infinity (returned as a value).
    ///
    /// # Panics
    /// Panics if the input is infinite.
    #[inline]
    pub fn tan(&self) -> Self {
        self.context.unwrap_fp(self.context.tan(&self.repr, None))
    }

    /// Calculate the arcsine of the floating point number.
    ///
    /// # Panics
    /// Panics if the input is infinite or `|self| > 1` (out of domain).
    #[inline]
    pub fn asin(&self) -> Self {
        self.context.unwrap_fp(self.context.asin(&self.repr, None))
    }

    /// Calculate the arccosine of the floating point number.
    ///
    /// # Panics
    /// Panics if the input is infinite or `|self| > 1` (out of domain).
    #[inline]
    pub fn acos(&self) -> Self {
        self.context.unwrap_fp(self.context.acos(&self.repr, None))
    }

    /// Calculate the arctangent of the floating point number. `atan(±inf) = ±π/2`.
    #[inline]
    pub fn atan(&self) -> Self {
        self.context.unwrap_fp(self.context.atan(&self.repr, None))
    }

    /// Calculate the arctangent of `self / x`.
    ///
    /// # Panics
    /// Panics if both arguments are zero.
    #[inline]
    pub fn atan2(&self, x: &Self) -> Self {
        self.context
            .unwrap_fp(self.context.atan2(&self.repr, &x.repr, None))
    }
}

impl<R: Round> Context<R> {
    /// Calculate π using the Chudnovsky algorithm with binary splitting.
    ///
    /// The Chudnovsky algorithm is one of the most efficient methods for
    /// high-precision π calculation, providing ~14.18 decimal digits per term.
    ///
    /// # Methodology
    /// We use Binary Splitting to evaluate the series. This technique transforms
    /// the linear-time summation into a recursive tree evaluation. By combining
    /// terms into large products, it allows the library to leverage fast
    /// multiplication algorithms (like Toom-3 or FFT) as the numbers grow,
    /// leading to significant performance gains over simple iterative summation.
    #[must_use]
    pub fn pi<const B: Word>(&self, cache: Option<&mut ConstCache>) -> Rounded<FBig<R, B>> {
        if let Some(c) = cache {
            return c.pi::<B, R>(self.precision);
        }

        // No shared cache: compute via a one-shot ConstCache so the Chudnovsky series
        // and the 426880·√10005·Q/T finalization live in exactly one place (see
        // ConstCache::pi), instead of being duplicated here.
        let mut fresh = ConstCache::new();
        fresh.pi::<B, R>(self.precision)
    }
}

impl<R: Round, const B: Word> FBig<R, B> {
    /// Calculate π with the given precision and the default rounding mode.
    #[inline]
    #[must_use]
    pub fn pi(precision: usize) -> Self {
        Context::<R>::new(precision).pi(None).value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::round::mode;

    #[test]
    fn test_atan_infinity_is_preserved() {
        let ctx = Context::<mode::HalfEven>::new(53);
        // atan(±inf) = ±π/2 — a finite result, preserved (not an error)
        let r = ctx.atan::<2>(&Repr::<2>::infinity(), None).unwrap().value();
        assert!(r.repr().sign() == Sign::Positive);
        // it should be approximately π/2
        assert!(r > FBig::<mode::HalfEven>::ONE);
    }

    /// Regression: a tiny *negative* argument used to panic in `reduce_to_quadrant`.
    /// `round()` of a value in (-1, 0) yields signed zero (exponent sentinel -1),
    /// which `IBig::try_from` now accepts as plain 0.
    #[test]
    fn test_trig_tiny_negative_no_panic() {
        let ctx = Context::<mode::HalfAway>::new(30);
        for &e in &[-1isize, -2, -10, -30] {
            // x = -1 * BASE^e, a tiny negative value
            let x = Repr::<10>::new(IBig::from(-1), e);
            let s = ctx.sin::<10>(&x, None).unwrap().value();
            let c = ctx.cos::<10>(&x, None).unwrap().value();
            let (ss, cc) = ctx.sin_cos::<10>(&x, None);
            let ss = ss.unwrap().value();
            let cc = cc.unwrap().value();
            // sin is odd, cos is even: sin(x) ≈ x (negative), cos(x) ≈ 1
            assert_eq!(s.sign(), Sign::Negative);
            assert_eq!(c.sign(), Sign::Positive);
            assert_eq!(ss.sign(), Sign::Negative);
            assert_eq!(cc.sign(), Sign::Positive);
        }
    }
}
