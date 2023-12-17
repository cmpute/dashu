use dashu_base::{AbsOrd, Approximation::*, EstimatedLog2, Sign};
use dashu_int::IBig;

use crate::{
    error::{assert_finite, assert_limited_precision},
    fbig::FBig,
    repr::{Context, Repr, Word},
    round::{Round, Rounded},
};

impl<const B: Word> EstimatedLog2 for Repr<B> {
    // currently a Word has at most 64 bits, so log2() < f32::MAX
    fn log2_bounds(&self) -> (f32, f32) {
        // log(s*B^e) = log(s) + e*log(B)
        let (logs_lb, logs_ub) = self.significand.log2_bounds();
        let (logb_lb, logb_ub) = if B.is_power_of_two() {
            let log = B.trailing_zeros() as f32;
            (log, log)
        } else {
            B.log2_bounds()
        };
        let e = self.exponent as f32;
        if self.exponent >= 0 {
            (logs_lb + e * logb_lb, logs_ub + e * logb_ub)
        } else {
            (logs_lb + e * logb_ub, logs_ub + e * logb_lb)
        }
    }

    fn log2_est(&self) -> f32 {
        let logs = self.significand.log2_est();
        let logb = if B.is_power_of_two() {
            B.trailing_zeros() as f32
        } else {
            B.log2_est()
        };
        logs + self.exponent as f32 * logb
    }
}

impl<R: Round, const B: Word> EstimatedLog2 for FBig<R, B> {
    #[inline]
    fn log2_bounds(&self) -> (f32, f32) {
        self.repr.log2_bounds()
    }

    #[inline]
    fn log2_est(&self) -> f32 {
        self.repr.log2_est()
    }
}

impl<R: Round, const B: Word> FBig<R, B> {
    /// Calculate the natural logarithm function (`log(x)`) on the float number.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str_native("1.234")?;
    /// assert_eq!(a.ln(), DBig::from_str_native("0.2103")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn ln(&self) -> Self {
        self.context.ln(&self.repr).value()
    }

    /// Calculate the natural logarithm function (`log(x+1)`) on the float number
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str_native("0.1234")?;
    /// assert_eq!(a.ln_1p(), DBig::from_str_native("0.11636")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn ln_1p(&self) -> Self {
        self.context.ln_1p(&self.repr).value()
    }
}

impl<R: Round> Context<R> {
    /// Calculate log(2)
    ///
    /// The precision of the output will be larger than self.precision
    #[inline]
    fn ln2<const B: Word>(&self) -> FBig<R, B> {
        // log(2) = 4L(6) + 2L(99)
        // see formula (24) from Gourdon, Xavier, and Pascal Sebah.
        // "The Logarithmic Constant: Log 2." (2004)
        4 * self.iacoth(6.into()) + 2 * self.iacoth(99.into())
    }

    /// Calculate log(2)
    ///
    /// The precision of the output will be larger than self.precision
    #[inline]
    fn ln10<const B: Word>(&self) -> FBig<R, B> {
        // log(10) = log(2) + log(5) = 3log(2) + 2L(9)
        3 * self.ln2() + 2 * self.iacoth(9.into())
    }

    /// Calculate log(B), for internal use only
    ///
    /// The precision of the output will be larger than self.precision
    #[inline]
    pub(crate) fn ln_base<const B: Word>(&self) -> FBig<R, B> {
        match B {
            2 => self.ln2(),
            10 => self.ln10(),
            i if i.is_power_of_two() => self.ln2() * i.trailing_zeros(),
            _ => self.ln(&Repr::new(Repr::<B>::BASE.into(), 0)).value(),
        }
    }

    /// Calculate L(n) = acoth(n) = atanh(1/n) = 1/2 log((n+1)/(n-1))
    ///
    /// This method is intended to be used in logarithm calculation,
    /// so the precision of the output will be larger than desired precision.
    fn iacoth<const B: Word>(&self, n: IBig) -> FBig<R, B> {
        /*
         * use Maclaurin series:
         *       1    1     n+1             1
         * atanh(—) = — log(———) =  Σ  ———————————
         *       n    2     n-1    i≥0 n²ⁱ⁺¹(2i+1)
         *
         * Therefore to achieve precision B^p, the series should be stopped at
         *    n²ⁱ⁺¹(2i+1) / n = B^p
         * => 2i*ln(n) + ln(2i+1) = p ln(B)
         * ~> 2i*ln(n) = p ln(B)
         * => 2i = p/log_B(n)
         *
         * There will be i summations when calculating the series, to prevent
         * loss of significant, we needs about log_B(i) guard digits.
         *    log_B(i)
         * <= log_B(p/2log_B(n))
         *  = log_B(p/2) - log_B(log_B(n))
         * <= log_B(p/2)
         */

        // extras digits are added to ensure precise result
        // TODO: test if we can use log_B(p/2log_B(n)) directly
        let guard_digits = (self.precision.log2_est() / B.log2_est()) as usize;
        let work_context = Self::new(self.precision + guard_digits + 2);

        let n = work_context.convert_int(n).value();
        let inv = FBig::ONE / n;
        let inv2 = inv.sqr();
        let mut sum = inv.clone();
        let mut pow = inv;

        let mut k: usize = 3;
        loop {
            pow *= &inv2;

            let increase = &pow / work_context.convert_int::<B>(k.into()).value();
            if increase < sum.sub_ulp() {
                return sum;
            }

            sum += increase;
            k += 2;
        }
    }

    /// Calculate the natural logarithm function (`log(x)`) on the float number under this context.
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
    /// let a = DBig::from_str_native("1.234")?;
    /// assert_eq!(context.ln(&a.repr()), Inexact(DBig::from_str_native("0.21")?, NoOp));
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn ln<const B: Word>(&self, x: &Repr<B>) -> Rounded<FBig<R, B>> {
        self.ln_internal(x, false)
    }

    /// Calculate the natural logarithm function (`log(x+1)`) on the float number under this context.
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
    /// let a = DBig::from_str_native("0.1234")?;
    /// assert_eq!(context.ln_1p(&a.repr()), Inexact(DBig::from_str_native("0.12")?, AddOne));
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn ln_1p<const B: Word>(&self, x: &Repr<B>) -> Rounded<FBig<R, B>> {
        self.ln_internal(x, true)
    }

    fn ln_internal<const B: Word>(&self, x: &Repr<B>, one_plus: bool) -> Rounded<FBig<R, B>> {
        assert_finite(x);
        assert_limited_precision(self.precision);

        if (one_plus && x.is_zero()) || (!one_plus && x.is_one()) {
            return Exact(FBig::ZERO);
        }

        // A simple algorithm:
        // - let log(x) = log(x/2^s) + slog2 where s = floor(log2(x))
        // - such that x*2^s is close to but larger than 1 (and x*2^s < 2)
        let guard_digits = (self.precision.log2_est() / B.log2_est()) as usize + 2;
        let mut work_precision = self.precision + guard_digits + one_plus as usize;
        let context = Context::<R>::new(work_precision);
        let x = FBig::new(context.repr_round_ref(x).value(), context);

        // When one_plus is true and |x| < 1/B, the input is fed into the Maclaurin without scaling
        let no_scaling = one_plus && x.log2_est() < -B.log2_est();

        let (s, mut x_scaled) = if no_scaling {
            (0, x)
        } else {
            let x = if one_plus { x + FBig::ONE } else { x };

            let log2 = x.log2_bounds().0;
            let s = log2 as isize - (log2 < 0.) as isize; // floor(log2(x))

            let x_scaled = if B == 2 {
                x >> s
            } else if s > 0 {
                x / (IBig::ONE << s as usize)
            } else {
                x * (IBig::ONE << (-s) as usize)
            };
            debug_assert!(x_scaled >= FBig::<R, B>::ONE);
            (s, x_scaled)
        };

        if s < 0 || x_scaled.repr.sign() == Sign::Negative {
            // when s or x_scaled is negative, the final addition is actually a subtraction,
            // therefore we need to double the precision to get the correct result
            work_precision += self.precision;
            x_scaled.context.precision = work_precision;
        };
        let work_context = Context::new(work_precision);

        // after the number is scaled to nearly one, use Maclaurin series on log(x) = 2atanh(z):
        // let z = (x-1)/(x+1) < 1, log(x) = 2atanh(z) = 2Σ(z²ⁱ⁺¹/(2i+1)) for i = 1,3,5,...
        // similar to iacoth, the required iterations stop at i = -p/2log_B(z), and we need log_B(i) guard bits
        let z = if no_scaling {
            let d = &x_scaled + (FBig::ONE + FBig::ONE);
            x_scaled / d
        } else {
            (&x_scaled - FBig::ONE) / (x_scaled + FBig::ONE)
        };
        let z2 = z.sqr();
        let mut pow = z.clone();
        let mut sum = z;

        let mut k: usize = 3;
        loop {
            pow *= &z2;

            let increase = &pow / work_context.convert_int::<B>(k.into()).value();
            if increase.abs_cmp(&sum.sub_ulp()).is_le() {
                break;
            }

            sum += increase;
            k += 2;
        }

        // compose the logarithm of the original number
        let result: FBig<R, B> = if no_scaling {
            2 * sum
        } else {
            2 * sum + s * work_context.ln2()
        };
        result.with_precision(self.precision)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::round::mode;

    #[test]
    fn test_iacoth() {
        let context = Context::<mode::Zero>::new(10);
        let binary_6 = context.iacoth::<2>(6.into()).with_precision(10).value();
        assert_eq!(binary_6.repr.significand, IBig::from(689));
        let decimal_6 = context.iacoth::<10>(6.into()).with_precision(10).value();
        assert_eq!(decimal_6.repr.significand, IBig::from(1682361183));

        let context = Context::<mode::Zero>::new(40);
        let decimal_6 = context.iacoth::<10>(6.into()).with_precision(40).value();
        assert_eq!(
            decimal_6.repr.significand,
            IBig::from_str_radix("1682361183106064652522967051084960450557", 10).unwrap()
        );

        let context = Context::<mode::Zero>::new(201);
        let binary_6 = context.iacoth::<2>(6.into()).with_precision(201).value();
        assert_eq!(
            binary_6.repr.significand,
            IBig::from_str_radix(
                "2162760151454160450909229890833066944953539957685348083415205",
                10
            )
            .unwrap()
        );
    }

    #[test]
    fn test_ln2_ln10() {
        let context = Context::<mode::Zero>::new(45);
        let decimal_ln2 = context.ln2::<10>().with_precision(45).value();
        assert_eq!(
            decimal_ln2.repr.significand,
            IBig::from_str_radix("693147180559945309417232121458176568075500134", 10).unwrap()
        );
        let decimal_ln10 = context.ln10::<10>().with_precision(45).value();
        assert_eq!(
            decimal_ln10.repr.significand,
            IBig::from_str_radix("230258509299404568401799145468436420760110148", 10).unwrap()
        );

        let context = Context::<mode::Zero>::new(180);
        let binary_ln2 = context.ln2::<2>().with_precision(180).value();
        assert_eq!(
            binary_ln2.repr.significand,
            IBig::from_str_radix("1062244963371879310175186301324412638028404515790072203", 10)
                .unwrap()
        );
        let binary_ln10 = context.ln10::<2>().with_precision(180).value();
        assert_eq!(
            binary_ln10.repr.significand,
            IBig::from_str_radix("882175346869410758689845931257775553286341791676474847", 10)
                .unwrap()
        );
    }
}
