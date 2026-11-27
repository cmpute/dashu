use crate::{
    error::{assert_finite_operands, assert_limited_precision, FpError, FpResult},
    fbig::FBig,
    helper_macros::{self, impl_binop_assign_by_taking},
    repr::{rounded_to_repr, Context, Repr, Word},
    round::{Round, Rounded, Rounding},
    utils::{
        ceil_usize, digit_len, shl_digits, shl_digits_in_place, short_window_words, split_digits,
        split_digits_ref,
    },
};
use core::cmp::Ordering;
use core::ops::{Div, DivAssign, Rem, RemAssign};
use dashu_base::{
    Approximation, BitTest, DivEuclid, DivRem, DivRemEuclid, EstimatedLog2, Inverse, RemEuclid,
    Sign, Signed, UnsignedAbs,
};
use dashu_int::{fast_div::ConstDivisor, high, modular::IntoRing, IBig, UBig};

/// Attach the dividend/divisor XOR sign to a zero quotient: the raw quotient significand is
/// `+0`, so the sign of a zero result (`0/finite`, or a finite/finite that rounds to zero) is
/// `sign(lhs) XOR sign(rhs)`.
fn make_div_repr<const B: Word>(
    sign_negative: bool,
    significand: IBig,
    exponent: isize,
) -> Repr<B> {
    if significand.is_zero() {
        if sign_negative {
            Repr::neg_zero()
        } else {
            Repr::zero()
        }
    } else {
        Repr::new(significand, exponent)
    }
}

macro_rules! impl_div_for_fbig {
    (impl $op:ident, $method:ident, $repr_method:ident) => {
        impl<R: Round, const B: Word> $op<FBig<R, B>> for FBig<R, B> {
            type Output = FBig<R, B>;
            fn $method(self, rhs: FBig<R, B>) -> Self::Output {
                let context = Context::max(self.context, rhs.context);
                // Route through `unwrap_fp` (not the Repr-level unwrap) so an exponent
                // overflow/underflow saturates to the directed endpoint, mode-aware.
                let result = context
                    .$repr_method(self.repr, rhs.repr)
                    .map(|r| r.map(|repr| FBig::new(repr, context)));
                context.unwrap_fp(result)
            }
        }

        impl<'l, R: Round, const B: Word> $op<FBig<R, B>> for &'l FBig<R, B> {
            type Output = FBig<R, B>;
            fn $method(self, rhs: FBig<R, B>) -> Self::Output {
                let context = Context::max(self.context, rhs.context);
                let result = context
                    .$repr_method(self.repr.clone(), rhs.repr)
                    .map(|r| r.map(|repr| FBig::new(repr, context)));
                context.unwrap_fp(result)
            }
        }

        impl<'r, R: Round, const B: Word> $op<&'r FBig<R, B>> for FBig<R, B> {
            type Output = FBig<R, B>;
            fn $method(self, rhs: &FBig<R, B>) -> Self::Output {
                let context = Context::max(self.context, rhs.context);
                let result = context
                    .$repr_method(self.repr, rhs.repr.clone())
                    .map(|r| r.map(|repr| FBig::new(repr, context)));
                context.unwrap_fp(result)
            }
        }

        impl<'l, 'r, R: Round, const B: Word> $op<&'r FBig<R, B>> for &'l FBig<R, B> {
            type Output = FBig<R, B>;
            fn $method(self, rhs: &FBig<R, B>) -> Self::Output {
                let context = Context::max(self.context, rhs.context);
                let result = context
                    .$repr_method(self.repr.clone(), rhs.repr.clone())
                    .map(|r| r.map(|repr| FBig::new(repr, context)));
                context.unwrap_fp(result)
            }
        }
    };
}

macro_rules! impl_rem_for_fbig {
    (impl $op:ident, $method:ident, $repr_method:ident) => {
        impl<R: Round, const B: Word> $op<FBig<R, B>> for FBig<R, B> {
            type Output = FBig<R, B>;
            /// Calculates the remainder of division: if `n` is the quotient `self / rhs`
            /// rounded to an integer under the rounding mode attached to the type, then
            /// the result is `self - n * rhs`.
            ///
            /// The attached mode therefore selects the remainder's convention: with
            /// [Zero](crate::round::mode::Zero) (the `FBig` default) the quotient is
            /// truncated and the remainder keeps the dividend's sign, like `%` on Rust's
            /// primitives; with `Down`/`Up` the quotient rounds floor/ceil-style (Python's
            /// `%` is the `Down` convention); with `HalfEven`/`HalfAway` the remainder is
            /// bounded by `|rhs|/2`, with ties to even or away from zero as in
            /// [`FBig::round`]. Independently of the quotient rule, the mode also rounds
            /// the remainder value itself down to the context precision. For a
            /// non-negative (Euclidean) remainder regardless of the mode, use
            /// [`RemEuclid`].
            ///
            /// # Examples
            ///
            /// ```
            /// # use core::str::FromStr;
            /// # use dashu_base::ParseError;
            /// # use dashu_float::{FBig, DBig, round::mode};
            /// // the binary default type truncates the quotient: 15 = 1×10 + 5
            /// let a = FBig::<mode::Zero, 10>::from_str("15")?;
            /// let b = FBig::<mode::Zero, 10>::from_str("10")?;
            /// assert_eq!(a % b, FBig::<mode::Zero, 10>::from_str("5")?);
            ///
            /// // the decimal type rounds the quotient half away from zero: 15 = 2×10 − 5
            /// assert_eq!(
            ///     DBig::from_str("15")? % DBig::from_str("10")?,
            ///     DBig::from_str("-5")?
            /// );
            /// # Ok::<(), ParseError>(())
            /// ```
            ///
            /// # Panics
            ///
            /// Panics if either operand is infinite.
            fn $method(self, rhs: FBig<R, B>) -> Self::Output {
                let context = Context::max(self.context, rhs.context);
                FBig::new(context.$repr_method(self.repr, rhs.repr).value(), context)
            }
        }

        impl<'l, R: Round, const B: Word> $op<FBig<R, B>> for &'l FBig<R, B> {
            type Output = FBig<R, B>;
            /// Calculates the remainder of division, rounding the quotient to an integer
            /// under the rounding mode attached to the type. See the [`Rem`] implementation
            /// on [`FBig`] for details.
            fn $method(self, rhs: FBig<R, B>) -> Self::Output {
                let context = Context::max(self.context, rhs.context);
                FBig::new(context.$repr_method(self.repr.clone(), rhs.repr).value(), context)
            }
        }

        impl<'r, R: Round, const B: Word> $op<&'r FBig<R, B>> for FBig<R, B> {
            type Output = FBig<R, B>;
            /// Calculates the remainder of division, rounding the quotient to an integer
            /// under the rounding mode attached to the type. See the [`Rem`] implementation
            /// on [`FBig`] for details.
            fn $method(self, rhs: &FBig<R, B>) -> Self::Output {
                let context = Context::max(self.context, rhs.context);
                FBig::new(context.$repr_method(self.repr, rhs.repr.clone()).value(), context)
            }
        }

        impl<'l, 'r, R: Round, const B: Word> $op<&'r FBig<R, B>> for &'l FBig<R, B> {
            type Output = FBig<R, B>;
            /// Calculates the remainder of division, rounding the quotient to an integer
            /// under the rounding mode attached to the type. See the [`Rem`] implementation
            /// on [`FBig`] for details.
            fn $method(self, rhs: &FBig<R, B>) -> Self::Output {
                let context = Context::max(self.context, rhs.context);
                FBig::new(
                    context
                        .$repr_method(self.repr.clone(), rhs.repr.clone())
                        .value(),
                    context,
                )
            }
        }
    };
}
impl_div_for_fbig!(impl Div, div, repr_div);
impl_rem_for_fbig!(impl Rem, rem, repr_rem);
impl_binop_assign_by_taking!(impl DivAssign<Self>, div_assign, div);
impl_binop_assign_by_taking!(impl RemAssign<Self>, rem_assign, rem);

impl<R: Round, const B: Word> DivEuclid<FBig<R, B>> for FBig<R, B> {
    type Output = IBig;
    #[inline]
    fn div_euclid(self, rhs: FBig<R, B>) -> Self::Output {
        let (num, den) = align_as_int(self, rhs);
        num.div_euclid(den)
    }
}

impl<R: Round, const B: Word> DivEuclid<FBig<R, B>> for &FBig<R, B> {
    type Output = IBig;
    #[inline]
    fn div_euclid(self, rhs: FBig<R, B>) -> Self::Output {
        self.clone().div_euclid(rhs)
    }
}

impl<R: Round, const B: Word> DivEuclid<&FBig<R, B>> for FBig<R, B> {
    type Output = IBig;
    #[inline]
    fn div_euclid(self, rhs: &FBig<R, B>) -> Self::Output {
        self.div_euclid(rhs.clone())
    }
}

impl<R: Round, const B: Word> DivEuclid<&FBig<R, B>> for &FBig<R, B> {
    type Output = IBig;
    #[inline]
    fn div_euclid(self, rhs: &FBig<R, B>) -> Self::Output {
        self.clone().div_euclid(rhs.clone())
    }
}

impl<R: Round, const B: Word> RemEuclid<FBig<R, B>> for FBig<R, B> {
    type Output = FBig<R, B>;
    #[inline]
    fn rem_euclid(self, rhs: FBig<R, B>) -> Self::Output {
        let r_exponent = self.repr.exponent.min(rhs.repr.exponent);
        let context = Context::max(self.context, rhs.context);

        let (num, den) = align_as_int(self, rhs);
        let r = num.rem_euclid(den);
        let mut r = context.convert_int(r.into()).value();
        if !r.repr.significand.is_zero() {
            r.repr.exponent += r_exponent;
        }
        r
    }
}

impl<R: Round, const B: Word> RemEuclid<FBig<R, B>> for &FBig<R, B> {
    type Output = FBig<R, B>;
    #[inline]
    fn rem_euclid(self, rhs: FBig<R, B>) -> Self::Output {
        self.clone().rem_euclid(rhs)
    }
}

impl<R: Round, const B: Word> RemEuclid<&FBig<R, B>> for FBig<R, B> {
    type Output = FBig<R, B>;
    #[inline]
    fn rem_euclid(self, rhs: &FBig<R, B>) -> Self::Output {
        self.rem_euclid(rhs.clone())
    }
}

impl<R: Round, const B: Word> RemEuclid<&FBig<R, B>> for &FBig<R, B> {
    type Output = FBig<R, B>;
    #[inline]
    fn rem_euclid(self, rhs: &FBig<R, B>) -> Self::Output {
        self.clone().rem_euclid(rhs.clone())
    }
}

impl<R: Round, const B: Word> DivRemEuclid<FBig<R, B>> for FBig<R, B> {
    type OutputDiv = IBig;
    type OutputRem = FBig<R, B>;
    #[inline]
    fn div_rem_euclid(self, rhs: FBig<R, B>) -> (IBig, FBig<R, B>) {
        let r_exponent = self.repr.exponent.min(rhs.repr.exponent);
        let context = Context::max(self.context, rhs.context);

        let (num, den) = align_as_int(self, rhs);
        let (q, r) = num.div_rem_euclid(den);
        let mut r = context.convert_int(r.into()).value();
        if !r.repr.significand.is_zero() {
            r.repr.exponent += r_exponent;
        }
        (q, r)
    }
}

impl<R: Round, const B: Word> DivRemEuclid<FBig<R, B>> for &FBig<R, B> {
    type OutputDiv = IBig;
    type OutputRem = FBig<R, B>;
    #[inline]
    fn div_rem_euclid(self, rhs: FBig<R, B>) -> (IBig, FBig<R, B>) {
        self.clone().div_rem_euclid(rhs)
    }
}

impl<R: Round, const B: Word> DivRemEuclid<&FBig<R, B>> for FBig<R, B> {
    type OutputDiv = IBig;
    type OutputRem = FBig<R, B>;
    #[inline]
    fn div_rem_euclid(self, rhs: &FBig<R, B>) -> (IBig, FBig<R, B>) {
        self.div_rem_euclid(rhs.clone())
    }
}

impl<R: Round, const B: Word> DivRemEuclid<&FBig<R, B>> for &FBig<R, B> {
    type OutputDiv = IBig;
    type OutputRem = FBig<R, B>;
    #[inline]
    fn div_rem_euclid(self, rhs: &FBig<R, B>) -> (IBig, FBig<R, B>) {
        self.clone().div_rem_euclid(rhs.clone())
    }
}

macro_rules! impl_div_primitive_with_fbig {
    ($($t:ty)*) => {$(
        helper_macros::impl_binop_with_primitive!(impl Div<$t>, div);
        helper_macros::impl_binop_assign_with_primitive!(impl DivAssign<$t>, div_assign);
    )*};
}
impl_div_primitive_with_fbig!(u8 u16 u32 u64 u128 usize UBig i8 i16 i32 i64 i128 isize IBig);
// TODO: we should specialize FBig / UBig or FBig / IBig for better efficiency

impl<R: Round, const B: Word> Inverse for FBig<R, B> {
    type Output = FBig<R, B>;

    #[inline]
    fn inv(self) -> Self::Output {
        self.context.unwrap_fp(self.context.inv(&self.repr))
    }
}

impl<R: Round, const B: Word> Inverse for &FBig<R, B> {
    type Output = FBig<R, B>;

    #[inline]
    fn inv(self) -> Self::Output {
        self.context.unwrap_fp(self.context.inv(&self.repr))
    }
}

impl<R: Round, const B: Word> FBig<R, B> {
    /// Calculate the multiplicative inverse (`1 / self`) of the floating point number.
    ///
    /// # Panics
    ///
    /// Panics if the precision is unlimited.
    #[inline]
    pub fn inv(&self) -> Self {
        self.context.unwrap_fp(self.context.inv(&self.repr))
    }
}

// Align two float by exponent such that they are both turned into integers
fn align_as_int<R: Round, const B: Word>(lhs: FBig<R, B>, rhs: FBig<R, B>) -> (IBig, IBig) {
    let ediff = lhs.repr.exponent - rhs.repr.exponent;
    let (mut num, mut den) = (lhs.repr.significand, rhs.repr.significand);
    if ediff >= 0 {
        shl_digits_in_place::<B>(&mut num, ediff as _);
    } else {
        shl_digits_in_place::<B>(&mut den, (-ediff) as _);
    }
    (num, den)
}

// Decide, under the rounding mode `R`, which of the two exact remainder candidates a
// division leaves: `r1` — the magnitude of `lhs mod rhs`, carrying the dividend's sign,
// meaning the truncated quotient is kept — or its complement `r2 = |rhs| − r1`, carrying
// the opposite sign, meaning the quotient is stepped away from zero. Returns true for
// the complement; `r1_zero` marks an exact multiple (every mode then keeps the quotient,
// regardless of `r1_vs_r2` being `Less`).
//
// This is `R::round_low_part` applied to the quotient `k + f`, where `k` is the truncated
// quotient, `f = ±r1/|rhs|` carries the quotient's sign, and the half test is `r1_vs_r2`.
// The shipped modes never inspect the integer beyond its sign (and, on an exact tie, its
// parity via `bit(0)`), so a small stand-in carrying those bits is passed instead of `k` —
// the Greater branch never has the quotient, and the other branches get its parity for
// free from the division that produces the remainder.
fn rem_rounds_away<R: Round>(
    quotient_sign: Sign,
    r1_zero: bool,
    r1_vs_r2: Ordering,
    quotient_odd: bool,
) -> bool {
    if r1_zero {
        return false;
    }
    let quotient_hint = if r1_vs_r2 == Ordering::Equal {
        // exact half-integer quotient: only HalfEven consults the parity
        match (quotient_sign, quotient_odd) {
            (Sign::Positive, false) => IBig::from(2),
            (Sign::Positive, true) => IBig::from(1),
            (Sign::Negative, false) => IBig::from(-2),
            (Sign::Negative, true) => IBig::from(-1),
        }
    } else {
        // off-tie: only the sign of the quotient can influence the rounding
        match quotient_sign {
            Sign::Positive => IBig::from(1),
            Sign::Negative => IBig::from(-1),
        }
    };
    !matches!(
        R::round_low_part::<_>(&quotient_hint, quotient_sign, || r1_vs_r2),
        Rounding::NoOp
    )
}

// ---------------------------------------------------------------------------
// Short-quotient fast path
//
// When the quotient needs far fewer digits than a full division would compute,
// decide the rounding from a certified high window of the quotient (the `high`
// kernels of dashu-int) instead. The window arrives with a TWO-SIDED error
// bound and — unlike a product window — no exactness flag, so every boundary
// case is declined: an exact division, a midpoint tie, a possible carry into
// the kept digits all fall back to the exact path, which resolves them with
// full information. The declined band is narrow (the error is a few ulps of
// the window's last word, kept far below the rounding midpoint by the sizing).
// ---------------------------------------------------------------------------

/// Engagement floors for the short-quotient path, in words.
///
/// * the divisor must reach the divide-and-conquer band (`MIN_DIVISOR_WORDS`,
///   matching the integer division's simple-case threshold) — a narrow divisor
///   has a schoolbook exact path that is linear in the dividend and always
///   cheaper;
/// * the window must be large enough for the short division's recursion to
///   pay for its rescaling overhead (`MIN_WINDOW_WORDS`);
/// * the window must not exceed twice the divisor (checked at the call) — the
///   exact division's cost grows with the quotient length, so a window much
///   wider than the divisor means the exact path's per-chunk work is already
///   smaller than one short division.
const MIN_DIVISOR_WORDS: usize = 32;
const MIN_WINDOW_WORDS: usize = 28;

/// Decide the correctly-rounded result from a certified high window of a
/// quotient: the true significand lies in `signif ± err_abs` (and is known to
/// be inexact). Returns `None` when the error band straddles a rounding
/// boundary; the caller then falls back to the exact quotient.
fn round_short_quotient<R: Round, const B: Word>(
    context: &Context<R>,
    signif: IBig,
    sign: Sign,
    exponent: isize,
    err_abs: &UBig,
) -> Option<Rounded<Repr<B>>> {
    let precision = context.precision();

    let digits = digit_len::<B>(&signif);
    if digits <= precision {
        // The window sizing guarantees a spare digit; bail out defensively.
        return None;
    }
    let shift = digits - precision;
    let exponent = exponent.checked_add(shift as isize)?;

    let (hi, lo) = split_digits_ref::<B>(&signif, shift);
    if lo.is_zero() {
        // The window's dropped digits are zero, so the error band reaches the
        // exact value (and a carry into the kept digits from below).
        return None;
    }
    // The radix power at the split, B^shift.
    let base_pow = shl_digits::<B>(&IBig::ONE, shift);

    // The true fraction is `lo + delta` with `delta ∈ [−err_abs, err_abs]`.
    // Decide only when the whole band lies strictly inside (0, B^shift) — no
    // carry into the kept digits, no exact zero — and strictly on one side of
    // the rounding midpoint.
    let err = IBig::from(err_abs.clone());
    let band_hi = &lo + &err;
    if band_hi >= base_pow {
        return None;
    }
    let band_lo = &lo - &err;
    if !band_lo.is_positive() {
        return None;
    }
    let ordering = if (&band_hi << 1) < base_pow {
        Ordering::Less
    } else if (&band_lo << 1) > base_pow {
        Ordering::Greater
    } else {
        // The band straddles the midpoint (an exact tie lies inside it).
        return None;
    };

    let hi_signed = sign * hi;
    let adjust = R::round_low_part(&hi_signed, sign, || ordering);
    let signif = hi_signed + adjust;
    // The fraction is certified strictly positive: the result is inexact.
    Some(Approximation::Inexact(
        rounded_to_repr(signif, exponent, sign == Sign::Negative),
        adjust,
    ))
}

impl<R: Round> Context<R> {
    /// Division kernel for finite operands of any width (see [`Self::repr_div_int`] for how the
    /// dividend is bounded), behind the certified short-quotient fast path (see `div_short`).
    ///
    /// A zero divisor is an error only for callers that can report it ([`Self::div`]); here a
    /// nonzero dividend yields the infinity as a value and `0 / 0` panics like a division by zero.
    pub(crate) fn repr_div<const B: Word>(&self, lhs: Repr<B>, rhs: Repr<B>) -> FpResult<Repr<B>> {
        assert_finite_operands(&lhs, &rhs);
        assert_limited_precision(self.precision);

        // the XOR sign of the operands labels the quotient, a zero quotient included (−0 / 5 = −0)
        let sign_negative = lhs.sign() != rhs.sign();
        let sign = Sign::from(sign_negative);

        if rhs.significand.is_zero() {
            if lhs.significand.is_zero() {
                // 0/0 is indeterminate; callers that can signal it (Context::div) check first.
                panic!("divisor must not be 0");
            }
            // finite / 0 = ±inf (sign = XOR), returned as a value
            return Ok(Approximation::Exact(Repr::infinity_with_sign(sign)));
        }
        if lhs.significand.is_zero() {
            // (a canonical zero has exponent 0 or −1, so the quotient's exponent stays in range)
            return Ok(Approximation::Exact(make_div_repr(sign_negative, IBig::ZERO, 0)));
        }

        // Fast path: decide the rounding from a certified high window of the
        // quotient (see `div_short`); the exact quotient is the fallback.
        if let Some(rounded) = self.div_short(&lhs, &rhs) {
            return self.finish_rounded_repr(rounded);
        }

        self.repr_div_int::<B>(
            sign,
            &lhs.significand,
            &rhs.significand.unsigned_abs(),
            lhs.exponent as i128 - rhs.exponent as i128,
        )
    }

    /// Correctly rounded `sign · numerator / denominator · B^exponent` for a nonzero `numerator`
    /// (of which only the magnitude counts) and a positive `denominator`: the exact quotient of
    /// the division, shared by the float division and [`Self::div_int`].
    ///
    /// The dividend is bounded *before* it is divided: it is scaled so that the truncated quotient
    /// has exactly `precision` or `precision + 1` digits, which a single division produces and
    /// which leaves at most one digit to drop. A narrow dividend is padded with zero digits; the
    /// excess low digits of a wide one are split off and kept as *sticky* digits through the
    /// rounding, so that no information is rounded away before the final (single) rounding step.
    /// (Rounding the dividend instead perturbs ties, direction and the exactness flag: the
    /// quotient of a *rounded* dividend can divide exactly where the true one doesn't, land on a
    /// false midpoint, or round the wrong way under directed modes.)
    fn repr_div_int<const B: Word>(
        &self,
        sign: Sign,
        numerator: &IBig,
        denominator: &UBig,
        exponent: i128,
    ) -> FpResult<Repr<B>> {
        assert_limited_precision(self.precision);
        let precision = self.precision;

        // The value becomes (hi + lo / B^k) / denominator · B^exponent, where `hi` spans exactly
        // `precision` digits more than the denominator. The truncated quotient q = hi / denominator
        // then has `precision` or `precision + 1` digits, so at most one digit is left to drop
        // when rounding.
        let num_digits = digit_len::<B>(numerator);
        let den_digits = digit_len::<B>(denominator.as_ibig());
        let bound = precision.saturating_add(den_digits);
        let k = num_digits.saturating_sub(bound); // sticky digits split off a wide numerator
        let shift = bound.saturating_sub(num_digits); // zero digits padding a narrow one
        let mut exponent = exponent + k as i128 - shift as i128;

        // The exponent is tracked beyond the `isize` range: as the exponent of a *padded* quotient
        // it can lie outside the range near the limits although the result fits (see the end of
        // this function). Fail before any work is done when the result certainly cannot fit — its
        // final exponent can only be larger, by one for the digit that may be dropped and by at
        // most `precision` when the trailing zero digits of the significand are folded into it.
        if exponent >= isize::MAX as i128 {
            return Err(FpError::Overflow(sign));
        }
        if exponent + precision as i128 <= isize::MIN as i128 {
            return Err(FpError::Underflow(sign));
        }

        let (q, rem, lo) = if k > 0 {
            let (hi, lo) = split_digits_ref::<B>(numerator, k);
            let (q, rem) = hi.unsigned_abs().div_rem(denominator);
            (q, rem, lo.unsigned_abs())
        } else {
            let hi = shl_digits::<B>(numerator, shift);
            let (q, rem) = hi.unsigned_abs().div_rem(denominator);
            (q, rem, UBig::ZERO)
        };

        // A quotient one digit too wide drops that digit here, together with the fraction below
        // it, in a single step: with q = truncated·B + digit,
        //     q + N / D = truncated + (digit·D + N) / (B·D).
        // (Rounding the digit on its own first would double-round the value.)
        let dropped = digit_len::<B>(q.as_ibig()) > precision;
        let (truncated, digit) = if dropped {
            exponent += 1;
            q.div_rem(B)
        } else {
            (q, 0)
        };

        // The part below the last kept digit is (digit·D + N) / (unit·D), where D = denominator·B^k,
        // N = rem·B^k + lo and `unit` is B if a digit was dropped, else 1. Against half a unit:
        //     2·N  vs  factor·D,   factor = unit − 2·digit
        //     2·N − factor·D = B^k·(2·rem − factor·denominator) + 2·lo
        // factor = 0: a tie iff N = 0. Any other factor but 1: below the half (rem < denominator,
        // 2·lo < 2·B^k). Factor 1: the bracket decides; 2·lo settles a deficit of one unit.
        // Evaluated lazily: the directed modes never need it.
        let half_test = || {
            let unit = if dropped { B } else { 1 };
            if dropped && digit > B - digit {
                // past the half by itself, regardless of the remainder
                return Ordering::Greater;
            }
            // in 0..=B here, since 2·digit <= unit
            let factor = unit - digit * 2;
            match factor {
                0 => {
                    if rem.is_zero() && lo.is_zero() {
                        Ordering::Equal
                    } else {
                        Ordering::Greater
                    }
                }
                1 => {
                    let twice_rem = &rem << 1;
                    match twice_rem.cmp(denominator) {
                        Ordering::Greater => Ordering::Greater,
                        // an exact tie unless the sticky digits are nonzero
                        Ordering::Equal => {
                            if lo.is_zero() {
                                Ordering::Equal
                            } else {
                                Ordering::Greater
                            }
                        }
                        Ordering::Less => {
                            if &twice_rem + UBig::ONE == *denominator {
                                // one unit short: the sticky digits decide
                                (&lo << 1).cmp(&UBig::from_word(B).pow(k))
                            } else {
                                Ordering::Less
                            }
                        }
                    }
                }
                // at least two units short of the half
                _ => Ordering::Less,
            }
        };

        let mut significand = IBig::from_parts(sign, truncated);
        let rounded = if rem.is_zero() && lo.is_zero() && digit == 0 {
            // nothing is left below the significand: the value is represented exactly
            None
        } else {
            Some(R::round_low_part(&significand, sign, half_test))
        };
        if let Some(adjust) = rounded {
            significand += adjust;
        }

        // The result is representable iff the exponent of its normalized form lies strictly
        // inside the range: `Repr::new` folds the trailing zero digits into the exponent and
        // `check_finite_exponent` rejects the sentinels. Below the range the folding has to be
        // done by hand, since the padded exponent does not fit an `isize` any more (an exact
        // quotient narrower than the precision, e.g. an integer divided by a huge power of the
        // base, is padded down past the bottom of the range and folds back into it).
        let repr = if exponent < isize::MIN as i128 {
            // (the temporary's exponent counts the folded digits)
            let folded = Repr::<B>::new(significand, 0);
            let canonical = exponent + folded.exponent as i128;
            if canonical <= isize::MIN as i128 {
                return Err(FpError::Underflow(sign));
            }
            let exponent = isize::try_from(canonical).map_err(|_| FpError::Overflow(sign))?;
            Repr {
                significand: folded.significand,
                exponent,
            }
        } else {
            Repr::<B>::new(significand, exponent as isize)
        };
        repr.check_finite_exponent()?;
        Ok(match rounded {
            None => Approximation::Exact(repr),
            Some(adjust) => Approximation::Inexact(repr, adjust),
        })
    }

    /// Certified short-quotient fast path for division (and `inv`). Returns
    /// `None` to decline — unlimited precision, operands too small for the
    /// kernel, a divisor wider than the window, an over-wide dividend, or an
    /// undecided rounding — in which case the caller computes the exact
    /// quotient.
    fn div_short<const B: Word>(&self, lhs: &Repr<B>, rhs: &Repr<B>) -> Option<Rounded<Repr<B>>> {
        if self.precision() == 0 {
            return None; // unlimited precision: every digit is significant
        }
        if lhs.significand.is_zero() || rhs.significand.is_zero() {
            return None; // signed zeros and /0 belong to the exact path
        }
        let ls = lhs.significand.sign();
        let (rs, rw) = rhs.significand.as_sign_words();
        // The word slices of an IBig are normalized (no trailing zero words,
        // by dashu-int's representation invariant).
        let wb = rw.len();

        // Window for the composed two-sided bound: 2n + 2 kernel ulps plus
        // slack for the <=2-bit significand rescale below. The window is never
        // clamped to the divisor: the kernel normalizes the denominator up to
        // the window with an exact power-of-two shift, so a narrower divisor
        // only means a wider shift. A divisor wider than the window (its
        // precision far exceeds the target's) is declined.
        let n0 = short_window_words::<B>(self.precision(), 1 << 12);
        let n = short_window_words::<B>(self.precision(), 2 * n0 + 8);
        if wb > n || wb < MIN_DIVISOR_WORDS || n < MIN_WINDOW_WORDS || n > 2 * wb {
            return None;
        }

        // An over-wide dividend (its precision far exceeds the divisor's plus
        // the target's) is declined before any operand is materialized: the
        // exact path splits it exactly.
        let (bl_a, bl_d) = (lhs.significand.bit_len(), rhs.significand.bit_len());
        if bl_a > bl_d + n * Word::BITS as usize + 2 {
            return None;
        }

        let b = UBig::from_words(rw);

        // Pad the dividend with base-B digits so the quotient is sized for the
        // window (bit length of numer − bit length of denom ≈ n·WORD_BITS);
        // the kernel verifies the exact band. One digit of adjustment absorbs
        // the float slack of the estimate; a second round should never be
        // needed but keeps the loop total.
        let (_, lb_ub) = B.log2_bounds();
        let target = (n * Word::BITS as usize) as f32 + (bl_d as f32 - bl_a as f32);
        // ceil of a possibly negative ratio (ceil_usize clamps negatives to 0)
        let mut t = ceil_usize(target / lb_ub);
        let mut numer = None;
        for _ in 0..3 {
            let mut candidate = lhs.significand.clone();
            shl_digits_in_place::<B>(&mut candidate, t);
            let mag = candidate.unsigned_abs();
            let gap = mag.bit_len() as isize - bl_d as isize;
            if gap > (n * Word::BITS as usize + 2) as isize {
                if t == 0 {
                    break; // over-wide dividend: declined
                }
                t -= 1;
            } else if gap < (n * Word::BITS as usize - 2) as isize {
                t += 1;
            } else {
                numer = Some(mag);
                break;
            }
        }
        let numer = numer?;

        let q = high::div_high(&numer, &b, n)?;
        let sigma = (n * Word::BITS as usize + bl_d) as isize - numer.bit_len() as isize;
        debug_assert!((-2..=2).contains(&sigma));
        // The window approximates (numer/denom)·2^sigma; rescale it by
        // 2^-sigma so the value keeps the exponent `lhs.exp − rhs.exp − t`.
        // The kernel's ulp error is rescaled by the same shift: a left shift
        // (sigma < 0) amplifies it to `(2n + 2)·2^|sigma|`, and a right shift
        // (sigma > 0) truncates the window by at most one ulp, which the
        // constant `+2` slack covers.
        let (signif, err_up) = if sigma <= 0 {
            (IBig::from(&q << (-sigma) as usize), (-sigma) as usize)
        } else {
            (IBig::from(&q >> sigma as usize), 0)
        };
        let err_abs = UBig::from(2 * n as u64 + 4) << err_up;

        let exponent = lhs
            .exponent
            .checked_sub(rhs.exponent)?
            .checked_sub(t as isize)?;
        round_short_quotient::<R, B>(self, signif, ls * rs, exponent, &err_abs)
    }

    pub(crate) fn repr_rem<const B: Word>(&self, lhs: Repr<B>, rhs: Repr<B>) -> Rounded<Repr<B>> {
        assert_finite_operands(&lhs, &rhs);

        let lhs_is_neg_zero = lhs.is_neg_zero();
        let (lhs_sign, lhs_signif) = lhs.significand.into_parts();
        let (rhs_sign, rhs_signif) = rhs.significand.into_parts();
        let quotient_sign = lhs_sign * rhs_sign;

        let significand = match lhs.exponent.cmp(&rhs.exponent) {
            Ordering::Equal => {
                // one division yields both the truncated quotient and remainder;
                // the quotient's parity breaks exact ties under HalfEven
                let (quotient, r1) = lhs_signif.div_rem(&rhs_signif);
                let r2 = rhs_signif - &r1;
                if rem_rounds_away::<R>(quotient_sign, r1.is_zero(), r1.cmp(&r2), quotient.bit(0)) {
                    IBig::from_parts(-lhs_sign, r2)
                } else {
                    IBig::from_parts(lhs_sign, r1)
                }
            }
            Ordering::Greater => {
                // if the least significant digit of lhs is higher than rhs, then we can
                // align lhs to rhs and do simple modulo operations. Reduce modulo
                // 2|rhs| rather than |rhs|: a single residue then yields the remainder
                // candidates *and* the parity of the truncated quotient, without
                // materializing the B^shift-aligned dividend (the exponent gap can be
                // too large for that).
                let modulo = ConstDivisor::new(&rhs_signif << 1);
                let shift = (lhs.exponent - rhs.exponent) as usize;
                let scaling = if B == 2 {
                    (UBig::ONE << shift).into_ring(&modulo)
                } else {
                    UBig::from_word(B).into_ring(&modulo).pow(&shift.into())
                };
                let r_full = (lhs_signif.into_ring(&modulo) * scaling).residue(); // |lhs| mod 2|rhs|
                let quotient_odd = r_full >= rhs_signif;
                let r1 = if quotient_odd {
                    r_full - &rhs_signif
                } else {
                    r_full
                };
                let r2 = rhs_signif - &r1;
                if rem_rounds_away::<R>(quotient_sign, r1.is_zero(), r1.cmp(&r2), quotient_odd) {
                    IBig::from_parts(-lhs_sign, r2)
                } else {
                    IBig::from_parts(lhs_sign, r1)
                }
            }
            Ordering::Less => {
                // otherwise we have to split lhs into two parts
                let shift = (rhs.exponent - lhs.exponent) as usize;
                let (hi, lo) = split_digits::<B>(lhs_signif.into(), shift);

                // the truncated quotient is hi div |rhs| and the high part of the
                // truncated remainder is hi mod |rhs| — both from one division
                let (quotient, mut r1) = hi.div_rem(&rhs_signif);
                let mut r2 = rhs_signif - &r1;

                shl_digits_in_place::<B>(&mut r1, shift);
                r1 += &lo;

                shl_digits_in_place::<B>(&mut r2, shift);
                r2 -= lo;

                if rem_rounds_away::<R>(quotient_sign, r1.is_zero(), r1.cmp(&r2), quotient.bit(0)) {
                    (-lhs_sign) * r2
                } else {
                    lhs_sign * r1
                }
            }
        };

        let exponent = lhs.exponent.min(rhs.exponent);
        if significand.is_zero() {
            // the sign of a zero remainder follows the dividend (±0)
            Approximation::Exact(if lhs_is_neg_zero {
                Repr::neg_zero()
            } else {
                Repr::zero()
            })
        } else {
            let repr = Repr::new(significand, exponent);
            match repr.check_finite_exponent() {
                Ok(()) => self.repr_round(repr),
                Err(e) => match e {
                    FpError::Overflow(sign) => {
                        Approximation::Inexact(Repr::infinity_with_sign(sign), Rounding::NoOp)
                    }
                    FpError::Underflow(sign) => {
                        Approximation::Inexact(Repr::zero_with_sign(sign), Rounding::NoOp)
                    }
                    _ => unreachable!(),
                },
            }
        }
    }

    /// Divide two floating point numbers under this context.
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
    /// let a = DBig::from_str("-1.234")?;
    /// let b = DBig::from_str("6.789")?;
    /// assert_eq!(context.div(&a.repr(), &b.repr()), Ok(Inexact(DBig::from_str("-0.18")?, NoOp)));
    /// # Ok::<(), ParseError>(())
    /// ```
    ///
    /// # Euclidean Division
    ///
    /// To do euclidean division on the float numbers (get an integer quotient and remainder, equivalent to C99's
    /// `fmod` and `remquo`), please use the methods provided by traits [DivEuclid], [RemEuclid] and [DivRemEuclid].
    ///
    pub fn div<const B: Word>(&self, lhs: &Repr<B>, rhs: &Repr<B>) -> FpResult<FBig<R, B>> {
        if lhs.is_infinite() || rhs.is_infinite() {
            return Err(FpError::InfiniteInput);
        }
        if lhs.significand.is_zero() && rhs.significand.is_zero() {
            return Err(FpError::Indeterminate); // 0/0
        }

        // No operand pre-shrinking here: the kernel bounds an over-wide dividend by an
        // exact split, keeping the dropped digits as rounding information. Rounding the
        // dividend *before* dividing — as this used to do — corrupts the result: the rounded
        // dividend can divide exactly where the true one doesn't (a false `Exact` flag),
        // land the quotient on a false midpoint, or invert the direction under directed
        // modes.
        Ok(self
            .repr_div(lhs.clone(), rhs.clone())?
            .map(|v| FBig::new(v, *self)))
    }

    /// Divide an integer by a positive integer, with correct rounding to this context's
    /// precision.
    ///
    /// The operands are integers, so the result is exact iff the quotient terminates within
    /// the precision; every other case is rounded once, by the context's rounding mode.
    /// This is the integer-ratio counterpart of [`Self::div`]: it neither rounds the operands
    /// nor converts them first.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// use dashu_base::Approximation::*;
    /// use dashu_float::{Context, round::{mode::HalfAway, Rounding::*}};
    /// use dashu_int::{IBig, UBig};
    ///
    /// let context = Context::<HalfAway>::new(4);
    /// // 1000/6 = 166.666… rounds half away from zero
    /// assert_eq!(
    ///     context.div_int::<10>(&1000.into(), &6u8.into()),
    ///     Ok(Inexact(DBig::from_parts(1667.into(), -1), AddOne))
    /// );
    /// // 1/4 terminates in base 10
    /// assert_eq!(
    ///     context.div_int::<10>(&1.into(), &4u8.into()),
    ///     Ok(Exact(DBig::from_str("0.25")?))
    /// );
    /// # Ok::<(), ParseError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Like [`Self::div`], a zero numerator over a zero denominator is `Indeterminate`, a
    /// nonzero numerator over zero yields the signed infinity as a value, and a result whose
    /// exponent leaves the representable range is an error. The latter needs an astronomically
    /// wide operand or precision.
    ///
    /// # Panics
    ///
    /// Panics if the precision is unlimited, as the quotient of two integers need not terminate.
    pub fn div_int<const B: Word>(
        &self,
        numerator: &IBig,
        denominator: &UBig,
    ) -> FpResult<FBig<R, B>> {
        let sign = numerator.sign();
        if denominator.is_zero() {
            if numerator.is_zero() {
                return Err(FpError::Indeterminate); // 0/0
            }
            // nonzero / 0 = ±inf, returned as a value
            return Ok(Approximation::Exact(FBig::new(Repr::infinity_with_sign(sign), *self)));
        }
        if numerator.is_zero() {
            return Ok(Approximation::Exact(FBig::new(Repr::zero(), *self)));
        }

        Ok(self
            .repr_div_int::<B>(sign, numerator, denominator, 0)?
            .map(|repr| FBig::new(repr, *self)))
    }

    /// Calculate the remainder of `lhs / rhs`, with the quotient rounded to an integer
    /// by this context's rounding mode.
    ///
    /// The remainder is `r = lhs - n * rhs`, where `n` is the quotient rounded under
    /// the mode (e.g. `Zero` truncates `n`, so `r` keeps the dividend's sign; the half
    /// modes bound `|r|` by `|rhs|/2`). The remainder value is exact; the returned flag
    /// only reports whether it had to be rounded down to the context precision.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::{DBig, FBig};
    /// use dashu_base::Approximation::*;
    /// use dashu_float::{Context, round::{mode::{HalfAway, Zero}, Rounding::*}};
    ///
    /// let a = DBig::from_str("6.789")?;
    /// let b = DBig::from_str("-1.234")?;
    ///
    /// // the quotient −5.503… rounds to −6 half away from zero
    /// let context = Context::<HalfAway>::new(3);
    /// assert_eq!(context.rem(&a.repr(), &b.repr()), Ok(Exact(DBig::from_str("-0.615")?)));
    ///
    /// // truncating the quotient instead: 6.789 = (−5)·(−1.234) + 0.619
    /// let context = Context::<Zero>::new(3);
    /// assert_eq!(
    ///     context.rem(&a.repr(), &b.repr()),
    ///     Ok(Exact(FBig::<Zero, 10>::from_str("0.619")?))
    /// );
    /// # Ok::<(), ParseError>(())
    /// ```
    pub fn rem<const B: Word>(&self, lhs: &Repr<B>, rhs: &Repr<B>) -> FpResult<FBig<R, B>> {
        if lhs.is_infinite() || rhs.is_infinite() {
            return Err(FpError::InfiniteInput);
        }
        Ok(self
            .repr_rem(lhs.clone(), rhs.clone())
            .map(|v| FBig::new(v, *self)))
    }

    /// Compute the multiplicative inverse of an `FBig`
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// use dashu_base::Approximation::*;
    /// use dashu_float::{Context, round::{mode::HalfAway, Rounding::*}};
    ///
    /// let context = Context::<HalfAway>::new(2);
    /// let a = DBig::from_str("-1.234")?;
    /// assert_eq!(context.inv(&a.repr()), Ok(Inexact(DBig::from_str("-0.81")?, NoOp)));
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn inv<const B: Word>(&self, f: &Repr<B>) -> FpResult<FBig<R, B>> {
        if f.is_infinite() {
            return Err(FpError::InfiniteInput);
        }
        // inv(±0) = ±inf (produced as a value by repr_div)
        Ok(self
            .repr_div(Repr::one(), f.clone())?
            .map(|v| FBig::new(v, *self)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::round::{mode, Rounding::AddOne, Rounding::NoOp, Rounding::SubOne};
    use dashu_base::Approximation::*;

    fn r2(sig: i32, exp: isize) -> Repr<2> {
        Repr::new(sig.into(), exp)
    }

    // Regression tests for the rounding bugs reported in issue #100 (and two same-class value
    // bugs found while investigating): `Context::div` used to round an over-wide dividend
    // *before* dividing (perturbing ties, direction and the exactness flag), and `repr_div`
    // returned exact quotients unreduced and rounded p+1-digit quotients on the wrong grid.
    #[test]
    fn test_div_quotient_reduced_and_flagged() {
        // 15/3 = 5 needs three digits at precision 2: the exact quotient must be rounded
        // (Zero -> 4), not returned unreduced with an Exact flag.
        let r = Context::<mode::Zero>::new(2)
            .div(&r2(15, 0), &r2(3, 0))
            .unwrap();
        assert!(matches!(r, Inexact(..)), "15/3 @ p2 must be inexact");
        assert_eq!(r.value().repr(), &r2(1, 2));

        // 5/1 at precision 1: same unreduced-quotient bug — 5 is not representable at p1.
        let r = Context::<mode::Zero>::new(1)
            .div(&r2(5, 0), &r2(1, 0))
            .unwrap();
        assert!(matches!(r, Inexact(..)), "5/1 @ p1 must be inexact");
        assert_eq!(r.value().repr(), &r2(1, 2));

        // 63/3 = 21 at precision 2 under Zero: the dividend is pre-split (not pre-rounded!),
        // and 21 truncates onto the 2-digit grid as 1·2^4 = 16.
        let v = Context::<mode::Zero>::new(2)
            .div(&r2(63, 0), &r2(3, 0))
            .unwrap()
            .value();
        assert_eq!(v.repr(), &r2(1, 4));
    }

    // The p+1-digit quotient rounding under a nearest mode, on both signs and an odd base.
    // The negative non-binary cases regress-checked here were mis-rounded by an early
    // version of the half comparison (a sign flip of B − 2·ql that should never happen:
    // only |ql| enters the rearranged comparison).
    #[test]
    fn test_div_overwide_quotient_nearest_modes() {
        // -21/2 = -10.5 in base 10 at precision 1: the p1 neighbours are -10 and -20 with
        // midpoint -15, so -10.5 rounds to -10 under every mode except Away (-11 is NOT on
        // the p1 grid; away from zero at this magnitude steps to -20).
        let r10 = |sig: i32, exp: isize| Repr::<10>::new(IBig::from(sig), exp);
        let v = Context::<mode::HalfEven>::new(1)
            .div(&r10(-21, 0), &r10(2, 0))
            .unwrap()
            .value();
        assert_eq!(v.repr(), &r10(-1, 1)); // -10

        // positive mirror: 21/2 = 10.5 -> 10
        let v = Context::<mode::HalfEven>::new(1)
            .div(&r10(21, 0), &r10(2, 0))
            .unwrap()
            .value();
        assert_eq!(v.repr(), &r10(1, 1));

        // -95/2 = -47.5: neighbours -40/-50, midpoint -45 -> -50 under HalfEven
        let v = Context::<mode::HalfEven>::new(1)
            .div(&r10(-95, 0), &r10(2, 0))
            .unwrap()
            .value();
        assert_eq!(v.repr(), &r10(-5, 1)); // -50

        // -41/2 = -20.5: neighbours -20/-30, midpoint -25 -> -20 under HalfEven
        let v = Context::<mode::HalfEven>::new(1)
            .div(&r10(-41, 0), &r10(2, 0))
            .unwrap()
            .value();
        assert_eq!(v.repr(), &r10(-2, 1)); // -20

        // odd base (exercises the doubled-remainder comparison arm): -8/2 = -4 in base 3
        // at precision 1 — neighbours -3/-6 with midpoint -4.5, so -4 rounds to -3;
        // -11/2 = -5.5 is past the midpoint and rounds to -6
        let r3 = |sig: i32, exp: isize| Repr::<3>::new(IBig::from(sig), exp);
        let v = Context::<mode::HalfEven>::new(1)
            .div(&r3(-8, 0), &r3(2, 0))
            .unwrap()
            .value();
        assert_eq!(v.repr(), &r3(-1, 1)); // -3
        let v = Context::<mode::HalfEven>::new(1)
            .div(&r3(-11, 0), &r3(2, 0))
            .unwrap()
            .value();
        assert_eq!(v.repr(), &r3(-2, 1)); // -6
    }

    #[test]
    fn test_div_directed_modes_on_wide_dividend() {
        // 7/-1 = -7 at precision 1 under Up must round toward +inf (-4). The old pre-shrink
        // rounded the dividend 7 up to 8 first, and 8/-1 = -8 then rounded *down*.
        let v = Context::<mode::Up>::new(1)
            .div(&r2(7, 0), &r2(-1, 0))
            .unwrap()
            .value();
        assert_eq!(v.repr(), &r2(-1, 2));

        // 31/4 = 7.75 in base 3 at precision 1 under HalfEven: the old pre-shrink turned it
        // into 30/4 = 7.5, a false midpoint that tied down to 6; the true value is above the
        // 7.5 midpoint of 6 and 9, so it rounds to 9.
        let v = Context::<mode::HalfEven>::new(1)
            .div(&Repr::<3>::new(IBig::from(31), 0), &Repr::<3>::new(IBig::from(4), 0))
            .unwrap()
            .value();
        assert_eq!(v.repr(), &Repr::<3>::new(IBig::from(1), 2));
    }

    // Two same-class value bugs in the p+1-digit quotient paths (found while investigating
    // #100): the final rounding used the integer grid instead of the precision-digit grid.
    #[test]
    fn test_div_p1_digit_quotient_rounds_on_precision_grid() {
        // 3/5 = 0.6 at precision 2 under HalfEven: 0.6·2^3 = 4.8 used to be rounded on the
        // integer grid to 5 (= 0.625, not even representable at p2); the p2 neighbours are
        // 0.5 and 0.75, so the result is 0.5.
        let v = Context::<mode::HalfEven>::new(2)
            .div(&r2(3, 0), &r2(5, 0))
            .unwrap()
            .value();
        assert_eq!(v.repr(), &r2(1, -1));

        // 14/3 = 4.67 at precision 2 under HalfEven: the quotient 100₂ carries p+1 digits;
        // the p2 grid is 100/110 (4 and 6) with midpoint 5, so 4.67 rounds to 4 (the integer
        // grid would give 5).
        let v = Context::<mode::HalfEven>::new(2)
            .div(&r2(14, 0), &r2(3, 0))
            .unwrap()
            .value();
        assert_eq!(v.repr(), &r2(1, 2));

        // 7/2 = 3.5 at precision 2 under HalfEven: the divisor normalizes to 1·2^1, so the
        // exact quotient 111·2^-1 used to be returned unreduced. 3.5 is the exact midpoint of
        // 11 (3) and 100 (4); ties to even gives 4.
        let v = Context::<mode::HalfEven>::new(2)
            .div(&r2(7, 0), &r2(2, 0))
            .unwrap()
            .value();
        assert_eq!(v.repr(), &r2(1, 2));
    }

    // The dividend's sticky low digits (split off by the width bound) must participate in the
    // rounding: they resolve exact ties and push directed modes the right way.
    #[test]
    fn test_div_sticky_low_digits() {
        // 15·2^-1 / 5 = 1.5 at precision 1 under HalfEven: exact tie between 1 and 2 -> 2.
        let v = Context::<mode::HalfEven>::new(1)
            .div(&r2(15, -1), &r2(5, 0))
            .unwrap()
            .value();
        assert_eq!(v.repr(), &r2(1, 1));

        // Same input under Up: the tie pushes up to 2.
        let v = Context::<mode::Up>::new(1)
            .div(&r2(15, -1), &r2(5, 0))
            .unwrap()
            .value();
        assert_eq!(v.repr(), &r2(1, 1));

        // 11/1 at precision 2 under HalfEven: the dividend 1011₂ is split at 1 digit; without
        // the sticky low bit the rounding would see an exact tie (10|1 -> even -> 10), with it
        // the fraction is 3/4 and the result is 1100₂ = 12.
        let v = Context::<mode::HalfEven>::new(2)
            .div(&r2(11, 0), &r2(1, 0))
            .unwrap()
            .value();
        assert_eq!(v.repr(), &r2(3, 2));

        // 25·2^-1 / 5 = 2.5 at precision 1 under Up: the p1 neighbours are 2 and 4, and a
        // directed mode picks the one in its direction regardless of the (midpoint 3).
        let v = Context::<mode::Up>::new(1)
            .div(&r2(25, -1), &r2(5, 0))
            .unwrap()
            .value();
        assert_eq!(v.repr(), &r2(1, 2));
    }

    #[test]
    fn test_div_by_zero_is_infinity() {
        let ctx = Context::<mode::HalfEven>::new(53);
        // finite / 0 = ±inf (a value, not an error); sign = XOR
        let pos = ctx.div::<2>(&r2(1, 0), &Repr::<2>::zero()).unwrap().value();
        assert!(pos.repr().is_infinite());
        assert_eq!(pos.repr().sign(), Sign::Positive);

        let neg = ctx
            .div::<2>(&r2(-1, 0), &Repr::<2>::zero())
            .unwrap()
            .value();
        assert_eq!(neg.repr().sign(), Sign::Negative);

        // 1 / -0 = -inf
        let neg2 = ctx
            .div::<2>(&r2(1, 0), &Repr::<2>::neg_zero())
            .unwrap()
            .value();
        assert_eq!(neg2.repr().sign(), Sign::Negative);
    }

    #[test]
    fn test_zero_over_zero_is_indeterminate() {
        let ctx = Context::<mode::HalfEven>::new(53);
        assert_eq!(
            ctx.div::<2>(&Repr::<2>::zero(), &Repr::<2>::zero()),
            Err(FpError::Indeterminate)
        );
    }

    #[test]
    fn test_inv_zero_is_infinity() {
        let ctx = Context::<mode::HalfEven>::new(53);
        let r = ctx.inv::<2>(&Repr::<2>::zero()).unwrap().value();
        assert!(r.repr().is_infinite());
        assert_eq!(r.repr().sign(), Sign::Positive);
    }

    #[test]
    fn test_fbig_div_zero_produces_infinity() {
        // FBig convenience layer: 1 / 0 yields an infinity-valued FBig (no panic).
        let one = FBig::<mode::HalfEven>::try_from(1.0f64).unwrap();
        let zero = FBig::<mode::HalfEven>::try_from(0.0f64).unwrap();
        let inf = one / zero;
        assert!(inf.repr().is_infinite());
    }

    #[test]
    #[should_panic]
    fn test_fbig_zero_over_zero_panics() {
        // 0 / 0 is indeterminate; the FBig layer panics.
        let zero = FBig::<mode::HalfEven>::try_from(0.0f64).unwrap();
        let _ = zero.clone() / zero;
    }

    // ---- exact-path kernel, through `Context::div_int` ----
    //
    // The reported double-rounding regressions (6/11, 493801/4, 31/3, …) are checked against an
    // exact oracle in `tests/div_prop.rs`; the tests below cover what only this module can steer.

    /// An integer ratio with the given parts.
    fn ratio(numerator: i64, denominator: u64) -> (IBig, UBig) {
        (IBig::from(numerator), UBig::from(denominator))
    }

    /// A float with the given value, flagged exact (`None`) or inexact with the adjustment
    /// that was applied to the truncated significand.
    fn rounded<R: Round, const B: Word>(
        value: i64,
        adjust: Option<Rounding>,
    ) -> Rounded<FBig<R, B>> {
        match adjust {
            None => Exact(FBig::from(value)),
            Some(adjust) => Inexact(FBig::from(value), adjust),
        }
    }

    #[test]
    fn test_div_int_rounding_decision() {
        // One input per outcome of comparing the dropped part with half a unit of the last kept
        // digit, all at precision 1 in base 10. The numerator's top `precision + denominator`
        // digits are divided by the denominator; a digit is dropped from the quotient when it
        // is 10 or more, and the digits below the divided part are the sticky digits.
        // (input, value under HalfEven, value under HalfAway)
        let cases = [
            // padded numerator, the quotient fits the precision
            ((12, 8), (2, Some(AddOne)), (2, Some(AddOne))), // 1.5: an exact tie, odd digit
            ((20, 8), (2, Some(NoOp)), (3, Some(AddOne))),   // 2.5: an exact tie, even digit
            ((11, 8), (1, Some(NoOp)), (1, Some(NoOp))),     // 1.375
            ((13, 8), (2, Some(AddOne)), (2, Some(AddOne))), // 1.625
            ((16, 8), (2, None), (2, None)),                 // 2: exact
            ((12, 5), (2, Some(NoOp)), (2, Some(NoOp))),     // 2.4: a fifth below the tie
            ((13, 5), (3, Some(AddOne)), (3, Some(AddOne))), // 2.6: a fifth above
            ((10, 5), (2, None), (2, None)),                 // 2: exact
            // padded numerator, one digit of the quotient is dropped
            ((19, 1), (20, Some(AddOne)), (20, Some(AddOne))), // 19: digit 9 is past the half on its own
            ((11, 1), (10, Some(NoOp)), (10, Some(NoOp))),     // 11: digit 1 is far below the half
            ((15, 1), (20, Some(AddOne)), (20, Some(AddOne))), // 15: an exact tie, odd
            ((25, 1), (20, Some(NoOp)), (30, Some(AddOne))),   // 25: an exact tie, even
            ((29, 2), (10, Some(NoOp)), (10, Some(NoOp))),     // 14.5: digit 4, remainder one half
            ((31, 2), (20, Some(AddOne)), (20, Some(AddOne))), // 15.5: digit 5, a remainder is left
            ((251, 10), (30, Some(AddOne)), (30, Some(AddOne))), // 25.1: digit 5, a remainder is left
            ((30, 2), (20, Some(AddOne)), (20, Some(AddOne))),   // 15: digit 5, an exact tie
            ((303, 10), (30, Some(NoOp)), (30, Some(NoOp))),     // 30.3: the dropped digit is 0
            ((20, 2), (10, None), (10, None)),                   // 10: exact
            ((99, 1), (100, Some(AddOne)), (100, Some(AddOne))), // 99: rounding up carries a digit
            // wide numerator: the digits cut off are the sticky part
            ((130, 2), (60, Some(NoOp)), (70, Some(AddOne))), // 65: an exact tie, even
            ((131, 2), (70, Some(AddOne)), (70, Some(AddOne))), // 65.5: sticky digits tip the tie
            ((125, 2), (60, Some(NoOp)), (60, Some(NoOp))),   // 62.5: no remainder, two below
            ((250, 1), (200, Some(NoOp)), (300, Some(AddOne))), // 250: digit 5, an exact tie
            ((257, 1), (300, Some(AddOne)), (300, Some(AddOne))), // 257: digit 5, sticky digits
            ((247, 1), (200, Some(NoOp)), (200, Some(NoOp))), // 247: digit 4, out of reach
            ((275, 1), (300, Some(AddOne)), (300, Some(AddOne))), // 275: digit 7, past the half
            ((225, 1), (200, Some(NoOp)), (200, Some(NoOp))), // 225: digit 2
            ((125, 5), (20, Some(NoOp)), (30, Some(AddOne))), // 25: a fifth below the tie, closed by the sticky digits
            ((124, 5), (20, Some(NoOp)), (20, Some(NoOp))), // 24.8: a fifth below, not quite closed
            ((126, 5), (30, Some(AddOne)), (30, Some(AddOne))), // 25.2: a fifth below, crossed
            ((119, 5), (20, Some(NoOp)), (20, Some(NoOp))), // 23.8: far below, large sticky digits
            ((2503, 10), (300, Some(AddOne)), (300, Some(AddOne))), // 250.3: digit 5, sticky digits
            ((3005, 10), (300, Some(NoOp)), (300, Some(NoOp))), // 300.5: the dropped digit is 0
        ];
        for ((num, den), even, away) in cases {
            let (num, den) = ratio(num, den);
            assert_eq!(
                Context::<mode::HalfEven>::new(1)
                    .div_int::<10>(&num, &den)
                    .unwrap(),
                rounded::<mode::HalfEven, 10>(even.0, even.1),
                "{num}/{den} under HalfEven"
            );
            assert_eq!(
                Context::<mode::HalfAway>::new(1)
                    .div_int::<10>(&num, &den)
                    .unwrap(),
                rounded::<mode::HalfAway, 10>(away.0, away.1),
                "{num}/{den} under HalfAway"
            );
        }
    }

    #[test]
    fn test_div_int_rounding_modes() {
        // 65.5 at 1 digit: the sticky digit settles the tie, so the result rounds away from
        // zero under every mode that looks at the size of the dropped part
        let (num, den) = ratio(131, 2);
        assert_eq!(
            Context::<mode::HalfEven>::new(1)
                .div_int::<10>(&num, &den)
                .unwrap(),
            rounded::<mode::HalfEven, 10>(70, Some(AddOne))
        );
        assert_eq!(
            Context::<mode::HalfAway>::new(1)
                .div_int::<10>(&num, &den)
                .unwrap(),
            rounded::<mode::HalfAway, 10>(70, Some(AddOne))
        );
        assert_eq!(
            Context::<mode::Zero>::new(1)
                .div_int::<10>(&num, &den)
                .unwrap(),
            rounded::<mode::Zero, 10>(60, Some(NoOp))
        );
        assert_eq!(
            Context::<mode::Away>::new(1)
                .div_int::<10>(&num, &den)
                .unwrap(),
            rounded::<mode::Away, 10>(70, Some(AddOne))
        );
        assert_eq!(
            Context::<mode::Up>::new(1)
                .div_int::<10>(&num, &den)
                .unwrap(),
            rounded::<mode::Up, 10>(70, Some(AddOne))
        );
        assert_eq!(
            Context::<mode::Down>::new(1)
                .div_int::<10>(&num, &den)
                .unwrap(),
            rounded::<mode::Down, 10>(60, Some(NoOp))
        );

        // the same value negated: the directed modes follow the sign
        let (num, den) = ratio(-131, 2);
        assert_eq!(
            Context::<mode::Zero>::new(1)
                .div_int::<10>(&num, &den)
                .unwrap(),
            rounded::<mode::Zero, 10>(-60, Some(NoOp))
        );
        assert_eq!(
            Context::<mode::Away>::new(1)
                .div_int::<10>(&num, &den)
                .unwrap(),
            rounded::<mode::Away, 10>(-70, Some(SubOne))
        );
        assert_eq!(
            Context::<mode::Up>::new(1)
                .div_int::<10>(&num, &den)
                .unwrap(),
            rounded::<mode::Up, 10>(-60, Some(NoOp))
        );
        assert_eq!(
            Context::<mode::Down>::new(1)
                .div_int::<10>(&num, &den)
                .unwrap(),
            rounded::<mode::Down, 10>(-70, Some(SubOne))
        );
    }

    #[test]
    fn test_div_int_odd_base() {
        // In base 3 at precision 1, the numerator 11k₃ is split into 11₃ = 4 and the sticky
        // digit k. The quotient 4 drops the digit 1, which is one third below the half, and
        // leaves no remainder: the sticky digit alone decides.
        let nine = || FBig::<mode::HalfEven, 3>::from(9u8);
        let (num, den) = ratio(12, 1);
        assert_eq!(
            Context::<mode::HalfEven>::new(1)
                .div_int::<3>(&num, &den)
                .unwrap(),
            Inexact(nine(), NoOp)
        ); // 110₃
        let (num, den) = ratio(13, 1);
        assert_eq!(
            Context::<mode::HalfEven>::new(1)
                .div_int::<3>(&num, &den)
                .unwrap(),
            Inexact(nine(), NoOp)
        ); // 111₃
        let (num, den) = ratio(14, 1);
        assert_eq!(
            Context::<mode::HalfEven>::new(1)
                .div_int::<3>(&num, &den)
                .unwrap(), // 112₃
            Inexact(FBig::<mode::HalfEven, 3>::from(18u8), AddOne)
        );
    }

    #[test]
    fn test_div_int_special_values() {
        // a zero numerator is an exact zero that keeps the precision
        let (num, den) = ratio(0, 7);
        let zero = Context::<mode::HalfEven>::new(24)
            .div_int::<10>(&num, &den)
            .unwrap();
        assert_eq!(zero, Exact(FBig::<mode::HalfEven, 10>::ZERO));
        assert_eq!(zero.value().precision(), 24);

        // nonzero / 0 = ±inf (a value, not an error)
        let (num, den) = ratio(5, 0);
        let pos = Context::<mode::HalfEven>::new(8)
            .div_int::<10>(&num, &den)
            .unwrap();
        assert!(
            matches!(pos, Exact(f) if f.repr().is_infinite() && f.repr().sign() == Sign::Positive)
        );
        let (num, den) = ratio(-5, 0);
        let neg = Context::<mode::HalfEven>::new(8)
            .div_int::<10>(&num, &den)
            .unwrap();
        assert!(
            matches!(neg, Exact(f) if f.repr().is_infinite() && f.repr().sign() == Sign::Negative)
        );

        // 0/0 is indeterminate
        let (num, den) = ratio(0, 0);
        assert_eq!(
            Context::<mode::HalfEven>::new(8).div_int::<10>(&num, &den),
            Err(FpError::Indeterminate)
        );
    }

    #[test]
    #[should_panic]
    fn test_div_int_unlimited_precision_panics() {
        let (num, den) = ratio(1, 3);
        drop(Context::<mode::HalfEven>::new(0).div_int::<10>(&num, &den));
    }

    // ---- exponent range ----
    //
    // A quotient is representable iff the exponent of its *normalized* form lies strictly
    // inside the range. The kernel pads a narrow dividend with zero digits before dividing,
    // so the exponent it works with can lie outside the range although the result does not:
    // an early check on it must only reject what certainly cannot fit.

    #[test]
    fn test_div_quotient_folds_back_into_range() {
        // 3·2^(MIN+64) / 2^10 at 100 digits: the dividend is padded with 99 zero digits, down
        // to the exponent MIN − 45. The quotient is exact, and its 98 trailing zero digits fold
        // back into an exponent inside the range. (Checked on the padded exponent, this would
        // be reported as an underflow.)
        let ctx = Context::<mode::HalfEven>::new(100);
        let lhs = Repr::<2>::new(IBig::from(3u8), isize::MIN + 64);
        let rhs = Repr::<2>::new(IBig::ONE, 10);
        let got = ctx.div(&lhs, &rhs).unwrap();
        assert_eq!(got.map(FBig::into_repr), Exact(Repr::new(IBig::from(3u8), isize::MIN + 54)));
    }

    #[test]
    fn test_div_exponent_difference_overflows_range() {
        // 1023·2^100 / ((2^60 − 1)·2^(MIN+60)) = 1.11111101…b·2^(MAX−10): the operands'
        // exponents differ by more than the range holds, but the narrow dividend is padded
        // with 53 zero digits and the 3-digit result fits. (The overflow of the bare exponent
        // difference used to be reported as an overflow of the result.)
        let ctx = Context::<mode::HalfEven>::new(3);
        let dividend = IBig::from(1023u32);
        let divisor = Repr::<2>::new((IBig::ONE << 60) - IBig::ONE, isize::MIN + 60);
        let got = ctx
            .div(&Repr::new(dividend.clone(), 100), &divisor)
            .unwrap();
        assert_eq!(
            got.clone().map(FBig::into_repr),
            Inexact(Repr::new(IBig::ONE, isize::MAX - 9), AddOne)
        );

        // the same division in the middle of the range rounds identically: only the exponent
        // of the result moves
        let moderate = ctx
            .div(&Repr::<2>::new(dividend, 0), &Repr::<2>::new(divisor.into_parts().0, 0))
            .unwrap();
        let expected = moderate.map(|f| {
            let (significand, exponent) = f.into_repr().into_parts();
            Repr::new(significand, exponent + isize::MAX + 41)
        });
        assert_eq!(got.map(FBig::into_repr), expected);

        // the mirror image at the bottom of the range: 78·2^(MIN+1) / (5·2^4) = 1111.1001…b·2^(MIN−3)
        // rounds up to 2^(MIN+1), which fits, although the exponent of its 4-digit form lies
        // below the range. (The underflow of that exponent used to be reported as the result's.)
        let lhs = Repr::<2>::new(IBig::from(78u8), isize::MIN + 1);
        let rhs = Repr::<2>::new(IBig::from(5u8), 4);
        let got = Context::<mode::HalfEven>::new(4).div(&lhs, &rhs).unwrap();
        assert_eq!(got.map(FBig::into_repr), Inexact(Repr::new(IBig::ONE, isize::MIN + 1), AddOne));
    }

    #[test]
    fn test_div_operand_on_the_sentinel() {
        // An operand sitting on the `+inf` sentinel exponent is not a canonical finite number,
        // but it can be built, and a quotient that keeps that exponent overflows.
        let on_sentinel = Repr::<2>::new(IBig::from(3u8), isize::MAX);
        let one = Repr::<2>::one();
        assert_eq!(
            Context::<mode::HalfEven>::new(1).div(&on_sentinel, &one),
            Err(FpError::Overflow(Sign::Positive))
        );
        assert_eq!(
            Context::<mode::HalfEven>::new(1).div(&one, &on_sentinel),
            Err(FpError::Underflow(Sign::Positive))
        );
    }

    #[test]
    fn test_div_overflow_is_not_underflow() {
        // −(2^200 + 1)·2^−174 / 2^(MIN+1) = −(2^200 + 1)·2^(MAX−174): a wide dividend with a
        // negative exponent over a divisor at the bottom of the range. The quotient is far too
        // large. (The error kind used to follow the sign of the dividend's exponent, which
        // reported this as an *underflow*.)
        let lhs = Repr::<2>::new(-(IBig::ONE << 200) - IBig::ONE, -174);
        let rhs = Repr::<2>::new(IBig::ONE, isize::MIN + 1);
        assert_eq!(
            Context::<mode::HalfEven>::new(8).div(&lhs, &rhs),
            Err(FpError::Overflow(Sign::Negative))
        );

        // the positive mirror
        let lhs = Repr::<2>::new((IBig::ONE << 200) + IBig::ONE, -174);
        assert_eq!(
            Context::<mode::HalfEven>::new(8).div(&lhs, &rhs),
            Err(FpError::Overflow(Sign::Positive))
        );

        // a wide dividend over a divisor at the top of the range is a genuine underflow
        let lhs = Repr::<2>::new((IBig::ONE << 200) + IBig::ONE, isize::MIN + 3);
        let rhs = Repr::<2>::new(IBig::ONE, isize::MAX - 1);
        assert_eq!(
            Context::<mode::HalfEven>::new(8).div(&lhs, &rhs),
            Err(FpError::Underflow(Sign::Positive))
        );
    }

    // ---- short-quotient path selection ----

    #[test]
    fn test_short_div_and_inv_engage() {
        let a = Repr::<2>::new((IBig::ONE << 2600) / 3 + 17, 3);
        let b = Repr::<2>::new((IBig::ONE << 2400) / 37 + (IBig::ONE << 2380) / 11 + 19, -7);
        let ctx = Context::<mode::HalfEven>::new(2400);
        assert!(ctx.div_short(&a, &b).is_some());
        assert!(ctx.div_short(&Repr::one(), &b).is_some());
        let a10 = Repr::<10>::new(a.significand.clone(), 3);
        let b10 = Repr::<10>::new(b.significand.clone(), -7);
        let ctx10 = Context::<mode::HalfAway>::new(750);
        assert!(ctx10.div_short(&a10, &b10).is_some());
        assert!(ctx10.div_short(&Repr::one(), &b10).is_some());
        assert!(Context::<mode::HalfEven>::new(20)
            .div_short(&r2(7, 0), &r2(3, 0))
            .is_none());
    }

    #[test]
    fn test_round_short_quotient_boundaries() {
        let ctx = Context::<mode::HalfEven>::new(2);
        // Six-bit windows drop four bits: zero tail, midpoint, carry boundary.
        for value in [32, 40, 63] {
            assert!(round_short_quotient::<mode::HalfEven, 2>(
                &ctx,
                IBig::from(value),
                Sign::Positive,
                0,
                &UBig::from(2u8)
            )
            .is_none());
        }
        // The entire error band for 37 lies below the midpoint 40.
        assert_eq!(
            round_short_quotient::<mode::HalfEven, 2>(
                &ctx,
                IBig::from(37),
                Sign::Positive,
                0,
                &UBig::ONE
            ),
            Some(Inexact(r2(1, 5), Rounding::NoOp))
        );
    }

    #[test]
    fn test_short_div_exact_and_midpoint_decline() {
        let p = 2400;
        let divisor: IBig = (IBig::ONE << 2400) / 37 + 19;
        let b = Repr::<2>::new(divisor.clone(), 0);
        let ctx = Context::<mode::HalfEven>::new(p);
        assert!(ctx.div_short(&b, &b).is_none());
        let a = Repr::<2>::new(divisor * ((IBig::ONE << p) + 1), -(p as isize));
        assert!(ctx.div_short(&a, &b).is_none());
    }

    #[test]
    fn test_short_div_exponent_overflow_declines() {
        let significand: IBig = (IBig::ONE << 2400) / 37 + 19;
        let a = Repr::<2>::new(significand.clone(), isize::MAX - 5);
        let b = Repr::<2>::new(significand, -10);
        let ctx = Context::<mode::HalfEven>::new(2400);
        assert!(ctx.div_short(&a, &b).is_none());
    }

    // The `FBig / FBig` operator routes through `unwrap_fp`, so an exponent underflow saturates to
    // the directed endpoint (not a mode-blind signed zero): 2^isize::MIN / 3 ≈ 2^(isize::MIN − 2)
    // underflows; Up → smallest positive, Down → +0.
    #[test]
    fn test_div_directed_underflow() {
        use dashu_int::IBig;
        let p = 53;
        let floor_up = FBig::<mode::Up, 2>::from_parts(IBig::ONE, isize::MIN)
            .with_precision(p)
            .value();
        let floor_down = FBig::<mode::Down, 2>::from_parts(IBig::ONE, isize::MIN)
            .with_precision(p)
            .value();
        let three_up = FBig::<mode::Up, 2>::from_parts(IBig::from(3), 0)
            .with_precision(p)
            .value();
        let three_down = FBig::<mode::Down, 2>::from_parts(IBig::from(3), 0)
            .with_precision(p)
            .value();
        let up = floor_up / &three_up;
        let down = floor_down / &three_down;
        assert_eq!(up.repr().significand(), &IBig::ONE);
        assert_eq!(up.repr().exponent(), isize::MIN);
        assert!(down.repr().is_pos_zero());
        assert!(up > down);
    }
}
