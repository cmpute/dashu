//! Opt-in cache of mathematical constants, enabling progressive refinement.

use core::fmt;

use dashu_base::{BitTest, EstimatedLog2};
use dashu_int::{IBig, UBig};

use crate::fbig::FBig;
use crate::math::consts::{chudnovsky_bs, merge};
use crate::repr::{Context, Repr, Word};
use crate::round::{Round, Rounded};
use crate::utils::ceil_usize;
use crate::{error::assert_limited_precision, math::consts::iacoth_bs};

/// Binary-splitting tree state — exact integers, losslessly extensible.
///
/// Represents `binary_split(start, num_terms)` as the universal triple
/// `(P, Q, T)`, where `start` is 0 for π and 1 for `L(n)` (whose `k=0` term
/// `1/n` is pulled out). These are pure integers: independent of base and
/// rounding mode. To extend to `new_terms > num_terms`, compute the right half
/// over the new range and merge with the universal `T' = T_l·Q_r + P_l·T_r`.
#[derive(Clone)]
pub(crate) struct CachedState {
    pub p: UBig,
    pub q: UBig,
    pub t: IBig,
    pub num_terms: usize,
}

/// An opt-in cache of mathematical constants.
///
/// Holds exact binary-splitting tree state so that repeated calls at increasing
/// precision *extend* prior work instead of recomputing from scratch. For
/// example, computing π at 100 digits and then at 1000 digits only pays for the
/// extra ~900 digits of work.
///
/// The cache is **base-free**: a single [`ConstCache`] serves any base. The base
/// and rounding mode are specified on each method call (e.g.
/// `cache.pi::<10, HalfAway>(100)` for 100 decimal digits).
///
/// `ConstCache` is a plain struct of big integers, so it is `Send + Sync`. The
/// methods take `&mut self` (they extend the cached state on a miss), so a caller
/// either owns one directly, or — to share it across many values and operations —
/// wraps it in `Rc<RefCell<ConstCache>>` as the
/// [`CachedFBig`](crate::CachedFBig) type does. To share one cache across
/// threads, wrap a `ConstCache` (or a `CachedFBig`) in `Arc<Mutex<..>>`.
///
/// # Examples
///
/// ```
/// use dashu_float::ConstCache;
/// use dashu_float::round::mode::HalfAway;
///
/// let mut cache = ConstCache::new();
/// // first call computes from scratch
/// let _pi_100 = cache.pi::<10, HalfAway>(100).value();
/// // second call at higher precision extends the cached state
/// let pi_1000 = cache.pi::<10, HalfAway>(1000).value();
/// assert!(pi_1000.to_string().starts_with("3.141592653589793"));
/// ```
pub struct ConstCache {
    pi: Option<CachedState>,
    /// `L(6)`, `L(9)`, `L(99)` — the sub-series used by ln2 / ln10.
    iacoth_6: Option<CachedState>,
    iacoth_9: Option<CachedState>,
    iacoth_99: Option<CachedState>,
    /// Base-free integer `floor(sqrt(10005) · 2^sqrt_10005_bits)`, reused by π.
    /// Unlike the series slots this holds a plain value (not a `(P,Q,T)` triple) and
    /// is extended by a fresh Karatsuba `UBig::sqrt` — Newton refinement would be no
    /// faster, since `UBig::sqrt` is already O(M(n)).
    sqrt_10005: Option<UBig>,
    sqrt_10005_bits: usize,
}

impl ConstCache {
    /// Create an empty cache.
    pub const fn new() -> Self {
        Self {
            pi: None,
            iacoth_6: None,
            iacoth_9: None,
            iacoth_99: None,
            sqrt_10005: None,
            sqrt_10005_bits: 0,
        }
    }

    /// `floor(sqrt(10005) · 2^bits)` as a base-free integer, cached and extended on
    /// demand. Used by [`pi`](Self::pi). Computed via Karatsuba `UBig::sqrt` (O(M(n))).
    /// Returns the value together with the number of bits it actually corresponds to
    /// (which may be larger than requested, when a higher-precision value is reused).
    fn sqrt_10005(&mut self, bits: usize) -> (UBig, usize) {
        if bits > self.sqrt_10005_bits {
            let n = UBig::from(10005u32) << (2 * bits);
            self.sqrt_10005 = Some(dashu_base::SquareRoot::sqrt(&n));
            self.sqrt_10005_bits = bits;
        }
        (self.sqrt_10005.as_ref().unwrap().clone(), self.sqrt_10005_bits)
    }

    /// π at `precision` base-`B` digits, rounded per `R`. Extends any prior π
    /// state cached in `self`.
    ///
    /// # Panics
    ///
    /// Panics if `precision` is 0.
    pub fn pi<const B: Word, R: Round>(&mut self, precision: usize) -> Rounded<FBig<R, B>> {
        assert_limited_precision(precision);

        let bits = bits_for_precision::<B>(precision);
        let num_terms = (bits * 100 / 4708) + 1;

        let (_p, q, t) = extend_or_compute(&mut self.pi, 0, num_terms, chudnovsky_bs);

        // Finalize: π = 426880·√10005·Q / T  (identical to Context::pi)
        let guard_bits = num_terms.bit_len() + 32;
        let work_bits = bits + guard_bits;
        let work_precision = precision_for_bits::<B>(work_bits);
        let work = Context::<R>::new(work_precision);

        // Finalize: π = 426880·√10005·Q / T. With √10005 ≈ isqrt_val·2^(-isqrt_bits)
        // from the base-free cached isqrt, this folds into a single integer ratio
        //   π = (426880 · isqrt_val · Q) / (T · 2^isqrt_bits),
        // avoiding any cross-base conversion of √10005 (convert_int is the fast path,
        // the same one used for Q and T).
        let (isqrt_val, isqrt_bits) = self.sqrt_10005(work_bits);
        let num = IBig::from(426_880) * IBig::from(isqrt_val) * IBig::from(q);
        let den = t << isqrt_bits;
        let num_f = work.convert_int::<B>(num).value();
        let den_f = work.convert_int::<B>(den).value();
        let pi = num_f / den_f;
        pi.with_precision(precision)
    }

    /// `L(n) = acoth(n)` at `precision` base-`B` digits, extending its cached
    /// series state. Only `n ∈ {6, 9, 99}` are cached (the sub-series of ln2 / ln10).
    fn iacoth<const N: u32, const B: Word, R: Round>(&mut self, precision: usize) -> FBig<R, B> {
        // terms until r_k < B^{-p}: (2k+1)·log_B(n) > p. The count is generously
        // over-provisioned (extra terms only add precision), so a plain (truncating)
        // cast suffices in place of a ceiling.
        let log_b_n = N.log2_est() / B.log2_est();
        let required_terms = (precision as f32 / (2.0 * log_b_n)) as usize + 10;

        let slot = match N {
            6 => &mut self.iacoth_6,
            9 => &mut self.iacoth_9,
            99 => &mut self.iacoth_99,
            _ => unreachable!("iacoth only caches n ∈ {{6, 9, 99}}"),
        };
        let (_p, q, t) = extend_or_compute(slot, 1, required_terms, |a, b| iacoth_bs(N, a, b));

        // L(n) = (Q + T) / (n·Q)
        let guard = ceil_usize(precision.log2_est() / B.log2_est()) + 2;
        let work = Context::<R>::new(precision + guard);
        let num = work.convert_int::<B>(q.as_ibig() + &t).value();
        let denom = work.convert_int::<B>(IBig::from(N) * &q).value();
        num / denom
    }

    /// ln(2) at `precision` base-`B` digits, reusing the cached `L(6)` and
    /// `L(99)` sub-series.
    ///
    /// # Panics
    ///
    /// Panics if `precision` is 0.
    pub fn ln2<const B: Word, R: Round>(&mut self, precision: usize) -> FBig<R, B> {
        // log(2) = 4·L(6) + 2·L(99)  (Gourdon & Sebah, "Log 2")
        let work = precision + combine_guard::<B>(precision);
        let l6 = self.iacoth::<6, B, R>(work);
        let l99 = self.iacoth::<99, B, R>(work);
        (4u8 * l6 + 2u8 * l99).with_precision(precision).value()
    }

    /// ln(10) at `precision` base-`B` digits, reusing the cached `L(6)`, `L(99)`,
    /// and `L(9)` sub-series.
    ///
    /// # Panics
    ///
    /// Panics if `precision` is 0.
    pub fn ln10<const B: Word, R: Round>(&mut self, precision: usize) -> FBig<R, B> {
        // log(10) = 3·log(2) + 2·L(9) = 3·(4·L(6) + 2·L(99)) + 2·L(9)
        //          = 12·L(6) + 6·L(99) + 2·L(9)
        // Flattening avoids the intermediate rounding of ln2 inside the product.
        let work = precision + combine_guard::<B>(precision);
        let l6 = self.iacoth::<6, B, R>(work);
        let l99 = self.iacoth::<99, B, R>(work);
        let l9 = self.iacoth::<9, B, R>(work);
        (12u8 * l6 + 6u8 * l99 + 2u8 * l9)
            .with_precision(precision)
            .value()
    }

    /// ln(B) at `precision` base-`B` digits, reusing the cached ln2 / ln10 where
    /// possible.
    ///
    /// # Panics
    ///
    /// Panics if `precision` is 0.
    pub fn ln_base<const B: Word, R: Round>(&mut self, precision: usize) -> FBig<R, B> {
        match B {
            2 => self.ln2::<B, R>(precision),
            10 => self.ln10::<B, R>(precision),
            b if b.is_power_of_two() => {
                // ln(2^k) = k·ln(2); evaluate ln2 at elevated precision so the
                // k·ln2 product survives the final round.
                let work = precision + combine_guard::<B>(precision);
                let bits = b.trailing_zeros() as usize;
                (bits * self.ln2::<B, R>(work))
                    .with_precision(precision)
                    .value()
            }
            _ => {
                // generic base: no cached L(n) sub-series applies, so compute
                // ln(B) directly through Context::ln on the base literal.
                let ctx = Context::<R>::new(precision);
                ctx.unwrap_fp(ctx.ln::<B>(
                    &Repr::new(Repr::<B>::BASE.into(), 0),
                    // no cache for the generic base (its L(n) isn't cached)
                    None,
                ))
            }
        }
    }

    /// Sum of `num_terms` across all populated cache slots.
    #[inline]
    pub fn total_terms(&self) -> usize {
        let sum = |s: &Option<CachedState>| s.as_ref().map_or(0, |s| s.num_terms);
        sum(&self.pi) + sum(&self.iacoth_6) + sum(&self.iacoth_9) + sum(&self.iacoth_99)
    }

    /// Sum of word counts across all cached big integers (P, Q, T, and the cached
    /// `√10005` isqrt).
    ///
    /// This reflects the underlying storage words used by the cached state.
    #[inline]
    pub fn total_words(&self) -> usize {
        let slot_words = |s: &Option<CachedState>| {
            s.as_ref().map_or(0, |s| {
                s.p.as_words().len() + s.q.as_words().len() + s.t.as_sign_words().1.len()
            })
        };
        slot_words(&self.pi)
            + slot_words(&self.iacoth_6)
            + slot_words(&self.iacoth_9)
            + slot_words(&self.iacoth_99)
            + self.sqrt_10005.as_ref().map_or(0, |s| s.as_words().len())
    }

    /// Clear all cached constant state, freeing the underlying memory.
    ///
    /// After calling `clear()`, the next constant computation will start from scratch
    /// rather than extending the prior cached state.
    #[inline]
    pub fn clear(&mut self) {
        self.pi = None;
        self.iacoth_6 = None;
        self.iacoth_9 = None;
        self.iacoth_99 = None;
        self.sqrt_10005 = None;
        self.sqrt_10005_bits = 0;
    }
}

impl Default for ConstCache {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// Ensure `slot` holds state for at least `target` terms, then return `(P, Q, T)`
/// covering `target` terms (or more, when an existing higher-precision state
/// already covers `target` — finalize then rounds down to the requested precision).
///
/// `range_bs(a, b)` computes the leaf-merged state over `[a, b)` and must handle
/// `a == b` by returning the identity `(1, 1, 0)`.
fn extend_or_compute<F>(
    slot: &mut Option<CachedState>,
    start: usize,
    target: usize,
    range_bs: F,
) -> (UBig, UBig, IBig)
where
    F: Fn(usize, usize) -> (UBig, UBig, IBig),
{
    match slot {
        // Already have >= target terms: reuse (extra terms only add precision).
        Some(s) if s.num_terms >= target => (s.p.clone(), s.q.clone(), s.t.clone()),
        // Have fewer terms: extend the right half [num_terms, target) and merge.
        Some(s) => {
            let (pr, qr, tr) = range_bs(s.num_terms, target);
            let (p, q, t) = merge(&s.p, &s.q, &s.t, &pr, &qr, &tr);
            *slot = Some(CachedState {
                p: p.clone(),
                q: q.clone(),
                t: t.clone(),
                num_terms: target,
            });
            (p, q, t)
        }
        // Cold: compute from `start`.
        None => {
            let (p, q, t) = range_bs(start, target);
            *slot = Some(CachedState {
                p: p.clone(),
                q: q.clone(),
                t: t.clone(),
                num_terms: target,
            });
            (p, q, t)
        }
    }
}

/// Reborrow an `Option<&mut ConstCache>` so it can be threaded into several
/// sequential sub-calls. `as_deref_mut` is the natural reborrow here; clippy's
/// `needless_option_as_deref` flags it (the deref target equals the referent),
/// so the lint is allowed at this single centralized spot.
#[inline]
#[allow(clippy::needless_option_as_deref)]
pub(crate) fn reborrow_cache<'a>(
    cache: &'a mut Option<&mut ConstCache>,
) -> Option<&'a mut ConstCache> {
    cache.as_deref_mut()
}

/// Number of bits needed to represent `precision` base-`B` digits exactly.
///
/// For power-of-two bases this is exact; for arbitrary bases it uses the upper
/// bound from [`EstimatedLog2`], which is far tighter than `ilog2(B) + 1`.
fn bits_for_precision<const B: Word>(precision: usize) -> usize {
    if B.is_power_of_two() {
        precision.saturating_mul(B.ilog2() as usize)
    } else {
        // ub ≥ log2(B) with error ≤ 2/256.  Multiply in f64 so the product
        // is exact for precision up to 2^53.  +1 guards float rounding.
        let ub = B.log2_bounds().1;
        ceil_usize(precision as f32 * ub) + 1
    }
}

/// Convert a work-precision expressed in bits back to base-`B` digits.
///
/// For base 2 the identity holds; for power-of-two bases it uses ceiling
/// division; for arbitrary bases it inverts the lower bound from
/// [`EstimatedLog2`] to get a tight ceiling.
fn precision_for_bits<const B: Word>(bits: usize) -> usize {
    if B.is_power_of_two() {
        let log2 = B.ilog2() as usize;
        (bits + log2 - 1) / log2
    } else {
        // lb ≤ log2(B), so 1/lb ≥ 1/log2(B).  +1 guards float rounding.
        let lb = B.log2_bounds().0;
        ceil_usize(bits as f32 / lb) + 1
    }
}

/// Guard digits added when combining sub-series, large enough that the linear
/// combination and its final round to `precision` are unaffected by summation
/// rounding (a few digits cover the constant multipliers and term count).
fn combine_guard<const B: Word>(precision: usize) -> usize {
    ceil_usize(precision.log2_est() / B.log2_est()) + 4
}

impl fmt::Debug for ConstCache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConstCache")
            .field("pi", &DebugSlot(&self.pi))
            .field("iacoth_6", &DebugSlot(&self.iacoth_6))
            .field("iacoth_9", &DebugSlot(&self.iacoth_9))
            .field("iacoth_99", &DebugSlot(&self.iacoth_99))
            .finish()
    }
}

/// Newtype so we can implement `Debug` for `&Option<CachedState>` via the
/// big-integer `Debug` formatters.
struct DebugSlot<'a>(&'a Option<CachedState>);

impl fmt::Debug for DebugSlot<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(s) => f
                .debug_struct("CachedState")
                .field("num_terms", &s.num_terms)
                .field("p", &s.p)
                .field("q", &s.q)
                .field("t", &s.t)
                .finish(),
            None => f.write_str("None"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::round::mode;
    use alloc::format;

    #[test]
    fn test_pi_matches_context() {
        // Cache miss must reproduce Context::pi exactly.
        for &precision in &[10usize, 50, 100] {
            let mut cache = ConstCache::new();
            let cached = cache.pi::<10, mode::HalfEven>(precision).value();
            let direct = Context::<mode::HalfEven>::new(precision)
                .pi::<10>(None)
                .value();
            assert_eq!(cached, direct, "pi mismatch at precision {precision}");
        }
    }

    #[test]
    fn test_pi_lower_precision_reuses() {
        // Compute at high precision, then a lower-precision request must round
        // down from the cached state and still be correct.
        let mut cache = ConstCache::new();
        let _pi_high = cache.pi::<10, mode::HalfEven>(200).value();
        // the slot now holds >=200 terms; a 50-digit request reuses it
        let pi_50 = cache.pi::<10, mode::HalfEven>(50).value();
        let direct = Context::<mode::HalfEven>::new(50).pi::<10>(None).value();
        assert_eq!(pi_50, direct);
    }

    #[test]
    fn test_pi_extension_matches_scratch() {
        // Extending 100 -> 1000 must be bit-identical to a from-scratch 1000-digit compute.
        let mut cache = ConstCache::new();
        let _pi_100 = cache.pi::<10, mode::HalfAway>(100).value();
        let pi_1000_extended = cache.pi::<10, mode::HalfAway>(1000).value();

        let direct = Context::<mode::HalfAway>::new(1000).pi::<10>(None).value();
        assert_eq!(pi_1000_extended, direct);
    }

    #[test]
    fn test_iacoth_matches_context() {
        let mut cache = ConstCache::new();
        // ln2 / ln10 via cache must match ln(2)/ln(10) computed independently
        // through Context::ln (a different, atanh-based algorithm) at several precisions.
        for &precision in &[20usize, 45, 80] {
            let cached_ln2 = cache
                .ln2::<10, mode::Zero>(precision)
                .with_precision(precision)
                .value();
            let ln2_ctx = Context::<mode::Zero>::new(precision);
            let direct_ln2 =
                ln2_ctx.unwrap_fp(ln2_ctx.ln::<10>(&Repr::new(2.into(), 0), None));
            assert_eq!(cached_ln2, direct_ln2, "ln2 mismatch at precision {precision}");

            let cached_ln10 = cache
                .ln10::<10, mode::Zero>(precision)
                .with_precision(precision)
                .value();
            let ln10_ctx = Context::<mode::Zero>::new(precision);
            let direct_ln10 =
                ln10_ctx.unwrap_fp(ln10_ctx.ln::<10>(&Repr::new(10.into(), 0), None));
            assert_eq!(cached_ln10, direct_ln10, "ln10 mismatch at precision {precision}");
        }
    }

    #[test]
    fn test_iacoth_extension_matches_scratch() {
        // Extend ln2 from low to high precision; result must match from-scratch.
        let mut cache = ConstCache::new();
        let _ln2_low = cache.ln2::<10, mode::HalfAway>(20);
        let ln2_high = cache.ln2::<10, mode::HalfAway>(120);

        let mut fresh = ConstCache::new();
        let direct = fresh.ln2::<10, mode::HalfAway>(120);
        assert_eq!(ln2_high, direct);
    }

    #[test]
    fn test_ln_base() {
        // binary base: ln(base) == ln(2)
        let mut cache = ConstCache::new();
        let ln_base = cache.ln_base::<2, mode::HalfAway>(50);
        let ln2 = cache.ln2::<2, mode::HalfAway>(50);
        assert_eq!(ln_base, ln2);

        // power-of-two base: ln(8) = 3·ln(2)
        let ln8 = cache.ln_base::<8, mode::HalfAway>(50);
        let expected = 3u8 * cache.ln2::<8, mode::HalfAway>(50);
        assert_eq!(ln8.with_precision(50).value(), expected.with_precision(50).value());
    }

    #[test]
    fn test_debug_shows_bigint_head_tail() {
        let mut cache = ConstCache::new();
        let _ = cache.pi::<10, mode::HalfAway>(100);
        let s = format!("{:?}", cache);
        assert!(s.contains("pi"));
        assert!(s.contains("num_terms"));
        // UBig/IBig Debug prints head..tail, so the output stays compact
        assert!(s.contains(".."), "Debug output should use head..tail truncation");
        assert!(s.len() < 512);
    }

    #[test]
    fn test_sqrt_10005_cached_and_counted() {
        // Computing π caches the base-free √10005 isqrt; total_words counts it, and
        // clear() frees it.
        let mut cache = ConstCache::new();
        assert_eq!(cache.total_terms(), 0);
        assert_eq!(cache.total_words(), 0);

        let _ = cache.pi::<10, mode::HalfAway>(200);
        // the isqrt is now cached (total_terms stays series-only; words include isqrt)
        assert!(cache.total_words() > 0);

        cache.clear();
        assert_eq!(cache.total_terms(), 0);
        assert_eq!(cache.total_words(), 0);

        // after clear, π recomputes from scratch and still matches the direct value
        let direct = Context::<mode::HalfAway>::new(50).pi::<10>(None).value();
        let after_clear = cache.pi::<10, mode::HalfAway>(50).value();
        assert_eq!(after_clear, direct);
    }

    #[test]
    fn test_sqrt_10005_reuse_higher_precision() {
        // A high-precision π call caches a high-bit isqrt; a later lower-precision
        // call must reuse it (no recompute) and still be correct.
        let mut cache = ConstCache::new();
        let _high = cache.pi::<2, mode::HalfEven>(1000);
        let words_after_high = cache.total_words();

        let low = cache.pi::<2, mode::HalfEven>(100).value();
        // word count unchanged ⇒ isqrt (and series) were reused, not recomputed
        assert_eq!(cache.total_words(), words_after_high);

        let direct = Context::<mode::HalfEven>::new(100).pi::<2>(None).value();
        assert_eq!(low, direct);
    }
}
