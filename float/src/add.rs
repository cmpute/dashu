use crate::{
    fbig::FBig,
    repr::{Context, Repr},
    round::{Round, Rounding},
    utils::{
        digit_len, shl_radix, shl_radix_in_place, shr_rem_radix, shr_rem_radix_in_place,
        split_radix_at,
    },
    Word,
};
use core::{
    cmp::Ordering,
    ops::{Add, AddAssign, Sub},
};

use dashu_base::Approximation;
use dashu_int::IBig;

impl<const B: Word, R: Round> Add for FBig<B, R> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        let context = Context::max(self.context, rhs.context);
        if self.repr.is_zero() {
            return FBig::new_raw(rhs.repr, context);
        }
        if rhs.repr.is_zero() {
            return FBig::new_raw(self.repr, context);
        }

        let sum = match self.repr.exponent.cmp(&rhs.repr.exponent) {
            Ordering::Equal => context
                .round(Repr::new(self.repr.significand + rhs.repr.significand, self.repr.exponent)),
            Ordering::Greater => context.repr_add_large_small(self.repr, &rhs.repr),
            Ordering::Less => context.repr_add_small_large(self.repr, &rhs.repr),
        };
        FBig::new_raw(sum.value(), context)
    }
}

impl<'r, const B: Word, R: Round> Add<&'r FBig<B, R>> for FBig<B, R> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: &FBig<B, R>) -> Self::Output {
        let context = Context::max(self.context, rhs.context);
        if self.repr.is_zero() {
            return FBig::new_raw(rhs.repr.clone(), context);
        }
        if rhs.repr.is_zero() {
            return FBig::new_raw(self.repr, context);
        }

        let sum = match self.repr.exponent.cmp(&rhs.repr.exponent) {
            Ordering::Equal => context.round(Repr::new(
                self.repr.significand + &rhs.repr.significand,
                self.repr.exponent,
            )),
            Ordering::Greater => context.repr_add_large_small(self.repr, &rhs.repr),
            Ordering::Less => context.repr_add_small_large(self.repr, &rhs.repr),
        };
        FBig::new_raw(sum.value(), context)
    }
}

impl<'l, const B: Word, R: Round> Add<FBig<B, R>> for &'l FBig<B, R> {
    type Output = FBig<B, R>;

    #[inline]
    fn add(self, rhs: FBig<B, R>) -> Self::Output {
        let context = Context::max(self.context, rhs.context);
        if self.repr.is_zero() {
            return FBig::new_raw(rhs.repr, context);
        }
        if rhs.repr.is_zero() {
            return FBig::new_raw(self.repr.clone(), context);
        }

        let sum = match self.repr.exponent.cmp(&rhs.repr.exponent) {
            Ordering::Equal => context.round(Repr::new(
                &self.repr.significand + rhs.repr.significand,
                self.repr.exponent,
            )),
            Ordering::Greater => context.repr_add_small_large(rhs.repr, &self.repr),
            Ordering::Less => context.repr_add_large_small(rhs.repr, &self.repr),
        };
        FBig::new_raw(sum.value(), context)
    }
}

impl<'l, 'r, const B: Word, R: Round> Add<&'r FBig<B, R>> for &'l FBig<B, R> {
    type Output = FBig<B, R>;

    #[inline]
    fn add(self, rhs: &FBig<B, R>) -> Self::Output {
        let context = Context::max(self.context, rhs.context);
        if self.repr.is_zero() {
            return FBig::new_raw(rhs.repr.clone(), context);
        }
        if rhs.repr.is_zero() {
            return FBig::new_raw(self.repr.clone(), context);
        }

        let sum = match self.repr.exponent.cmp(&rhs.repr.exponent) {
            Ordering::Equal => context.round(Repr::new(
                &self.repr.significand + &rhs.repr.significand,
                self.repr.exponent,
            )),
            Ordering::Greater => context.repr_add_large_small(self.repr.clone(), &rhs.repr),
            Ordering::Less => context.repr_add_small_large(self.repr.clone(), &rhs.repr),
        };
        FBig::new_raw(sum.value(), context)
    }
}

impl<const B: Word, R: Round> AddAssign for FBig<B, R> {
    fn add_assign(&mut self, rhs: Self) {
        *self = core::mem::take(self) + rhs
    }
}
impl<const B: Word, R: Round> AddAssign<&FBig<B, R>> for FBig<B, R> {
    fn add_assign(&mut self, rhs: &FBig<B, R>) {
        *self = core::mem::take(self) + rhs
    }
}

impl<const B: Word, R: Round> Sub for FBig<B, R> {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        let context = Context::max(self.context, rhs.context);
        FBig {
            repr: context.repr_add_large_small(self.repr, &-rhs.repr).value(),
            context,
        }
    }
}

impl<const B: Word, R: Round> Sub for &FBig<B, R> {
    type Output = FBig<B, R>;
    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        // TODO(next): let repr_add_large_small accepts a rhs sign input
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
    /// Round sum = `significand * B ^ exponent` with the low part (value, precision).
    /// If the sum is actually from a subtraction and the low part is not zero, `is_sub` should be true.
    fn repr_round_sum<const B: Word>(
        &self,
        mut significand: IBig,
        mut exponent: isize,
        mut low: (IBig, usize),
        is_sub: bool,
    ) -> Approximation<Repr<B>, Rounding> {
        let rnd_precision = self.precision + is_sub as usize; // use one extra digit to prevent cancellation in rounding

        // align to precision again
        let digits = digit_len::<B>(&significand);
        match digits.cmp(&rnd_precision) {
            Ordering::Equal => {}
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
            }
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
            Approximation::Inexact(Repr::new(significand + adjust, exponent), adjust)
        }
    }

    // lhs + rhs, assuming lhs.exponent >= rhs.exponent
    pub(crate) fn repr_add_large_small<const B: Word>(
        &self,
        mut lhs: Repr<B>,
        rhs: &Repr<B>,
    ) -> Approximation<Repr<B>, Rounding> {
        debug_assert!(lhs.exponent >= rhs.exponent);

        // use one extra digit when subtracting to prevent cancellation in rounding
        let is_sub = lhs.significand.sign() != rhs.significand.sign();
        let rnd_precision = self.precision + is_sub as usize;

        let ediff = (lhs.exponent - rhs.exponent) as usize;
        let ldigits = lhs.digits();
        let rdigits_est = rhs.digits_ub(); // overestimate

        // align the exponent
        let low: (IBig, usize); // (value of low part, precision of the low part)
        let (significand, exponent) =
            if rdigits_est + 1 < ediff && rdigits_est + 1 + rnd_precision < ldigits + ediff {
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
                }; // low_prec >= 2
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

        self.repr_round_sum(significand, exponent, low, is_sub)
    }

    // lhs + rhs, assuming lhs.exponent <= rhs.exponent
    pub(crate) fn repr_add_small_large<const B: Word>(
        &self,
        mut lhs: Repr<B>,
        rhs: &Repr<B>,
    ) -> Approximation<Repr<B>, Rounding> {
        debug_assert!(lhs.exponent <= rhs.exponent);

        // the following implementation should be exactly the same as `repr_add_large_small`
        // other than lhs and rhs are swapped. See `repr_add_large_small` for full documentation
        let is_sub = lhs.significand.sign() != rhs.significand.sign();
        let rnd_precision = self.precision + is_sub as usize;

        let ediff = (rhs.exponent - lhs.exponent) as usize;
        let rdigits = rhs.digits();
        let ldigits_est = lhs.digits_ub();

        // align the exponent
        let low: (IBig, usize);
        let (significand, exponent) =
            if ldigits_est + 1 < ediff && ldigits_est + 1 + rnd_precision < rdigits + ediff {
                // if lhs is much smaller than rhs, direct round on the lhs
                let low_prec = if rdigits >= rnd_precision {
                    2
                } else {
                    (rnd_precision - rdigits) + 1
                };
                low = (lhs.significand.signum(), low_prec);
                (rhs.significand.clone(), rhs.exponent)
            } else if rdigits >= self.precision {
                // if the rhs already exceeds the desired precision, just align lhs
                let r = shr_rem_radix_in_place::<B>(&mut lhs.significand, ediff);
                low = (r, ediff);
                (&rhs.significand + lhs.significand, rhs.exponent)
            } else if ediff + rdigits > self.precision {
                // if the shifted rhs exceeds the desired precision, align lhs and rhs to precision
                let lshift = self.precision - rdigits;
                let rshift = ediff - lshift;
                let r = shr_rem_radix_in_place::<B>(&mut lhs.significand, rshift);
                let rhs_signif = shl_radix::<B>(&rhs.significand, lshift);

                low = (r, rshift);
                (rhs_signif + lhs.significand, rhs.exponent - lshift as isize)
            } else {
                // otherwise directly shift rhs to required position
                let rhs_signif = shl_radix::<B>(&rhs.significand, ediff);
                low = (IBig::ZERO, 0);
                (rhs_signif + lhs.significand, lhs.exponent)
            };

        self.repr_round_sum(significand, exponent, low, is_sub)
    }

    pub fn add<const B: Word>(
        &self,
        lhs: &FBig<B, R>,
        rhs: &FBig<B, R>,
    ) -> Approximation<FBig<B, R>, Rounding> {
        let context = Context::max(lhs.context, rhs.context);
        let sum = if lhs.repr.is_zero() {
            context.round(lhs.repr.clone())
        } else if rhs.repr.is_zero() {
            context.round(rhs.repr.clone())
        } else {
            match lhs.repr.exponent.cmp(&rhs.repr.exponent) {
                Ordering::Equal => context.round(Repr::new(
                    &lhs.repr.significand + &rhs.repr.significand,
                    lhs.repr.exponent,
                )),
                Ordering::Greater => context.repr_add_large_small(lhs.repr.clone(), &rhs.repr),
                Ordering::Less => context.repr_add_small_large(lhs.repr.clone(), &rhs.repr),
            }
        };
        sum.map(|v| FBig::new_raw(v, context))
    }
}
