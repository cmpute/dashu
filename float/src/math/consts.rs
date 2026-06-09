use crate::{
    error::assert_limited_precision,
    fbig::FBig,
    repr::{Context, Word},
    round::{Round, Rounded},
};
use dashu_base::{BitTest, Sign, UnsignedAbs};
use dashu_int::{IBig, UBig};

use crate::math::cache::MathCache;

#[derive(Clone)]
pub(crate) struct PiState {
    pub num_terms: usize,
    pub p: UBig,
    pub q: UBig,
    pub t: IBig,
}

static PI_CACHE: MathCache<PiState> = MathCache::new();

/// Clears the global caches for mathematical constants.
///
/// This function is used to free the memory held by the global caches
/// for high-precision mathematical constants (like π).
pub fn clear_math_caches() {
    PI_CACHE.clear();
}

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
    /// # Performance Note
    /// The incremental `MathCache` provides the most benefit for monotonically increasing
    /// precision requests (e.g. iterative refinement). For completely random high-precision
    /// requests, the overhead of merging large unbalanced binary-split blocks may sometimes
    /// exceed the cost of recomputing the perfectly balanced tree from scratch.
    #[must_use]
    pub fn pi<const B: Word>(&self) -> Rounded<FBig<R, B>> {
        const CACHE_REUSE_THRESHOLD: usize = 8;
        const BITS_PER_TERM: usize = 47; // ~14.18 decimal digits * log2(10)

        // Heuristic: below this threshold, the overhead of acquiring the lock and cloning
        // the Arc can exceed the cost of computing from scratch under high concurrency.
        // 100 terms (~1400 digits) is a conservative value that performs well across
        // a range of hardware. Tune upward for highly parallel workloads.
        const MIN_CACHE_TERMS: usize = 100;

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
        let required_bits = num_terms * BITS_PER_TERM;

        let (q, t) = if num_terms < MIN_CACHE_TERMS
            || PI_CACHE
                .peek(|state| {
                    state.num_terms >= num_terms
                        && state.q.bit_len() > required_bits * CACHE_REUSE_THRESHOLD
                })
                .unwrap_or(false)
        {
            let (_, q, t) = chudnovsky_bs(0, num_terms);
            (q, t)
        } else {
            let state = PI_CACHE.get_or_compute(
                num_terms,
                |state| state.num_terms,
                |existing_state, target_terms| {
                    existing_state.map_or_else(
                        || {
                            let (p, q, t) = chudnovsky_bs(0, target_terms);
                            PiState {
                                num_terms: target_terms,
                                p,
                                q,
                                t,
                            }
                        },
                        |cached| {
                            let (p_new, q_new, t_new) =
                                chudnovsky_bs(cached.num_terms, target_terms);
                            let p_total = &cached.p * p_new;
                            let q_total = &cached.q * &q_new;
                            let t_total = IBig::from(q_new) * &cached.t
                                + IBig::from(cached.p.clone()) * t_new;
                            PiState {
                                num_terms: target_terms,
                                p: p_total,
                                q: q_total,
                                t: t_total,
                            }
                        },
                    )
                },
            );
            (state.q.clone(), state.t.clone())
        };

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

/// Binary splitting implementation for the Chudnovsky series.
/// Returns (P, Q, T) for the range [a, b).
fn chudnovsky_bs(a: usize, b: usize) -> (UBig, UBig, IBig) {
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

    let p = &p_l * &p_r;
    let q = &q_l * &q_r;
    // T = T_L * Q_R + T_R * P_L
    let t = IBig::from(q_r) * t_l + IBig::from(p_l) * t_r;
    (p, q, t)
}

impl<R: Round, const B: Word> FBig<R, B> {
    /// Calculate π with the given precision and the default rounding mode.
    #[inline]
    #[must_use]
    pub fn pi(precision: usize) -> Self {
        Context::<R>::new(precision).pi().value()
    }
}
