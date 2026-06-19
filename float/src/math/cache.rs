//! Opt-in cache of mathematical constants, enabling progressive refinement.

use core::cell::RefCell;
use core::fmt;

use dashu_base::{BitTest, EstimatedLog2};
use dashu_int::{IBig, UBig};

use crate::fbig::FBig;
use crate::math::consts::{chudnovsky_bs, merge};
use crate::repr::{Context, Word};
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

/// The cache interior: one slot per series, holding only the **largest** state
/// computed so far for that series. The binary-splitting integers are
/// base-independent, so a single cache serves any base.
///
/// A smaller-precision request reuses the cached (higher-precision) state and
/// rounds down at finalize time — no per-precision map is needed.
struct ConstCache {
    pi: Option<CachedState>,
    /// `L(6)`, `L(9)`, `L(99)` — the sub-series used by ln2 / ln10.
    iacoth_6: Option<CachedState>,
    iacoth_9: Option<CachedState>,
    iacoth_99: Option<CachedState>,
}

impl ConstCache {
    const fn new() -> Self {
        Self {
            pi: None,
            iacoth_6: None,
            iacoth_9: None,
            iacoth_99: None,
        }
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
        let (_lb, ub) = B.log2_bounds();
        (precision as f64 * ub as f64).ceil() as usize + 1
    }
}

/// Convert a work-precision expressed in bits back to base-`B` digits.
///
/// For base 2 the identity holds; for power-of-two bases it uses ceiling
/// division; for arbitrary bases it inverts the lower bound from
/// [`EstimatedLog2`] to get a tight ceiling.
fn precision_for_bits<const B: Word>(bits: usize) -> usize {
    if B == 2 {
        bits
    } else if B.is_power_of_two() {
        let log2 = B.ilog2() as usize;
        (bits + log2 - 1) / log2
    } else {
        // lb ≤ log2(B), so 1/lb ≥ 1/log2(B).  +1 guards float rounding.
        let (lb, _ub) = B.log2_bounds();
        (bits as f64 / lb as f64).ceil() as usize + 1
    }
}

/// Guard digits added when combining sub-series, large enough that the linear
/// combination and its final round to `precision` are unaffected by summation
/// rounding (a few digits cover the constant multipliers and term count).
fn combine_guard<const B: Word>(precision: usize) -> usize {
    (precision.log2_est() / B.log2_est()) as usize + 4
}

/// An opt-in cache for mathematical constants.
///
/// Holds exact binary-splitting tree state so that repeated calls at increasing
/// precision *extend* prior work instead of recomputing from scratch. For
/// example, computing π at 100 digits and then at 1000 digits only pays for the
/// extra ~900 digits of work.
///
/// The cache is **base-free**: a single [`MathCache`] serves any base. The base
/// is specified on each method call (e.g. `cache.pi::<10, HalfAway>(100)` for
/// 100 decimal digits).
///
/// # Threading
///
/// Owned per-thread; the cache is filled on miss via interior mutability.
/// [`MathCache`] is `Send + !Sync`: a single cache may be moved between threads,
/// but not shared by reference. To share one cache across threads, wrap it:
///
/// ```
/// use dashu_float::MathCache;
/// use std::sync::{Arc, Mutex};
///
/// let cache = MathCache::new();
/// let shared = Arc::new(Mutex::new(cache));
/// ```
///
/// [`Context`](crate::repr::Context) and [`FBig`] are unaffected by this type —
/// [`Context::pi`](crate::repr::Context::pi) and friends still recompute from
/// scratch. [`MathCache`] is purely additive.
pub struct MathCache {
    inner: RefCell<ConstCache>,
}
// Safety: all fields are `Send` (UBig/IBig/plain data). `!Sync` comes from the
// `RefCell`. `MathCache` is therefore `Send + !Sync`: single-thread ownership,
// but movable between threads and wrappable in `Arc<Mutex<..>>` for sharing.

impl Default for MathCache {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl MathCache {
    /// Create an empty cache.
    pub const fn new() -> Self {
        Self {
            inner: RefCell::new(ConstCache::new()),
        }
    }

    /// π at `precision` base-`B` digits, rounded per `R`. Extends any prior π
    /// state cached in `self`.
    ///
    /// # Panics
    ///
    /// Panics if `precision` is 0.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_float::MathCache;
    /// use dashu_float::round::mode::HalfAway;
    ///
    /// let cache = MathCache::new();
    /// // first call computes from scratch
    /// let _pi_100 = cache.pi::<10, HalfAway>(100).value();
    /// // second call at higher precision extends the cached state
    /// let pi_1000 = cache.pi::<10, HalfAway>(1000).value();
    /// assert!(pi_1000.to_string().starts_with("3.141592653589793"));
    /// ```
    pub fn pi<const B: Word, R: Round>(&self, precision: usize) -> Rounded<FBig<R, B>> {
        assert_limited_precision(precision);

        let bits = bits_for_precision::<B>(precision);
        let num_terms = (bits * 100 / 4708) + 1;

        let (_p, q, t) = {
            let mut cache = self.inner.borrow_mut();
            extend_or_compute(&mut cache.pi, 0, num_terms, chudnovsky_bs)
        };

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
    fn iacoth<const B: Word, R: Round>(&self, n: u32, precision: usize) -> FBig<R, B> {
        // terms until r_k < B^{-p}: (2k+1)·log_B(n) > p. The count is generously
        // over-provisioned (extra terms only add precision), so a plain (truncating)
        // cast suffices in place of a ceiling.
        let log_b_n = n.log2_est() / B.log2_est();
        let required_terms = (precision as f32 / (2.0 * log_b_n)) as usize + 10;

        let (_p, q, t) = {
            let mut cache = self.inner.borrow_mut();
            let slot = match n {
                6 => &mut cache.iacoth_6,
                9 => &mut cache.iacoth_9,
                99 => &mut cache.iacoth_99,
                _ => unreachable!("iacoth only caches n ∈ {{6, 9, 99}}"),
            };
            extend_or_compute(slot, 1, required_terms, |a, b| iacoth_bs(n, a, b))
        };

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
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_float::MathCache;
    /// use dashu_float::round::mode::HalfAway;
    ///
    /// let cache = MathCache::new();
    /// let ln2 = cache.ln2::<10, HalfAway>(50);
    /// // 0.69314718055994530941723212145817656807550013436025
    /// assert!(ln2.to_string().starts_with("0.6931471805599453"));
    /// ```
    pub fn ln2<const B: Word, R: Round>(&self, precision: usize) -> FBig<R, B> {
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
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_float::MathCache;
    /// use dashu_float::round::mode::HalfAway;
    ///
    /// let cache = MathCache::new();
    /// let ln10 = cache.ln10::<10, HalfAway>(50);
    /// // 2.30258509299404568401799145468436420760110148862877
    /// assert!(ln10.to_string().starts_with("2.3025850929940456"));
    /// ```
    pub fn ln10<const B: Word, R: Round>(&self, precision: usize) -> FBig<R, B> {
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
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_float::MathCache;
    /// use dashu_float::round::mode::HalfAway;
    ///
    /// // for a binary cache, ln(base) = ln(2)
    /// let cache = MathCache::new();
    /// let ln_base = cache.ln_base::<2, HalfAway>(50);
    /// let ln2 = cache.ln2::<2, HalfAway>(50);
    /// assert_eq!(ln_base, ln2);
    /// ```
    pub fn ln_base<const B: Word, R: Round>(&self, precision: usize) -> FBig<R, B> {
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
                crate::repr::Context::<R>::new(precision)
                    .ln(&crate::repr::Repr::<B>::new(crate::repr::Repr::<B>::BASE.into(), 0))
                    .value()
            }
        }
    }
}

impl fmt::Debug for MathCache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Avoid dumping MB-sized big-integers: report term counts and bit lengths only.
        let cache = self.inner.borrow();
        f.debug_struct("MathCache").finish()?;
        f.write_str(" {\n")?;
        fmt_slot(f, "pi", &cache.pi)?;
        fmt_slot(f, "iacoth_6", &cache.iacoth_6)?;
        fmt_slot(f, "iacoth_9", &cache.iacoth_9)?;
        fmt_slot(f, "iacoth_99", &cache.iacoth_99)?;
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
    use alloc::format;

    #[test]
    fn test_pi_matches_context() {
        // Cache miss must reproduce Context::pi exactly.
        for &precision in &[10usize, 50, 100] {
            let cache = MathCache::new();
            let cached = cache.pi::<10, mode::HalfEven>(precision).value();
            let direct = crate::repr::Context::<mode::HalfEven>::new(precision)
                .pi::<10>()
                .value();
            assert_eq!(cached, direct, "pi mismatch at precision {precision}");
        }
    }

    #[test]
    fn test_pi_lower_precision_reuses() {
        // Compute at high precision, then a lower-precision request must round
        // down from the cached state and still be correct.
        let cache = MathCache::new();
        let _pi_high = cache.pi::<10, mode::HalfEven>(200).value();
        // the slot now holds >=200 terms; a 50-digit request reuses it
        let pi_50 = cache.pi::<10, mode::HalfEven>(50).value();
        let direct = crate::repr::Context::<mode::HalfEven>::new(50)
            .pi::<10>()
            .value();
        assert_eq!(pi_50, direct);
    }

    #[test]
    fn test_pi_extension_matches_scratch() {
        // Extending 100 -> 1000 must be bit-identical to a from-scratch 1000-digit compute.
        let cache = MathCache::new();
        let _pi_100 = cache.pi::<10, mode::HalfAway>(100).value();
        let pi_1000_extended = cache.pi::<10, mode::HalfAway>(1000).value();

        let direct = crate::repr::Context::<mode::HalfAway>::new(1000)
            .pi::<10>()
            .value();
        assert_eq!(pi_1000_extended, direct);
    }

    #[test]
    fn test_iacoth_matches_context() {
        use crate::repr::{Context, Repr};

        let cache = MathCache::new();
        // ln2 / ln10 via cache must match ln(2)/ln(10) computed independently
        // through Context::ln (a different, atanh-based algorithm) at several precisions.
        for &precision in &[20usize, 45, 80] {
            let cached_ln2 = cache
                .ln2::<10, mode::Zero>(precision)
                .with_precision(precision)
                .value();
            let direct_ln2 = Context::<mode::Zero>::new(precision)
                .ln::<10>(&Repr::new(2.into(), 0))
                .value();
            assert_eq!(cached_ln2, direct_ln2, "ln2 mismatch at precision {precision}");

            let cached_ln10 = cache
                .ln10::<10, mode::Zero>(precision)
                .with_precision(precision)
                .value();
            let direct_ln10 = Context::<mode::Zero>::new(precision)
                .ln::<10>(&Repr::new(10.into(), 0))
                .value();
            assert_eq!(cached_ln10, direct_ln10, "ln10 mismatch at precision {precision}");
        }
    }

    #[test]
    fn test_iacoth_extension_matches_scratch() {
        // Extend ln2 from low to high precision; result must match from-scratch.
        let cache = MathCache::new();
        let _ln2_low = cache.ln2::<10, mode::HalfAway>(20);
        let ln2_high = cache.ln2::<10, mode::HalfAway>(120);

        let fresh = MathCache::new();
        let direct = fresh.ln2::<10, mode::HalfAway>(120);
        assert_eq!(ln2_high, direct);
    }

    #[test]
    fn test_ln_base() {
        // binary base: ln(base) == ln(2)
        let cache = MathCache::new();
        let ln_base = cache.ln_base::<2, mode::HalfAway>(50);
        let ln2 = cache.ln2::<2, mode::HalfAway>(50);
        assert_eq!(ln_base, ln2);

        // power-of-two base: ln(8) = 3·ln(2)
        let ln8 = cache.ln_base::<8, mode::HalfAway>(50);
        let expected = FBig::from(3) * cache.ln2::<8, mode::HalfAway>(50);
        assert_eq!(
            ln8.with_precision(50).value(),
            expected.with_precision(50).value()
        );
    }

    #[test]
    fn test_debug_does_not_dump_bigints() {
        let cache = MathCache::new();
        let _ = cache.pi::<10, mode::HalfAway>(100);
        let s = format!("{:?}", cache);
        assert!(s.contains("pi"));
        assert!(s.contains("num_terms"));
        // a 100-digit cached π has large integers; the Debug output should stay small
        assert!(s.len() < 512);
    }
}
