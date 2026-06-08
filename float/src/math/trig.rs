use crate::{
    error::assert_limited_precision,
    fbig::FBig,
    math::FpResult,
    repr::{Context, Repr, Word},
    round::Round,
};
use core::cmp::Ordering;
use core::convert::TryFrom;
use dashu_base::{AbsOrd, RemEuclid, Sign};
use dashu_int::IBig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Quadrant {
    First,
    Second,
    Third,
    Fourth,
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
    fn reduce_to_quadrant<const B: Word>(self, x: &Repr<B>) -> (Self, FBig<R, B>, Quadrant) {
        let work_context = self.compute_work_context_trig(x);
        let x_f = FBig::<R, B>::new(work_context.repr_round(x.clone()).value(), work_context);

        let pi = work_context.pi::<B>().value();
        let half_pi = &pi / 2;
        let x_scaled: FBig<R, B> = &x_f / &half_pi;
        let k_f = x_scaled.round();
        let r = x_f - &k_f * half_pi;
        let Ok(k) = IBig::try_from(k_f) else {
            unreachable!(
                "round() always returns an integer and trig functions ensure input is finite"
            );
        };

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
    #[must_use]
    pub fn sin<const B: Word>(&self, x: &Repr<B>) -> FpResult<B> {
        if x.is_infinite() {
            return FpResult::NaN;
        }
        assert_limited_precision(self.precision);

        if x.is_zero() {
            let res = FBig::<R, B>::ZERO.with_precision(self.precision);
            return FpResult::Normal(res.map(|v| v.repr));
        }

        let (work_context, r, quadrant) = self.reduce_to_quadrant(x);

        // 3. Evaluate the reduced series based on the quadrant
        let res = match quadrant {
            Quadrant::First => work_context.sin_internal(&r),
            Quadrant::Second => work_context.cos_internal(&r),
            Quadrant::Third => -work_context.sin_internal(&r),
            Quadrant::Fourth => -work_context.cos_internal(&r),
        };
        FpResult::Normal(res.with_precision(self.precision).map(|v| v.repr))
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
    #[must_use]
    pub fn cos<const B: Word>(&self, x: &Repr<B>) -> FpResult<B> {
        if x.is_infinite() {
            return FpResult::NaN;
        }
        assert_limited_precision(self.precision);

        if x.is_zero() {
            let res = FBig::<R, B>::ONE.with_precision(self.precision);
            return FpResult::Normal(res.map(|v| v.repr));
        }

        let (work_context, r, quadrant) = self.reduce_to_quadrant(x);

        // 3. Evaluate the reduced series based on the quadrant
        let res = match quadrant {
            Quadrant::First => work_context.cos_internal(&r),
            Quadrant::Second => -work_context.sin_internal(&r),
            Quadrant::Third => -work_context.cos_internal(&r),
            Quadrant::Fourth => work_context.sin_internal(&r),
        };
        FpResult::Normal(res.with_precision(self.precision).map(|v| v.repr))
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
    #[must_use]
    pub fn sin_cos<const B: Word>(&self, x: &Repr<B>) -> (FpResult<B>, FpResult<B>) {
        if x.is_infinite() {
            return (FpResult::NaN, FpResult::NaN);
        }
        assert_limited_precision(self.precision);

        if x.is_zero() {
            let s = FBig::<R, B>::ZERO.with_precision(self.precision);
            let c = FBig::<R, B>::ONE.with_precision(self.precision);
            return (FpResult::Normal(s.map(|v| v.repr)), FpResult::Normal(c.map(|v| v.repr)));
        }

        let (work_context, r, quadrant) = self.reduce_to_quadrant(x);

        let (sin_r, cos_r) = work_context.sin_cos_internal(&r);

        let (s, c) = match quadrant {
            Quadrant::First => (sin_r, cos_r),
            Quadrant::Second => (cos_r, -sin_r),
            Quadrant::Third => (-sin_r, -cos_r),
            Quadrant::Fourth => (-cos_r, sin_r),
        };

        (
            FpResult::Normal(s.with_precision(self.precision).map(|v| v.repr)),
            FpResult::Normal(c.with_precision(self.precision).map(|v| v.repr)),
        )
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
    /// Near odd multiples of π/2, the result returns `Infinite`.
    #[must_use]
    pub fn tan<const B: Word>(&self, x: &Repr<B>) -> FpResult<B> {
        if x.is_infinite() {
            return FpResult::NaN;
        }
        assert_limited_precision(self.precision);

        if x.is_zero() {
            let res = FBig::<R, B>::ZERO.with_precision(self.precision);
            return FpResult::Normal(res.map(|v| v.repr));
        }

        let (work_context, r, quadrant) = self.reduce_to_quadrant(x);
        let (sin_r, cos_r) = work_context.sin_cos_internal(&r);

        let (s_f, c_f) = match quadrant {
            Quadrant::First => (sin_r, cos_r),
            Quadrant::Second => (cos_r, -sin_r),
            Quadrant::Third => (-sin_r, -cos_r),
            Quadrant::Fourth => (-cos_r, sin_r),
        };

        if c_f.repr.is_zero() {
            return FpResult::Infinite;
        }
        FpResult::Normal(self.div(&s_f.repr, &c_f.repr).map(|v| v.repr))
    }

    /// Calculate the arcsine of the floating point representation.
    ///
    /// # Methodology
    /// Uses the identity: `asin(x) = atan(x / sqrt(1 - x^2))`
    /// Returns `NaN` if `|x| > 1`.
    #[must_use]
    pub fn asin<const B: Word>(&self, x: &Repr<B>) -> FpResult<B> {
        if x.is_infinite() {
            return FpResult::NaN;
        }
        assert_limited_precision(self.precision);

        let x_orig = FBig::<R, B>::new(x.clone(), *self);
        // Domain check: |x| must be <= 1
        if x_orig.abs_cmp(&FBig::ONE).is_gt() {
            return FpResult::NaN;
        }

        let guard_digits = 50;
        let work_precision = self.precision + guard_digits;
        let work_context = Self::new(work_precision);

        let x_f = FBig::<R, B>::new(work_context.repr_round(x.clone()).value(), work_context);

        let res = work_context.asin_internal(&x_f);
        FpResult::Normal(res.with_precision(self.precision).map(|v| v.repr))
    }

    fn asin_internal<const B: Word>(self, x_f: &FBig<R, B>) -> FBig<R, B> {
        let one = FBig::<R, B>::ONE.with_precision(self.precision).value();
        let x2 = x_f.sqr();
        let d = self.sqrt(&(one - x2).repr).value();

        if d.repr.is_zero() {
            let pi = self.pi::<B>().value();
            let half_pi: FBig<R, B> = pi / 2;
            if x_f.sign() == Sign::Positive {
                return half_pi;
            }
            return -half_pi;
        }

        self.atan_with_reduction(&(x_f / d))
    }

    /// Calculate the arccosine of the floating point representation.
    ///
    /// # Methodology
    /// Uses the identity: `acos(x) = pi/2 - asin(x)`.
    /// Higher precision is used internally to avoid catastrophic cancellation near x ≈ 1.
    #[must_use]
    pub fn acos<const B: Word>(&self, x: &Repr<B>) -> FpResult<B> {
        if x.is_infinite() {
            return FpResult::NaN;
        }
        assert_limited_precision(self.precision);

        let x_orig = FBig::<R, B>::new(x.clone(), *self);
        // Domain check: |x| must be <= 1
        if x_orig.abs_cmp(&FBig::ONE).is_gt() {
            return FpResult::NaN;
        }

        let guard_digits = 50;
        let work_precision = self.precision + guard_digits;
        let work_context = Self::new(work_precision);

        let x_f = FBig::<R, B>::new(work_context.repr_round(x.clone()).value(), work_context);

        let asin_x = work_context.asin_internal(&x_f);
        let pi = work_context.pi::<B>().value();
        let half_pi: FBig<R, B> = pi / 2;
        let res: FBig<R, B> = half_pi - asin_x;
        FpResult::Normal(res.with_precision(self.precision).map(|v| v.repr))
    }

    /// Calculate the arctangent of the floating point representation.
    #[must_use]
    pub fn atan<const B: Word>(&self, x: &Repr<B>) -> FpResult<B> {
        if x.is_infinite() {
            let pi = self.pi::<B>().value();
            let half_pi: FBig<R, B> = pi / 2;
            let res: FBig<R, B> = if x.sign() == Sign::Positive {
                half_pi
            } else {
                -half_pi
            };
            return FpResult::Normal(res.with_precision(self.precision).map(|v| v.repr));
        }

        assert_limited_precision(self.precision);

        if x.is_zero() {
            let res = FBig::<R, B>::ZERO.with_precision(self.precision);
            return FpResult::Normal(res.map(|v| v.repr));
        }

        let guard_digits = 50;
        let work_precision = self.precision + guard_digits;
        let work_context = Self::new(work_precision);

        let x_f = FBig::<R, B>::new(work_context.repr_round(x.clone()).value(), work_context);
        let res = work_context.atan_with_reduction(&x_f);
        FpResult::Normal(res.with_precision(self.precision).map(|v| v.repr))
    }

    /// Internal arctangent that includes range reduction but no guard digit allocation.
    fn atan_with_reduction<const B: Word>(self, x_f: &FBig<R, B>) -> FBig<R, B> {
        let sign = x_f.sign();
        let mut x_abs = x_f.clone();
        if sign == Sign::Negative {
            x_abs = -x_abs;
        }
        let mut res = if x_abs >= FBig::<R, B>::ONE.with_precision(self.precision).value() {
            let pi = self.pi::<B>().value();
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
    /// Returns `NaN` if both arguments are zero.
    #[must_use]
    pub fn atan2<const B: Word>(&self, y: &Repr<B>, x: &Repr<B>) -> FpResult<B> {
        if y.is_zero() && x.is_zero() {
            return FpResult::NaN;
        }

        assert_limited_precision(self.precision);

        let guard_digits = 50;
        let work_precision = self.precision + guard_digits;
        let work_context = Self::new(work_precision);

        // Handle Infinities according to IEEE 754
        if y.is_infinite() || x.is_infinite() {
            let (sy, sx) = (y.sign() == Sign::Positive, x.sign() == Sign::Positive);
            let res: FBig<R, B> = match (y.is_infinite(), x.is_infinite(), sy, sx) {
                (true, true, true, true) => work_context.pi::<B>().value() / 4,
                (true, true, true, false) => work_context.pi::<B>().value() * 3 / 4,
                (true, true, false, true) => {
                    let pi4: FBig<R, B> = work_context.pi::<B>().value() / 4;
                    -pi4
                }
                (true, true, false, false) => {
                    let pi34: FBig<R, B> = work_context.pi::<B>().value() * 3 / 4;
                    -pi34
                }
                (true, false, true, _) => work_context.pi::<B>().value() / 2,
                (true, false, false, _) => {
                    let half_pi: FBig<R, B> = work_context.pi::<B>().value() / 2;
                    -half_pi
                }
                (false, true, _, true) => FBig::<R, B>::ZERO.with_precision(work_precision).value(),
                (false, true, true, false) => work_context.pi::<B>().value(),
                (false, true, false, false) => -work_context.pi::<B>().value(),
                _ => unreachable!(),
            };
            // Note: atan2(finite, +inf) returns unsigned ZERO. IEEE 754 requires signed zero,
            // but `Repr` does not distinguish signed zero.
            return FpResult::Normal(res.with_precision(self.precision).map(|v| v.repr));
        }

        let y_f = FBig::<R, B>::new(work_context.repr_round(y.clone()).value(), work_context);
        let x_f = FBig::<R, B>::new(work_context.repr_round(x.clone()).value(), work_context);

        match x_f.cmp(&FBig::<R, B>::ZERO) {
            Ordering::Greater => {
                let res = work_context.atan_with_reduction(&(y_f / x_f));
                FpResult::Normal(res.with_precision(self.precision).map(|v| v.repr))
            }
            Ordering::Less => {
                let pi = work_context.pi::<B>().value();
                let y_sign = y_f.sign();
                let atan_yx = work_context.atan_with_reduction(&(y_f / x_f));
                let res = if y_sign == Sign::Positive {
                    atan_yx + pi
                } else {
                    atan_yx - pi
                };
                FpResult::Normal(res.with_precision(self.precision).map(|v| v.repr))
            }
            Ordering::Equal => {
                // x == 0 case
                let pi = work_context.pi::<B>().value();
                let half_pi: FBig<R, B> = pi / 2;
                if y_f > FBig::<R, B>::ZERO {
                    FpResult::Normal(half_pi.with_precision(self.precision).map(|v| v.repr))
                } else {
                    let res = -half_pi;
                    FpResult::Normal(res.with_precision(self.precision).map(|v| v.repr))
                }
            }
        }
    }
}

impl<R: Round, const B: Word> FBig<R, B> {
    /// Calculate the sine of the floating point number.
    ///
    /// # Panics
    /// Panics if the input is infinite or the result is not representable as a normal value.
    #[inline]
    #[must_use]
    pub fn sin(&self) -> Self {
        self.context.sin(&self.repr).value(&self.context)
    }

    /// Calculate the cosine of the floating point number.
    ///
    /// # Panics
    /// Panics if the input is infinite or the result is not representable as a normal value.
    #[inline]
    #[must_use]
    pub fn cos(&self) -> Self {
        self.context.cos(&self.repr).value(&self.context)
    }

    /// Calculate both the sine and cosine of the floating point number.
    ///
    /// This is more efficient than calling `sin` and `cos` separately.
    ///
    /// # Panics
    /// Panics if the input is infinite or the results are not representable as normal values.
    #[inline]
    #[must_use]
    pub fn sin_cos(&self) -> (Self, Self) {
        let (s, c) = self.context.sin_cos(&self.repr);
        (s.value(&self.context), c.value(&self.context))
    }

    /// Calculate the tangent of the floating point number.
    ///
    /// Returns `FpResult` to safely handle non-finite results (e.g., at singularities).
    #[inline]
    #[must_use]
    pub fn tan(&self) -> FpResult<B> {
        self.context.tan(&self.repr)
    }

    /// Calculate the arcsine of the floating point number.
    ///
    /// Returns `FpResult` to safely handle domain errors (e.g., |x| > 1).
    #[inline]
    #[must_use]
    pub fn asin(&self) -> FpResult<B> {
        self.context.asin(&self.repr)
    }

    /// Calculate the arccosine of the floating point number.
    ///
    /// Returns `FpResult` to safely handle domain errors (e.g., |x| > 1).
    #[inline]
    #[must_use]
    pub fn acos(&self) -> FpResult<B> {
        self.context.acos(&self.repr)
    }

    /// Calculate the arctangent of the floating point number.
    ///
    /// # Panics
    /// Panics if the result is not representable as a normal value.
    #[inline]
    #[must_use]
    pub fn atan(&self) -> Self {
        self.context.atan(&self.repr).value(&self.context)
    }

    /// Calculate the arctangent of y / x.
    ///
    /// Returns `FpResult` to safely handle special cases like (0,0) or infinities.
    #[inline]
    #[must_use]
    pub fn atan2(&self, x: &Self) -> FpResult<B> {
        self.context.atan2(&self.repr, &x.repr)
    }
}
