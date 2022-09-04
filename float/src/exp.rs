use core::convert::TryInto;

use crate::{
    fbig::FBig,
    repr::{Context, Word, Repr},
    round::{Round, Rounded},
};
use dashu_base::{Approximation::*, DivRemEuclid, EstimatedLog2};
use dashu_int::{IBig, Sign, UBig};

impl<R: Round, const B: Word> FBig<R, B> {
    #[inline]
    pub fn powi(&self, exp: IBig) -> FBig<R, B> {
        self.context.powi(self, exp).value()
    }

    #[inline]
    pub fn exp(&self) -> FBig<R, B> {
        self.context.exp(self).value()
    }
}

impl<R: Round> Context<R> {
    pub fn powi<const B: Word>(&self, base: &FBig<R, B>, exp: IBig) -> Rounded<FBig<R, B>> {
        let (exp_sign, exp) = exp.into_parts();
        if exp_sign == Sign::Negative {
            // if the exponent is negative, then negate the exponent
            // TODO: optimize this branch
            let rev_context = Context::<R::Reverse>::new(self.precision + 1);
            let inv = rev_context.repr_div(Repr::one(), &base.repr).value();
            let inv = FBig::new_raw(inv, *self);
            return self.powi(&inv, exp.into());
        }
        if exp.is_zero() {
            return Exact(FBig::ONE);
        } else if exp.is_one() {
            return Exact(base.clone());
        }

        // increase working precision when the exponent is large
        let guard_digits = exp.bit_len() + 2; // heuristic
        // TODO: let the function associated with Context accept arbitrary rounding mode
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

    #[inline]
    pub fn exp<const B: Word>(&self, x: &FBig<R, B>) -> Rounded<FBig<R, B>> {
        self.exp_internal(x, false)
    }

    #[inline]
    pub fn exp_m1<const B: Word>(&self, x: &FBig<R, B>) -> Rounded<FBig<R, B>> {
        self.exp_internal(x, true)
    }

    fn exp_internal<const B: Word>(&self, x: &FBig<R, B>, minus_one: bool) -> Rounded<FBig<R, B>> {
        if x.repr.is_zero() {
            return Exact(FBig::ONE);
        } else if x.repr.significand.sign() == Sign::Negative {
            // TODO: optimize this branch
            let rev_context = Context::<R::Reverse>::new(self.precision);
            let exp = self.exp_internal(&-x.clone(), minus_one).value();
            let inv = rev_context.div(&FBig::ONE, &exp.with_rounding::<R::Reverse>());
            return inv.map(|v| v.with_rounding::<R>());
        }

        // A simple algorithm:
        // - let r = (x - s logB) / B^n, where s = floor(x / logB), such that r < B^-n.
        // - if the target precision is p digits, then there're only about p/m terms in Tyler series
        // - finally, exp(x) = B^s * exp(r)^(B^n)
        // - the optimal n is √p as given by MPFR

        let no_scaling = minus_one && x.log2_est() < -B.log2_est();
        let (s, n, r) = if no_scaling {
            // if minus_one is true and x is already small (x < 1/B),
            // then directly evaluate the Taylor series without scaling
            (0, 0, x.clone())
        } else {
            let logb = self.ln_base::<B>();
            let (s, r) = x.div_rem_euclid(logb);

            // here m is roughly equal to sqrt(self.precision)
            let n = 1usize << ((usize::BITS - self.precision.leading_zeros()) / 2);
            (s.try_into().expect("exponent is too large"), n, r)
        };

        // Taylor series: exp(r) = 1 + Σ(rⁱ/i!)
        // There will be about p/log_B(r) summations when calculating the series, to prevent
        // loss of significant, we needs about log_B(p) guard digits.
        // TODO(next): test if we can use log_B(p/2log_B(n)) directly
        let bn = UBig::from_word(B).pow(n);
        let guard_digits = ((self.precision / 2).log2_est() / B.log2_est()) as usize;
        let pow_precision = self.precision + guard_digits + 2;
        let r = r.with_precision(pow_precision + bn.bit_len()).value() / &bn;

        let mut factorial = IBig::ONE;
        let mut pow = r.clone();
        let mut sum = if no_scaling {
            r.clone()
        } else {
            FBig::ONE + &r
        };
        dbg!(&r);

        let mut k = 2;
        loop {
            dbg!(&sum);
            factorial *= k;
            pow *= &r;

            let next = &sum + &pow / &factorial;
            if next == sum {
                break;
            }
            sum = next;
            k += 1;
        }

        if no_scaling {
            sum.with_precision(self.precision)
        } else {
            dbg!(n, sum.powi(Repr::<B>::BASE.pow(n)));
            let res_context = Context::<R>::new(pow_precision);
            let res = res_context.powi(&sum, bn.into()).value() << s;
            dbg!(self.precision);
            res.with_precision(self.precision)
        }
    }
}
