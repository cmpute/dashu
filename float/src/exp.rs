use core::convert::TryInto;

use crate::{
    error::{check_inf, check_precision_limited, panic_power_negative_base},
    fbig::FBig,
    repr::{Context, Repr, Word},
    round::{Round, Rounded},
};
use dashu_base::{Approximation::*, BitTest, DivRemEuclid, EstimatedLog2, Sign};
use dashu_int::IBig;

impl<R: Round, const B: Word> FBig<R, B> {
    /// Raise the floating point number to an integer power.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::error::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str_native("-1.234")?;
    /// assert_eq!(a.powi(10.into()), DBig::from_str_native("8.188")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn powi(&self, exp: IBig) -> FBig<R, B> {
        self.context.powi(&self.repr, exp).value()
    }

    /// Raise the floating point number to an floating point power.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::error::ParseError;
    /// # use dashu_float::DBig;
    /// let x = DBig::from_str_native("1.23")?;
    /// let y = DBig::from_str_native("-4.56")?;
    /// assert_eq!(x.powf(&y), DBig::from_str_native("0.389")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn powf(&self, exp: &Self) -> Self {
        let context = Context::max(self.context, exp.context);
        context.powf(&self.repr, &exp.repr).value()
    }

    /// Calculate the exponential function (`eˣ`) on the floating point number.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::error::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str_native("-1.234")?;
    /// assert_eq!(a.exp(), DBig::from_str_native("0.2911")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn exp(&self) -> FBig<R, B> {
        self.context.exp(&self.repr).value()
    }

    /// Calculate the exponential minus one function (`eˣ-1`) on the floating point number.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::error::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str_native("-0.1234")?;
    /// assert_eq!(a.exp_m1(), DBig::from_str_native("-0.11609")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn exp_m1(&self) -> FBig<R, B> {
        self.context.exp_m1(&self.repr).value()
    }
}

// TODO: give the exact formulation of required guard bits

impl<R: Round> Context<R> {
    /// Raise the floating point number to an integer power under this context.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::error::ParseError;
    /// # use dashu_float::DBig;
    /// use dashu_base::Approximation::*;
    /// use dashu_float::{Context, round::{mode::HalfAway, Rounding::*}};
    ///
    /// let context = Context::<HalfAway>::new(2);
    /// let a = DBig::from_str_native("-1.234")?;
    /// assert_eq!(context.powi(&a.repr(), 10.into()), Inexact(DBig::from_str_native("8.2")?, AddOne));
    /// # Ok::<(), ParseError>(())
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the precision is unlimited and the exponent is negative. In this case, the exact
    /// result is likely to have infinite digits.
    pub fn powi<const B: Word>(&self, base: &Repr<B>, exp: IBig) -> Rounded<FBig<R, B>> {
        check_inf(base);

        let (exp_sign, exp) = exp.into_parts();
        if exp_sign == Sign::Negative {
            // if the exponent is negative, then negate the exponent
            // note that do the inverse at last requires less guard bits
            check_precision_limited(self.precision); // TODO: we can allow this if the inverse is exact (only when significand is one?)

            let guard_bits = self.precision.bit_len() * 2; // heuristic
            let rev_context = Context::<R::Reverse>::new(self.precision + guard_bits);
            let pow = rev_context.powi(base, exp.into()).value();
            let inv = rev_context.repr_div(Repr::one(), &pow.repr);
            let repr = inv.and_then(|v| self.repr_round(v));
            return repr.map(|v| FBig::new(v, *self));
        }
        if exp.is_zero() {
            return Exact(FBig::ONE);
        } else if exp.is_one() {
            let repr = self.repr_round_ref(base);
            return repr.map(|v| FBig::new(v, *self));
        }

        let work_context = if self.is_limited() {
            // increase working precision when the exponent is large
            let guard_digits = exp.bit_len() + self.precision.bit_len(); // heuristic
            Context::<R>::new(self.precision + guard_digits)
        } else {
            Context::<R>::new(0)
        };

        // binary exponentiation from left to right
        let mut p = exp.bit_len() - 2;
        let mut res = work_context.square(base);
        loop {
            if exp.bit(p) {
                res = res.and_then(|v| work_context.mul(v.repr(), base));
            }
            if p == 0 {
                break;
            }
            p -= 1;
            res = res.and_then(|v| work_context.square(v.repr()));
        }

        res.and_then(|v| v.with_precision(self.precision))
    }

    /// Raise the floating point number to an floating point power under this context.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::error::ParseError;
    /// # use dashu_float::DBig;
    /// use dashu_base::Approximation::*;
    /// use dashu_float::{Context, round::{mode::HalfAway, Rounding::*}};
    ///
    /// let context = Context::<HalfAway>::new(2);
    /// let x = DBig::from_str_native("1.23")?;
    /// let y = DBig::from_str_native("-4.56")?;
    /// assert_eq!(context.powf(&x.repr(), &y.repr()), Inexact(DBig::from_str_native("0.39")?, AddOne));
    /// # Ok::<(), ParseError>(())
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the precision is unlimited.
    pub fn powf<const B: Word>(&self, base: &Repr<B>, exp: &Repr<B>) -> Rounded<FBig<R, B>> {
        check_inf(base);
        check_precision_limited(self.precision); // TODO: we can allow it if exp is integer

        // shortcuts
        if exp.is_zero() {
            return Exact(FBig::ONE);
        } else if exp.is_one() {
            let repr = self.repr_round_ref(base);
            return repr.map(|v| FBig::new(v, *self));
        }
        if base.sign() == Sign::Negative {
            // TODO: we should allow negative base when exp is an integer
            panic_power_negative_base()
        }

        // x^y = exp(y*log(x)), use a simple rule for guard bits
        let guard_digits = 10 + self.precision.log2_est() as usize;
        let work_context = Context::<R>::new(self.precision + guard_digits);

        let res = work_context
            .ln(base)
            .and_then(|v| work_context.mul(&v.repr, exp))
            .and_then(|v| work_context.exp(&v.repr));
        res.and_then(|v| v.with_precision(self.precision))
    }

    /// Calculate the exponential function (`eˣ`) on the floating point number under this context.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::error::ParseError;
    /// # use dashu_float::DBig;
    /// use dashu_base::Approximation::*;
    /// use dashu_float::{Context, round::{mode::HalfAway, Rounding::*}};
    ///
    /// let context = Context::<HalfAway>::new(2);
    /// let a = DBig::from_str_native("-1.234")?;
    /// assert_eq!(context.exp(&a.repr()), Inexact(DBig::from_str_native("0.29")?, NoOp));
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn exp<const B: Word>(&self, x: &Repr<B>) -> Rounded<FBig<R, B>> {
        self.exp_internal(x, false)
    }

    /// Calculate the exponential minus one function (`eˣ-1`) on the floating point number under this context.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::error::ParseError;
    /// # use dashu_float::DBig;
    /// use dashu_base::Approximation::*;
    /// use dashu_float::{Context, round::{mode::HalfAway, Rounding::*}};
    ///
    /// let context = Context::<HalfAway>::new(2);
    /// let a = DBig::from_str_native("-0.1234")?;
    /// assert_eq!(context.exp_m1(&a.repr()), Inexact(DBig::from_str_native("-0.12")?, SubOne));
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn exp_m1<const B: Word>(&self, x: &Repr<B>) -> Rounded<FBig<R, B>> {
        self.exp_internal(x, true)
    }

    // TODO: change reduction to (x - s log2) / 2ⁿ, so that the final powering is always base 2, and doesn't depends on powi.
    //       the powering exp(r)^(2ⁿ) could be optimized by noticing (1+x)^2 - 1 = x^2 + 2x
    //       consider this change after having a benchmark

    fn exp_internal<const B: Word>(&self, x: &Repr<B>, minus_one: bool) -> Rounded<FBig<R, B>> {
        check_inf(x);
        check_precision_limited(self.precision);

        if x.is_zero() {
            return match minus_one {
                false => Exact(FBig::ONE),
                true => Exact(FBig::ZERO),
            };
        }

        // A simple algorithm:
        // - let r = (x - s logB) / Bⁿ, where s = floor(x / logB), such that r < B⁻ⁿ.
        // - if the target precision is p digits, then there're only about p/m terms in Tyler series
        // - finally, exp(x) = Bˢ * exp(r)^(Bⁿ)
        // - the optimal n is √p as given by MPFR

        // Maclaurin series: exp(r) = 1 + Σ(rⁱ/i!)
        // There will be about p/log_B(r) summations when calculating the series, to prevent
        // loss of significant, we needs about log_B(p) guard digits.
        let series_guard_digits = (self.precision.log2_est() / B.log2_est()) as usize + 2;
        let pow_guard_digits = (self.precision.bit_len() as f32 * B.log2_est() * 2.) as usize; // heuristic
        let work_precision;

        // When minus_one is true and |x| < 1/B, the input is fed into the Maclaurin series without scaling
        let no_scaling = minus_one && x.log2_est() < -B.log2_est();
        let (s, n, r) = if no_scaling {
            // if minus_one is true and x is already small (x < 1/B),
            // then directly evaluate the Maclaurin series without scaling
            if x.sign() == Sign::Negative {
                // extra digits are required to prevent cancellation during the summation
                work_precision = self.precision + 2 * series_guard_digits;
            } else {
                work_precision = self.precision + series_guard_digits;
            }
            let context = Context::<R>::new(work_precision);
            (0, 0, FBig::new(context.repr_round_ref(x).value(), context))
        } else {
            work_precision = self.precision + series_guard_digits + pow_guard_digits;
            let context = Context::<R>::new(work_precision);
            let x = FBig::new(context.repr_round_ref(x).value(), context);
            let logb = context.ln_base::<B>();
            let (s, r) = x.div_rem_euclid(logb);

            // here m is roughly equal to sqrt(self.precision)
            let n = 1usize << (self.precision.bit_len() / 2);
            let s: isize = s.try_into().expect("exponent is too large");
            (s, n, r)
        };

        let r = r >> n as isize;
        let mut factorial = IBig::ONE;
        let mut pow = r.clone();
        let mut sum = if no_scaling {
            r.clone()
        } else {
            FBig::ONE + &r
        };

        let mut k = 2;
        loop {
            factorial *= k;
            pow *= &r;
            // TODO: use &pow / &factorial < ulp as stop criteria?

            let next = &sum + &pow / &factorial;
            if next == sum {
                break;
            }
            sum = next;
            k += 1;
        }

        if no_scaling {
            sum.with_precision(self.precision)
        } else if minus_one {
            // add extra digits to compensate for the subtraction
            Context::<R>::new(self.precision + self.precision / 8 + 1) // heuristic
                .powi(sum.repr(), Repr::<B>::BASE.pow(n))
                .map(|v| (v << s) - FBig::ONE)
                .and_then(|v| v.with_precision(self.precision))
        } else {
            self.powi(sum.repr(), Repr::<B>::BASE.pow(n))
                .map(|v| v << s)
        }
    }
}
