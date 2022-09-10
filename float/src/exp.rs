use core::convert::TryInto;

use crate::{
    fbig::FBig,
    repr::{Context, Repr, Word},
    round::{Round, Rounded}, error::check_precision_limited,
};
use dashu_base::{Approximation::*, BitTest, DivRemEuclid, EstimatedLog2, Sign};
use dashu_int::IBig;

impl<R: Round, const B: Word> FBig<R, B> {
    #[inline]
    pub fn powi(&self, exp: IBig) -> FBig<R, B> {
        self.context.powi(self, exp).value()
    }

    #[inline]
    pub fn exp(&self) -> FBig<R, B> {
        self.context.exp(self).value()
    }

    #[inline]
    pub fn exp_m1(&self) -> FBig<R, B> {
        self.context.exp_m1(self).value()
    }
}

// TODO: give the exact formulation of required guard bits

impl<R: Round> Context<R> {
    pub fn powi<_R: Round, const B: Word>(
        &self,
        base: &FBig<_R, B>,
        exp: IBig,
    ) -> Rounded<FBig<R, B>> {
        check_precision_limited(self.precision);

        let (exp_sign, exp) = exp.into_parts();
        if exp_sign == Sign::Negative {
            // if the exponent is negative, then negate the exponent
            // note that do the inverse at last requires less guard bits
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
            let repr = self.repr_round_ref(&base.repr);
            return repr.map(|v| FBig::new(v, *self));
        }

        // increase working precision when the exponent is large
        let guard_digits = exp.bit_len() + self.precision.bit_len(); // heuristic
        let work_context = Context::<R>::new(self.precision + guard_digits);

        // binary exponentiation from left to right
        let mut p = exp.bit_len() - 2;
        let mut res = work_context.square(&base);
        loop {
            if exp.bit(p) {
                res = res.and_then(|v| work_context.mul(&v, &base));
            }
            if p == 0 {
                break;
            }
            p -= 1;
            res = res.and_then(|v| work_context.square(&v));
        }

        res.and_then(|v| v.with_precision(self.precision))
    }

    // TODO: implement powf

    #[inline]
    pub fn exp<_R: Round, const B: Word>(&self, x: &FBig<_R, B>) -> Rounded<FBig<R, B>> {
        self.exp_internal(x, false)
    }

    #[inline]
    pub fn exp_m1<_R: Round, const B: Word>(&self, x: &FBig<_R, B>) -> Rounded<FBig<R, B>> {
        self.exp_internal(x, true)
    }

    // TODO: change reduction to (x - s log2) / 2^n, so that the final powering is always base 2, and doesn't depends on powi.
    //       the powering exp(r)^(2^n) could be optimized by noticing (1+x)^2 - 1 = x^2 + 2x
    //       consider this change after having a benchmark

    fn exp_internal<_R: Round, const B: Word>(
        &self,
        x: &FBig<_R, B>,
        minus_one: bool,
    ) -> Rounded<FBig<R, B>> {
        check_precision_limited(self.precision);

        if x.repr.is_zero() {
            return match minus_one {
                false => Exact(FBig::ONE),
                true => Exact(FBig::ZERO),
            };
        }

        // A simple algorithm:
        // - let r = (x - s logB) / B^n, where s = floor(x / logB), such that r < B^-n.
        // - if the target precision is p digits, then there're only about p/m terms in Tyler series
        // - finally, exp(x) = B^s * exp(r)^(B^n)
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
            if x.repr.sign() == Sign::Negative {
                // extra digits are required to prevent cancellation during the summation
                work_precision = self.precision + 2 * series_guard_digits;
            } else {
                work_precision = self.precision + series_guard_digits;
            }
            (0, 0, x.clone())
        } else {
            work_precision = self.precision + series_guard_digits + pow_guard_digits;
            let logb = Context::new(work_precision).ln_base::<B>();
            let (s, r) = x.div_rem_euclid(logb);

            // here m is roughly equal to sqrt(self.precision)
            let n = 1usize << (self.precision.bit_len() / 2);
            let s: isize = s.try_into().expect("exponent is too large");
            (s, n, r)
        };

        let r = r
            .with_rounding::<R>()
            .with_precision(work_precision)
            .value()
            >> n as isize;
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
                .powi(&sum, Repr::<B>::BASE.pow(n))
                .map(|v| (v << s) - FBig::ONE)
                .and_then(|v| v.with_precision(self.precision))
        } else {
            self.powi(&sum, Repr::<B>::BASE.pow(n)).map(|v| v << s)
        }
    }
}
