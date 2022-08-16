use crate::{
    Word,
    round::{Rounding, Round},
    repr::{Context, Repr},
    fbig::FBig,
    utils::{digit_len, shr_rem_radix, shr_rem_radix_in_place, shl_radix_in_place, split_radix_at},
};
use core::{ops::{Add, Sub}, cmp::Ordering};

use dashu_base::Approximation;
use dashu_int::IBig;

impl<const B: Word, R: Round> Add for FBig<B, R> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        let context = Context::max(self.context, rhs.context);
        FBig {
            repr: context.repr_add_on_lhs(self.repr, &rhs.repr).value(),
            context
        }
    }
}

impl<const B: Word, R: Round> Sub for FBig<B, R> {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        let context = Context::max(self.context, rhs.context);
        FBig {
            repr: context.repr_add_on_lhs(self.repr, &-rhs.repr).value(),
            context
        }
    }
}

impl<const B: Word, R: Round> Add for &FBig<B, R> {
    type Output = FBig<B, R>;
    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        let context = Context::max(self.context, rhs.context);
        FBig {
            repr: context.repr_add_on_lhs(self.repr.clone(), &rhs.repr).value(),
            context
        }
    }
}
impl<const B: Word, R: Round> Sub for &FBig<B, R> {
    type Output = FBig<B, R>;
    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        self.add(&(-rhs))
    }
}
impl<const B: Word, R: Round> Sub<FBig<B, R>> for &FBig<B, R> {
    type Output = FBig<B, R>;
    #[inline]
    fn sub(self, rhs: FBig<B, R>) -> Self::Output {
        self.add(&(-rhs))
    }
}

impl<R: Round> Context<R> {
    // lhs + rhs, assuming lhs.exponent > rhs.exponent
    pub(crate) fn repr_add_on_lhs<const B: Word>(&self, lhs: Repr<B>, rhs: &Repr<B>) -> Approximation<Repr<B>, Rounding> {
        // TODO: debug_assert!(lhs.exponent >= rhs.exponent);
        let (mut lhs, rhs) = if lhs.exponent > rhs.exponent {
            (lhs, rhs)
        } else {
            (rhs.clone(), &lhs)
        };
        // TODO: move this shortcut
        if rhs.is_zero() {
            return Approximation::Exact(lhs);
        }

        let is_sub = lhs.significand.sign() != rhs.significand.sign();
        let rnd_precision = self.precision + is_sub as usize; // use one extra digit to prevent cancellation in rounding

        let ediff = (lhs.exponent - rhs.exponent) as usize;
        let ldigits = lhs.digits();
        let rdigits_est = rhs.significand.log2_bounds().1 / Repr::<B>::BASE.log2_bounds().0; // TODO: make this separate functions, e.g. digit_len_ub?
        let rdigits_est = rdigits_est as usize + 1; // overestimate

        // align the exponent
        let mut low: (IBig, usize); // (value of low part, precision of the low part)
        let (mut significand, mut exponent) = if rdigits_est + 1 < ediff && rdigits_est + 1 + self.precision < ldigits + ediff {
            // if rhs is much smaller than lhs, direct round on the rhs
            /*
             * lhs: |=========|
             * rhs:                  |========|
             *                |<--- ediff --->|
             *      |< precision >|
             */

            // In this case, the actual significand of rhs doesn't matter,
            // we can just replace it with 1 for correct rounding
            let low_prec = if ldigits >= rnd_precision {
                2
            } else {
                (rnd_precision - ldigits) + 1
            };
            low = (rhs.significand.signum(), low_prec);
            (lhs.significand, lhs.exponent)
        } else if ldigits >= self.precision {
            // if the lhs already exceeds the desired precision, just align rhs
            /* Before:
             * lhs: |==============|
             * rhs:      |==============|
             *              ediff  |<-->|
             *    precision  |<--->|
             * 
             * After:
             * lhs: |==============|
             * rhs:      |=========|xxxx|
             */
            let (rhs_signif, r) = shr_rem_radix::<B>(&rhs.significand, ediff);
            low = (r, ediff);
            (lhs.significand + rhs_signif, lhs.exponent)
        } else if ediff + ldigits > self.precision {
            // if the shifted lhs exceeds the desired precision, align lhs and rhs to precision
            /* Before:
             * lhs: |=========|
             * rhs:      |==============|
             *                |< ediff >|
             *      |< precision >|
             * 
             * After:
             * lhs: |=========0000|
             * rhs:      |========|xxxxx|
             *        lshift  |<->|
             *            rshift  |<--->|
             */
            let lshift = self.precision - ldigits;
            let rshift = ediff - lshift;
            let (rhs_signif, r) = shr_rem_radix::<B>(&rhs.significand, rshift);
            shl_radix_in_place::<B>(&mut lhs.significand, lshift);

            low = (r, rshift);
            (lhs.significand + rhs_signif, lhs.exponent - lshift as isize)
        } else {
            // otherwise directly shift lhs to required position
            /* Before:
             * lhs: |==========|
             * rhs:       |==============|
             *                 |< ediff >|
             *      |<------ precision ----->|
             * 
             * After:
             * lhs: |==========0000000000|
             * rhs:       |==============|
             */
            shl_radix_in_place::<B>(&mut lhs.significand, ediff);
            low = (IBig::ZERO, 0);
            (lhs.significand + &rhs.significand, rhs.exponent)
        };

        // align to precision again
        let digits = digit_len::<B>(&significand);
        match digits.cmp(&rnd_precision) {
            Ordering::Equal => {},
            Ordering::Greater => {
                // Shrink if the result has more digits than desired precision
                /* 
                 * lhs:         |=========0000|
                 * rhs:              |========|xxxxx|
                 * sum:        |==============|xxxxx|
                 * precision:  |<----->|
                 * shrink:     |=======|xxxxxxxxxxxx|
                 */
                let shift = digits - rnd_precision;
                let mut r = shr_rem_radix_in_place::<B>(&mut significand, shift);
                exponent += shift as isize;
                shl_radix_in_place::<B>(&mut r, low.1);
                low.0 += r;
                low.1 += shift;
            },
            Ordering::Less => {
                // Expand to low parts if the result has less digits than desired precision.
                // It's necessary only when lhs and rhs has different sign and a cancellation might happen.
                /* 
                 * lhs:  |=========0000|
                 * rhs:  |=============|xxxxx|
                 * sum:          |=====|xxxxx|
                 * precision+1:  |<------>|
                 * shift:              |<>|
                 * expanded:     |========|xx|
                 */
                if !low.0.is_zero() && is_sub {
                    let (low_val, low_prec) = low;
                    let shift = low_prec.min(rnd_precision - digits);
                    let (pad, low_val) = split_radix_at::<B>(low_val, low_prec - shift);
                    shl_radix_in_place::<B>(&mut significand, shift);
                    exponent -= shift as isize;
                    significand += pad;
                    low = (low_val, low_prec - shift);
                }
            }
        };

        // perform rounding
        if low.0.is_zero() {
            Approximation::Exact(Repr::new(significand, exponent))
        } else {
            // By now significand should have at least full precision. After adjustment, the digits length
            // could be one more than the precision. We don't shrink the extra digit.
            let adjust = R::round_fract::<B>(&significand, low.0, low.1);
            Approximation::InExact(Repr::new(significand + adjust, exponent), adjust)
        }
    }

    // lhs + rhs, assuming lhs.exponent < rhs.exponent
    pub(crate) fn repr_add_on_rhs<const B: Word>(&self, lhs: Repr<B>, rhs: &Repr<B>) -> Approximation<Repr<B>, Rounding> {
        // TODO(next): implement this after adding more tests
        debug_assert!(lhs.exponent <= rhs.exponent);
        unimplemented!()
    }

    pub fn add<const B: Word>(&self, lhs: &FBig<B, R>, rhs: &FBig<B, R>) -> Approximation<Repr<B>, Rounding> {
        unimplemented!()
    }
}