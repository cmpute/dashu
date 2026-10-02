use dashu_base::{
    EstimatedLog2,
    Sign::{self, *},
};
use dashu_int::{high, IBig, UBig};

use crate::{
    add::cancel_zero,
    error::{FpError, FpResult},
    fbig::FBig,
    helper_macros,
    repr::{rounded_to_repr, Context, Repr, Word},
    round::{Round, Rounded, Rounding},
    utils::{ceil_usize, digit_len, split_digits_ref},
};
use core::cmp::Ordering;
use core::ops::{Mul, MulAssign};

// ---------------------------------------------------------------------------
// Short-product fast paths
//
// When both operands carry many more digits than the target precision, the
// exact product computes a long tail that rounding immediately discards — for
// equal-precision operands, about half of the multiplication work. Instead,
// compute only a certified high window of the product (the `high` kernels of
// dashu-int) and decide the rounding from the window directly.
//
// The window arrives with a one-sided error bound (the true product is at most
// `err` above the window value) and a sticky flag (false means the window is
// exact). That is exactly enough to reproduce the exact path's value, rounding
// flag and exactness wrapper: whenever the error band provably stays on one
// side of every rounding boundary, the classification is decided from the
// window; otherwise the fast path declines and the caller falls back to the
// exact product. The fallback window is narrow — the error is a few ulps of
// the window's last word, which the window sizing keeps far below the
// rounding midpoint.
// ---------------------------------------------------------------------------

/// Word size of [`Word`] on this target, used by the short-product sizing math.
pub(crate) const WORD_BITS: usize = core::mem::size_of::<Word>() * 8;

/// Number of words up to and including the last nonzero word.
pub(crate) fn trim_word_len(words: &[Word]) -> usize {
    words.iter().rposition(|&w| w != 0).map_or(0, |i| i + 1)
}

/// Window size (in words) for a short product that must certify rounding to
/// `precision` base-`B` digits when the product window undershoots the truth by
/// up to `err_ulps` units of its own least significant word.
///
/// The window needs (1) enough spare digits that the error stays well below
/// the rounding midpoint and cannot carry into the kept digits, and (2) one
/// further spare digit so the rounding split never degenerates (the window
/// always carries strictly more digits than the target precision).
pub(crate) fn short_window_words<const B: Word>(precision: usize, err_ulps: usize) -> usize {
    let (_, b_ub) = B.log2_bounds();
    // ceil(log2(err_ulps + 1)) as an exact bit length, avoiding `f32` methods
    // that are std-only on this crate's MSRV.
    let err_bits = usize::BITS - err_ulps.leading_zeros();
    let need_bits = precision as f32 * b_ub + err_bits as f32 + b_ub + 11.0;
    // Two spare words: one keeps the margin over the error bound even when
    // the product's top word is zero (the window then loses up to one word of
    // fill), and one keeps the split non-degenerate.
    ceil_usize(need_bits / WORD_BITS as f32) + 2
}

/// Largest extended window (beyond the smaller operand) offered to the kernel,
/// in bits (96 words on a 64-bit target). Extended windows always run the
/// windowed sweep directly, so they stay in the kernel's base-case band.
const EXT_WINDOW_LIMIT_BITS: usize = 96 * 64;

/// Clamp the wanted window to what the kernel accepts: at most two words
/// beyond the smaller operand. Clamping down is safe — the classifier's
/// defensive checks catch the rare case where the narrower window cannot
/// certify the rounding. Extended windows beyond the sweep's size band are
/// declined (they would always run the quadratic sweep).
fn clamp_window(want: usize, min_words: usize) -> Option<usize> {
    let n = want.min(min_words + 2);
    if n > min_words && n * WORD_BITS > EXT_WINDOW_LIMIT_BITS {
        None
    } else {
        Some(n)
    }
}

/// Heuristic gate: is a short product worthwhile for these operand and window
/// sizes? Costs are expressed in word-multiplication units.
fn short_path_worthwhile(wa: usize, wb: usize, n: usize) -> bool {
    // The windowed sweep does roughly half a window-sized multiplication (the
    // upper triangle) plus linear passes over the operands; the fixed term
    // covers the allocations, the word-level conversions and the
    // classification comparisons.
    const SWEEP_NUM: u128 = 11; // over 20
    const LINEAR: u128 = 2;
    const FIXED: u128 = 120;
    let full = wa as u128 * wb as u128;
    let short = n as u128 * n as u128 * SWEEP_NUM / 20 + (wa as u128 + wb as u128) * LINEAR + FIXED;
    full > short
}

/// Decide the correctly-rounded result from a certified high-product window.
///
/// `sig` is the positive window significand — already scaled up by the dropped
/// words for bases that cannot absorb them into the exponent — `err_abs`
/// bounds the one-sided shortfall (`0 <= true - sig < err_abs`), and `sticky`
/// tells whether the shortfall is known to be nonzero (a `false` flag makes
/// `sig` exact). Returns `None` when the error band straddles a rounding
/// boundary; the caller then falls back to the exact product.
fn round_high_product<R: Round, const B: Word>(
    context: &Context<R>,
    sig: IBig,
    sign: Sign,
    exponent: isize,
    sticky: bool,
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
    let adjust = if !lo.is_zero() || sticky {
        let bshift: IBig = if B.is_power_of_two() {
            IBig::ONE << (shift * B.trailing_zeros() as usize)
        } else {
            UBig::from_word(B).pow(shift).into()
        };
        let twice = &lo << 1;
        let ordering = if !sticky {
            // The window is exact: classify lo alone — exactly the comparison
            // `round_fract` makes for the exact value.
            twice.cmp(&bshift)
        } else {
            // The discarded digits of the true value are lo + delta with
            // 0 < delta < err_abs. Compare the doubled values against the
            // base power to classify against the rounding midpoint (and
            // against a carry into the kept digits) without constructing the
            // midpoint itself.
            let tail = &lo + err_abs;
            if (&tail << 1) <= bshift {
                // Below the midpoint even at the worst shortfall.
                Ordering::Less
            } else if twice > bshift {
                if tail >= bshift {
                    // The shortfall may carry into the kept digits.
                    return None;
                }
                Ordering::Greater
            } else if twice == bshift {
                if tail >= bshift {
                    return None;
                }
                // Exactly on the midpoint: the nonzero shortfall breaks the
                // tie upwards.
                Ordering::Greater
            } else {
                // Within the error band of the midpoint.
                return None;
            }
        };

        let hi_signed = if sign == Sign::Negative {
            -hi.clone()
        } else {
            hi.clone()
        };
        R::round_low_part(&hi_signed, sign, || ordering)
    } else {
        // Zero shortfall and zero discarded digits: the value is exact at this
        // split — the same shortcut `round_fract` takes for a zero fraction.
        Rounding::NoOp
    };

    let hi_signed = if sign == Sign::Negative { -hi } else { hi };
    let sig = hi_signed + adjust;
    Some(dashu_base::Approximation::Inexact(
        rounded_to_repr(sig, exponent, sign == Sign::Negative),
        adjust,
    ))
}

/// Shared `FBig * FBig` body: route through `Context::mul` + `unwrap_fp` so that an exponent
/// overflow/underflow saturates to the directed endpoint (not the mode-blind `±∞`/signed zero the
/// raw `Repr` kernel produces). `Context::mul` re-derives `FpError::Overflow/Underflow` from the
/// saturated `Repr`, and `unwrap_fp` picks the mode-aware endpoint.
#[inline]
fn mul_fbig<R: Round, const B: Word>(lhs: &FBig<R, B>, rhs: &FBig<R, B>) -> FBig<R, B> {
    let context = Context::max(lhs.context, rhs.context);
    context.unwrap_fp(context.mul(&lhs.repr, &rhs.repr))
}

impl<R: Round, const B: Word> Mul<&FBig<R, B>> for &FBig<R, B> {
    type Output = FBig<R, B>;

    #[inline]
    fn mul(self, rhs: &FBig<R, B>) -> Self::Output {
        mul_fbig(self, rhs)
    }
}

impl<R: Round, const B: Word> Mul<&FBig<R, B>> for FBig<R, B> {
    type Output = FBig<R, B>;

    #[inline]
    fn mul(self, rhs: &FBig<R, B>) -> Self::Output {
        mul_fbig(&self, rhs)
    }
}

impl<R: Round, const B: Word> Mul<FBig<R, B>> for &FBig<R, B> {
    type Output = FBig<R, B>;

    #[inline]
    fn mul(self, rhs: FBig<R, B>) -> Self::Output {
        mul_fbig(self, &rhs)
    }
}

impl<R: Round, const B: Word> Mul<FBig<R, B>> for FBig<R, B> {
    type Output = FBig<R, B>;

    #[inline]
    fn mul(self, rhs: FBig<R, B>) -> Self::Output {
        mul_fbig(&self, &rhs)
    }
}

helper_macros::impl_binop_assign_by_taking!(impl MulAssign<Self>, mul_assign, mul);

macro_rules! impl_mul_primitive_with_fbig {
    ($($t:ty)*) => {$(
        helper_macros::impl_binop_with_primitive!(impl Mul<$t>, mul);
        helper_macros::impl_binop_assign_with_primitive!(impl MulAssign<$t>, mul_assign);
    )*};
}
impl_mul_primitive_with_fbig!(u8 u16 u32 u64 u128 usize UBig i8 i16 i32 i64 i128 isize IBig);

impl<R: Round, const B: Word> FBig<R, B> {
    /// Compute the square of this number (`self * self`)
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str("-1.234")?;
    /// assert_eq!(a.sqr(), DBig::from_str("1.523")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn sqr(&self) -> Self {
        self.context.unwrap_fp(self.context.sqr(&self.repr))
    }

    /// Compute the cubic of this number (`self * self * self`)
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str("-1.234")?;
    /// assert_eq!(a.cubic(), DBig::from_str("-1.879")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn cubic(&self) -> Self {
        self.context.unwrap_fp(self.context.cubic(&self.repr))
    }

    /// Fused multiply–add with a single rounding: `c + sign·(self * b)`.
    ///
    /// Unlike `(self * b) + c`, which rounds twice, `fma` rounds the exact
    /// `self * b + c` once. `sign` scales the product: [`Sign::Positive`] gives
    /// `self*b + c`, [`Sign::Negative`] gives `c − self*b`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::{ParseError, Sign};
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str("1.5")?;
    /// let b = DBig::from_str("2.0")?;
    /// let c = DBig::from_str("0.1")?;
    /// // 1.5*2.0 + 0.1 = 3.1
    /// assert_eq!(a.fma(&b, &c, Sign::Positive), DBig::from_str("3.1")?);
    /// // 0.1 − 1.5*2.0 = −2.9
    /// assert_eq!(a.fma(&b, &c, Sign::Negative), DBig::from_str("-2.9")?);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn fma(&self, b: &Self, c: &Self, sign: Sign) -> Self {
        let context = Context::max(self.context, Context::max(b.context, c.context));
        context.unwrap_fp(context.fma(&self.repr, &b.repr, &c.repr, sign))
    }
}

impl<R: Round> Context<R> {
    /// Certified high-product fast path for multiplication. Returns `None` to
    /// decline (unlimited precision, tiny operands, or an undecided rounding);
    /// the caller then computes the exact product.
    pub(crate) fn mul_short<const B: Word>(
        &self,
        lhs: &Repr<B>,
        rhs: &Repr<B>,
    ) -> Option<Rounded<Repr<B>>> {
        if self.precision() == 0 {
            return None; // unlimited precision: every digit is significant
        }
        if lhs.significand.is_zero() || rhs.significand.is_zero() {
            return None; // let the exact path produce the signed zeros
        }
        let (ls, lw) = lhs.significand.as_sign_words();
        let (rs, rw) = rhs.significand.as_sign_words();
        let wa = trim_word_len(lw);
        let wb = trim_word_len(rw);

        // Size the window, then once more with the actual error bound
        // (n + 2 ulps) that this window implies. The window may extend up to
        // two words beyond the smaller operand (the equal-precision case).
        let n_want = short_window_words::<B>(self.precision(), 1 << 12);
        let n_want = short_window_words::<B>(self.precision(), n_want + 2);
        let n = clamp_window(n_want, wa.min(wb))?;
        if !short_path_worthwhile(wa, wb, n) {
            return None;
        }

        let dropped = wa + wb - n;
        // Power-of-two bases fold the dropped words into the exponent; any
        // remainder bits that do not make up whole base digits stay as (zero)
        // low words of the significand, with the error bound scaled
        // accordingly — same as for bases that are not powers of two, which
        // always keep the full dropped words as padding.
        let lb = B.trailing_zeros() as usize;
        let fold_bits = WORD_BITS * dropped;
        let pad_bits = if B.is_power_of_two() {
            fold_bits % lb
        } else {
            fold_bits
        };
        let exponent = lhs.exponent.checked_add(rhs.exponent)?;
        let exponent = if B.is_power_of_two() {
            exponent.checked_add((fold_bits / lb) as isize)?
        } else {
            exponent
        };

        let a = UBig::from_words(&lw[..wa]);
        let b = UBig::from_words(&rw[..wb]);
        let (window, sticky) = high::mul_high(&a, &b, n);

        let sig = if pad_bits == 0 {
            IBig::from(window)
        } else {
            IBig::from(window << pad_bits)
        };
        let err_abs = IBig::from(n as u64 + 2) << pad_bits;
        round_high_product::<R, B>(self, sig, ls * rs, exponent, sticky, &err_abs)
    }

    /// Certified high-product fast path for squaring. See [`Self::mul_short`].
    ///
    /// Unlike multiplication, squaring never extends the window past the
    /// operand: the dedicated squaring kernels already exploit the symmetric
    /// product, so an extended windowed sweep cannot beat them. (Squaring
    /// still benefits when the operand carries many more digits than the
    /// target precision.)
    pub(crate) fn sqr_short<const B: Word>(&self, f: &Repr<B>) -> Option<Rounded<Repr<B>>> {
        if self.precision() == 0 || f.significand.is_zero() {
            return None;
        }
        let (_, fw) = f.significand.as_sign_words();
        let wa = trim_word_len(fw);

        let n_want = short_window_words::<B>(self.precision(), 1 << 12);
        let n_want = short_window_words::<B>(self.precision(), n_want + 2);
        let n = n_want.min(wa);
        if n_want > wa || !short_path_worthwhile(wa, wa, n) {
            return None;
        }

        let dropped = 2 * wa - n;
        let lb = B.trailing_zeros() as usize;
        let fold_bits = WORD_BITS * dropped;
        let pad_bits = if B.is_power_of_two() {
            fold_bits % lb
        } else {
            fold_bits
        };
        let exponent = f.exponent.checked_mul(2)?;
        let exponent = if B.is_power_of_two() {
            exponent.checked_add((fold_bits / lb) as isize)?
        } else {
            exponent
        };

        let a = UBig::from_words(&fw[..wa]);
        let (window, sticky) = high::sqr_high(&a, n);

        let sig = if pad_bits == 0 {
            IBig::from(window)
        } else {
            IBig::from(window << pad_bits)
        };
        let err_abs = IBig::from(n as u64 + 2) << pad_bits;
        round_high_product::<R, B>(self, sig, Sign::Positive, exponent, sticky, &err_abs)
    }

    /// Certified high-product fast path for the cubic power: a short square
    /// followed by a short multiply, with the composed error bound. See
    /// [`Self::mul_short`].
    pub(crate) fn cubic_short<const B: Word>(&self, f: &Repr<B>) -> Option<Rounded<Repr<B>>> {
        if self.precision() == 0 || f.significand.is_zero() {
            return None; // the exact path keeps the -0 sign handling
        }
        let (fs, fw) = f.significand.as_sign_words();
        let wa = trim_word_len(fw);

        // The composed bound is (n1 + 2) + (n2 + 2) + 2 ulps of the final
        // window; size both windows against it.
        let n0 = short_window_words::<B>(self.precision(), 1 << 13);
        let n_want = short_window_words::<B>(self.precision(), 2 * n0 + 8);
        let n = clamp_window(n_want, wa)?;
        if !short_path_worthwhile(wa, wa, n) {
            return None;
        }

        let a = UBig::from_words(&fw[..wa]);
        let (square, sticky1) = high::sqr_high(&a, n);
        // The square's window may carry a leading zero word; the second
        // window clamps to it (plus the two-word extension). The extension
        // rescales the square's shortfall onto a finer final window, which
        // the composed bound below accounts for explicitly.
        let square_words = trim_word_len(square.as_words());
        let n2 = clamp_window(n, square_words.min(wa))?;
        let (window, sticky2) = high::mul_high(&square, &a, n2);

        let dropped = (2 * wa - n) + (square_words + wa - n2);
        let lb = B.trailing_zeros() as usize;
        let fold_bits = WORD_BITS * dropped;
        let pad_bits = if B.is_power_of_two() {
            fold_bits % lb
        } else {
            fold_bits
        };
        let exponent = f.exponent.checked_mul(3)?;
        let exponent = if B.is_power_of_two() {
            exponent.checked_add((fold_bits / lb) as isize)?
        } else {
            exponent
        };

        let sig = if pad_bits == 0 {
            IBig::from(window)
        } else {
            IBig::from(window << pad_bits)
        };
        // Composed bound: (n + 2) ulps from the square, amplified by
        // `a / 2^(square_words + wa - n2)·W < 2^((n2 - square_words)·W)` when
        // the second window extends past the square's word length, plus
        // (n2 + 2) ulps from the short product.
        let square_err_shift = n2.saturating_sub(square_words) * WORD_BITS;
        let err_abs = ((IBig::from(n as u64 + 2) << square_err_shift) + IBig::from(n2 as u64 + 2))
            << pad_bits;
        round_high_product::<R, B>(self, sig, fs, exponent, sticky1 || sticky2, &err_abs)
    }
}

impl<R: Round> Context<R> {
    /// Multiply two floating point numbers under this context.
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
    /// assert_eq!(
    ///     context.mul(&a.repr(), &b.repr()),
    ///     Ok(Inexact(DBig::from_str("-8.4")?, SubOne))
    /// );
    /// # Ok::<(), ParseError>(())
    /// ```
    pub fn mul<const B: Word>(&self, lhs: &Repr<B>, rhs: &Repr<B>) -> FpResult<FBig<R, B>> {
        if lhs.is_infinite() || rhs.is_infinite() {
            return Err(FpError::InfiniteInput);
        }

        // Fast path: when the operands carry far more digits than the target
        // precision, decide the rounding from a certified high window of the
        // product instead of the full exact product.
        if let Some(rounded) = self.mul_short(lhs, rhs) {
            return self.finish_rounded(rounded);
        }

        // Exact product of the full operands, then round. (An earlier version shrank each operand
        // to 2*precision — via `repr_round_ref`, which rounds each operand *correctly* to 2p digits —
        // before multiplying. But rounding the operands *before* multiplying perturbs the product
        // by the accumulated operand-rounding error (~2^-2p relative), so rounding that perturbed
        // product to `precision` could land 1 ulp off the exact-product-rounded value when the true
        // product sat near a rounding boundary. The exact product is always correctly rounded; the
        // shrink only mattered for operands far larger than the target precision, which is uncommon.)
        let repr = lhs * rhs;
        let repr = if repr.is_infinite() {
            return Err(FpError::Overflow(repr.sign()));
        } else if repr.significand.is_zero()
            && !lhs.significand.is_zero()
            && !rhs.significand.is_zero()
        {
            return Err(FpError::Underflow(repr.sign()));
        } else {
            repr
        };
        // The rounded form can still leave the finite exponent range when the
        // product's significand is much wider than the precision (the split
        // exponent `exponent + shift` saturates); report that as overflow too.
        self.finish_rounded(self.repr_round(repr))
    }

    /// Calculate the square of the floating point number under this context.
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
    /// assert_eq!(context.sqr(&a.repr()), Ok(Inexact(DBig::from_str("1.5")?, NoOp)));
    /// # Ok::<(), ParseError>(())
    /// ```
    pub fn sqr<const B: Word>(&self, f: &Repr<B>) -> FpResult<FBig<R, B>> {
        if f.is_infinite() {
            return Err(FpError::InfiniteInput);
        }

        // Fast path: certified high window of the square (see `Context::mul`).
        if let Some(rounded) = self.sqr_short(f) {
            return self.finish_rounded(rounded);
        }

        // Exact square of the full significand, then round. (An earlier version shrank the operand
        // to 2*precision before squaring, but that pre-rounding perturbs the square and could leave
        // the result 1 ulp off the correctly-rounded value near a rounding boundary — same issue
        // as `mul`. The dedicated `sqr` kernel is still used; it just gets the full significand.)
        let exponent = f.exponent.checked_mul(2).ok_or({
            // sqr always produces a non-negative result
            if f.exponent > 0 {
                FpError::Overflow(Positive)
            } else {
                FpError::Underflow(Positive)
            }
        })?;
        let repr = Repr::new(f.significand.sqr().into(), exponent);
        let repr = repr.check_finite_exponent()?;
        self.finish_rounded(self.repr_round(repr))
    }

    /// Calculate the cubic of the floating point number under this context.
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
    /// assert_eq!(context.cubic(&a.repr()), Ok(Inexact(DBig::from_str("-1.9")?, SubOne)));
    /// # Ok::<(), ParseError>(())
    /// ```
    pub fn cubic<const B: Word>(&self, f: &Repr<B>) -> FpResult<FBig<R, B>> {
        if f.is_infinite() {
            return Err(FpError::InfiniteInput);
        }

        // Fast path: two chained certified high products (square, then
        // multiply) with a composed error bound (see `Context::mul`).
        if let Some(rounded) = self.cubic_short(f) {
            return self.finish_rounded(rounded);
        }

        // Exact cube of the full significand, then round. (An earlier version shrank the operand
        // to 3*precision before cubing, but that pre-rounding perturbs the cube and could leave the
        // result 1 ulp off the correctly-rounded value near a rounding boundary — same issue as
        // `mul`. The dedicated `cubic` kernel is still used; it just gets the full significand.)
        let repr = if f.significand.is_zero() {
            // cubic(±0) = ±0 (odd power preserves sign)
            if f.is_neg_zero() {
                Repr::neg_zero()
            } else {
                Repr::zero()
            }
        } else {
            let sign = f.sign();
            let exponent = f.exponent.checked_mul(3).ok_or({
                if f.exponent > 0 {
                    FpError::Overflow(sign)
                } else {
                    FpError::Underflow(sign)
                }
            })?;
            let repr = Repr::new(f.significand.cubic(), exponent);
            repr.check_finite_exponent()?
        };
        self.finish_rounded(self.repr_round(repr))
    }

    /// Fused multiply–add under this context: `c + sign·(a·b)`, rounded once.
    ///
    /// The product `a·b` is formed exactly, then added to `c` with a single
    /// rounding (reusing the aligned-then-round path of [`add`](Self::add), so the
    /// severe-cancellation and sticky-tail handling is identical — including the
    /// single guard digit an effective subtraction may leave in the result).
    /// `sign` scales the product: [`Sign::Positive`] → `a·b + c`,
    /// [`Sign::Negative`] → `c − a·b`.
    ///
    /// Returns [`FpError::InfiniteInput`] if any operand is infinite (matching
    /// [`add`](Self::add)/[`mul`](Self::mul); dashu rejects infinite operands
    /// outright, so the IEEE-754 `inf·0` / `inf−inf` indeterminate forms do not
    /// arise). [`Overflow`](FpError::Overflow)/[`Underflow`](FpError::Underflow)
    /// propagate from the product's exponent.
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::str::FromStr;
    /// # use dashu_base::{Approximation::*, ParseError, Sign};
    /// # use dashu_float::{Context, DBig, round::{mode::HalfAway, Rounding::*}};
    /// let context = Context::<HalfAway>::new(2);
    /// let a = DBig::from_str("1.5")?;
    /// let b = DBig::from_str("2.0")?;
    /// let c = DBig::from_str("0.1")?;
    /// assert_eq!(
    ///     context.fma(&a.repr(), &b.repr(), &c.repr(), Sign::Positive),
    ///     Ok(Exact(DBig::from_str("3.1")?))
    /// );
    /// # Ok::<(), ParseError>(())
    /// ```
    pub fn fma<const B: Word>(
        &self,
        a: &Repr<B>,
        b: &Repr<B>,
        c: &Repr<B>,
        sign: Sign,
    ) -> FpResult<FBig<R, B>> {
        if a.is_infinite() || b.is_infinite() || c.is_infinite() {
            return Err(FpError::InfiniteInput);
        }

        // Exact product a·b. No operand shrinking (unlike Context::mul's 2p bound):
        // a cancellation between the product and c can expose arbitrarily low
        // product digits, so the full exact product is required for a correctly-
        // rounded result. The `Repr` product saturates exponent overflow/underflow
        // to the infinity/zero sentinels, so detect those as Context::mul does.
        let prod = a * b;
        let prod = if prod.is_infinite() {
            return Err(FpError::Overflow(prod.sign()));
        } else if prod.significand.is_zero() && !a.significand.is_zero() && !b.significand.is_zero()
        {
            return Err(FpError::Underflow(prod.sign()));
        } else {
            prod
        };

        // Add c to sign·(a·b) with a single rounding. The product is exact, so the
        // only rounding is in the add step — the same path as Context::add/sub.
        let sum = if prod.significand.is_zero() {
            // a·b == ±0: the signed zero product adds nothing to c.
            self.repr_round_ref(c)
        } else {
            let signed_prod = if sign == Negative { prod.neg() } else { prod };
            if c.significand.is_zero() {
                // c == ±0: the result is sign·(a·b), rounded once.
                self.repr_round(signed_prod)
            } else {
                match c.exponent.cmp(&signed_prod.exponent) {
                    Ordering::Equal => self.repr_round(cancel_zero::<R, B>(
                        &c.significand + signed_prod.significand,
                        c.exponent,
                    )),
                    Ordering::Greater => {
                        self.repr_add_large_small(c.clone(), &signed_prod, Positive)
                    }
                    Ordering::Less => self.repr_add_small_large(c.clone(), &signed_prod, Positive),
                }
            }
        };
        self.finish_rounded(sum)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::round::mode;
    use dashu_int::IBig;

    /// Reference: `c + sign·(a·b)` computed exactly at `4p+32` digits then rounded
    /// down to `p`. A correctly-rounded `fma` must agree with this.
    fn oracle<const B: Word, R: Round>(
        a: &Repr<B>,
        b: &Repr<B>,
        c: &Repr<B>,
        sign: Sign,
        p: usize,
    ) -> FBig<R, B> {
        let hi = Context::<R>::new(p * 4 + 32);
        let prod = hi.mul(a, b).unwrap().value();
        let signed = if sign == Negative { -prod } else { prod };
        let sum = hi.add(c, signed.repr()).unwrap().value();
        sum.with_precision(p).value()
    }

    fn r<const B: Word>(sig: i128, exp: isize) -> Repr<B> {
        Repr::new(IBig::from(sig), exp)
    }

    /// Force-round `v`'s significand to exactly `p` digits. (`with_precision` is a
    /// no-op when the context precision already equals `p`; the guard digit an
    /// effective subtraction leaves lives in the significand, beyond the context
    /// precision, so it must be rounded away explicitly.)
    fn round_sig<R: Round, const B: Word>(v: &FBig<R, B>, p: usize) -> FBig<R, B> {
        let ctx = Context::<R>::new(p);
        FBig::new(ctx.repr_round_ref(v.repr()).value(), ctx)
    }

    /// `fma` matches the high-precision oracle across fixed inputs, precisions,
    /// both signs, base 10. (FMA reuses the add path, so on an effective
    /// subtraction it may carry one guard digit — like `Context::sub` — so we
    /// re-round to `p` before comparing to the exactly-`p` oracle.)
    #[test]
    fn test_fma_matches_oracle_decimal() {
        // (a sig, a exp, b sig, b exp, c sig, c exp)
        let cases: &[(i128, isize, i128, isize, i128, isize)] = &[
            (15, -1, 20, -1, 10, -1),     // 1.5·2.0 + 0.1
            (123, -2, 456, -2, 789, -2),  // 1.23·4.56 + 7.89
            (101, -2, 99, -2, -9999, -4), // 1.01·0.99 − 0.9999 ≈ 0 (cancellation, a≠b)
            (999, -2, 101, -1, -1, 2),    // 9.99·10.1 − 100 (mild cancel, diff exponents)
        ];
        for &(asg, ae, bsg, be, csg, ce) in cases {
            for &p in &[2usize, 5, 20] {
                let (a, b, c) = (r::<10>(asg, ae), r::<10>(bsg, be), r::<10>(csg, ce));
                let ctx = Context::<mode::HalfAway>::new(p);
                for sign in [Positive, Negative] {
                    let got = ctx.fma(&a, &b, &c, sign).unwrap().value();
                    let want = oracle::<10, mode::HalfAway>(&a, &b, &c, sign, p);
                    assert_eq!(
                        round_sig(&got, p),
                        want,
                        "fma mismatch p={p} sign={sign:?} a={asg}e{ae} b={bsg}e{be} c={csg}e{ce}"
                    );
                }
            }
        }
    }

    /// Base-2 spot check (HalfEven).
    #[test]
    fn test_fma_matches_oracle_binary() {
        let (a, b, c) = (r::<2>(5, -2), r::<2>(3, -1), r::<2>(7, -3)); // 1.25, 1.5, 0.875
        for &p in &[4usize, 10, 30] {
            let ctx = Context::<mode::HalfEven>::new(p);
            for sign in [Positive, Negative] {
                let got = ctx.fma(&a, &b, &c, sign).unwrap().value();
                let want = oracle::<2, mode::HalfEven>(&a, &b, &c, sign, p);
                assert_eq!(round_sig(&got, p), want, "base-2 fma mismatch p={p} sign={sign:?}");
            }
        }
    }

    /// A zero product ⇒ result is `c`; a zero `c` ⇒ result is `a·b`.
    #[test]
    fn test_fma_zero_operands() {
        let ctx = Context::<mode::HalfAway>::new(5);
        let (z, a, c) = (r::<10>(0, 0), r::<10>(3, 0), r::<10>(7, 0));
        // a·b == 0 (z·a): result is c.
        assert_eq!(ctx.fma(&z, &a, &c, Positive).unwrap().value().repr(), &c);
        // c == 0: result is a·b (3·3 = 9).
        assert_eq!(ctx.fma(&a, &a, &z, Positive).unwrap().value().repr(), &r::<10>(9, 0));
    }

    /// Any infinite operand ⇒ `InfiniteInput`.
    #[test]
    fn test_fma_infinity_is_error() {
        let ctx = Context::<mode::HalfAway>::new(5);
        let (inf, a) = (Repr::<10>::infinity(), r::<10>(3, 0));
        assert_eq!(ctx.fma(&inf, &a, &a, Positive), Err(FpError::InfiniteInput));
        assert_eq!(ctx.fma(&a, &a, &inf, Positive), Err(FpError::InfiniteInput));
    }

    /// An exact-zero result is `-0` under roundTowardNegative (Down), exercising
    /// the `cancel_zero` path (IEEE 754 §6.3).
    #[test]
    fn test_fma_exact_zero_is_neg_zero_under_down() {
        let ctx = Context::<mode::Down>::new(5);
        // 2·3 + (-6) = 0 exactly.
        let (a, b, c) = (r::<10>(2, 0), r::<10>(3, 0), r::<10>(-6, 0));
        let got = ctx.fma(&a, &b, &c, Positive).unwrap().value();
        assert!(got.repr().is_neg_zero(), "expected -0, got {:?}", got.repr());
    }

    // The `FBig * FBig` operator routes through `Context::mul` + `unwrap_fp`, so an exponent
    // underflow saturates to the directed endpoint (not a mode-blind signed zero): 2^isize::MIN · 0.5
    // = 2^(isize::MIN − 1) underflows; Up → smallest positive, Down → +0.
    #[test]
    fn test_mul_directed_underflow() {
        let p = 53;
        let floor_up = FBig::<mode::Up, 2>::from_parts(IBig::ONE, isize::MIN)
            .with_precision(p)
            .value();
        let floor_down = FBig::<mode::Down, 2>::from_parts(IBig::ONE, isize::MIN)
            .with_precision(p)
            .value();
        let half_up = FBig::<mode::Up, 2>::from_parts(IBig::ONE, -1)
            .with_precision(p)
            .value();
        let half_down = FBig::<mode::Down, 2>::from_parts(IBig::ONE, -1)
            .with_precision(p)
            .value();
        let up = floor_up * &half_up;
        let down = floor_down * &half_down;
        assert_eq!(up.repr().significand(), &IBig::ONE);
        assert_eq!(up.repr().exponent(), isize::MIN);
        assert!(down.repr().is_pos_zero());
        assert!(up > down);
    }

    // ---- short-product fast paths ----

    use dashu_base::Approximation::*;

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

    fn sig(words: &[Word]) -> IBig {
        IBig::from(UBig::from_words(words))
    }

    /// Scale a word count tuned for 64-bit words to the current word size, so
    /// the fixed operands always carry enough bits for the largest tested
    /// precision (500 decimal digits) on every target.
    fn scale(n: usize) -> usize {
        n * (64 / WORD_BITS).max(1)
    }

    /// Assert the short path matches the unlimited-precision oracle re-rounded
    /// to the same precision, in value, rounding flag and exactness wrapper.
    fn assert_short_equals_oracle<R: Round, const B: Word>(
        got: &Rounded<Repr<B>>,
        oracle: Rounded<FBig<R, B>>,
        what: &str,
    ) {
        let Inexact(wv, wr) = oracle else {
            panic!("{what}: oracle unexpectedly exact");
        };
        let Inexact(gv, gr) = got else {
            panic!("{what}: short path returned Exact");
        };
        assert_eq!(gv, wv.repr(), "{what}: value mismatch");
        assert_eq!(*gr, wr, "{what}: rounding flag mismatch");
    }

    fn check_mul_short<R: Round, const B: Word>(p: usize, a: &Repr<B>, b: &Repr<B>) {
        let ctx = Context::<R>::new(p);
        let oracle = Context::<R>::new(0)
            .mul(a, b)
            .unwrap()
            .value()
            .with_precision(p);
        let got = match ctx.mul_short(a, b) {
            Some(g) => g,
            None => panic!("mul short declined: p={p} a={a:?} b={b:?}"),
        };
        assert_short_equals_oracle(&got, oracle, "mul");
        // End-to-end through the public method.
        assert_eq!(ctx.mul(a, b).unwrap().value().repr(), &got.value());
    }

    fn check_sqr_short<R: Round, const B: Word>(p: usize, f: &Repr<B>) {
        let ctx = Context::<R>::new(p);
        let oracle = Context::<R>::new(0)
            .sqr(f)
            .unwrap()
            .value()
            .with_precision(p);
        let got = ctx.sqr_short(f).expect("short path declined");
        assert_short_equals_oracle(&got, oracle, "sqr");
        assert_eq!(ctx.sqr(f).unwrap().value().repr(), &got.value());
    }

    fn check_cubic_short<R: Round, const B: Word>(p: usize, f: &Repr<B>) {
        let ctx = Context::<R>::new(p);
        let oracle = Context::<R>::new(0)
            .cubic(f)
            .unwrap()
            .value()
            .with_precision(p);
        let got = ctx.cubic_short(f).expect("short path declined");
        assert_short_equals_oracle(&got, oracle, "cubic");
        assert_eq!(ctx.cubic(f).unwrap().value().repr(), &got.value());
    }

    /// Correctness check through the public API only: the short path may or
    /// may not engage (structured operands can collapse the certified margin,
    /// in which case declining to the exact product is the correct outcome).
    fn check_public_matches_oracle<R: Round, const B: Word>(
        p: usize,
        a: &Repr<B>,
        b: &Repr<B>,
        f: &Repr<B>,
    ) {
        let ctx = Context::<R>::new(p);
        let oracle = Context::<R>::new(0);
        assert_eq!(
            ctx.mul(a, b).unwrap().value().repr(),
            oracle
                .mul(a, b)
                .unwrap()
                .value()
                .with_precision(p)
                .value()
                .repr()
        );
        assert_eq!(
            ctx.sqr(f).unwrap().value().repr(),
            oracle
                .sqr(f)
                .unwrap()
                .value()
                .with_precision(p)
                .value()
                .repr()
        );
        assert_eq!(
            ctx.cubic(f).unwrap().value().repr(),
            oracle
                .cubic(f)
                .unwrap()
                .value()
                .with_precision(p)
                .value()
                .repr()
        );
    }

    /// All six modes and both standard bases over the same (a, b) pair.
    fn check_all_modes<const B: Word>(p: usize, a: &Repr<B>, b: &Repr<B>, f: &Repr<B>) {
        check_mul_short::<mode::Zero, B>(p, a, b);
        check_mul_short::<mode::Away, B>(p, a, b);
        check_mul_short::<mode::Up, B>(p, a, b);
        check_mul_short::<mode::Down, B>(p, a, b);
        check_mul_short::<mode::HalfEven, B>(p, a, b);
        check_mul_short::<mode::HalfAway, B>(p, a, b);
        check_sqr_short::<mode::Zero, B>(p, f);
        check_sqr_short::<mode::HalfEven, B>(p, f);
        check_sqr_short::<mode::HalfAway, B>(p, f);
        check_cubic_short::<mode::Zero, B>(p, f);
        check_cubic_short::<mode::HalfEven, B>(p, f);
        check_cubic_short::<mode::HalfAway, B>(p, f);
    }

    /// Fixed LCG operand sweep across the mandated precision set, both bases,
    /// all six modes (mul) and the nearest/directed representatives (sqr/cubic).
    #[test]
    fn test_short_products_match_oracle() {
        let cases: &[(u64, usize, u64, usize)] = &[
            (1, 64, 2, 64),
            (3, 64, 4, 90),
            (5, 90, 6, 64),
            (7, 64, 8, 260),
        ];
        for &(sa, wa, sb, wb) in cases {
            let (aw, bw) = (lcg_words(sa, scale(wa)), lcg_words(sb, scale(wb)));
            for &p in &[2usize, 5, 20, 50, 100, 500] {
                for &(ea, eb) in &[(0isize, 0isize), (-500, 333)] {
                    check_all_modes::<2>(
                        p,
                        &Repr::<2>::new(sig(&aw), ea),
                        &Repr::<2>::new(sig(&bw), eb),
                        &Repr::<2>::new(sig(&aw), ea),
                    );
                    check_all_modes::<10>(
                        p,
                        &Repr::<10>::new(sig(&aw), ea),
                        &Repr::<10>::new(sig(&bw), eb),
                        &Repr::<10>::new(sig(&aw), ea),
                    );
                }
            }
        }
    }

    /// Equal-precision operands — the common `FBig` shape where each
    /// significand sits exactly at the context precision — engage the short
    /// path through the extended window (up to two words beyond the
    /// operands). Squaring declines here: its dedicated kernels already
    /// exploit the symmetric product. Engagement is asserted only when the
    /// extended window has a comfortable spare over the precision; on narrow
    /// words the spare can shrink enough that an individual input may
    /// legitimately fall inside the certified error band and decline.
    #[test]
    fn test_short_mul_equal_precision_engages() {
        // (precision in bits, operand words for that precision on a 64-bit
        // target; `scale` keeps the bit count constant on narrower words)
        for &(p, wa) in &[(2048usize, 32usize), (4096, 64)] {
            let wa = scale(wa);
            let aw = lcg_words(91, wa);
            let a2 = Repr::<2>::new(sig(&aw), 0);
            let ctx = Context::<mode::HalfEven>::new(p);
            if (wa + 2) * WORD_BITS - p >= 100 {
                assert!(
                    ctx.mul_short(&a2, &a2).is_some(),
                    "equal-precision mul short path declined (p={p})"
                );
                check_mul_short::<mode::HalfEven, 2>(p, &a2, &a2);
                check_cubic_short::<mode::HalfEven, 2>(p, &a2);
            } else {
                check_public_matches_oracle::<mode::HalfEven, 2>(p, &a2, &a2, &a2);
            }
        }
        // Decimal equivalent: 2000 digits ≈ 6644 bits.
        let p10 = 2000;
        let w10 = 310; // > p·log2(10)/64 ≈ 104 words, scaled below
        let w10 = w10 * (64 / WORD_BITS);
        let aw = lcg_words(93, w10);
        let a10 = Repr::<10>::new(sig(&aw), 0);
        check_mul_short::<mode::HalfAway, 10>(p10, &a10, &a10);
    }

    /// Negative operands: the classification runs on the magnitude, so every
    /// sign combination must mirror the positive result.
    #[test]
    fn test_short_products_negative_operands() {
        let aw = lcg_words(11, scale(64));
        let bw = lcg_words(22, scale(72));
        for &p in &[20usize, 100] {
            let b2 = Repr::<2>::new(sig(&bw), -7);
            let na2 = Repr::<2>::new(-sig(&aw), 3);
            check_mul_short::<mode::HalfEven, 2>(p, &na2, &b2);
            check_mul_short::<mode::Down, 2>(p, &na2, &b2);
            check_mul_short::<mode::Up, 2>(p, &na2, &b2);
            check_sqr_short::<mode::HalfEven, 2>(p, &na2);
            check_cubic_short::<mode::HalfEven, 2>(p, &na2);
            check_cubic_short::<mode::Down, 2>(p, &na2);
            let b10 = Repr::<10>::new(sig(&bw), -7);
            let na10 = Repr::<10>::new(-sig(&aw), 3);
            check_mul_short::<mode::HalfAway, 10>(p, &na10, &b10);
            check_sqr_short::<mode::HalfAway, 10>(p, &na10);
            check_cubic_short::<mode::HalfAway, 10>(p, &na10);
        }
    }

    /// Sparse power-of-two patterns give products with long zero runs around
    /// the rounding split. The chained cubic window can collapse its certified
    /// margin on such operands — declining to the exact product is then the
    /// correct outcome — so only mul/sqr assert engagement; all three
    /// operations assert correctness through the public API.
    #[test]
    fn test_short_mul_sparse_patterns() {
        let len = scale(64);
        for &k in &[1usize, 31, 62] {
            let mid = if k == 62 { len - 2 } else { scale(k) };
            let mut aw = vec![0 as Word; len];
            aw[0] = 1;
            aw[len - 1] = 1;
            let mut bw = vec![0 as Word; len];
            bw[0] = 1; // odd low word: keep normalization from shrinking the significand
            bw[mid] = 3;
            bw[len - 1] = 5;
            let (a2, b2) = (Repr::<2>::new(sig(&aw), 0), Repr::<2>::new(sig(&bw), 0));
            for &p in &[2usize, 20, 50, 100, 500] {
                check_mul_short::<mode::HalfEven, 2>(p, &a2, &b2);
                check_mul_short::<mode::Down, 2>(p, &a2, &b2);
                check_sqr_short::<mode::HalfEven, 2>(p, &b2);
                check_sqr_short::<mode::HalfAway, 2>(p, &a2);
                check_public_matches_oracle::<mode::HalfEven, 2>(p, &a2, &b2, &b2);
                check_public_matches_oracle::<mode::Down, 2>(p, &a2, &b2, &a2);
            }
        }
    }

    /// Base-10 significands with trailing zero words: the dropped part of the
    /// product is exactly zero. Such operands collapse the chained cubic
    /// window's certified margin (and with it the engagement), so only mul
    /// asserts engagement; correctness of all three operations goes through
    /// the public API.
    #[test]
    fn test_short_mul_exact_windows() {
        let len = scale(64);
        let mut aw = vec![0 as Word; len];
        aw[len - 1] = 7;
        let mut bw = vec![0 as Word; len];
        bw[len - 1] = 9;
        let (a10, b10) = (Repr::<10>::new(sig(&aw), 0), Repr::<10>::new(sig(&bw), 0));
        for &p in &[2usize, 50, 500] {
            check_mul_short::<mode::HalfEven, 10>(p, &a10, &b10);
            check_mul_short::<mode::HalfAway, 10>(p, &a10, &b10);
            check_public_matches_oracle::<mode::HalfEven, 10>(p, &a10, &b10, &a10);
            check_public_matches_oracle::<mode::Zero, 10>(p, &a10, &b10, &b10);
        }
    }

    /// Direct classifier checks on the tie and carry-guard branches.
    #[test]
    fn test_round_high_product_boundaries() {
        // Exact tie: value = hi·2^shift + 2^(shift-1) with no shortfall. The
        // shortfall of 1 keeps the tie decided for shift >= 2 (the error band
        // cannot reach the kept digits).
        for &(hi_val, shift) in &[(7u32, 3usize), (10, 2), (123, 8)] {
            let hi = IBig::from(hi_val);
            let p = digit_len::<2>(&hi);
            let tie = (&hi << shift) + (IBig::ONE << (shift - 1));
            let ctx = Context::<mode::HalfEven>::new(p);
            let got = round_high_product::<mode::HalfEven, 2>(
                &ctx,
                tie.clone(),
                Positive,
                0,
                false,
                &IBig::ONE,
            )
            .expect("exact tie should be decided");
            let want = ctx.repr_round_ref(&Repr::<2>::new(tie.clone(), 0));
            let Inexact(gv, gr) = got else {
                panic!("short path returned Exact")
            };
            let Inexact(wv, wr) = want else {
                panic!("oracle exact")
            };
            assert_eq!(&gv, &wv, "tie value mismatch");
            assert_eq!(gr, wr, "tie flag mismatch");

            // A nonzero shortfall breaks the same tie upwards; the oracle is
            // the rounding of tie + epsilon (any value in (tie, tie+1) rounds
            // identically, so tie + 1 stands in).
            let above = round_high_product::<mode::HalfEven, 2>(
                &ctx,
                tie.clone(),
                Positive,
                0,
                true,
                &IBig::ONE,
            )
            .expect("broken tie should be decided");
            let want_above = ctx.repr_round_ref(&Repr::<2>::new(tie + 1, 0));
            let Inexact(gv2, gr2) = above else { panic!() };
            let Inexact(wv2, wr2) = want_above else {
                panic!("oracle exact")
            };
            assert_eq!(&gv2, &wv2, "broken tie value mismatch");
            assert_eq!(gr2, wr2, "broken tie flag mismatch");
        }

        // Carry guard: lo = 2^shift − 1 with a shortfall of 2 must decline
        // (the truth may carry into the kept digits).
        let ctx = Context::<mode::HalfEven>::new(4);
        let val = IBig::from(79u8); // = 9·8 + 7, shift 3, lo = 7 = 2^3 − 1
        assert!(round_high_product::<mode::HalfEven, 2>(
            &ctx,
            val.clone(),
            Positive,
            0,
            true,
            &IBig::from(2u8)
        )
        .is_none());
        // The same value without a shortfall is decided (rounds up).
        assert!(
            round_high_product::<mode::HalfEven, 2>(&ctx, val, Positive, 0, false, &IBig::ONE)
                .is_some()
        );

        // Midpoint band: lo within the shortfall of the midpoint must decline
        // (at shift = 1 the midpoint is 1 and a shortfall of 1 reaches the
        // kept digits — the same carry guard in miniature).
        let val = IBig::from(5u8); // p=2, shift 1, lo = 1 = the midpoint itself
        let ctx = Context::<mode::HalfEven>::new(2);
        assert!(round_high_product::<mode::HalfEven, 2>(
            &ctx,
            val,
            Positive,
            0,
            true,
            &IBig::from(2u8)
        )
        .is_none());
    }

    /// Exponent saturation must keep flowing through the exact path's error
    /// semantics: the short path declines on exponent overflow.
    #[test]
    fn test_short_mul_exponent_overflow_declines() {
        let aw = lcg_words(33, scale(64));
        let a = Repr::<2>::new(sig(&aw), isize::MAX - 5);
        let b = Repr::<2>::new(sig(&aw), 10);
        let ctx = Context::<mode::HalfEven>::new(50);
        assert!(ctx.mul_short(&a, &b).is_none());
        assert_eq!(ctx.mul(&a, &b), Err(FpError::Overflow(Positive)));

        let c = Repr::<2>::new(sig(&aw), isize::MIN + 5);
        let d = Repr::<2>::new(sig(&aw), -10);
        assert!(ctx.mul_short(&c, &d).is_none());
        assert_eq!(ctx.mul(&c, &d), Err(FpError::Underflow(Positive)));
    }

    /// A power-of-two base whose per-digit bit count does not divide the
    /// dropped bit count (base 8 on 64-bit words: 64·dropped mod 3 ≠ 0) must
    /// pad the remainder bits onto the significand instead of folding them
    /// into the exponent by floor division (which mis-scales the value).
    #[test]
    fn test_short_mul_power_of_two_base_exponent_folding() {
        let aw = lcg_words(71, scale(64));
        let bw = lcg_words(72, scale(64));
        let p = 100; // octal digits; the window drops 121 words ≡ 1 mod 3
        let a8 = Repr::<8>::new(sig(&aw), 3);
        let b8 = Repr::<8>::new(sig(&bw), -2);
        let ctx = Context::<mode::HalfEven>::new(p);
        assert!(ctx.mul_short(&a8, &b8).is_some(), "base-8 short path declined");
        check_mul_short::<mode::HalfEven, 8>(p, &a8, &b8);
        check_sqr_short::<mode::HalfEven, 8>(p, &a8);
        check_cubic_short::<mode::HalfEven, 8>(p, &a8);
    }

    /// Sweeping the exponent sum across the finite-range boundary: whenever
    /// the unlimited-precision oracle saturates to the infinity, the public
    /// operation must report the corresponding error — including the case
    /// where the fast path's `normalize` bumps a checked exponent exactly
    /// onto the sentinel (which the fast path's own `checked_add` cannot
    /// see).
    #[test]
    fn test_short_mul_exponent_saturation_parity() {
        let aw = lcg_words(77, scale(64));
        let p = 50usize;
        let ctx = Context::<mode::HalfEven>::new(p);
        let oracle_ctx = Context::<mode::HalfEven>::new(0);
        let b = Repr::<2>::new(sig(&aw), 0);
        for delta in 0..(scale(8192) as isize) {
            let a = Repr::<2>::new(sig(&aw), isize::MAX - delta);
            let got = ctx.mul(&a, &b);
            match oracle_ctx.mul(&a, &b) {
                // The unlimited product itself can saturate in `normalize`
                // (its trailing-zero fold crosses the sentinel): both paths
                // must report overflow.
                Err(FpError::Overflow(_)) => {
                    assert!(
                        matches!(got, Err(FpError::Overflow(_))),
                        "delta={delta}: expected overflow, got {:?}",
                        got.map(|v| v.value().repr().clone())
                    );
                }
                Err(e) => panic!("delta={delta}: unexpected oracle error {e:?}"),
                Ok(want) => {
                    let want = want.value().with_precision(p);
                    let want_repr = match &want {
                        Exact(v) | Inexact(v, _) => v.repr(),
                    };
                    if want_repr.is_infinite() {
                        assert!(
                            matches!(got, Err(FpError::Overflow(_))),
                            "delta={delta}: expected overflow, got {:?}",
                            got.map(|v| v.value().repr().clone())
                        );
                    } else {
                        let got = got.expect("finite oracle but the public op errored");
                        assert_eq!(got.value().repr(), want_repr, "delta={delta}");
                    }
                }
            }
        }
    }

    /// Rounding can push the *result* out of the finite exponent range even
    /// when the exact intermediate is finite: a wide significand rounded to a
    /// small precision needs `exponent + shift` beyond `isize::MAX`. This
    /// used to panic in debug builds and wrap the exponent in release builds.
    #[test]
    fn test_round_exponent_saturation() {
        let aw = lcg_words(55, scale(64));
        let p = 50usize;
        let ctx = Context::<mode::HalfEven>::new(p);

        // Multiplication: the exact product is finite (exponent well below
        // the sentinel), but rounding 128 words down to 50 digits overflows.
        let a = Repr::<2>::new(sig(&aw), isize::MAX - 7000);
        let b = Repr::<2>::new(sig(&aw), 10);
        assert_eq!(ctx.mul(&a, &b), Err(FpError::Overflow(Positive)));
        let na = Repr::<2>::new(-sig(&aw), isize::MAX - 7000);
        assert_eq!(ctx.mul(&na, &b), Err(FpError::Overflow(Negative)));

        // Square and cubic reach the same state through their exact paths.
        assert_eq!(ctx.sqr(&a), Err(FpError::Overflow(Positive)));
        assert_eq!(ctx.cubic(&na), Err(FpError::Overflow(Negative)));

        // Addition: the aligned sum keeps the huge exponent and its wide
        // significand rounds past the range.
        let hi = Repr::<2>::new(sig(&aw), isize::MAX - 1000);
        assert_eq!(ctx.add(&hi, &b), Err(FpError::Overflow(Positive)));
        assert_eq!(ctx.sub(&hi, &b), Err(FpError::Overflow(Positive)));

        // `with_precision` has no error channel: the value saturates to the
        // infinity sentinel instead of panicking.
        let exact = FBig::<mode::HalfEven, 2>::from_parts(sig(&aw), isize::MAX - 1000);
        let rounded = exact.with_precision(p);
        assert!(rounded.value().repr().is_infinite());
    }
}
