use crate::{
    Word,
    round::{Rounding, Round},
    repr::{Context, Repr},
    fbig::FBig,
    utils::{shl_radix, digit_len, shr_rem_radix, shr_rem_radix_in_place, shl_radix_in_place},
};
use core::{ops::{Add, Sub}, cmp::Ordering};

use dashu_base::{Approximation, DivRemAssign};
use dashu_int::IBig;

impl<const B: Word, R: Round> Add for FBig<B, R> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        let context = Context::max(self.context, rhs.context);
        FBig {
            repr: context.add(&self.repr, &rhs.repr).value(),
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
            repr: context.add(&self.repr, &-rhs.repr).value(),
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
            repr: context.add(&self.repr, &rhs.repr).value(),
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
    // TODO: let add take reference after implementing add_assign
    pub fn add<const B: Word>(&self, lhs: &Repr<B>, rhs: &Repr<B>) -> Approximation<Repr<B>, Rounding> {
        // put the oprand of lower exponent on the right
        let (lhs, rhs) = if lhs.exponent >= rhs.exponent {
            (lhs, rhs)
        } else {
            (rhs, lhs)
        };

        let ediff = (lhs.exponent - rhs.exponent) as usize;
        let ldigits = lhs.digits();
        // TODO: short cut if rhs is much smaller than lhs

        // align the exponent
        let mut low: IBig;
        let mut low_digits: usize;
        let (mut significand, mut exponent) = if ediff + ldigits > self.precision {
            // if the shifted lhs exceeds the desired precision, align lhs and rhs to precision
            /* Before:
             * lhs: |=========|
             * rhs:      |==============|
             *                |< ediff >|
             * After:
             * lhs: |=========0000|
             * rhs:      |========|xxxxx|
             *      |< precision >|
             *        lshift  |<->|
             *            rshift  |<--->|
             */
            // TODO: make this case standalone
            let lshift = if self.precision > ldigits { self.precision - ldigits } else { 0 };
            let rshift = ediff - lshift;
            let (rhs_signif, r) = shr_rem_radix::<B>(&rhs.significand, rshift);
            let lhs_signif = shl_radix::<B>(&lhs.significand, lshift);

            // do addition
            low = r;
            low_digits = rshift;
            (lhs_signif + rhs_signif, lhs.exponent - lshift as isize)
        } else {
            // otherwise directly shift lhs to required position
            /* Before:
             * lhs: |==========|
             * rhs:       |==============|
             *                 |< ediff >|
             * After:
             * lhs: |==========0000000000|
             * rhs:       |==============|
             *      |<------ precision ----->|
             */
            let lhs_signif = shl_radix::<B>(&lhs.significand, ediff);
            low = IBig::ZERO;
            low_digits = 0;
            (lhs_signif + &rhs.significand, rhs.exponent)
        };

        // align to precision again
        let digits = digit_len::<B>(&significand);
        match digits.cmp(&self.precision) {
            Ordering::Equal => {},
            Ordering::Greater => {
                // shrink if the result has more digits than desired precision
                /* 
                    * lhs:         |=========0000|
                    * rhs:              |========|xxxxx|
                    * sum:        |==============|xxxxx|
                    * precision:  |<----->|
                    * shrink:     |=======|xxxxxxxxxxxx|
                    */
                let shift = digits - self.precision;
                let r = shr_rem_radix_in_place::<B>(&mut significand, shift);
                exponent += shift as isize;
                low += r * Repr::<B>::BASE.pow(low_digits);
                low_digits += shift;
            },
            Ordering::Less => {
                // expand to low parts if the result has less digits than desired precision
                /* 
                    * lhs:  |=========0000|
                    * rhs:       |========|xxxxx|
                    * sum:          |=====|xxxxx|
                    * precision:    |<----->|
                    * expand:       |=======|xxx|
                    */
                let shift = low_digits.min(self.precision - digits);
                shl_radix_in_place::<B>(&mut significand, shift);
                let lower = shr_rem_radix_in_place::<B>(&mut low, low_digits - shift);
                significand += low;
                low = lower;
                low_digits = low_digits - shift;
            }
        };

        // perform rounding
        if low_digits == 0 {
            Approximation::Exact(Repr::new(significand, exponent))
        } else {
            let adjust = R::round_fract::<B>(&significand, low, low_digits);
            Approximation::InExact(Repr::new(significand + adjust, exponent), adjust)
        }
    }

    pub fn add_assign<const B: Word>(&self, lhs: &mut Repr<B>, rhs: &Repr<B>) -> Approximation<(), Rounding> {
        unimplemented!()
    }

    // TODO: add a separate sub, sub_assign function
}