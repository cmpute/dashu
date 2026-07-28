use crate::{
    error::{assert_finite_operands, FpError, FpResult},
    fbig::FBig,
    helper_macros,
    repr::{Context, Repr, Word},
    round::{Round, Rounded},
    utils::{digit_len, shl_digits, shl_digits_in_place, split_digits, split_digits_ref},
};
use core::{
    cmp::Ordering,
    ops::{Add, AddAssign, Sub, SubAssign},
};

use dashu_base::Sign::{self, *};
use dashu_int::{IBig, UBig};

/// Build a `Repr` from a cancellation result, producing `-0` (instead of `+0`) when the
/// significand is zero and the rounding mode is roundTowardNegative (IEEE 754 §6.3).
pub(crate) fn cancel_zero<R: Round, const B: Word>(sig: IBig, exp: isize) -> Repr<B> {
    if sig.is_zero() && R::IS_ROUND_TOWARD_NEGATIVE {
        Repr::neg_zero()
    } else {
        Repr::new(sig, exp)
    }
}

impl<R: Round, const B: Word> Add for FBig<R, B> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        let context = Context::max(self.context, rhs.context);
        let sum = context.addsub_vv(self.repr, rhs.repr, Positive);
        FBig::new(sum.value(), context)
    }
}

impl<R: Round, const B: Word> Add<&FBig<R, B>> for FBig<R, B> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: &FBig<R, B>) -> Self::Output {
        let context = Context::max(self.context, rhs.context);
        let sum = context.addsub_vr(self.repr, &rhs.repr, Positive);
        FBig::new(sum.value(), context)
    }
}

impl<R: Round, const B: Word> Add<FBig<R, B>> for &FBig<R, B> {
    type Output = FBig<R, B>;

    #[inline]
    fn add(self, rhs: FBig<R, B>) -> Self::Output {
        let context = Context::max(self.context, rhs.context);
        let sum = context.addsub_rv(&self.repr, rhs.repr, Positive);
        FBig::new(sum.value(), context)
    }
}

impl<R: Round, const B: Word> Add<&FBig<R, B>> for &FBig<R, B> {
    type Output = FBig<R, B>;

    #[inline]
    fn add(self, rhs: &FBig<R, B>) -> Self::Output {
        let context = Context::max(self.context, rhs.context);
        let sum = context.addsub_rr(&self.repr, &rhs.repr, Positive);
        FBig::new(sum.value(), context)
    }
}

impl<R: Round, const B: Word> Sub for FBig<R, B> {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        let context = Context::max(self.context, rhs.context);
        let sum = context.addsub_vv(self.repr, rhs.repr, Negative);
        FBig::new(sum.value(), context)
    }
}

impl<R: Round, const B: Word> Sub<&FBig<R, B>> for FBig<R, B> {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: &FBig<R, B>) -> Self::Output {
        let context = Context::max(self.context, rhs.context);
        let sum = context.addsub_vr(self.repr, &rhs.repr, Negative);
        FBig::new(sum.value(), context)
    }
}

impl<R: Round, const B: Word> Sub<FBig<R, B>> for &FBig<R, B> {
    type Output = FBig<R, B>;

    #[inline]
    fn sub(self, rhs: FBig<R, B>) -> Self::Output {
        let context = Context::max(self.context, rhs.context);
        let sum = context.addsub_rv(&self.repr, rhs.repr, Negative);
        FBig::new(sum.value(), context)
    }
}

impl<R: Round, const B: Word> Sub<&FBig<R, B>> for &FBig<R, B> {
    type Output = FBig<R, B>;

    #[inline]
    fn sub(self, rhs: &FBig<R, B>) -> Self::Output {
        let context = Context::max(self.context, rhs.context);
        let sum = context.addsub_rr(&self.repr, &rhs.repr, Negative);
        FBig::new(sum.value(), context)
    }
}

helper_macros::impl_binop_assign_by_taking!(impl AddAssign<Self>, add_assign, add);
helper_macros::impl_binop_assign_by_taking!(impl SubAssign<Self>, sub_assign, sub);

macro_rules! impl_add_sub_primitive_with_fbig {
    ($($t:ty)*) => {$(
        helper_macros::impl_binop_with_primitive!(impl Add<$t>, add);
        helper_macros::impl_binop_assign_with_primitive!(impl AddAssign<$t>, add_assign);
        helper_macros::impl_binop_with_primitive!(impl Sub<$t>, sub);
        helper_macros::impl_binop_assign_with_primitive!(impl SubAssign<$t>, sub_assign);
    )*};
}
impl_add_sub_primitive_with_fbig!(u8 u16 u32 u64 u128 usize UBig i8 i16 i32 i64 i128 isize IBig);

impl<R: Round> Context<R> {
    /// Round sum = `significand * B ^ exponent` with the low part (value, precision).
    /// If the sum is actually from a subtraction and the low part is not zero, `is_sub` should be true.
    pub(crate) fn repr_round_sum<const B: Word>(
        &self,
        mut significand: IBig,
        mut exponent: isize,
        mut low: (IBig, usize),
        is_sub: bool,
    ) -> Rounded<Repr<B>> {
        // A zero produced by exact cancellation is -0 only under roundTowardNegative (Down),
        // +0 otherwise (IEEE 754 §6.3).
        let neg_cancel = is_sub && R::IS_ROUND_TOWARD_NEGATIVE;
        let make_repr = |sig: IBig, exp: isize| -> Repr<B> {
            if sig.is_zero() && neg_cancel {
                Repr::neg_zero()
            } else {
                Repr::new(sig, exp)
            }
        };

        if !self.is_limited() {
            // short cut for unlimited precision
            return Rounded::Exact(make_repr(significand, exponent));
        }

        // use one extra digit to prevent cancellation in rounding
        let rnd_precision = self.precision + is_sub as usize;

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
                let (signif_hi, mut signif_lo) = split_digits::<B>(significand, shift);
                significand = signif_hi;
                exponent += shift as isize;
                shl_digits_in_place::<B>(&mut signif_lo, low.1);
                low.0 += signif_lo;
                low.1 += shift;
            }
            Ordering::Less => {
                // Expand to low parts if the result has less digits than desired precision.
                /*
                 * A possible case when lhs and rhs have different sign:
                 * lhs:  |=========0000|
                 * rhs:  |=============|xxxxx|
                 * sum:          |=====|xxxxx|
                 * precision+1:  |<------>|
                 * shift:              |<>|
                 * expanded:     |========|xx|
                 */
                if !low.0.is_zero() {
                    let (low_val, low_prec) = low;
                    let shift = low_prec.min(rnd_precision - digits);
                    let (pad, low_val) = split_digits::<B>(low_val, low_prec - shift);
                    shl_digits_in_place::<B>(&mut significand, shift);
                    exponent -= shift as isize;
                    significand += pad;
                    low = (low_val, low_prec - shift);
                }
            }
        };

        // perform rounding
        if low.0.is_zero() {
            Rounded::Exact(make_repr(significand, exponent))
        } else {
            // By now significand should have at least full precision. After adjustment, the digits length
            // could be one more than the precision. We don't shrink the extra digit.
            let adjust = R::round_fract::<B>(&significand, low.0, low.1);
            Rounded::Inexact(make_repr(significand + adjust, exponent), adjust)
        }
    }

    // lhs + rhs_sign * rhs, assuming lhs.exponent >= rhs.exponent
    pub(crate) fn repr_add_large_small<const B: Word>(
        &self,
        mut lhs: Repr<B>,
        rhs: &Repr<B>,
        rhs_sign: Sign,
    ) -> Rounded<Repr<B>> {
        debug_assert!(lhs.exponent >= rhs.exponent);

        // use one extra digit when subtracting to prevent cancellation in rounding
        let is_sub = lhs.significand.sign() != rhs_sign * rhs.significand.sign();
        let rnd_precision = self.precision + is_sub as usize;

        let ediff = (lhs.exponent - rhs.exponent) as usize;
        let ldigits = lhs.digits();
        let rdigits_est = rhs.digits_ub(); // overestimate

        // align the exponent
        let low: (IBig, usize); // (value of low part, precision of the low part)
        let (significand, exponent) =
            if self.is_limited() && is_sub && rdigits_est + self.precision >= ldigits + ediff {
                // The smaller operand (`rhs`, lower exponent) reaches the larger
                // operand's `precision`-digit window — its top digit is at or above the
                // window edge (`rdigits + precision >= ldigits + ediff`, i.e. `rhs`'s top
                // position `rdigits - ediff` is `>= ldigits - precision`). An effective
                // subtraction can then cancel and lose leading digits, which the trimmed
                // path cannot recover (its single re-expand in `repr_round_sum` collapses
                // a genuinely small difference to the wrong value — e.g. `1.00 -
                // 0.99999999` at precision 3 to `0` instead of `1e-8`, or `0.5 - 0.4375`
                // at precision 1 to `0` instead of `0.0625`). So form the exact difference
                // at full operand width and let the shared `repr_round_sum` round it once
                // (with the same guard digit as the trimmed path, so no low tail is
                // needed). The complement (`<`) is the trimmed/negligible region where
                // `rhs` stays strictly below the window and no cancellation is possible.
                shl_digits_in_place::<B>(&mut lhs.significand, ediff);
                low = (IBig::ZERO, 0);
                match rhs_sign {
                    Positive => (lhs.significand + &rhs.significand, rhs.exponent),
                    Negative => (lhs.significand - &rhs.significand, rhs.exponent),
                }
            } else if self.is_limited()
                && rdigits_est + 1 < ediff
                && rdigits_est + 1 + rnd_precision < ldigits + ediff
            {
                // rhs is entirely below lhs's rounding window, so only its sign
                // contributes to the rounding; replace it with a unit sticky tail
                // (`|low| = 1`).
                //
                // The sticky must be positioned at rhs's *real* magnitude, i.e. `ediff`
                // digits below lhs's exponent — NOT at `precision - ldigits`. Positioning
                // by `ediff` keeps the sticky genuinely sub-ULP (|1| << B^ediff, and the
                // branch guard guarantees ediff >= 3), so it can never land on a rounding
                // tie. Positioning by `precision - ldigits` instead let the re-expand drag
                // the sticky up to the LSB, where for base 2 + round-to-nearest it equals
                // exactly half (1 == B^0 == ½·B^1) and injected a spurious ULP — e.g.
                // `1 + 2^-100` at precision 10 returned `513·2^-9` instead of `1`.
                low = (rhs_sign * rhs.significand.signum(), ediff);
                (lhs.significand, lhs.exponent)
            } else if self.is_limited() && ldigits >= self.precision {
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
                let (rhs_signif, r) = split_digits_ref::<B>(&rhs.significand, ediff);
                low = (rhs_sign * r, ediff);
                (lhs.significand + rhs_sign * rhs_signif, lhs.exponent)
            } else if self.is_limited() && ediff + ldigits > self.precision {
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
                let (rhs_signif, r) = split_digits_ref::<B>(&rhs.significand, rshift);
                shl_digits_in_place::<B>(&mut lhs.significand, lshift);

                low = (rhs_sign * r, rshift);
                (lhs.significand + rhs_sign * rhs_signif, lhs.exponent - lshift as isize)
            } else {
                // otherwise directly shift lhs to required position
                /* Before:
                 * lhs: |==========|
                 * rhs:       |==============|
                 *                 |< ediff >|
                 *      |<------ precision ------>|
                 *
                 * After:
                 * lhs: |==========0000000000|
                 * rhs:       |==============|
                 */
                shl_digits_in_place::<B>(&mut lhs.significand, ediff);
                low = (IBig::ZERO, 0);
                match rhs_sign {
                    Positive => (lhs.significand + &rhs.significand, rhs.exponent),
                    Negative => (lhs.significand - &rhs.significand, rhs.exponent),
                }
            };

        self.repr_round_sum(significand, exponent, low, is_sub)
    }

    // lhs + rhs_sign * rhs, assuming lhs.exponent <= rhs.exponent
    pub(crate) fn repr_add_small_large<const B: Word>(
        &self,
        lhs: Repr<B>,
        rhs: &Repr<B>,
        rhs_sign: Sign,
    ) -> Rounded<Repr<B>> {
        debug_assert!(lhs.exponent <= rhs.exponent);

        // the following implementation should be exactly the same as `repr_add_large_small`
        // other than lhs and rhs are swapped. See `repr_add_large_small` for full documentation
        let is_sub = lhs.significand.sign() != rhs_sign * rhs.significand.sign();
        let rnd_precision = self.precision + is_sub as usize;

        let ediff = (rhs.exponent - lhs.exponent) as usize;
        let rdigits = rhs.digits();
        let ldigits_est = lhs.digits_ub();

        // align the exponent
        let low: (IBig, usize);
        let (significand, exponent) =
            if self.is_limited() && is_sub && ldigits_est + self.precision >= rdigits + ediff {
                // Symmetric counterpart of the guard in `repr_add_large_small` (see there
                // for the rationale); here the lower-exponent operand is `lhs`. Form the
                // exact difference at full operand width and let the shared
                // `repr_round_sum` round it once.
                let rhs_signif = shl_digits::<B>(&rhs.significand, ediff);
                low = (IBig::ZERO, 0);
                (rhs_sign * rhs_signif + lhs.significand, lhs.exponent)
            } else if self.is_limited()
                && ldigits_est + 1 < ediff
                && ldigits_est + 1 + rnd_precision < rdigits + ediff
            {
                // lhs is entirely below rhs's rounding window, so only its sign
                // contributes; replace it with a unit sticky tail positioned at lhs's
                // real magnitude (`ediff` digits below rhs's exponent). See
                // `repr_add_large_small` for why the position must be `ediff` and not
                // `precision - rdigits`.
                low = (lhs.significand.signum(), ediff);
                (rhs_sign * rhs.significand.clone(), rhs.exponent)
            } else if self.is_limited() && rdigits >= self.precision {
                // if the rhs already exceeds the desired precision, just align lhs
                let (lhs_signif, r) = split_digits::<B>(lhs.significand, ediff);
                low = (r, ediff);
                match rhs_sign {
                    Positive => (lhs_signif + &rhs.significand, rhs.exponent),
                    Negative => (lhs_signif - &rhs.significand, rhs.exponent),
                }
            } else if self.is_limited() && ediff + rdigits > self.precision {
                // if the shifted rhs exceeds the desired precision, align lhs and rhs to precision
                let lshift = self.precision - rdigits;
                let rshift = ediff - lshift;
                let (lhs_signif, r) = split_digits::<B>(lhs.significand, rshift);
                let rhs_signif = shl_digits::<B>(&rhs.significand, lshift);

                low = (r, rshift);
                (rhs_sign * rhs_signif + lhs_signif, rhs.exponent - lshift as isize)
            } else {
                // otherwise directly shift rhs to required position
                let rhs_signif = shl_digits::<B>(&rhs.significand, ediff);
                low = (IBig::ZERO, 0);
                (rhs_sign * rhs_signif + lhs.significand, lhs.exponent)
            };

        self.repr_round_sum(significand, exponent, low, is_sub)
    }

    /// Add or subtract two finite floats, consuming both operands.
    ///
    /// Computes `lhs + rhs_sign · rhs`: [`Sign::Positive`] adds, [`Sign::Negative`]
    /// subtracts. This is the low-level ownership-aware kernel shared by the
    /// `+`/`-` operators and [`add`](Self::add)/[`sub`](Self::sub); unlike those it
    /// returns the raw rounded [`Repr`] (no [`FBig`] wrapping, no [`Result`]) and
    /// reuses the owned significand buffer of `lhs`. The `_vr`/`_rv`/`_rr`
    /// siblings cover the other ownership combinations and share this contract.
    ///
    /// # Panics
    ///
    /// Panics if either operand is infinite (matching the other `repr_*` kernels).
    ///
    /// # Examples
    ///
    /// ```
    /// use dashu_base::Sign::*;
    /// use dashu_float::{Context, Repr, round::mode::HalfEven};
    /// use dashu_int::IBig;
    ///
    /// let ctx = Context::<HalfEven>::new(4);
    /// let a = Repr::<10>::new(IBig::from(1234), -3); // 1.234
    /// let b = Repr::<10>::new(IBig::from(5678), -4); // 0.5678
    /// // 1.234 + 0.5678 = 1.8018, rounds (HalfEven, prec 4) to 1.802
    /// assert_eq!(ctx.addsub_vv(a.clone(), b.clone(), Positive).value(),
    ///            Repr::<10>::new(IBig::from(1802), -3));
    /// // 1.234 - 0.5678 = 0.6662 (exact at prec 4)
    /// assert_eq!(ctx.addsub_vv(a, b, Negative).value(),
    ///            Repr::<10>::new(IBig::from(6662), -4));
    /// ```
    pub fn addsub_vv<const B: Word>(
        &self,
        lhs: Repr<B>,
        rhs: Repr<B>,
        rhs_sign: Sign,
    ) -> Rounded<Repr<B>> {
        assert_finite_operands(&lhs, &rhs);
        if lhs.is_pos_zero() {
            // With rhs_sign = Negative, round `-rhs` directly rather than negating
            // *after* rounding. For the asymmetric modes (Up = toward +∞, Down =
            // toward −∞), `round(-x) != -round(x)`: rounding `rhs` toward +∞ then
            // negating rounds in the wrong direction, so `0 - rhs` would land one
            // ULP off (truncated instead of rounded away from the result). This
            // applies to every `addsub_*` variant's lhs-zero path.
            self.repr_round(match rhs_sign {
                Positive => rhs,
                Negative => Repr::new(-rhs.significand, rhs.exponent),
            })
        } else if rhs.is_pos_zero() {
            self.repr_round(lhs)
        } else {
            match lhs.exponent.cmp(&rhs.exponent) {
                Ordering::Equal => {
                    let sig = match rhs_sign {
                        Positive => lhs.significand + &rhs.significand,
                        Negative => lhs.significand - &rhs.significand,
                    };
                    self.repr_round(cancel_zero::<R, B>(sig, lhs.exponent))
                }
                Ordering::Greater => self.repr_add_large_small(lhs, &rhs, rhs_sign),
                Ordering::Less => self.repr_add_small_large(lhs, &rhs, rhs_sign),
            }
        }
    }

    /// Like [`addsub_vv`](Self::addsub_vv) but with the right operand borrowed
    /// (the left operand is consumed). See `addsub_vv` for the full contract.
    pub fn addsub_vr<const B: Word>(
        &self,
        lhs: Repr<B>,
        rhs: &Repr<B>,
        rhs_sign: Sign,
    ) -> Rounded<Repr<B>> {
        assert_finite_operands(&lhs, rhs);
        if lhs.is_pos_zero() {
            self.repr_round(match rhs_sign {
                Positive => rhs.clone(),
                Negative => Repr::new(-&rhs.significand, rhs.exponent),
            })
        } else if rhs.is_pos_zero() {
            self.repr_round(lhs)
        } else {
            match lhs.exponent.cmp(&rhs.exponent) {
                Ordering::Equal => {
                    let sig = match rhs_sign {
                        Positive => lhs.significand + &rhs.significand,
                        Negative => lhs.significand - &rhs.significand,
                    };
                    self.repr_round(cancel_zero::<R, B>(sig, lhs.exponent))
                }
                Ordering::Greater => self.repr_add_large_small(lhs, rhs, rhs_sign),
                Ordering::Less => self.repr_add_small_large(lhs, rhs, rhs_sign),
            }
        }
    }

    /// Like [`addsub_vv`](Self::addsub_vv) but with the left operand borrowed and
    /// the right operand consumed. See `addsub_vv` for the full contract.
    pub fn addsub_rv<const B: Word>(
        &self,
        lhs: &Repr<B>,
        mut rhs: Repr<B>,
        rhs_sign: Sign,
    ) -> Rounded<Repr<B>> {
        assert_finite_operands(lhs, &rhs);
        // Bake the sign into the owned rhs so the kernel — which takes its first
        // operand by value — can move `rhs` into that slot with `Positive`. This
        // is the mirror of `addsub_vv`'s owned-lhs path; the value is identical.
        rhs.significand *= rhs_sign;
        if lhs.is_pos_zero() {
            self.repr_round(rhs)
        } else if rhs.is_pos_zero() {
            self.repr_round_ref(lhs)
        } else {
            match lhs.exponent.cmp(&rhs.exponent) {
                Ordering::Equal => self.repr_round(cancel_zero::<R, B>(
                    &lhs.significand + rhs.significand,
                    lhs.exponent,
                )),
                Ordering::Greater => self.repr_add_small_large(rhs, lhs, Positive),
                Ordering::Less => self.repr_add_large_small(rhs, lhs, Positive),
            }
        }
    }

    /// Like [`addsub_vv`](Self::addsub_vv) but with both operands borrowed (the
    /// larger-exponent operand is cloned once for in-place alignment). See
    /// `addsub_vv` for the full contract.
    pub fn addsub_rr<const B: Word>(
        &self,
        lhs: &Repr<B>,
        rhs: &Repr<B>,
        rhs_sign: Sign,
    ) -> Rounded<Repr<B>> {
        assert_finite_operands(lhs, rhs);
        if lhs.is_pos_zero() {
            match rhs_sign {
                Positive => self.repr_round_ref(rhs),
                Negative => self.repr_round_ref(&Repr::new(-&rhs.significand, rhs.exponent)),
            }
        } else if rhs.is_pos_zero() {
            self.repr_round_ref(lhs)
        } else {
            match lhs.exponent.cmp(&rhs.exponent) {
                Ordering::Equal => {
                    let sig = match rhs_sign {
                        Positive => &lhs.significand + &rhs.significand,
                        Negative => &lhs.significand - &rhs.significand,
                    };
                    self.repr_round(cancel_zero::<R, B>(sig, lhs.exponent))
                }
                Ordering::Greater => self.repr_add_large_small(lhs.clone(), rhs, rhs_sign),
                Ordering::Less => self.repr_add_small_large(lhs.clone(), rhs, rhs_sign),
            }
        }
    }

    /// Add two floating point numbers under this context.
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
    /// let b = DBig::from_str("6.789")?;
    /// assert_eq!(context.add(&a.repr(), &b.repr()), Ok(Inexact(DBig::from_str("8.0")?, NoOp)));
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn add<const B: Word>(&self, lhs: &Repr<B>, rhs: &Repr<B>) -> FpResult<FBig<R, B>> {
        if lhs.is_infinite() || rhs.is_infinite() {
            return Err(FpError::InfiniteInput);
        }
        Ok(self
            .addsub_rr(lhs, rhs, Positive)
            .map(|v| FBig::new(v, *self)))
    }

    /// Subtract two floating point numbers under this context.
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
    /// let b = DBig::from_str("6.789")?;
    /// assert_eq!(
    ///     context.sub(&a.repr(), &b.repr()),
    ///     Ok(Inexact(DBig::from_str("-5.6")?, SubOne))
    /// );
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn sub<const B: Word>(&self, lhs: &Repr<B>, rhs: &Repr<B>) -> FpResult<FBig<R, B>> {
        if lhs.is_infinite() || rhs.is_infinite() {
            return Err(FpError::InfiniteInput);
        }
        Ok(self
            .addsub_rr(lhs, rhs, Negative)
            .map(|v| FBig::new(v, *self)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::round::mode::{HalfAway, HalfEven};

    // Build a normalized Repr from a small integer significand and an exponent.
    fn r<const B: Word>(sig: i128, exp: isize) -> Repr<B> {
        Repr::new(IBig::from(sig), exp)
    }

    // Severe cancellation must not collapse a genuinely small difference to 0.
    // Pristine returned `0` for the first two rows: the trimmed alignment path
    // keeps only a bounded low tail and its single re-expand can't recover the
    // lost leading digits.
    #[test]
    fn sub_severe_cancellation_decimal() {
        let ctx = Context::<HalfAway>::new(3);
        // 1.00 - 0.99999999 = 1e-8 (exactly representable at precision 3)
        assert_eq!(
            ctx.sub(&r::<10>(100, -2), &r::<10>(99999999, -8))
                .unwrap()
                .value()
                .repr(),
            &r::<10>(1, -8)
        );
        // 1.00 - 0.99950001 = 4.9999e-4, rounds to 5.00e-4 (HalfAway)
        assert_eq!(
            ctx.sub(&r::<10>(100, -2), &r::<10>(99950001, -8))
                .unwrap()
                .value()
                .repr(),
            &r::<10>(500, -6)
        );
    }

    #[test]
    fn sub_severe_cancellation_binary() {
        let ctx = Context::<HalfEven>::new(10);
        // 2^20 - (2^20 - 1) = 1, with the operands 20 exponent positions apart
        assert_eq!(
            ctx.sub(&r::<2>(1, 20), &r::<2>((1i128 << 20) - 1, 0))
                .unwrap()
                .value()
                .repr(),
            &r::<2>(1, 0)
        );
        // same magnitude gap but the smaller-exponent operand is on the left
        assert_eq!(
            ctx.sub(&r::<2>((1i128 << 20) - 1, 0), &r::<2>(1, 20))
                .unwrap()
                .value()
                .repr(),
            &r::<2>(-1, 0)
        );
        // 2^30 - (2^30 - 1) = 1
        assert_eq!(
            ctx.sub(&r::<2>(1, 30), &r::<2>((1i128 << 30) - 1, 0))
                .unwrap()
                .value()
                .repr(),
            &r::<2>(1, 0)
        );
    }

    // Effective subtraction reached through `Context::add` (opposite signs) must
    // be fixed as well.
    #[test]
    fn add_effective_severe_cancellation() {
        let ctx = Context::<HalfEven>::new(10);
        // 2^20 + (-(2^20 - 1)) = 1
        assert_eq!(
            ctx.add(&r::<2>(1, 20), &r::<2>(-((1i128 << 20) - 1), 0))
                .unwrap()
                .value()
                .repr(),
            &r::<2>(1, 0)
        );
    }

    // The public operator path (`a - b`) routes through the same kernel.
    #[test]
    fn sub_operator_severe_cancellation() {
        let a = FBig::<HalfEven, 2>::from_parts(IBig::from(1), 20);
        let b = FBig::<HalfEven, 2>::from_parts(IBig::from((1i128 << 20) - 1), 0);
        assert_eq!((a - b).repr(), &r::<2>(1, 0));
    }

    // Mild subtractions — the smaller operand stays below the larger's precision
    // window — must keep their existing behavior and not be diverted to the
    // full-width path.
    #[test]
    fn sub_mild_unchanged() {
        let ctx = Context::<HalfAway>::new(3);
        // 101 - 0.2 = 100.8, kept as 1008 * 10^-1 (one guard digit, as before)
        assert_eq!(
            ctx.sub(&r::<10>(101, 0), &r::<10>(2, -1))
                .unwrap()
                .value()
                .repr(),
            &r::<10>(1008, -1)
        );
    }

    // Regression for the branch-1 signum-proxy bug (SUM-BUG.md §2c): when the larger
    // operand has fewer digits than the precision and a negligible operand is added,
    // the sticky proxy must be positioned at the operand's *real* magnitude (`ediff`),
    // not at `precision - ldigits`. The old positioning let the re-expand drag the
    // sticky up to the LSB, where for base 2 + round-to-nearest it equals exactly half
    // and injected a spurious ULP: `1 + 2^-100` at precision 10 gave `513*2^-9` (=
    // 1.00195…) instead of `1`.
    #[test]
    fn add_negligible_short_operand_no_spurious_ulp() {
        // base 2 + HalfAway: the exact tie case
        let ctx = Context::<HalfAway>::new(10);
        assert_eq!(
            ctx.add(&r::<2>(1, 0), &r::<2>(1, -100))
                .unwrap()
                .value()
                .repr(),
            &r::<2>(1, 0)
        );
        assert_eq!(
            ctx.sub(&r::<2>(1, 0), &r::<2>(1, -100))
                .unwrap()
                .value()
                .repr(),
            &r::<2>(1, 0)
        );
        // larger short operand (digits < precision), negligible addend
        let ctx = Context::<HalfAway>::new(50);
        assert_eq!(
            ctx.add(&r::<2>(0x12345, 0), &r::<2>(1, -200))
                .unwrap()
                .value()
                .repr(),
            &r::<2>(0x12345, 0)
        );
        // base 10 was never affected (1 < ½·10), but check it stays correct
        let ctx = Context::<HalfAway>::new(10);
        assert_eq!(
            ctx.add(&r::<10>(1, 0), &r::<10>(1, -100))
                .unwrap()
                .value()
                .repr(),
            &r::<10>(1, 0)
        );
    }

    // The four ownership-aware `addsub_*` variants must produce the same value as
    // each other (and therefore as `Context::add`/`sub`, which delegate to
    // `addsub_rr`) across every input shape. The absolute correctness of
    // `addsub_rr` is covered by the `ctx.add`/`ctx.sub` tests above; this guards
    // that consuming vs borrowing an operand never changes the result.
    fn assert_addsub_variants_agree<R: Round, const B: Word>(
        ctx: &Context<R>,
        a: &Repr<B>,
        b: &Repr<B>,
    ) {
        let add = ctx.addsub_rr(a, b, Positive).value();
        let sub = ctx.addsub_rr(a, b, Negative).value();
        assert_eq!(&ctx.addsub_vv(a.clone(), b.clone(), Positive).value(), &add);
        assert_eq!(&ctx.addsub_vr(a.clone(), b, Positive).value(), &add);
        assert_eq!(&ctx.addsub_rv(a, b.clone(), Positive).value(), &add);
        assert_eq!(&ctx.addsub_vv(a.clone(), b.clone(), Negative).value(), &sub);
        assert_eq!(&ctx.addsub_vr(a.clone(), b, Negative).value(), &sub);
        assert_eq!(&ctx.addsub_rv(a, b.clone(), Negative).value(), &sub);
    }

    #[test]
    fn addsub_variants_agree() {
        let ctx = Context::<HalfAway>::new(3);
        // lhs has the larger exponent
        assert_addsub_variants_agree(&ctx, &r::<10>(1234, -2), &r::<10>(567, -5));
        // rhs has the larger exponent
        assert_addsub_variants_agree(&ctx, &r::<10>(567, -5), &r::<10>(1234, -2));
        // equal exponents
        assert_addsub_variants_agree(&ctx, &r::<10>(123, -2), &r::<10>(456, -2));
        // lhs is +0 (exercises the lhs-zero short-circuit in every variant)
        assert_addsub_variants_agree(&ctx, &r::<10>(0, 0), &r::<10>(123, -2));
        // rhs is +0
        assert_addsub_variants_agree(&ctx, &r::<10>(123, -2), &r::<10>(0, 0));
        // a negative operand (effective subtraction reached through `add`)
        assert_addsub_variants_agree(&ctx, &r::<10>(-123, -2), &r::<10>(456, -2));
    }

    // Severe cancellation must recover through every ownership path — especially
    // `addsub_rv`, which bakes the sign into the owned rhs and routes it as the
    // kernel's by-value (first) operand, a structurally different path from
    // vv/vr/rr.
    #[test]
    fn addsub_severe_cancellation_all_variants() {
        let ctx = Context::<HalfEven>::new(10);
        let big = r::<2>(1, 20); // 2^20
        let near = r::<2>((1i128 << 20) - 1, 0); // 2^20 - 1
                                                 // 2^20 - (2^20 - 1) = 1
        assert_eq!(&ctx.addsub_vv(big.clone(), near.clone(), Negative).value(), &r::<2>(1, 0));
        assert_eq!(&ctx.addsub_vr(big.clone(), &near, Negative).value(), &r::<2>(1, 0));
        assert_eq!(&ctx.addsub_rr(&big, &near, Negative).value(), &r::<2>(1, 0));
        // reversed: (2^20 - 1) - 2^20 = -1, through both rhs-owned and lhs-owned paths
        assert_eq!(&ctx.addsub_rv(&near, big.clone(), Negative).value(), &r::<2>(-1, 0));
        assert_eq!(&ctx.addsub_vv(near, big, Negative).value(), &r::<2>(-1, 0));
    }
}
