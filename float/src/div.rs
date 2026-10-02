use crate::{
    error::{assert_finite_operands, assert_limited_precision, FpError, FpResult},
    fbig::FBig,
    helper_macros::{self, impl_binop_assign_by_taking},
    mul::{short_window_words, trim_word_len, WORD_BITS},
    repr::{rounded_to_repr, Context, Repr, Word},
    round::{Round, Rounded, Rounding},
    utils::{
        ceil_usize, digit_len, shl_digits, shl_digits_in_place, split_digits, split_digits_ref,
    },
};
use core::cmp::Ordering;
use core::ops::{Div, DivAssign, Rem, RemAssign};
use dashu_base::{
    AbsOrd, Approximation, BitTest, DivEuclid, DivRem, DivRemEuclid, EstimatedLog2, Inverse,
    RemEuclid, Sign, Signed, UnsignedAbs,
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
/// quotient: the true significand lies in `sig ± err_abs` (and is known to be
/// inexact). Returns `None` when the error band straddles a rounding boundary;
/// the caller then falls back to the exact quotient.
fn round_short_quotient<R: Round, const B: Word>(
    context: &Context<R>,
    sig: IBig,
    sign: Sign,
    exponent: isize,
    err_abs: &IBig,
) -> Option<Rounded<Repr<B>>> {
    let precision = context.precision();

    let digits = digit_len::<B>(&sig);
    if digits <= precision {
        // The window sizing guarantees a spare digit; bail out defensively.
        return None;
    }
    let shift = digits - precision;
    let exponent = exponent.checked_add(shift as isize)?;

    let (hi, lo) = split_digits_ref::<B>(&sig, shift);
    if lo.is_zero() {
        // The window's dropped digits are zero, so the error band reaches the
        // exact value (and a carry into the kept digits from below).
        return None;
    }
    let bshift: IBig = if B.is_power_of_two() {
        IBig::ONE << (shift * B.trailing_zeros() as usize)
    } else {
        UBig::from_word(B).pow(shift).into()
    };

    // The true fraction is `lo + delta` with `delta ∈ [−err_abs, err_abs]`.
    // Decide only when the whole band lies strictly inside (0, B^shift) — no
    // carry into the kept digits, no exact zero — and strictly on one side of
    // the rounding midpoint.
    let band_hi = &lo + err_abs;
    if band_hi >= bshift {
        return None;
    }
    let band_lo = &lo - err_abs;
    if !band_lo.is_positive() {
        return None;
    }
    let ordering = if (&band_hi << 1) < bshift {
        Ordering::Less
    } else if (&band_lo << 1) > bshift {
        Ordering::Greater
    } else {
        // The band straddles the midpoint (an exact tie lies inside it).
        return None;
    };

    let hi_signed = if sign == Sign::Negative {
        -hi.clone()
    } else {
        hi.clone()
    };
    let adjust = R::round_low_part(&hi_signed, sign, || ordering);
    let hi_signed = if sign == Sign::Negative { -hi } else { hi };
    let sig = hi_signed + adjust;
    // The fraction is certified strictly positive: the result is inexact.
    Some(Approximation::Inexact(
        rounded_to_repr(sig, exponent, sign == Sign::Negative),
        adjust,
    ))
}

impl<R: Round> Context<R> {
    /// Division kernel for an already-bounded dividend: `lhs` must carry at most
    /// `rhs.digits() + precision` digits (the `FBig` operators' operands always do — a
    /// significand is at most `precision + 1` digits and the divisor at least one). For an
    /// arbitrary [`Repr`] dividend use [`Context::div`], which bounds it exactly (see
    /// [`Self::repr_div_split`]).
    pub(crate) fn repr_div<const B: Word>(&self, lhs: Repr<B>, rhs: Repr<B>) -> FpResult<Repr<B>> {
        debug_assert!(
            digit_len::<B>(&lhs.significand) <= digit_len::<B>(&rhs.significand) + self.precision
        );
        // Fast path: decide the rounding from a certified high window of the
        // quotient (see `mul_short`); the exact quotient is the fallback.
        if let Some(rounded) = self.div_short(&lhs, &rhs) {
            return self.finish_rounded_repr(rounded);
        }
        self.repr_div_split(lhs, rhs, IBig::ZERO, 0)
    }

    /// Certified short-quotient fast path for division (and `inv`). Returns
    /// `None` to decline — unlimited precision, operands too small for the
    /// kernel, a divisor wider than the window, an over-wide dividend, or an
    /// undecided rounding — in which case the caller computes the exact
    /// quotient.
    pub(crate) fn div_short<const B: Word>(
        &self,
        lhs: &Repr<B>,
        rhs: &Repr<B>,
    ) -> Option<Rounded<Repr<B>>> {
        if self.precision() == 0 {
            return None; // unlimited precision: every digit is significant
        }
        if lhs.significand.is_zero() || rhs.significand.is_zero() {
            return None; // signed zeros and /0 belong to the exact path
        }
        let (ls, lw) = lhs.significand.as_sign_words();
        let (rs, rw) = rhs.significand.as_sign_words();
        let wa = trim_word_len(lw);
        let wb = trim_word_len(rw);

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
        if bl_a > bl_d + n * WORD_BITS + 2 {
            return None;
        }

        let a = UBig::from_words(&lw[..wa]);
        let b = UBig::from_words(&rw[..wb]);

        // Pad the dividend with base-B digits so the quotient is sized for the
        // window (bit length of numer − bit length of denom ≈ n·WORD_BITS);
        // the kernel verifies the exact band. One digit of adjustment absorbs
        // the float slack of the estimate; a second round should never be
        // needed but keeps the loop total.
        let (_, lb_ub) = B.log2_bounds();
        let target = (n * WORD_BITS) as f32 + (bl_d as f32 - bl_a as f32);
        // ceil of a possibly negative ratio (ceil_usize clamps negatives to 0)
        let mut t = ceil_usize(target / lb_ub);
        let mut numer = None;
        for _ in 0..3 {
            let mut candidate = IBig::from(a.clone());
            shl_digits_in_place::<B>(&mut candidate, t);
            let mag = candidate.unsigned_abs();
            let gap = mag.bit_len() as isize - bl_d as isize;
            if gap > (n * WORD_BITS + 2) as isize {
                if t == 0 {
                    break; // over-wide dividend: declined
                }
                t -= 1;
            } else if gap < (n * WORD_BITS - 2) as isize {
                t += 1;
            } else {
                numer = Some(mag);
                break;
            }
        }
        let numer = numer?;

        let q = high::div_high(&numer, &b, n)?;
        let sigma = (n * WORD_BITS + bl_d) as isize - numer.bit_len() as isize;
        debug_assert!((-2..=2).contains(&sigma));
        // The window approximates (numer/denom)·2^sigma; rescale it by
        // 2^-sigma so the value keeps the exponent `lhs.exp − rhs.exp − t`.
        // A right shift (sigma > 0) truncates by at most one ulp, folded into
        // the error bound.
        let (sig, err_up) = if sigma <= 0 {
            (IBig::from(&q << (-sigma) as usize), 0)
        } else {
            (IBig::from(&q >> sigma as usize), sigma as usize)
        };
        let err_abs = IBig::from(2 * n as u64 + 4) << err_up;

        let exponent = lhs
            .exponent
            .checked_sub(rhs.exponent)?
            .checked_sub(t as isize)?;
        round_short_quotient::<R, B>(self, sig, ls * rs, exponent, &err_abs)
    }

    /// [`Self::repr_div`] for an arbitrary-width dividend: the excess low digits below the
    /// `divisor digits + precision` bound are split off as `lo` and carried through the
    /// division as sticky rounding information, so no information is ever lost. (Rounding the
    /// dividend instead — as an earlier version of `Context::div` did — perturbs ties,
    /// direction and the exactness flag: the quotient of a *rounded* dividend can divide
    /// exactly where the true one doesn't, land on a false midpoint, or round the wrong way
    /// under directed modes.)
    ///
    /// The width check runs on the cheap `digits_ub`/`digits_lb` estimates, but the split
    /// point itself must be the EXACT digit count: splitting at exactly
    /// `divisor digits + precision` digits guarantees the high part is not below the divisor,
    /// which keeps the kernel's padding branches (they rescale the remainder without
    /// rescaling the sticky low part) off the `k > 0` path entirely. A lower split — e.g.
    /// derived from the bounds — is unsound for that reason.
    pub(crate) fn repr_div_any_width<const B: Word>(
        &self,
        lhs: Repr<B>,
        rhs: Repr<B>,
    ) -> FpResult<Repr<B>> {
        // (digits_ub of a zero dividend is 0, so zero takes the fast path too)
        if lhs.digits_ub() <= rhs.digits_lb() + self.precision {
            return self.repr_div(lhs, rhs);
        }

        // exact split (the sign of the dividend is preserved on both parts)
        let ddigits = digit_len::<B>(&rhs.significand);
        let k = digit_len::<B>(&lhs.significand).saturating_sub(ddigits + self.precision);
        if k == 0 {
            return self.repr_div(lhs, rhs);
        }
        let (hi, lo) = split_digits::<B>(lhs.significand, k);
        debug_assert_eq!(digit_len::<B>(&hi), ddigits + self.precision);
        // the kernel normalizes the divisor to be positive by negating BOTH operands, so the
        // sticky low part must follow the (negated) dividend's sign
        let lo = rhs.significand.sign() * lo;
        self.repr_div_split(
            Repr {
                significand: hi,
                exponent: lhs.exponent,
            },
            rhs,
            lo,
            k,
        )
    }

    /// The division kernel proper: computes `lhs / rhs` correctly rounded to this context's
    /// precision. `(lo, k)` describes the part of the dividend below its least significant
    /// kept digit: the true dividend is `(lhs.significand · B^k + lo) · B^lhs.exponent`, and
    /// the pair participates in the final rounding as a sticky fraction.
    fn repr_div_split<const B: Word>(
        &self,
        lhs: Repr<B>,
        rhs: Repr<B>,
        lo: IBig,
        k: usize,
    ) -> FpResult<Repr<B>> {
        assert_finite_operands(&lhs, &rhs);
        assert_limited_precision(self.precision);

        let sign_negative = lhs.sign() != rhs.sign();
        let sign = if sign_negative {
            Sign::Negative
        } else {
            Sign::Positive
        };

        if rhs.significand.is_zero() {
            if lhs.significand.is_zero() {
                // 0/0 is indeterminate; callers that can signal it (Context::div) check first,
                // otherwise fall through to div_rem which panics on division by zero.
            } else {
                // finite / 0 = ±inf (sign = XOR), returned as a value
                return Ok(Approximation::Exact(Repr::infinity_with_sign(sign)));
            }
        }

        // Work with a positive divisor so that the quotient and remainder from `div_rem`
        // (truncated division: the remainder carries the dividend's sign) keep the value's
        // sign in `q`, and `round_ratio` below sees a plain (integer + fraction) split.
        // Negation is O(1) (a sign flip on the shared buffer).
        let (num, den) = if rhs.significand.is_positive() {
            (lhs.significand, rhs.significand)
        } else {
            (-lhs.significand, -rhs.significand)
        };

        let (mut q, mut r) = num.div_rem(&den);
        let mut e = lhs
            .exponent
            .checked_add(k as isize)
            .and_then(|e| e.checked_sub(rhs.exponent))
            .ok_or({
                // lhs.exponent >= 0 whenever the addition overflows, and < 0 whenever the
                // subtraction underflows (see the digit bound on `k` above)
                if lhs.exponent >= 0 {
                    FpError::Overflow(sign)
                } else {
                    FpError::Underflow(sign)
                }
            })?;

        // From here on the digit counts must be EXACT (the `digits_ub`/`digits_lb` estimates
        // only bound a value): each padding shift below has to land the scaled operand on an
        // exact digit position, which the rounding invariants below lean on. `q`, `r` and
        // `den` are plain IBigs here (the bounds API lives on Repr), so `digit_len` is also
        // the cheapest exact source for them.
        let mut qdigits = digit_len::<B>(&q);

        // Exact division with the quotient already within the precision: nothing to scale and
        // nothing to round (the `lo` sticky part is empty exactly when k == 0).
        if r.is_zero() && k == 0 && qdigits <= self.precision {
            return Ok(Approximation::Exact(
                make_div_repr(sign_negative, q, e).check_finite_exponent()?,
            ));
        }

        if q.is_zero() {
            // num < den: scale the remainder up so the quotient has ~precision digits
            let ddigits = digit_len::<B>(&den);
            let rdigits = digit_len::<B>(&r); // rdigits <= ddigits
            let shift = ddigits + self.precision - rdigits;
            shl_digits_in_place::<B>(&mut r, shift);
            e = e
                .checked_sub(shift as isize)
                .ok_or(FpError::Underflow(sign))?;
            let (q0, r0) = r.div_rem(&den);
            q = q0;
            r = r0;
            // the scaled dividend has exactly ddigits+precision digits, so the quotient has
            // `precision` or `precision + 1` digits; with rdigits == ddigits the remainder is
            // still strictly below the divisor, which rules out the carry to B^precision
            qdigits = if rdigits == ddigits {
                self.precision
            } else {
                digit_len::<B>(&q)
            };
        } else if qdigits < self.precision {
            // TODO: here the operations can be optimized: 1. prevent double power, 2. q += q0 can be |= if B is power of 2
            let shift = self.precision - qdigits;
            shl_digits_in_place::<B>(&mut q, shift);
            shl_digits_in_place::<B>(&mut r, shift);
            e = e
                .checked_sub(shift as isize)
                .ok_or(FpError::Underflow(sign))?;

            let (q0, r0) = r.div_rem(&den);
            q += q0;
            r = r0;
            // q·B^shift ≤ B^precision − B^shift and q0 < B^shift, so the sum stays below
            // B^precision: the scaled quotient has exactly `precision` digits
            qdigits = self.precision;
        }

        // At this point the quotient is `(q + num_f/den_f)·B^e` with `den_f = den·B^k` and
        // `num_f = r·B^k + lo` (the remainder plus the dividend's sticky low part), where
        // `|num_f| < den_f` and `q` has at most `precision + 1` digits.
        let den_f = if k > 0 { shl_digits::<B>(&den, k) } else { den };
        let num_f = if k > 0 {
            shl_digits::<B>(&r, k) + lo
        } else {
            r
        };

        // An over-wide quotient (an exact division such as 15/3 at precision 2, or the p+1
        // digit quotient the scaling above can produce) must be rounded to the precision —
        // but in a *single* step: the dropped digit joins the fraction, so no information is
        // double-rounded away. The dividend bound guarantees the quotient carries at most one
        // extra digit, so exactly one digit is dropped here.
        let repr = if qdigits > self.precision {
            debug_assert_eq!(qdigits, self.precision + 1);
            let (qh, ql) = split_digits::<B>(q, 1);
            let e2 = e.saturating_add(1);

            // The fraction is (ql + num_f/den_f)/B; ql (when nonzero) and num_f both carry
            // the dividend's sign, and |num_f| < den_f, so the fraction's sign is ql's, or
            // num_f's when ql = 0. Its magnitude against the half is
            //   |ql·den_f + num_f|·2  vs  B·den_f,
            // and since ql and num_f share their sign, |ql·den_f + num_f| rearranges to a
            // comparison of |num_f| alone:
            //   |num_f|·2  vs  (B − 2·|ql|)·den_f
            // (halving both sides when B − 2·|ql| is even — always, for an even base).
            // The comparison lives in the closure so the directed modes (which ignore it)
            // never pay for it.
            let ql_is_zero = ql.is_zero();
            let low_sign = if ql_is_zero { num_f.sign() } else { ql.sign() };

            if ql_is_zero && num_f.is_zero() {
                Approximation::Exact(make_div_repr(sign_negative, qh, e2))
            } else {
                let adjust = R::round_low_part(&qh, low_sign, || {
                    // ql is a single base-B digit (|ql| < B), so its arithmetic runs in Word;
                    // on the comparison path below 2·|ql| ≤ B, so the subtraction cannot
                    // underflow.
                    let ql_mag: Word = ql.unsigned_abs().try_into().unwrap();
                    if ql_mag > B - ql_mag {
                        // 2·|ql| > B: the fraction's magnitude is past the half regardless of
                        // the remainder
                        Ordering::Greater
                    } else {
                        let diff = B - ql_mag * 2; // B − 2·|ql|
                        if diff % 2 == 0 {
                            // compare |num_f| against (B − 2·|ql|)/2 · den_f; the factors 0
                            // and 1 (the base-2 hot path) skip the multiplication
                            match diff / 2 {
                                0 => num_f.abs_cmp(&IBig::ZERO),
                                1 => num_f.abs_cmp(&den_f),
                                f => num_f.abs_cmp(&(f * &den_f)),
                            }
                        } else {
                            // odd base: double the remainder side instead of halving
                            (&num_f + &num_f).abs_cmp(&(diff * &den_f))
                        }
                    }
                });
                Approximation::Inexact(make_div_repr(sign_negative, qh + adjust, e2), adjust)
            }
        } else if num_f.is_zero() {
            Approximation::Exact(make_div_repr(sign_negative, q, e))
        } else {
            let adjust = R::round_ratio(&q, num_f, &den_f);
            Approximation::Inexact(make_div_repr(sign_negative, q + adjust, e), adjust)
        };
        let repr = match repr {
            Approximation::Exact(v) => Approximation::Exact(v.check_finite_exponent()?),
            Approximation::Inexact(v, flag) => {
                Approximation::Inexact(v.check_finite_exponent()?, flag)
            }
        };
        Ok(repr)
    }

    pub(crate) fn repr_rem<const B: Word>(&self, lhs: Repr<B>, rhs: Repr<B>) -> Rounded<Repr<B>> {
        assert_finite_operands(&lhs, &rhs);

        let lhs_is_neg_zero = lhs.is_neg_zero();
        let (lhs_sign, lhs_signif) = lhs.significand.into_parts();
        let (rhs_sign, rhs_signif) = rhs.significand.into_parts();
        let quotient_sign = lhs_sign * rhs_sign;

        use core::cmp::Ordering;
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
            match Repr::new(significand, exponent).check_finite_exponent() {
                Ok(repr) => self.repr_round(repr),
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

        // No operand pre-shrinking here: `repr_div_any_width` bounds an over-wide dividend by
        // an exact split, keeping the dropped digits as rounding information. Rounding the
        // dividend *before* dividing — as this used to do — corrupts the result: the rounded
        // dividend can divide exactly where the true one doesn't (a false `Exact` flag),
        // land the quotient on a false midpoint, or invert the direction under directed
        // modes.
        Ok(self
            .repr_div_any_width(lhs.clone(), rhs.clone())?
            .map(|v| FBig::new(v, *self)))
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
    use crate::round::mode;
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

    // The `FBig / FBig` operator routes through `unwrap_fp`, so an exponent underflow saturates to
    // the directed endpoint (not a mode-blind signed zero): 2^isize::MIN / 3 ≈ 2^(isize::MIN − 2)
    // underflows; Up → smallest positive, Down → +0.
    // ---- short-quotient fast path ----

    /// Deterministic pseudo-random words (top word forced nonzero).
    fn lcg_words(seed: u64, len: usize) -> Vec<Word> {
        // The generator state is u64 so the helper works for every word size;
        // each word folds both halves of the state into it.
        let mut s = seed | 1;
        (0..len)
            .map(|i| {
                s = s
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let w = ((s >> 32) ^ s) as Word;
                if i + 1 == len && w == 0 {
                    1
                } else {
                    w
                }
            })
            .collect()
    }

    fn sig_of(words: &[Word]) -> IBig {
        IBig::from(UBig::from_words(words))
    }

    /// Words a p-digit significand occupies in base B (one spare word for the
    /// guard digit a previous rounding may have left).
    fn words_for_precision(p: usize, base: Word) -> usize {
        let (_, lb) = base.log2_bounds();
        let bits = p as f32 * lb + WORD_BITS as f32;
        (bits as usize) / WORD_BITS + 2
    }

    /// Compare the short path against the p+60-digit oracle re-rounded to p,
    /// in value, rounding flag and exactness wrapper (the fuzz convention for
    /// division: a quotient has no finite exact form to compare against).
    fn check_div_short<R: Round, const B: Word>(p: usize, a: &Repr<B>, b: &Repr<B>) {
        let ctx = Context::<R>::new(p);
        let oracle = Context::<R>::new(p + 60)
            .div(a, b)
            .unwrap()
            .value()
            .with_precision(p);
        let got = ctx
            .div_short(a, b)
            .unwrap_or_else(|| panic!("div short declined: p={p} a={a:?} b={b:?}"));
        let Inexact(wv, wr) = oracle else {
            panic!("oracle unexpectedly exact: p={p} a={a:?} b={b:?}")
        };
        let Inexact(ref gv, ref gr) = got else {
            panic!("short path returned Exact")
        };
        assert_eq!(gv, wv.repr(), "value mismatch: p={p} a={a:?} b={b:?}",);
        assert_eq!(*gr, wr, "rounding flag mismatch: p={p} a={a:?} b={b:?}");
        // End-to-end through the public method.
        assert_eq!(ctx.div(a, b).unwrap().value().repr(), &got.value());
    }

    /// Fixed LCG operand sweep across precisions, bases and rounding modes.
    #[test]
    fn test_short_div_matches_oracle() {
        // (dividend words, divisor words) relative to the precision-sized word
        // count: equal precision (the primary target), a wider dividend (the
        // kernel drops its low words into the error bound), and a narrower
        // divisor (the window extends past it by a couple of words).
        // Sizes comfortably past the engagement floors (a divisor of at least
        // 32 words, i.e. 2048 bits, and a window of at least 28 words).
        for &(d_lead, b_lead) in &[(0isize, 0isize), (6, 0), (0, -2)] {
            for &p in &[2200usize, 3500] {
                let w = words_for_precision(p, 2) as isize;
                let a = Repr::<2>::new(sig_of(&lcg_words(0x51ed, (w + d_lead) as usize)), 3);
                let b = Repr::<2>::new(sig_of(&lcg_words(0x270d, (w + b_lead) as usize)), -7);
                check_div_short::<mode::Zero, 2>(p, &a, &b);
                check_div_short::<mode::Down, 2>(p, &a, &b);
                check_div_short::<mode::HalfEven, 2>(p, &a, &b);
            }
            for &p in &[700usize, 1200] {
                let w = words_for_precision(p, 10) as isize;
                let a = Repr::<10>::new(sig_of(&lcg_words(0x7a05, (w + d_lead) as usize)), 11);
                let b = Repr::<10>::new(sig_of(&lcg_words(0x3607, (w + b_lead) as usize)), -3);
                check_div_short::<mode::Away, 10>(p, &a, &b);
                check_div_short::<mode::Up, 10>(p, &a, &b);
                check_div_short::<mode::HalfAway, 10>(p, &a, &b);
            }
        }
    }

    /// All six modes agree with the oracle on one fixed pair.
    #[test]
    fn test_short_div_all_modes() {
        let (p, w) = (2400usize, words_for_precision(2400, 2));
        let a = Repr::<2>::new(sig_of(&lcg_words(0xbeef, w)), 5);
        let b = Repr::<2>::new(sig_of(&lcg_words(0xf00d, w - 1)), -2);
        check_div_short::<mode::Zero, 2>(p, &a, &b);
        check_div_short::<mode::Away, 2>(p, &a, &b);
        check_div_short::<mode::Up, 2>(p, &a, &b);
        check_div_short::<mode::Down, 2>(p, &a, &b);
        check_div_short::<mode::HalfEven, 2>(p, &a, &b);
        check_div_short::<mode::HalfAway, 2>(p, &a, &b);
    }

    /// The inverse goes through the same fast path (`repr_div`), with a
    /// single-word dividend padded up to the window.
    #[test]
    fn test_short_inv_matches_oracle() {
        let p = 2600;
        let w = words_for_precision(p, 2);
        let f = Repr::<2>::new(sig_of(&lcg_words(0x5eed, w)), 0);
        let ctx = Context::<mode::HalfEven>::new(p);
        let oracle = Context::<mode::HalfEven>::new(p + 60)
            .inv(&f)
            .unwrap()
            .value()
            .with_precision(p);
        let got = ctx.div_short(&Repr::one(), &f).expect("inv short declined");
        let Inexact(wv, _) = oracle else {
            panic!("oracle unexpectedly exact")
        };
        let Inexact(ref gv, _) = got else {
            panic!("short path returned Exact")
        };
        assert_eq!(gv, wv.repr());
        assert_eq!(ctx.inv(&f).unwrap().value().repr(), &got.value());
    }

    /// Boundary-leaning shapes where the short path must decline and the
    /// public API must reproduce the exact path bit for bit: an exact
    /// quotient, a near-midpoint quotient, negative operands.
    #[test]
    fn test_short_div_public_matches_oracle_tolerant() {
        let (p, w) = (2400usize, words_for_precision(2400, 2));
        let ctx_oracle = |a: &Repr<2>, b: &Repr<2>| {
            Context::<mode::HalfEven>::new(p + 60)
                .div(a, b)
                .unwrap()
                .value()
                .with_precision(p)
                .value()
                .repr()
                .clone()
        };
        let ctx = Context::<mode::HalfEven>::new(p);

        // Exact division: numerator == denominator -> quotient 1, Exact.
        let words = lcg_words(0xfeed, w);
        let a = Repr::<2>::new(sig_of(&words), 0);
        assert_eq!(ctx.div(&a, &a).unwrap().value().repr(), &ctx_oracle(&a, &a));

        // Half the divisor: exact quotient 0.5.
        let mut half = words.clone();
        half[0] &= !1; // even significand, halved
        let b = Repr::<2>::new(sig_of(&half) >> 1, 0);
        assert_eq!(ctx.div(&a, &b).unwrap().value().repr(), &ctx_oracle(&a, &b));

        // Negative signs on both operands.
        let (na, nb) = (
            Repr::<2>::new(-sig_of(&words), 4),
            Repr::<2>::new(-sig_of(&lcg_words(0xd00d, w - 2)), -1),
        );
        assert_eq!(ctx.div(&na, &nb).unwrap().value().repr(), &ctx_oracle(&na, &nb));

        // Small operands: far below the engagement floor.
        let (small_a, small_b) = (r2(12345, 0), r2(6789, 0));
        assert_eq!(
            ctx.div(&small_a, &small_b).unwrap().value().repr(),
            &ctx_oracle(&small_a, &small_b)
        );
    }

    /// Exponent saturation must keep flowing through the exact path's error
    /// semantics: the short path declines on exponent overflow.
    #[test]
    fn test_short_div_exponent_overflow_declines() {
        let aw = lcg_words(33, words_for_precision(2400, 2));
        // a huge exponent difference: (MAX − 5) − (−10) overflows on subtraction
        let a = Repr::<2>::new(sig_of(&aw), isize::MAX - 5);
        let b = Repr::<2>::new(sig_of(&lcg_words(44, words_for_precision(2400, 2))), -10);
        let ctx = Context::<mode::HalfEven>::new(2400);
        assert!(ctx.div_short(&a, &b).is_none());
        assert_eq!(ctx.div(&a, &b), Err(FpError::Overflow(Sign::Positive)));

        let c = Repr::<2>::new(sig_of(&aw), isize::MIN + 5);
        let d = Repr::<2>::new(sig_of(&lcg_words(55, words_for_precision(2400, 2))), 10);
        assert!(ctx.div_short(&c, &d).is_none());
        assert_eq!(ctx.div(&c, &d), Err(FpError::Underflow(Sign::Positive)));
    }

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
