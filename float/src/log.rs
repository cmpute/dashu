use dashu_base::{
    utils::{next_down, next_up},
    AbsOrd,
    Approximation::*,
    EstimatedLog2, Sign,
};
use dashu_int::IBig;

use crate::{
    error::{assert_finite, assert_limited_precision, FpError, FpResult},
    fbig::FBig,
    math::cache::{reborrow_cache, ConstCache},
    repr::{Context, Repr, Word},
    round::{Round, Rounded},
    utils::ceil_usize,
};
use core::cmp::Ordering;

impl<const B: Word> EstimatedLog2 for Repr<B> {
    // currently a Word has at most 64 bits, so log2() < f32::MAX
    fn log2_bounds(&self) -> (f32, f32) {
        if self.significand.is_zero() {
            return (f32::NEG_INFINITY, f32::NEG_INFINITY);
        }

        // log(s*B^e) = log(s) + e*log(B)
        let (logs_lb, logs_ub) = self.significand.log2_bounds();
        let (logb_lb, logb_ub) = if B.is_power_of_two() {
            let log = B.trailing_zeros() as f32;
            (log, log)
        } else {
            B.log2_bounds()
        };
        let e = self.exponent as f32;
        let (lb, ub) = if self.exponent >= 0 {
            (logs_lb + e * logb_lb, logs_ub + e * logb_ub)
        } else {
            (logs_lb + e * logb_ub, logs_ub + e * logb_lb)
        };
        (next_down(lb), next_up(ub))
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
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str("1.234")?;
    /// assert_eq!(a.ln(), DBig::from_str("0.2103")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn ln(&self) -> Self {
        self.context.unwrap_fp(self.context.ln(&self.repr, None))
    }

    /// Calculate the natural logarithm function (`log(x+1)`) on the float number
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str("0.1234")?;
    /// assert_eq!(a.ln_1p(), DBig::from_str("0.11636")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn ln_1p(&self) -> Self {
        self.context.unwrap_fp(self.context.ln_1p(&self.repr, None))
    }
}

impl<R: Round> Context<R> {
    /// Calculate log(2)
    ///
    /// The precision of the output will be larger than self.precision
    #[inline]
    fn ln2<const B: Word>(&self, cache: Option<&mut ConstCache>) -> FBig<R, B> {
        if let Some(c) = cache {
            return c.ln2::<B, R>(self.precision);
        }
        // log(2) = 4L(6) + 2L(99)
        // see formula (24) from Gourdon, Xavier, and Pascal Sebah.
        // "The Logarithmic Constant: Log 2." (2004)
        4 * self.iacoth(6.into()) + 2 * self.iacoth(99.into())
    }

    /// Calculate log(10)
    ///
    /// The precision of the output will be larger than self.precision
    #[inline]
    fn ln10<const B: Word>(&self, cache: Option<&mut ConstCache>) -> FBig<R, B> {
        if let Some(c) = cache {
            return c.ln10::<B, R>(self.precision);
        }
        // log(10) = log(2) + log(5) = 3log(2) + 2L(9)
        3 * self.ln2(None) + 2 * self.iacoth(9.into())
    }

    /// Calculate log(B), for internal use only
    ///
    /// The precision of the output will be larger than self.precision
    #[inline]
    pub(crate) fn ln_base<const B: Word>(&self, cache: Option<&mut ConstCache>) -> FBig<R, B> {
        if let Some(c) = cache {
            return c.ln_base::<B, R>(self.precision);
        }
        match B {
            2 => self.ln2(None),
            10 => self.ln10(None),
            i if i.is_power_of_two() => self.ln2(None) * i.trailing_zeros(),
            _ => self.unwrap_fp(self.ln(&Repr::new(Repr::<B>::BASE.into(), 0), None)),
        }
    }

    /// Calculate L(n) = acoth(n) = atanh(1/n) = 1/2 log((n+1)/(n-1))
    ///
    /// This method is intended to be used in logarithm calculation,
    /// so the precision of the output will be larger than desired precision.
    ///
    /// Evaluated by binary splitting (see [`iacoth_bs`][crate::math::consts::iacoth_bs]):
    /// the exact integer tree state `(P, Q, T)` over `[1, N)` satisfies
    /// `L(n) = (Q + T)/(n·Q)`, with `Q` kept at O(p) digits by the ratio-form
    /// term recurrence.
    fn iacoth<const B: Word>(&self, n: IBig) -> FBig<R, B> {
        let n: u32 = (&n).try_into().expect("iacoth argument must fit in u32");

        // number of series terms until r_k < B^{-p}:  (2k+1)·log_B(n) > p.
        // The count is generously over-provisioned, so a truncating cast stands in
        // for a ceiling.
        let log_b_n = n.log2_est() / B.log2_est();
        let num_terms = (self.precision as f32 / (2.0 * log_b_n)) as usize + 10;

        let (_p, q, t) = crate::math::consts::iacoth_bs(n, 1, num_terms + 1);

        // L(n) = (Q + T) / (n·Q). Extra guard digits absorb the division's rounding
        // (the binary-splitting state is exact, so only this single round loses anything).
        let guard_digits = ceil_usize(self.precision.log2_est() / B.log2_est());
        let work_context = Self::new(self.precision + guard_digits + 2);

        let num = work_context.convert_int::<B>(q.as_ibig() + &t).value();
        let denom = work_context.convert_int::<B>(IBig::from(n) * &q).value();
        num / denom
    }

    /// Calculate the natural logarithm function (`log(x)`) on the float number under this context.
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
    /// let a = DBig::from_str("1.234")?;
    /// assert_eq!(context.ln(&a.repr(), None), Ok(Inexact(DBig::from_str("0.21")?, NoOp)));
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn ln<const B: Word>(
        &self,
        x: &Repr<B>,
        cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        if x.is_infinite() {
            return Err(FpError::InfiniteInput);
        }
        if x.significand.is_zero() {
            // ln(±0) = -inf (a value, not an error)
            return Ok(Exact(FBig::new(Repr::neg_infinity(), *self)));
        }
        if x.sign() == Sign::Negative {
            return Err(FpError::OutOfDomain);
        }
        Ok(self.ln_internal(x, false, cache))
    }

    /// Calculate the natural logarithm function (`log(x+1)`) on the float number under this context.
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
    /// let a = DBig::from_str("0.1234")?;
    /// assert_eq!(context.ln_1p(&a.repr(), None), Ok(Inexact(DBig::from_str("0.12")?, AddOne)));
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn ln_1p<const B: Word>(
        &self,
        x: &Repr<B>,
        cache: Option<&mut ConstCache>,
    ) -> FpResult<FBig<R, B>> {
        if x.is_infinite() {
            return Err(FpError::InfiniteInput);
        }
        // Domain of ln_1p is x > -1. x == -1 gives -inf; x < -1 is out of domain.
        if x.sign() == Sign::Negative && !x.significand.is_zero() {
            match FBig::<R, B>::new(x.clone(), *self).abs_cmp(&FBig::ONE) {
                Ordering::Greater => return Err(FpError::OutOfDomain), // x < -1
                Ordering::Equal => return Ok(Exact(FBig::new(Repr::neg_infinity(), *self))),
                _ => {}
            }
        }
        Ok(self.ln_internal(x, true, cache))
    }

    fn ln_internal<const B: Word>(
        &self,
        x: &Repr<B>,
        one_plus: bool,
        mut cache: Option<&mut ConstCache>,
    ) -> Rounded<FBig<R, B>> {
        assert_finite(x);
        assert_limited_precision(self.precision);

        if !one_plus && x.is_one() {
            return Exact(FBig::ZERO); // ln(1) = +0
        }
        if one_plus && x.significand.is_zero() {
            // ln_1p(±0) = ±0
            let zero = if x.is_neg_zero() {
                FBig::new(Repr::neg_zero(), *self)
            } else {
                FBig::ZERO
            };
            return Exact(zero);
        }

        // A simple algorithm:
        // - let log(x) = log(x/2^s) + slog2 where s = floor(log2(x))
        // - such that x*2^s is close to but larger than 1 (and x*2^s < 2)
        let guard_digits = ceil_usize(self.precision.log2_est() / B.log2_est()) + 2;
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
            2 * sum + s * work_context.ln2::<B>(reborrow_cache(&mut cache))
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
        let decimal_ln2 = context.ln2::<10>(None).with_precision(45).value();
        assert_eq!(
            decimal_ln2.repr.significand,
            IBig::from_str_radix("693147180559945309417232121458176568075500134", 10).unwrap()
        );
        let decimal_ln10 = context.ln10::<10>(None).with_precision(45).value();
        assert_eq!(
            decimal_ln10.repr.significand,
            IBig::from_str_radix("230258509299404568401799145468436420760110148", 10).unwrap()
        );

        let context = Context::<mode::Zero>::new(180);
        let binary_ln2 = context.ln2::<2>(None).with_precision(180).value();
        assert_eq!(
            binary_ln2.repr.significand,
            IBig::from_str_radix("1062244963371879310175186301324412638028404515790072203", 10)
                .unwrap()
        );
        let binary_ln10 = context.ln10::<2>(None).with_precision(180).value();
        assert_eq!(
            binary_ln10.repr.significand,
            IBig::from_str_radix("882175346869410758689845931257775553286341791676474847", 10)
                .unwrap()
        );
    }
}

#[cfg(test)]
mod bench_iacoth {
    use super::*;
    use crate::round::mode;
    use std::hint::black_box;
    use std::time::Instant;

    /// Master's iterative iacoth: full-precision FBig Maclaurin series.
    fn iacoth_iter<R: Round, const B: Word>(precision: usize, n: u32) -> FBig<R, B> {
        let guard = (precision.log2_est() / B.log2_est()) as usize;
        let work = Context::<R>::new(precision + guard + 2);
        let nf = work.convert_int::<B>(n.into()).value();
        let inv = FBig::<R, B>::ONE / nf;
        let inv2 = inv.sqr();
        let mut sum = inv.clone();
        let mut pow = inv;
        let mut k: usize = 3;
        loop {
            pow *= &inv2;
            let increase = &pow / work.convert_int::<B>(k.into()).value();
            if increase < sum.sub_ulp() {
                return sum;
            }
            sum += increase;
            k += 2;
        }
    }

    /// ln(2) via the iterative series: 4·L(6) + 2·L(99).
    fn ln2_iter<R: Round, const B: Word>(precision: usize) -> FBig<R, B> {
        let l6 = iacoth_iter::<R, B>(precision, 6);
        let l99 = iacoth_iter::<R, B>(precision, 99);
        (4u8 * l6 + 2u8 * l99).with_precision(precision).value()
    }

    /// ln(2) via the current binary-splitting path.
    fn ln2_bs<R: Round, const B: Word>(precision: usize) -> FBig<R, B> {
        Context::<R>::new(precision)
            .ln2::<B>(None)
            .with_precision(precision)
            .value()
    }

    #[test]
    #[ignore]
    fn bench_ln2_iter_vs_binary_splitting() {
        let precisions: &[usize] = &[50, 200, 1_000, 5_000, 10_000];
        eprintln!("\nln(2) computation: iterative (master) vs binary splitting (this branch)");
        eprintln!("{:>8} {:>14} {:>14} {:>10}", "digits", "iterative", "bin-split", "speedup");
        for &p in precisions {
            // correctness: agree to p-5 digits
            let bs = ln2_bs::<mode::Zero, 10>(p);
            let it = ln2_iter::<mode::Zero, 10>(p);
            let check_digits = p.saturating_sub(5).max(1);
            assert_eq!(
                bs.clone().with_precision(check_digits),
                it.clone().with_precision(check_digits),
                "mismatch at p={p}"
            );

            let reps = if p <= 200 {
                50
            } else if p <= 1_000 {
                10
            } else {
                1
            };
            let t0 = Instant::now();
            for _ in 0..reps {
                black_box(ln2_iter::<mode::Zero, 10>(p));
            }
            let t_iter = t0.elapsed() / reps as u32;

            let t0 = Instant::now();
            for _ in 0..reps {
                black_box(ln2_bs::<mode::Zero, 10>(p));
            }
            let t_bs = t0.elapsed() / reps as u32;

            let speedup = t_iter.as_secs_f64() / t_bs.as_secs_f64();
            eprintln!("{:>8} {:>11.2?} {:>14.2?} {:>9.2}x", p, t_iter, t_bs, speedup);
        }
    }
}

#[cfg(test)]
mod bench_pi_sqrt {
    use super::*;
    use crate::round::mode;
    use std::hint::black_box;
    use std::time::Instant;

    #[test]
    #[ignore]
    fn bench_pi_repeat_call() {
        // First call computes the series + sqrt; second call reuses both.
        use crate::math::cache::ConstCache;
        let precisions: &[usize] = &[500, 5_000];
        eprintln!("\nπ repeat-call (ConstCache): cold vs warm (sqrt + series reused)");
        eprintln!("{:>8} {:>12} {:>12} {:>10}", "digits", "cold", "warm", "warm/cold");
        for &p in precisions {
            let mut c = ConstCache::new();
            let t0 = std::time::Instant::now();
            black_box(c.pi::<10, mode::Zero>(p));
            let cold = t0.elapsed();
            let t0 = std::time::Instant::now();
            black_box(c.pi::<10, mode::Zero>(p));
            let warm = t0.elapsed();
            eprintln!(
                "{:>8} {:>12.2?} {:>12.2?} {:>9.2}x",
                p,
                cold,
                warm,
                cold.as_secs_f64() / warm.as_secs_f64()
            );
        }
    }
    #[test]
    #[ignore]
    fn bench_pi_vs_sqrt10005() {
        let precisions: &[usize] = &[50, 500, 5_000];
        eprintln!("\nπ (Chudnovsky) vs its sqrt(10005) sub-computation");
        eprintln!("{:>8} {:>12} {:>12} {:>10}", "digits", "pi_total", "sqrt10005", "sqrt %");
        for &p in precisions {
            // time the full pi computation
            let reps = if p <= 500 { 10 } else { 1 };
            let t0 = Instant::now();
            for _ in 0..reps {
                black_box(Context::<mode::Zero>::new(p).pi::<10>(None).value());
            }
            let t_pi = t0.elapsed() / reps as u32;

            // time just sqrt(10005) at the same working precision pi uses
            // (work precision ≈ p + guard; use p*2 bits of significand to mirror it)
            let ctx = Context::<mode::Zero>::new(p);
            let arg = ctx.convert_int::<10>(10005i32.into()).value();
            let t0 = Instant::now();
            for _ in 0..reps {
                black_box(ctx.unwrap_fp(ctx.sqrt(&arg.repr)));
            }
            let t_sqrt = t0.elapsed() / reps as u32;

            let pct = 100.0 * t_sqrt.as_secs_f64() / t_pi.as_secs_f64();
            eprintln!("{:>8} {:>12.2?} {:>12.2?} {:>8.1}%", p, t_pi, t_sqrt, pct);
        }
    }
}
