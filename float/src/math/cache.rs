//! Opt-in cache of mathematical constants, enabling progressive refinement.

use core::fmt;

use dashu_base::{BitTest, EstimatedLog2};
use dashu_int::{IBig, UBig};

use crate::fbig::FBig;
use crate::math::consts::{chudnovsky_bs, merge};
use crate::repr::{Context, Repr, Word};
use crate::round::{Round, Rounded};
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
}

impl ConstCache {
    /// Create an empty cache.
    pub const fn new() -> Self {
        Self {
            pi: None,
            iacoth_6: None,
            iacoth_9: None,
            iacoth_99: None,
        }
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
        let work_precision = precision_for_bits::<B>(bits + guard_bits);
        let work = Context::<R>::new(work_precision);

        let q_f = work.convert_int::<B>(q.into()).value();
        let t_f = work.convert_int::<B>(t).value();
        let sqrt_10005 = work
            .sqrt(&work.convert_int::<B>(10005.into()).value().repr)
            .value();
        let c = work.convert_int::<B>(426_880.into()).value();
        let pi = (c * sqrt_10005 * q_f) / t_f;
        pi.with_precision(precision)
    }

    /// `L(n) = acoth(n)` at `precision` base-`B` digits, extending its cached
    /// series state. Only `n ∈ {6, 9, 99}` are cached (the sub-series of ln2 / ln10).
    fn iacoth<const B: Word, R: Round>(&mut self, n: u32, precision: usize) -> FBig<R, B> {
        // terms until r_k < B^{-p}: (2k+1)·log_B(n) > p. The count is generously
        // over-provisioned (extra terms only add precision), so a plain (truncating)
        // cast suffices in place of a ceiling.
        let log_b_n = n.log2_est() / B.log2_est();
        let required_terms = (precision as f32 / (2.0 * log_b_n)) as usize + 10;

        let slot = match n {
            6 => &mut self.iacoth_6,
            9 => &mut self.iacoth_9,
            99 => &mut self.iacoth_99,
            _ => unreachable!("iacoth only caches n ∈ {{6, 9, 99}}"),
        };
        let (_p, q, t) = extend_or_compute(slot, 1, required_terms, |a, b| iacoth_bs(n, a, b));

        // L(n) = (Q + T) / (n·Q)
        let guard = (precision.log2_est() / B.log2_est()) as usize + 2;
        let work = Context::<R>::new(precision + guard);
        let num = work.convert_int::<B>(q.as_ibig() + &t).value();
        let denom = work.convert_int::<B>(IBig::from(n) * &q).value();
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
        let l6 = self.iacoth::<B, R>(6, work);
        let l99 = self.iacoth::<B, R>(99, work);
        (FBig::from(4) * l6 + FBig::from(2) * l99)
            .with_precision(precision)
            .value()
    }

    /// ln(10) at `precision` base-`B` digits, reusing the cached ln2 and `L(9)`
    /// sub-series.
    ///
    /// # Panics
    ///
    /// Panics if `precision` is 0.
    pub fn ln10<const B: Word, R: Round>(&mut self, precision: usize) -> FBig<R, B> {
        // log(10) = log(2) + log(5) = 3·log(2) + 2·L(9).
        // ln2 is requested at the elevated work precision so that the 3·ln2 term
        // keeps enough guard digits through the final round.
        let work = precision + combine_guard::<B>(precision);
        let l2 = self.ln2::<B, R>(work);
        let l9 = self.iacoth::<B, R>(9, work);
        (FBig::from(3) * l2 + FBig::from(2) * l9)
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
                (FBig::from(bits) * self.ln2::<B, R>(work))
                    .with_precision(precision)
                    .value()
            }
            _ => {
                // generic base: no cached L(n) sub-series applies, so compute
                // ln(B) directly through Context::ln on the base literal.
                Context::<R>::new(precision)
                    .ln::<B>(
                        &Repr::new(Repr::<B>::BASE.into(), 0),
                        // no cache for the generic base (its L(n) isn't cached)
                        None,
                    )
                    .value()
            }
        }
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

/// `ceil(x) as usize` without `f64::ceil`, which is `std`-only on this crate's
/// MSRV (the `f64` inherent methods only landed in `core` in Rust 1.85). Valid for
/// non-negative `x` within `usize` range, which always holds for the precision/bit
/// estimates computed here.
fn ceil_usize(x: f64) -> usize {
    let i = x as usize;
    if x > i as f64 {
        i + 1
    } else {
        i
    }
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
        ceil_usize(precision as f64 * ub as f64) + 1
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
        ceil_usize(bits as f64 / lb as f64) + 1
    }
}

/// Guard digits added when combining sub-series, large enough that the linear
/// combination and its final round to `precision` are unaffected by summation
/// rounding (a few digits cover the constant multipliers and term count).
fn combine_guard<const B: Word>(precision: usize) -> usize {
    (precision.log2_est() / B.log2_est()) as usize + 4
}

impl fmt::Debug for ConstCache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Avoid dumping MB-sized big-integers: report term counts and bit lengths only.
        f.debug_struct("ConstCache").finish()?;
        f.write_str(" {\n")?;
        fmt_slot(f, "pi", &self.pi)?;
        fmt_slot(f, "iacoth_6", &self.iacoth_6)?;
        fmt_slot(f, "iacoth_9", &self.iacoth_9)?;
        fmt_slot(f, "iacoth_99", &self.iacoth_99)?;
        f.write_str("}")
    }
}

fn fmt_slot(f: &mut fmt::Formatter<'_>, name: &str, slot: &Option<CachedState>) -> fmt::Result {
    match slot {
        Some(s) => f
            .debug_struct(name)
            .field("num_terms", &s.num_terms)
            .field("p_bits", &s.p.bit_len())
            .field("q_bits", &s.q.bit_len())
            .field("t_bits", &s.t.bit_len())
            .finish()
            .and(f.write_str("\n")),
        None => f
            .debug_struct(name)
            .field("num_terms", &0usize)
            .finish()
            .and(f.write_str("\n")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::round::mode;

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
            let direct_ln2 = Context::<mode::Zero>::new(precision)
                .ln::<10>(&Repr::new(2.into(), 0), None)
                .value();
            assert_eq!(cached_ln2, direct_ln2, "ln2 mismatch at precision {precision}");

            let cached_ln10 = cache
                .ln10::<10, mode::Zero>(precision)
                .with_precision(precision)
                .value();
            let direct_ln10 = Context::<mode::Zero>::new(precision)
                .ln::<10>(&Repr::new(10.into(), 0), None)
                .value();
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
        let expected = FBig::from(3) * cache.ln2::<8, mode::HalfAway>(50);
        assert_eq!(ln8.with_precision(50).value(), expected.with_precision(50).value());
    }

    #[test]
    fn test_debug_does_not_dump_bigints() {
        let mut cache = ConstCache::new();
        let _ = cache.pi::<10, mode::HalfAway>(100);
        let s = format!("{:?}", cache);
        assert!(s.contains("pi"));
        assert!(s.contains("num_terms"));
        // a 100-digit cached π has large integers; the Debug output should stay small
        assert!(s.len() < 512);
    }
}
