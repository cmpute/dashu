
use dashu_int::{IBig, Sign::Positive};

use crate::{
    fbig::FBig,
    repr::{Context, Repr, Word},
    round::Round
};

impl<R: Round> Context<R> {
    /// Calculate log(2)
    /// 
    /// The precision of the output will be larger than self.precision
    fn ln2<const B: Word>(&self) -> FBig<B, R> {
        // log(2) = 4L(6) + 2L(99)
        // see formula (24) from Gourdon, Xavier, and Pascal Sebah.
        // "The Logarithmic Constant: Log 2." (2004)

        // TODO: implement FBig multiplication with primitive integers
        let two = FBig::from_word(2);
        let four = FBig::from_word(4);

        four * self.atanh_inv(6.into()) + two * self.atanh_inv(99.into())
    }

    fn ln10<const B: Word>(&self) -> FBig<B, R> {
        // log(10) = log(2) + log(5) = 3log(2) + 2L(13)
        // see example (17) from "The Logarithmic Constant: Log 2"
        unimplemented!()
    }

    fn ln_int<const B: Word>(&self, n: IBig) -> FBig<B, R> {
        // log(k) = (k//2)*log2 + 2L(2k-1)
        // log(k*2^n) = log(k) + n*log(2)
        unimplemented!()
    }

    /// Calculate L(n) = atanh(1/n) = 1/2 log((n+1)/(n-1))
    /// 
    /// The precision of the output will be larger than self.precision
    fn atanh_inv<const B: Word>(&self, n: IBig) -> FBig<B, R> {
        /*
         *       1    1     n+1             1
         * atanh(—) = — log(———) =  Σ  ———————————
         *       n    2     n-1    i≥0 n²ⁱ⁺¹(2i+1)
         * 
         * Therefore to achieve precision B^p, the series should be stopped at
         *    n²ⁱ⁺¹(2i+1) >= B^p
         * => (2i+1)ln(n) + ln(2i+1) >= p ln(B)
         * => (2i+1)ln(n) >= p ln(B)
         * => i >= (p/log_B(n) - 1) / 2
         * 
         * There will be i summations when calculating the series, to prevent
         * loss of significant, we needs log_B(i) guard digits.
         *    log_B[(p/log_B(n) - 1) / 2]
         * <= log_B(p/2log_B(n))
         *  = log_B(p/2) - log_B(log_B(n))
         * <= log_B(p/2)
         */
        let work_iters = self.precision / 2; // TODO: calculate expected iterations
        let work_prec = self.precision; // TODO: add guard bits
        let work_context = Self::new(work_prec);

        let n_repr = Repr::<B>::new(n, 0);
        let n = FBig::new_raw(work_context.repr_round(n_repr).value(), work_context);
        let inv = FBig::ONE / n;
        let inv2 = &inv * &inv; // TODO: implement a square() function
        let mut sum = inv.clone();
        let mut pow = inv.clone();

        for i in 1..work_iters {
            pow *= &inv2;
            // let term = pow / (2*i + 1);
            // sum += term;

            // TODO: let from_integer and other constants has precision 0, and provide a function to shrink precision to fit
        }
        unimplemented!()
    }
}
