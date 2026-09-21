//! The internal [`Mag`] type — a nonnegative magnitude bound, the radius of a [`Ball`](crate::ball::Ball).
//!
//! A `Mag` is a fixed-width normalized binary significand `man · 2^exp` plus the two sentinels
//! `0` and `+∞`, with the defining property that **every operation rounds away from zero** — a
//! `Mag` is a rigorous upper bound by construction, the single invariant the Ziv certification
//! rests on. It is the round-up magnitude-bound design popularized by ball arithmetic for
//! arbitrary precision real computation (as re-validated in the `dashu-ball` crate), reduced to
//! the subset float's error propagation needs and generalized from a fixed `u64` significand to
//! [`Word`] width so products stay native-width on every target.
//!
//! `pub(crate)` and permanently internal: no public Mag API is planned (it would constrain the
//! internal layout), and it is deliberately not shared with `dashu-ball`'s independent `u64` Mag.
//! Everything here is `core`-only — the crate builds without `std`.

use core::cmp::Ordering;

use dashu_int::{DoubleWord, IBig, Word};

use crate::repr::Repr;
use crate::utils::log2_base;

/// The smallest normalized significand: `2^(Word::BITS − 1)`. A `Mag` whose significand equals
/// this is an exact power of two.
const MAG_ONE_HALF: Word = 1 << (Word::BITS - 1);

/// A nonnegative magnitude bound (`mid ± rad`): `man · 2^(exp − Word::BITS)` with a
/// normalized significand (`man ∈ [2^(BITS−1), 2^BITS)`), plus the sentinels
/// `0` (`man == 0, exp == 0`) and `+∞` (`man == 0, exp == isize::MAX`).
///
/// `Copy` and allocation-free — radii flow through tight series loops by value.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Mag {
    man: Word,
    exp: isize,
}

impl Mag {
    /// The magnitude `0`.
    pub(crate) const ZERO: Mag = Mag { man: 0, exp: 0 };

    /// The magnitude `+∞`.
    pub(crate) const INFINITY: Mag = Mag {
        man: 0,
        exp: isize::MAX,
    };

    /// The magnitude `1`.
    pub(crate) const ONE: Mag = Mag {
        man: MAG_ONE_HALF,
        exp: 1,
    };

    /// Returns `true` if this is the `0` sentinel.
    #[inline]
    pub(crate) const fn is_zero(&self) -> bool {
        self.man == 0 && self.exp == 0
    }

    /// Returns `true` if this is the `+∞` sentinel.
    #[inline]
    pub(crate) const fn is_infinite(&self) -> bool {
        self.man == 0 && self.exp != 0
    }

    /// Returns `true` if this is a sentinel (`0` or `+∞`), i.e. the significand is zero.
    #[inline]
    const fn is_special(&self) -> bool {
        self.man == 0
    }

    // ========================================================================
    // Construction (all round up — a `Mag` is an upper bound)
    // ========================================================================

    /// The exact power of two `2^exp`; an exponent beyond the finite range saturates to
    /// `+∞` (a sound over-bound).
    pub(crate) const fn from_pow2(exp: isize) -> Mag {
        // value = 2^exp  ⇔  man = MAG_ONE_HALF, stored exp = exp + 1
        let e = exp.saturating_add(1);
        if e == isize::MAX {
            Mag::INFINITY
        } else {
            Mag {
                man: MAG_ONE_HALF,
                exp: e,
            }
        }
    }

    /// The magnitude `BASE^exp`, rounded up — the one-ulp source for the radius folding
    /// (`ulp_mag` in `ball.rs`). Exact for `BASE = 2`.
    pub(crate) fn from_base_pow<const B: Word>(exp: isize) -> Mag {
        if B == 2 {
            Mag::from_pow2(exp) // exact
        } else {
            Mag::ONE.scale_by_base_pow::<B>(exp, true)
        }
    }

    /// The smallest `Mag` bounding `x` from above. Every `Word` is represented exactly.
    pub(crate) fn from_word(x: Word) -> Mag {
        if x == 0 {
            Mag::ZERO
        } else {
            let n = Word::BITS - x.leading_zeros(); // significant bits in x
            build(x << (Word::BITS - n), n as isize)
        }
    }

    /// The smallest `Mag` bounding the magnitude of `n` from above (sign ignored), reading the
    /// top `Word::BITS` bits directly through [`IBig::as_sign_words`] — O(1), no allocation.
    pub(crate) fn from_int(n: &IBig) -> Mag {
        significand_bound(n, true)
    }

    /// The smallest `Mag` bounding the magnitude of the float `repr` from above
    /// (`|significand| · BASE^exponent`, sign ignored).
    pub(crate) fn from_repr<const B: Word>(repr: &Repr<B>) -> Mag {
        if repr.significand().is_zero() {
            // ±0 has magnitude 0; ±∞ exceeds every finite bound
            return if repr.is_infinite() {
                Mag::INFINITY
            } else {
                Mag::ZERO
            };
        }
        significand_bound(repr.significand(), true).scale_by_base_pow::<B>(repr.exponent(), true)
    }

    /// The largest `Mag` bounding the magnitude of `repr` from below — the twin used wherever a
    /// midpoint magnitude appears in a denominator. `±0` → `ZERO`; `±∞` → `INFINITY`.
    pub(crate) fn from_repr_lower<const B: Word>(repr: &Repr<B>) -> Mag {
        if repr.significand().is_zero() {
            return if repr.is_infinite() {
                Mag::INFINITY
            } else {
                Mag::ZERO
            };
        }
        significand_bound(repr.significand(), false).scale_by_base_pow::<B>(repr.exponent(), false)
    }

    // ========================================================================
    // Arithmetic — round up (the propagation direction)
    // ========================================================================

    /// `self + other`, rounded up. `+∞` propagates; `0` is the identity.
    pub(crate) fn add(&self, other: &Mag) -> Mag {
        if self.is_zero() {
            return *other;
        }
        if other.is_zero() {
            return *self;
        }
        if self.is_infinite() || other.is_infinite() {
            return Mag::INFINITY;
        }
        add_up(self.man, self.exp, other.man, other.exp)
    }

    /// `self · other`, rounded up. `0 · ∞ = 0` — a zero bound times anything is still a zero
    /// bound; the radius-propagation formulas rely on this.
    pub(crate) fn mul(&self, other: &Mag) -> Mag {
        if self.is_zero() || other.is_zero() {
            Mag::ZERO
        } else if self.is_infinite() || other.is_infinite() {
            Mag::INFINITY
        } else {
            let prod = (self.man as DoubleWord) * (other.man as DoubleWord);
            // (prod >> BITS) ≤ 2^BITS − 2, so the +1 round-up never overflows the Word
            let man = ((prod >> Word::BITS) + 1) as Word;
            finish_fixmul(man, self.exp.saturating_add(other.exp))
        }
    }

    /// `self / other`, rounded up. Division by `0` yields `+∞`.
    pub(crate) fn div(&self, other: &Mag) -> Mag {
        if self.is_special() || other.is_special() {
            if other.is_zero() || self.is_infinite() {
                Mag::INFINITY
            } else {
                Mag::ZERO
            }
        } else {
            let q = (((self.man as DoubleWord) << Word::BITS) / (other.man as DoubleWord)) + 1;
            norm_large_up(q, self.exp.saturating_sub(other.exp))
        }
    }

    /// `self · 2^e`. Exact; sentinels pass through unchanged.
    pub(crate) fn mul_pow2(&self, e: isize) -> Mag {
        if self.is_special() {
            *self
        } else {
            build(self.man, self.exp.saturating_add(e))
        }
    }

    /// `self^n`, rounded up (left-to-right binary exponentiation). `∞^n = ∞` for all `n`
    /// (including 0); `x^0 = 1` otherwise. Private: only [`Mag::exp_upper`] chains powers.
    fn pow(&self, n: usize) -> Mag {
        if self.is_infinite() {
            return Mag::INFINITY;
        }
        match n {
            0 => Mag::ONE,
            1 => *self,
            2 => self.mul(self),
            _ => {
                let mut y = *self;
                for i in (0..usize_bits(n) as usize - 1).rev() {
                    y = y.mul(&y);
                    if (n >> i) & 1 == 1 {
                        y = y.mul(self);
                    }
                }
                y
            }
        }
    }

    // ========================================================================
    // Round-DOWN twins (lower bounds; used by radius-propagation denominators)
    // ========================================================================

    /// An upper bound on `max(0, self − other)`, rounded up and floored at `0`. Currently
    /// exercised only by the round-up bracket tests (the propagation rules subtract in the
    /// `sub_down` direction).
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn sub(&self, other: &Mag) -> Mag {
        if self.is_infinite() {
            return Mag::INFINITY;
        }
        if other.is_infinite() {
            return Mag::ZERO;
        }
        if self.is_zero() || other.is_zero() {
            return *self; // 0 − y = 0 ;  x − 0 = x
        }
        sub_impl(self.man, self.exp, other.man, other.exp, true)
    }

    /// A lower bound on `max(0, self − other)`, floored at `0`.
    pub(crate) fn sub_down(&self, other: &Mag) -> Mag {
        if other.is_zero() {
            return *self;
        }
        if self.is_zero() || other.is_infinite() {
            return Mag::ZERO;
        }
        if self.is_infinite() {
            return Mag::INFINITY;
        }
        sub_impl(self.man, self.exp, other.man, other.exp, false)
    }

    /// A lower bound on `self · other`.
    pub(crate) fn mul_down(&self, other: &Mag) -> Mag {
        if self.is_zero() || other.is_zero() {
            Mag::ZERO
        } else if self.is_infinite() || other.is_infinite() {
            Mag::INFINITY
        } else {
            let prod = (self.man as DoubleWord) * (other.man as DoubleWord);
            let man = (prod >> Word::BITS) as Word; // floor — no +1
            finish_fixmul(man, self.exp.saturating_add(other.exp))
        }
    }

    // ========================================================================
    // The one transcendental bound float needs: exp
    // ========================================================================

    /// An upper bound on `e^self` (`self ≥ 0`), by halve-then-pow: for `v = self · 2⁻ʲ ∈ (0, 1)`,
    /// `e^t ≤ 1 + 2t` on `[0, 1]` (the difference `1 + 2t − e^t` is nonnegative there — it is `0`
    /// at `t = 0` and peaks at `2·ln 2 − 1 > 0` in between), so
    /// `e^self ≤ (1 + 2v)^(2ʲ)`, evaluated with the round-up [`Mag::pow`]. `j` is the top-bit
    /// position, capped so `2ʲ` fits a `usize`; beyond the cap any finite radius is dwarfed, so
    /// `+∞` (always sound) is returned. Integer-only — no libm, `core`-clean.
    pub(crate) fn exp_upper(&self) -> Mag {
        if self.is_zero() {
            return Mag::ONE; // e^0 = 1 exactly
        }
        if self.is_infinite() {
            return Mag::INFINITY;
        }
        // value ∈ [2^(exp−1), 2^exp): j = max(exp, 0) brings v into (0, 1)
        let j = self.exp.max(0) as usize;
        if j + 1 >= usize::BITS as usize {
            return Mag::INFINITY;
        }
        let v = self.mul_pow2(-(j as isize));
        (Mag::ONE.add(&v.mul_pow2(1))).pow(1usize << j)
    }

    // ========================================================================
    // Base-aware boundary helpers (from_repr's exponent arm and the Ziv radius export)
    // ========================================================================

    /// `self · BASE^e`, rounded in the requested direction. Base awareness exists only here —
    /// between `Mag`s a radius is a real magnitude, base-free. `BASE = 2` scales exactly. For
    /// any other base the scaling goes through the fixed-point `log₂ BASE` floor from
    /// [`log2_base`], shifted one fixed-point ulp when that is the sound side (see the
    /// selection below).
    fn scale_by_base_pow<const B: Word>(&self, e: isize, round_up: bool) -> Mag {
        if e == 0 || self.is_special() {
            return *self;
        }
        if B == 2 {
            return self.mul_pow2(e);
        }
        // `⌈e·log₂BASE⌉` / `⌊e·log₂BASE⌋` — the ratio is a fixed-point constant bracketing
        // the true log₂ from either side, so the scale is tight to a fraction of a bit for
        // any `e` in the `isize` range, with no `BASE^|e|` power ever built.
        let (l, fb) = log2_base::<B>();
        // The sound side flips with the sign of `e`: a coefficient above the true log₂ bounds
        // the product from above only for `e ≥ 0`, one below only for `e < 0`.
        let ratio: i128 = if round_up == (e >= 0) {
            (l + 1) as i128
        } else {
            l as i128
        };
        let k = div_scaled(e, ratio, fb, round_up);
        self.mul_pow2(k)
    }

    /// The radius as a `Repr`: a **sound upper bound** is all the Ziv containment test needs.
    /// `0` → `+0`; `+∞` → `+∞`. For `BASE = 2` the value `man · 2^(exp − BITS)` is an exact
    /// `Repr`. For any other base the radius is rounded outward to a base power *keeping the
    /// significand* (`man · BASE^k`): `k = ⌈(exp − BITS)·log_B 2⌉` bounds the `2^(exp−BITS)`
    /// factor from above, and the ≤ `Word::BITS`-bit `man` rides along instead of being
    /// dropped — the export stays within one rounding step of the true radius (≤ `BASE`×)
    /// instead of up to `BASE × 2^BITS / man`×.
    pub(crate) fn to_repr<const B: Word>(self) -> Repr<B> {
        if self.is_zero() {
            return Repr::zero();
        }
        if self.is_infinite() {
            return Repr::infinity();
        }
        // finite nonzero: value < 2^exp (man < 2^BITS) and ≥ 2^(exp−1)
        if B == 2 {
            let e = self
                .exp
                .saturating_sub(Word::BITS as isize)
                .clamp(isize::MIN + 1, isize::MAX - 1);
            Repr::new(IBig::from(self.man), e)
        } else {
            // `2^(exp−BITS) ≤ B^k` ⇔ `k ≥ (exp−BITS)·log_B 2` — the reciprocal direction of
            // [`log2_base`], so this is a *division* by the bracketing ratio (multiplying by
            // it would scale by `log₂ B`, not `log_B 2`). The divisor is picked per sign so
            // the ceil lands on the sound side (`log_B 2 ≤ true` for `a ≥ 0`, `≥ true` for
            // `a < 0`).
            let (l, fb) = log2_base::<B>();
            let a = (self.exp.saturating_sub(Word::BITS as isize)) as i128;
            let den: i128 = if a >= 0 { l as i128 } else { l as i128 + 1 };
            let scaled = a * (1i128 << fb);
            let q = scaled.div_euclid(den);
            let k = if scaled.rem_euclid(den) != 0 {
                q + 1
            } else {
                q
            };
            let k = k.clamp(isize::MIN as i128 + 1, isize::MAX as i128 - 1) as isize;
            Repr::new(IBig::from(self.man), k)
        }
    }
}

// ============================================================================
// Internal helpers
// ============================================================================

/// Assemble a `Mag` from an already-normalized significand and exponent, mapping a
/// saturated-to-`MAX` exponent to `+∞`.
#[inline]
const fn build(man: Word, exp: isize) -> Mag {
    if exp == isize::MAX {
        Mag::INFINITY
    } else {
        Mag { man, exp }
    }
}

/// Round a too-large significand down to exactly `Word::BITS` bits, rounding *up* and bumping
/// the exponent (half-to-away-from-zero on the dropped bit, applied until stable — at most
/// twice, since one `+1` carry can push a maximally-rounded significand over the top again).
#[inline]
fn norm_large_up(mut raw: DoubleWord, mut exp: isize) -> Mag {
    let top: DoubleWord = (1 as DoubleWord) << Word::BITS;
    while raw >= top {
        raw = (raw >> 1) + (raw & 1); // ceil(raw / 2)
        exp = exp.saturating_add(1);
    }
    build(raw as Word, exp)
}

/// Finish a fixmul (`(prod >> BITS) ± 1`): if the significand dropped below the normalized
/// range, shift it left once (value-preserving) and decrease the exponent.
#[inline]
fn finish_fixmul(mut man: Word, mut exp: isize) -> Mag {
    if man < MAG_ONE_HALF {
        man <<= 1;
        exp = exp.saturating_sub(1);
    }
    build(man, exp)
}

/// Core of [`Mag::add`]: the smaller addend's discarded bits are over-counted (`+1`).
fn add_up(mx: Word, ex: isize, my: Word, ey: isize) -> Mag {
    let bits = Word::BITS as isize;
    let shift = ex.saturating_sub(ey);
    if shift >= bits {
        // `other` is below one ulp of `self`: round up to self + 1 ulp
        norm_large_up(mx as DoubleWord + 1, ex)
    } else if shift > 0 {
        let raw = mx as DoubleWord + (my >> shift as u32) as DoubleWord + 1;
        norm_large_up(raw, ex)
    } else if shift == 0 {
        norm_large_up(mx as DoubleWord + my as DoubleWord, ex)
    } else if shift <= -bits {
        norm_large_up(my as DoubleWord + 1, ey)
    } else {
        let raw = my as DoubleWord + (mx >> (-shift) as u32) as DoubleWord + 1;
        norm_large_up(raw, ey)
    }
}

/// Shared core of `sub`/`sub_down`. Assumes both inputs finite & nonzero; a minuend below the
/// subtrahend's exponent floors at `0`.
fn sub_impl(mx: Word, ex: isize, my: Word, ey: isize, round_up: bool) -> Mag {
    let bits = Word::BITS as isize;
    if ex < ey {
        return Mag::ZERO;
    }
    let shift = ex.saturating_sub(ey);
    if shift == 0 {
        if mx <= my {
            return Mag::ZERO;
        }
        from_double_rounded((mx - my) as DoubleWord, round_up).mul_pow2(ex.saturating_sub(bits))
    } else if shift >= bits {
        if round_up {
            // `other` is below one ulp: the tightest round-up is `self` itself
            Mag { man: mx, exp: ex }
        } else {
            from_double_rounded((mx - 1) as DoubleWord, false).mul_pow2(ex.saturating_sub(bits))
        }
    } else {
        // shift ∈ [1, BITS); the exact difference fits in a DoubleWord
        let d = ((mx as DoubleWord) << shift as u32) - (my as DoubleWord);
        from_double_rounded(d, round_up).mul_pow2(ey.saturating_sub(bits))
    }
}

/// Encode a double-word magnitude into a normalized `Mag`, rounding the significand up (or
/// down) on any bit dropped below the `Word::BITS` width.
fn from_double_rounded(x: DoubleWord, round_up: bool) -> Mag {
    if x == 0 {
        return Mag::ZERO;
    }
    if (x >> Word::BITS) == 0 {
        return Mag::from_word(x as Word);
    }
    // x ∈ [2^BITS, 2^(2·BITS)): n significant bits in [BITS+1, 2·BITS]
    let n = 2 * Word::BITS - x.leading_zeros();
    let shift = n - Word::BITS;
    let hi = (x >> shift) as Word;
    let has_low = x & (((1 as DoubleWord) << shift) - 1) != 0;
    let mm = (hi as DoubleWord) + ((has_low && round_up) as DoubleWord);
    if (mm >> Word::BITS) != 0 {
        // +1 carried into the top bit: collapse to ONE_HALF, exponent up one more
        build(MAG_ONE_HALF, n as isize + 1)
    } else {
        build(mm as Word, n as isize)
    }
}

/// The O(1) significand magnitude bound: take the top `Word::BITS` bits directly from the
/// little-endian word buffer, rounding up (or truncating, for the lower twin). The top bits
/// always lie within the top two words, and the right-shift that extracts them is exactly the
/// top word's leading-zero count (`hi ≠ 0`, so `shift ∈ [1, Word::BITS]` — when `hi` fills its
/// word the shift drops the whole second word, which is then the `has_low` payload). The
/// unconditional `+1` costs at most one Mag-ulp and is what makes `from_repr` allocation-free.
fn significand_bound(sig: &IBig, round_up: bool) -> Mag {
    let (_, words) = sig.as_sign_words();
    let len = words.len();
    if len == 0 {
        return Mag::ZERO;
    }
    if len == 1 {
        return Mag::from_word(words[0]);
    }
    let bits = Word::BITS as usize;
    let hi = words[len - 1];
    let next = words[len - 2];
    let bit_len = (len - 1) * bits + (bits - hi.leading_zeros() as usize);
    let shift = bits - hi.leading_zeros() as usize; // ∈ [1, bits]
    let combined = ((hi as DoubleWord) << bits) | (next as DoubleWord);
    let top = (combined >> shift) as Word; // the top `bits` significant bits
    let has_low = (combined & (((1 as DoubleWord) << shift) - 1)) != 0
        || words[..len - 2].iter().any(|&w| w != 0);
    let mm = (top as DoubleWord) + ((has_low && round_up) as DoubleWord);
    if (mm >> bits) != 0 {
        // +1 carried into the top bit: collapse to ONE_HALF, exponent up one more
        build(MAG_ONE_HALF, bit_len as isize + 1)
    } else {
        build(mm as Word, bit_len as isize)
    }
}

/// The number of significant bits of `n > 0`.
#[inline]
fn usize_bits(n: usize) -> u32 {
    usize::BITS - n.leading_zeros()
}

/// `⌈a·ratio/2^frac_bits⌉` (`up`) or `⌊a·ratio/2^frac_bits⌋`, with `ratio` a `2^-frac_bits`
/// fixed-point constant. Which side of the [`log2_base`] bracket is the *sound* `ratio`
/// depends on the rounding direction **and** the sign of `a` — the caller picks it.
///
/// The fraction width is what makes this sound at exponent scale. A small rational (`28/93`,
/// `30102/100000`, `3322/1000`, `33218/10000`) errs by ~5·10⁻⁵ *relative* — and that error
/// multiplies the exponent. For the ~10¹⁴-range exponent of a huge argument it overshot the
/// outward power of ten by `10^1.2e10`: a radius that loose is worse than useless, since the
/// Ziv containment test then tried to align that gap and died on an out-of-memory allocation
/// (plain `exp(1.9e14)`, and every transcendental built on it).
///
/// `frac_bits` is capped so the ratio stays below `2^62`: with `|a| ≤ 2^63` the product
/// `a · ratio` still fits `i128`.
fn div_scaled(a: isize, ratio: i128, frac_bits: u32, up: bool) -> isize {
    let den = 1i128 << frac_bits;
    let scaled = (a as i128) * ratio;
    let (q, r) = (scaled.div_euclid(den), scaled.rem_euclid(den));
    let q = if up && r != 0 { q + 1 } else { q };
    q.clamp(isize::MIN as i128, isize::MAX as i128) as isize
}

// ============================================================================
// Ordering — Mag is genuinely totally ordered
// ============================================================================

impl PartialOrd for Mag {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Mag {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.man == other.man && self.exp == other.exp {
            return Ordering::Equal;
        }
        // sentinels (at most one side is special, since equality was ruled out)
        if self.is_zero() {
            return Ordering::Less;
        }
        if other.is_zero() {
            return Ordering::Greater;
        }
        if self.is_infinite() {
            return Ordering::Greater;
        }
        if other.is_infinite() {
            return Ordering::Less;
        }
        // both finite nonzero: compare by exponent, then significand
        match self.exp.cmp(&other.exp) {
            Ordering::Equal => self.man.cmp(&other.man),
            ord => ord,
        }
    }
}

impl core::fmt::Debug for Mag {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.is_zero() {
            write!(f, "Mag(0)")
        } else if self.is_infinite() {
            write!(f, "Mag(inf)")
        } else {
            write!(f, "Mag({} * 2^{})", self.man, self.exp.saturating_sub(Word::BITS as isize))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cmp::Ordering::*;

    /// A `Mag` value as an exact dyadic `(numerator, power-of-two exponent)`.
    fn dy(m: &Mag) -> (IBig, isize) {
        (IBig::from(m.man), m.exp - Word::BITS as isize)
    }

    /// An integer as a dyadic at exponent 0.
    fn dyi(n: IBig) -> (IBig, isize) {
        (n, 0)
    }

    /// Exact comparison of two dyadics (align at the smaller exponent, shifting the
    /// larger-exponent numerators up — integer-exact).
    fn dycmp(a: (IBig, isize), b: (IBig, isize)) -> Ordering {
        if a.1 == b.1 {
            a.0.cmp(&b.0)
        } else {
            let lo = a.1.min(b.1);
            (a.0 << (a.1 - lo) as usize).cmp(&(b.0 << (b.1 - lo) as usize))
        }
    }

    /// Every finite nonzero `Mag` in the arguments has its top significand bit set.
    fn assert_normalized(ms: &[&Mag]) {
        for m in ms {
            if !m.is_special() {
                assert!(m.man >= MAG_ONE_HALF, "{m:?} not normalized");
            }
        }
    }

    #[test]
    fn sentinels_and_ordering() {
        assert!(Mag::ZERO.is_zero() && !Mag::ZERO.is_infinite());
        assert!(Mag::INFINITY.is_infinite() && !Mag::INFINITY.is_zero());
        assert!(Mag::ONE == Mag::from_word(1));
        assert!(Mag::ZERO < Mag::ONE);
        assert!(Mag::ONE < Mag::INFINITY);
        assert_eq!(Mag::from_pow2(0), Mag::ONE);
    }

    #[test]
    fn from_word_and_pow2_are_exact() {
        for x in [1u64, 2, 3, 7, 1 << 32, (1 << 63) + 1, u64::MAX] {
            let x = x as Word;
            let m = Mag::from_word(x);
            assert_eq!(dycmp(dy(&m), dyi(IBig::from(x))), Equal, "{x}");
        }
        for e in [-100isize, -1, 0, 1, 63, 100] {
            let m = Mag::from_pow2(e);
            let exact = (IBig::ONE, e);
            assert_eq!(dycmp(dy(&m), exact), Equal, "2^{e}");
        }
        // saturation: beyond the finite exponent range → +∞ (a sound over-bound)
        assert!(Mag::from_pow2(isize::MAX - 1).is_infinite());
        assert!(!Mag::from_pow2(isize::MIN + 1).is_infinite());
        assert_normalized(&[&Mag::from_word(3), &Mag::from_pow2(-5)]);
    }

    #[test]
    fn add_sub_mul_round_up() {
        let a = Mag::from_word(3);
        let b = Mag::from_word(5);
        let tiny = Mag::from_word(1).mul_pow2(-80);
        // exact: 3 + 5 = 8 ≤ add, and add ≤ exact + one Mag-ulp
        assert!(dycmp(dy(&a.add(&b)), dyi(8.into())) != Less);
        assert!(dycmp(dy(&a.mul(&b)), dyi(15.into())) != Less);
        assert!(dycmp(dy(&a.mul_down(&b)), dyi(15.into())) != Greater);
        // sub: 5 − 3 = 2
        let d = b.sub(&a);
        assert!(dycmp(dy(&d), dyi(2.into())) != Less);
        // sub floors at zero: 3 − 5 = 0, and any − tiny = any
        assert!(a.sub(&b).is_zero());
        assert_eq!(a.sub(&tiny), a);
        // 0 · ∞ = 0 and ÷0 → ∞
        assert!(Mag::ZERO.mul(&Mag::INFINITY).is_zero());
        assert!(a.div(&Mag::ZERO).is_infinite());
        assert!(Mag::ZERO.div(&a).is_zero());
        // far-apart exponents: the smaller operand is absorbed into one ulp
        let big = Mag::from_pow2(100);
        assert!(dycmp(dy(&big.add(&a)), dy(&big)) == Greater);
        assert!(dycmp(dy(&big.add(&a)), dy(&big.mul_pow2(1))) == Less);
        assert_normalized(&[&a.add(&b), &a.mul(&b), &d, &big.add(&a)]);
    }

    #[test]
    fn div_rounds_up() {
        let a = Mag::from_word(7);
        let b = Mag::from_word(3);
        let q = a.div(&b);
        // q·b ≥ a (cross-multiplied, dyadic-exact)
        let (qm, qe) = dy(&q);
        let (bm, be) = dy(&b);
        let prod = (qm * bm, qe + be);
        assert!(dycmp(prod, dyi(7.into())) != Less);
        // and q is within a factor ~2 of the true quotient (7/3 = 2.33): q < 4
        assert!(dycmp(dy(&q), dyi(4.into())) == Less);
    }

    #[test]
    fn from_int_rounds_up_at_width() {
        // fits a Word: exact
        let small = IBig::from(12345u32);
        assert_eq!(dycmp(dy(&Mag::from_int(&small)), dyi(small.clone())), Equal);
        // 1, 2, 3 words: upper-bounded
        let two_words = (IBig::ONE << (Word::BITS as usize + 37)) + IBig::from(0xdeadbeefu32);
        let m = Mag::from_int(&two_words);
        assert!(dycmp(dy(&m), dyi(two_words.clone())) != Less);
        // bit length is preserved
        assert_eq!(m.exp, (Word::BITS as isize) + 38);
        let three_words = (IBig::ONE << (2 * Word::BITS as usize + 1)) - IBig::ONE;
        let m3 = Mag::from_int(&three_words);
        assert!(dycmp(dy(&m3), dyi(three_words.clone())) != Less);
        assert_normalized(&[&m, &m3]);
    }

    #[test]
    fn from_repr_brackets_base2() {
        let sig = (IBig::ONE << (Word::BITS as usize + 37)) + IBig::from(0xdeadbeefu32);
        for e in [-200isize, -3, 0, 5, 137] {
            let r = Repr::<2>::new(sig.clone(), e);
            let exact = (sig.clone(), e);
            assert!(
                dycmp(dy(&Mag::from_repr(&r)), exact.clone()) != Less,
                "from_repr({e}) under-bounds"
            );
            assert!(
                dycmp(dy(&Mag::from_repr_lower(&r)), exact) != Greater,
                "from_repr_lower({e}) over-bounds"
            );
        }
        // exact when the significand fits a Word (any width)
        let s = IBig::from(0x1234u32);
        let r = Repr::<2>::new(s.clone(), -20);
        assert_eq!(dycmp(dy(&Mag::from_repr(&r)), (s, -20isize)), Equal);
        // sentinels
        assert!(Mag::from_repr(&Repr::<2>::zero()).is_zero());
        assert!(Mag::from_repr(&Repr::<2>::infinity()).is_infinite());
        assert!(Mag::from_repr_lower(&Repr::<2>::infinity()).is_infinite());
    }

    #[test]
    fn from_repr_brackets_base10() {
        // value = sig · 10^e — bracket against the exact power by cross-multiplication
        let sig = IBig::from(123456789u32);
        for e in [-40isize, -7, 0, 9] {
            let r = Repr::<10>::new(sig.clone(), e);
            let pow = IBig::from(dashu_int::UBig::from(10u8).pow(e.unsigned_abs()));
            let (um, ue) = dy(&Mag::from_repr(&r));
            let (lm, le) = dy(&Mag::from_repr_lower(&r));
            if e >= 0 {
                // up ≥ sig·10^e ; lo ≤ sig·10^e
                assert!(dycmp((um, ue), (sig.clone() * &pow, 0)) != Less, "up {e}");
                assert!(dycmp((lm, le), (sig.clone() * &pow, 0)) != Greater, "lo {e}");
            } else {
                // up·10^|e| ≥ sig ; lo·10^|e| ≤ sig
                assert!(dycmp((um * &pow, ue), dyi(sig.clone())) != Less, "up {e}");
                assert!(dycmp((lm * &pow, le), dyi(sig.clone())) != Greater, "lo {e}");
            }
        }
    }

    #[test]
    fn exp_upper_bounds() {
        // e^0 = 1 exactly
        assert_eq!(Mag::ZERO.exp_upper(), Mag::ONE);
        // exp_upper(1) = (1 + 2·½)^2 = 4 (the pow chain's round-ups keep it ≥ 4, barely above)
        let one = Mag::from_word(1);
        assert!(dycmp(dy(&one.exp_upper()), dyi(4.into())) != Less);
        assert!(dycmp(dy(&one.exp_upper()), dyi(5.into())) == Less);
        // e^½ = 1.6487… ≤ exp_upper(from_pow2(-1)) = 1 + 2·½ = 2
        let half = Mag::from_pow2(-1);
        assert!(dycmp(dy(&half.exp_upper()), dyi(2.into())) != Less);
        // small t: the bound must cover 1 + t (since e^t ≥ 1 + t) and stay within
        // 1 + 2t + one Mag-ulp (the add's round-up — width-dependent)
        let t = Mag::from_pow2(-30);
        let (bm, be) = dy(&t.exp_upper());
        // b ≥ 1 + 2^-30:  b · 2^30 ≥ 2^30 + 1
        assert!(dycmp((bm.clone(), be + 30), dyi((IBig::ONE << 30) + 1)) != Less);
        // b ≤ 1 + 2^-29 + 2^(1-BITS):  aligned at 2^-(BITS+29)
        let bits = Word::BITS as isize;
        let s = (bits + 29) as usize;
        let rhs =
            (IBig::ONE << s) + (IBig::ONE << (s - 29)) + (IBig::ONE << (s - bits as usize + 1));
        assert!(dycmp((bm, be + s as isize), (rhs, 0isize)) != Greater);
        // monotone: a bigger input never yields a smaller bound
        assert!(Mag::from_pow2(3).exp_upper() >= Mag::from_pow2(1).exp_upper());
        // saturation
        assert!(Mag::INFINITY.exp_upper().is_infinite());
        assert!(Mag::from_pow2(1000).exp_upper().is_infinite());
    }

    #[test]
    fn to_repr_base2_is_exact() {
        assert_eq!(Mag::ZERO.to_repr::<2>(), Repr::<2>::zero());
        assert_eq!(Mag::INFINITY.to_repr::<2>(), Repr::<2>::infinity());
        let m = Mag::from_word(7);
        assert_eq!(m.to_repr::<2>(), Repr::<2>::new(IBig::from(7u8), 0));
        let m = Mag::from_pow2(-100);
        assert_eq!(m.to_repr::<2>(), Repr::<2>::new(IBig::ONE, -100));
        // the value of a finite Mag is exactly representable in base 2 — a non-power-of-two
        // significand round-trips to the same value
        let m = Mag::from_word(3);
        let r = m.to_repr::<2>();
        assert_eq!(dycmp((r.significand().clone(), r.exponent()), dy(&m)), Equal);
    }

    /// Exact `sig · 10^er ≥ man · 2^et` — the exported decimal `Repr` covers the dyadic Mag
    /// value. A negative decimal exponent is cleared by cross-multiplying with `5^(−er)`
    /// (`10^−m = 2^−m·5^−m`), so no negative-power `pow` is ever built.
    fn covers_base10(sig: IBig, er: isize, man: IBig, et: isize) -> bool {
        if er >= 0 {
            let pow10 = IBig::from(dashu_int::UBig::from(10u8).pow(er as usize));
            dycmp((sig * pow10, 0), (man, et)) != Less
        } else {
            let m = (-er) as usize;
            let five_m = IBig::from(dashu_int::UBig::from(5u8).pow(m));
            dycmp((sig, 0), (man * five_m, et + er)) != Less
        }
    }

    #[test]
    fn log2_base_values_are_pinned() {
        // The fixed-point bracket must satisfy `l ≤ log₂ BASE · 2^fb < l+1` on every target:
        // the i686 CI target once shifted the walk's normalization by the `Word` width (a
        // `u32` `leading_zeros`) and returned `log2(10) ≈ 35`. Each `floor` below is
        // `⌊log₂ BASE · 10^18⌋`, a width-independent decimal bracket of the true value.
        const D: i128 = 1_000_000_000_000_000_000; // 10^18
        let assert_bracket = |floor: i128, (l, fb): (u64, u32)| {
            let (l, den) = (l as i128, 1i128 << fb);
            // exact-integer form of `floor/1e18 ≤ l/2^fb < (floor+1)/1e18`
            assert!(
                (l + 1) * D > floor * den && l * D < (floor + 1) * den,
                "log2 fixed point {l}/2^{fb} misses [{floor}, {floor}+1)·10^-18"
            );
        };
        assert_bracket(D, log2_base::<2>()); // exact: log2(2) = 1
        assert_bracket(1_584_962_500_721_156_181, log2_base::<3>()); // 1.5849625007…
        assert_bracket(3_321_928_094_887_362_347, log2_base::<10>()); // 3.3219280949…
    }

    #[test]
    fn generic_base_powers_are_one_bit_tight() {
        // `from_base_pow` (the ulp source) must land within one bit of the exact `BASE^|e|`
        // bit length, on either side, for every base and a spread of exponents — the
        // fixed-point `log₂ BASE` bracket in action (no `BASE^|e|` power is built on the
        // implementation side; `pow` here is the test oracle).
        fn assert_one_bit_tight<const BASE: Word>() {
            for e in [-5000isize, -257, -3, -1, 1, 2, 255, 4096] {
                let mag = Mag::from_base_pow::<BASE>(e).mul_pow2(-e);
                // mag = round_up(BASE^e)·2^(−e): with bits = bit_len(BASE^|e|), the
                // exact BASE^e·2^(−e) lies in [2^sh, 2^(sh+1)] where sh = bits−1−e for
                // e > 0 and sh = −e−bits for e < 0; the one-bit round-up can touch the
                // upper end
                let bits = {
                    use dashu_base::BitTest as _;
                    dashu_int::UBig::from_word(BASE)
                        .pow(e.unsigned_abs())
                        .bit_len()
                } as isize;
                let sh = if e > 0 { bits - 1 - e } else { -e - bits };
                let v = dy(&mag);
                assert!(
                    dycmp(v.clone(), (IBig::ONE, sh)) != Less
                        && dycmp(v, (IBig::ONE, sh + 1)) != Greater,
                    "base {BASE} e {e}: not one-bit tight"
                );
            }
        }
        macro_rules! check_base {
            ($base:expr) => {{
                const BASE: Word = $base as Word;
                // a base that does not fit this target's `Word` truncates into a different
                // value (e.g. `1u64 << 40` under 16-bit limbs) — nothing to test there
                if u64::from(BASE) == $base {
                    assert_one_bit_tight::<BASE>();
                }
            }};
        }
        check_base!(3);
        check_base!(5);
        check_base!(7);
        check_base!(10);
        check_base!(12);
        check_base!(100);
        check_base!(1u64 << 20);
        check_base!((1u64 << 40) + 1);
    }

    // The bound side flips with the sign of the exponent: a coefficient *above* the true
    // log₂ BASE only bounds the product from above for `e ≥ 0`, one *below* only for
    // `e < 0`. Below |e| ≈ 2^frac_bits the fixed-point slack hides the difference; from
    // there up, the wrong side under-shoots the rounding by whole units of the binary
    // exponent. The oracle is a wider walk of the same shape (10 extra fraction bits),
    // exact enough to pin ⌈e·log₂BASE⌉ for |e| < 2^(frac_bits + 10).
    fn assert_sound_huge_negative<const BASE: Word>() {
        let (_, fb) = log2_base::<BASE>();
        // match log2_base's width-independent normalization
        let int_bits: u32 = Word::BITS - 1 - BASE.leading_zeros();
        let mut m: u128 = (BASE as u128) << (63 - int_bits);
        let mut frac: u128 = 0;
        let mut i = fb + 10;
        loop {
            if i == 0 {
                break;
            }
            i -= 1;
            m = (m * m) >> 63;
            if m >> 64 != 0 {
                frac |= 1 << i;
                m >>= 1;
            }
        }
        let wide = (((int_bits as u128) << (fb + 10)) | frac) as i128;
        let lo_ceil = |e: i128| {
            // ⌈e·log₂BASE⌉ from the guaranteed-lower bound `wide/2^(fb+10)`; the
            // dropped tail is below one output unit for |e| < 2^(fb+10).
            let num = e * wide;
            let den = 1i128 << (fb + 10);
            num.div_euclid(den) + (num.rem_euclid(den) != 0) as i128
        };
        for j in 1..=4096isize {
            // Sweep the extreme reachable negative exponent: just below 2^fb when
            // that fits the platform's `isize`, otherwise just above `isize::MIN`
            // (32-bit targets cannot express the 2^fb-scale exponent at all).
            let e = (-(1i128 << fb) - j as i128).max(isize::MIN as i128 + 1) as isize;
            let got = Mag::from_base_pow::<BASE>(e).exp - 1;
            assert!(
                got as i128 >= lo_ceil(e as i128),
                "base {BASE} e {e}: binary exponent {got} undershoots ⌈e·log₂BASE⌉ = {}",
                lo_ceil(e as i128)
            );
        }
    }

    #[test]
    fn generic_base_pow_sound_for_huge_negative_exponents() {
        macro_rules! check_base {
            ($base:expr) => {{
                const BASE: Word = $base as Word;
                // a base that does not fit this target's `Word` truncates into a different
                // value (e.g. `1u64 << 40` under 16-bit limbs) — nothing to test there
                if u64::from(BASE) == $base {
                    assert_sound_huge_negative::<BASE>();
                }
            }};
        }
        check_base!(100);
        check_base!(1000);
    }

    #[test]
    fn to_repr_base10_is_upper_bound() {
        // The export keeps the significand and bounds the `2^(exp−BITS)` factor by an outward
        // base power; the bound must cover the exact dyadic value (negative exponents included).
        let m = Mag::from_word(7);
        let r = m.to_repr::<10>();
        let (man, et) = dy(&m);
        assert!(covers_base10(r.significand().clone(), r.exponent(), man, et));
        // a tiny radius: 2^-100 exports as ~10^-49 riding on the normalized significand —
        // tighter than the old mantissa-dropping 10^-30 export would suggest, and still exact
        let m = Mag::from_pow2(-100);
        let r = m.to_repr::<10>();
        assert!(r.exponent() >= -50);
        assert!(!r.significand().is_zero());
        let (man, et) = dy(&m);
        assert!(covers_base10(r.significand().clone(), r.exponent(), man, et));
    }
}
