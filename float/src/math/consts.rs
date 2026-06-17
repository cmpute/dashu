use crate::{
    error::assert_limited_precision,
    fbig::FBig,
    repr::{Context, Word},
    round::{Round, Rounded},
};
use dashu_base::{BitTest, Sign, UnsignedAbs};
use dashu_int::{IBig, UBig};

impl<R: Round> Context<R> {
    /// Calculate π using the Chudnovsky algorithm with binary splitting.
    ///
    /// The Chudnovsky algorithm is one of the most efficient methods for
    /// high-precision π calculation, providing ~14.18 decimal digits per term.
    ///
    /// # Methodology
    /// We use Binary Splitting to evaluate the series. This technique transforms
    /// the linear-time summation into a recursive tree evaluation. By combining
    /// terms into large products, it allows the library to leverage fast
    /// multiplication algorithms (like Toom-3 or FFT) as the numbers grow,
    /// leading to significant performance gains over simple iterative summation.
    ///
    /// // TODO: consider adding a static cache for π at common precisions.
    #[must_use]
    pub fn pi<const B: Word>(&self) -> Rounded<FBig<R, B>> {
        assert_limited_precision(self.precision);

        // Calculate required bits based on target precision in base B.
        // bits = ceil(precision * log2(B))
        let bits = if B.is_power_of_two() {
            self.precision.saturating_mul(B.ilog2() as usize)
        } else {
            self.precision.saturating_mul(B.ilog2() as usize + 1)
        };

        let num_terms = (bits * 100 / 4708) + 1;
        let guard_bits = num_terms.bit_len() + 32;
        let work_bits = bits + guard_bits;

        // Evaluate the series components using binary splitting
        let (_p, q, t) = chudnovsky_bs(0, num_terms);

        // Final formula: pi = (426880 * sqrt(10005) * Q) / T

        // Convert work bits back to base B precision.
        // precision_B = ceil(work_bits / log2(B))
        let work_precision = if B == 2 {
            work_bits
        } else {
            work_bits / B.ilog2() as usize + 1
        };
        let work_context = Self::new(work_precision);

        let q_f = work_context.convert_int::<B>(q.into()).value();
        let t_f = work_context.convert_int::<B>(t).value();

        let sqrt_10005 = work_context
            .sqrt(&work_context.convert_int::<B>(10005.into()).value().repr)
            .value();
        let constant = work_context.convert_int::<B>(426_880.into()).value();

        let pi = (constant * sqrt_10005 * q_f) / t_f;
        pi.with_precision(self.precision)
    }
}

/// Universal binary-splitting merge:
/// `combine((P_l,Q_l,T_l), (P_r,Q_r,T_r)) = (P_l·P_r, Q_l·Q_r, T_l·Q_r + P_l·T_r)`.
///
/// This operation is associative, so the `(P, Q, T)` for a range is independent of
/// how the recursion splits it — which is exactly what lets a cached partial tree
/// state be extended by merging in a freshly computed right half.
pub(crate) fn merge(
    pl: &UBig,
    ql: &UBig,
    tl: &IBig,
    pr: &UBig,
    qr: &UBig,
    tr: &IBig,
) -> (UBig, UBig, IBig) {
    let p = pl * pr;
    let q = ql * qr;
    // re-interpret the magnitudes as signed without cloning the big integers
    let t = qr.as_ibig() * tl + pl.as_ibig() * tr;
    (p, q, t)
}

/// Binary splitting implementation for the Chudnovsky series.
/// Returns `(P, Q, T)` for the range `[a, b)`. An empty range `[a, a)` yields the
/// identity `(1, 1, 0)`, so callers may safely merge a cached left state with a
/// right half that starts exactly where the left one ended.
pub(crate) fn chudnovsky_bs(a: usize, b: usize) -> (UBig, UBig, IBig) {
    if a >= b {
        return (UBig::ONE, UBig::ONE, IBig::ZERO);
    }
    if b - a == 1 {
        // Base case: calculate single term
        if a == 0 {
            return (UBig::ONE, UBig::ONE, IBig::from_parts_const(Sign::Positive, 13_591_409));
        }

        let k = a as u64;
        let p = UBig::from(6 * k - 5) * (2 * k - 1) * (6 * k - 1);
        let q = UBig::from(k).pow(3) * UBig::from_u64(10_939_058_860_032_000);
        let t_val = IBig::from_parts_const(Sign::Positive, 13_591_409)
            + IBig::from_parts_const(Sign::Positive, 545_140_134) * k;
        let t_abs = &p * t_val.unsigned_abs();
        let t = IBig::from(t_abs) * Sign::from(a % 2 == 1);
        return (p, q, t);
    }

    // Recursive step
    let mid = (a + b) / 2;
    let (p_l, q_l, t_l) = chudnovsky_bs(a, mid);
    let (p_r, q_r, t_r) = chudnovsky_bs(mid, b);

    // T = T_L * Q_R + T_R * P_L  (the universal merge)
    merge(&p_l, &q_l, &t_l, &p_r, &q_r, &t_r)
}

/// Binary splitting for `L(n) = acoth(n) = Σ_{k≥0} 1/(n^{2k+1}(2k+1))` over `[1, b)`.
///
/// Term ratio (k≥1): `r_k/r_{k-1} = p_k/q_k` with `p_k = 2k-1`, `q_k = (2k+1)·n²`.
/// The `k=0` term `r_0 = 1/n` is pulled out; over `[1, b)` the tree state satisfies
/// `T/Q = n · Σ_{k=1}^{b-1} 1/((2k+1)·n^{2k+1})`, hence `L(n) = (Q + T)/(n·Q)`.
///
/// Using the ratio form (rather than independent `1/q_k` terms) keeps
/// `Q = Π(2k+1)·n²` at O(p) digits: each leaf multiplies only small integers
/// (with `n²` folded in), no `n.pow(2k+1)` per leaf.
pub(crate) fn iacoth_bs(n: u32, a: usize, b: usize) -> (UBig, UBig, IBig) {
    debug_assert!(a >= 1, "iacoth_bs leaf index must be >= 1");
    if a >= b {
        return (UBig::ONE, UBig::ONE, IBig::ZERO); // identity
    }
    // Precomputed initial block [1, 1+K): skip its K leaves on every fresh
    // computation. Because the merge is associative, the constant triple is
    // identical to the recursively computed state regardless of split order.
    // It only applies at the series start (a == 1); recursive/extend calls have
    // a >= 1 + K and never reach this branch.
    if a == 1 {
        if let Some((k, p0, q0, t0)) = iacoth_initial_block(n) {
            // the precomputed block covers [1, 1+k); use it when [a, b) reaches
            // past its end (b > k)
            if b > k {
                let (pr, qr, tr) = iacoth_bs(n, 1 + k, b);
                return merge(&p0, &q0, &t0, &pr, &qr, &tr);
            }
        }
    }
    if b - a == 1 {
        // leaf at k = a (a >= 1): (p_a, q_a, p_a), p_a = 2a-1, q_a = (2a+1)·n²
        let pa = UBig::from(2 * a as u64 - 1);
        let n2 = UBig::from(n).pow(2);
        let qa = UBig::from(2 * a as u64 + 1) * n2;
        let ta = IBig::from(pa.clone());
        return (pa, qa, ta);
    }
    let mid = (a + b) / 2;
    let (pl, ql, tl) = iacoth_bs(n, a, mid);
    let (pr, qr, tr) = iacoth_bs(n, mid, b);
    merge(&pl, &ql, &tl, &pr, &qr, &tr) // universal merge
}

/// Precomputed binary-splitting state for `L(n) = acoth(n)` over the first `K`
/// terms `[1, 1+K)`, stored as `(K, P, Q, T)`. `K` is chosen (per `n`) so that
/// `P`, `Q` and `|T|` each fit in a `u32`. Since `DoubleWord` is `u32`/`u64`/`u128`
/// for `Word = u16`/`u32`/`u64`, a `u32`-sized literal is accepted by
/// [`UBig::from_dword`] / [`IBig::from_parts_const`] on **every** configuration —
/// so this single set of constants is portable without needing to detect the
/// `Word` width (which is internal to `dashu-int`). The constants also use the
/// inline small-integer representation, so instantiating them never allocates.
///
/// Only the sub-series that back ln2 / ln10 (`n ∈ {6, 9, 99}`) are precomputed;
/// π cannot use this trick because its 2-term `T` already overflows a `u32`.
fn iacoth_initial_block(n: u32) -> Option<(usize, UBig, UBig, IBig)> {
    match n {
        // L(6) over [1, 5): 4 leaves.
        6 => Some(IACOTH_6_INITIAL),
        // L(9) over [1, 4): 3 leaves.
        9 => Some(IACOTH_9_INITIAL),
        // L(99) over [1, 3): 2 leaves.
        99 => Some(IACOTH_99_INITIAL),
        _ => None,
    }
}

/// `(K, P, Q, T)` for `L(6)` over `[1, 5)` (4 leaves).
const IACOTH_6_INITIAL: (usize, UBig, UBig, IBig) = (
    4,
    UBig::from_dword(105),
    UBig::from_dword(1587237120),
    IBig::from_parts_const(Sign::Positive, 14946549),
);
/// `(K, P, Q, T)` for `L(9)` over `[1, 4)` (3 leaves).
const IACOTH_9_INITIAL: (usize, UBig, UBig, IBig) = (
    3,
    UBig::from_dword(15),
    UBig::from_dword(55801305),
    IBig::from_parts_const(Sign::Positive, 231351),
);
/// `(K, P, Q, T)` for `L(99)` over `[1, 3)` (2 leaves).
const IACOTH_99_INITIAL: (usize, UBig, UBig, IBig) = (
    2,
    UBig::from_dword(3),
    UBig::from_dword(1440894015),
    IBig::from_parts_const(Sign::Positive, 49008),
);

impl<R: Round, const B: Word> FBig<R, B> {
    /// Calculate π with the given precision and the default rounding mode.
    #[inline]
    #[must_use]
    pub fn pi(precision: usize) -> Self {
        Context::<R>::new(precision).pi().value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Independently (left-fold) merge the first `k` leaves of `L(n)` and check
    /// that the result matches the precomputed constant triple. Guards against
    /// transcription errors in the `IACOTH_*_INITIAL` literals; correctness of the
    /// finalized `L(n)` values is covered by `log::tests` and `cache::tests`.
    #[test]
    fn test_iacoth_initial_blocks() {
        fn check(n: u32, expected: &(usize, UBig, UBig, IBig)) {
            let k = expected.0;
            // independently (left-fold) merge the first k leaves
            let mut acc = (UBig::ONE, UBig::ONE, IBig::ZERO);
            for kk in 1..=k {
                let pa = UBig::from(2 * kk as u64 - 1);
                let qa = UBig::from(2 * kk as u64 + 1) * UBig::from(n).pow(2);
                let ta = IBig::from(pa.clone());
                let (p, q, t) = merge(&acc.0, &acc.1, &acc.2, &pa, &qa, &ta);
                acc = (p, q, t);
            }
            assert_eq!(acc.0, expected.1, "P mismatch for n={n}");
            assert_eq!(acc.1, expected.2, "Q mismatch for n={n}");
            assert_eq!(acc.2, expected.3, "T mismatch for n={n}");
            // the seed branch must reproduce the same state as the full recursion
            assert_eq!(iacoth_bs(n, 1, 1 + k), (acc.0, acc.1, acc.2));
        }
        check(6, &IACOTH_6_INITIAL);
        check(9, &IACOTH_9_INITIAL);
        check(99, &IACOTH_99_INITIAL);
    }
}
