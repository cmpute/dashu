//! Newton (reciprocal based) division algorithm.
//!
//! This module implements division via an approximate reciprocal of the divisor
//! (Newton's method), followed by multiplication-based quotient development
//! with a bounded correction loop. For very large operands the cost is
//! dominated by plain multiplications, which is asymptotically faster than the
//! recursive divide-and-conquer (Burnikel-Ziegler) path.
//!
//! The reciprocal follows the precision-doubling scheme from Brent & Zimmermann,
//! "Modern Computer Arithmetic" (Algorithm 3.5 / ApproximateReciprocal). For a
//! normalized `n`-word divisor `D` it computes an `(n+1)`-word value `V`
//! satisfying
//!
//! ```text
//!     D * V  <  B^(2n)  <  D * (V + 2)
//! ```
//!
//! where `B = 2^WORD_BITS`. In particular `V` is an underestimate of
//! `B^(2n) / D` (within 2), so quotient estimates produced from `V` never
//! overshoot and need at most a constant number of additive corrections.
//!
//! The quotient block development then follows Möller's "mu" division:
//! estimate a block `q = floor(U_hi * V / B^n)`, subtract `q * D` from the
//! running dividend, and add `D` back while the remainder is still too large.

use crate::{
    add,
    arch::word::Word,
    cmp,
    helper_macros::debug_assert_zero,
    math::FastDivideNormalized2,
    memory::{array_layout, Memory},
    mul,
    primitive::{highest_dword, WORD_BITS},
    Sign,
};
use alloc::alloc::Layout;

/// Below this divisor length (in words) the reciprocal is computed directly
/// with an exact schoolbook division of `B^(2n)` by the divisor. The Newton
/// iteration only kicks in for larger divisors.
const RECIP_BASE: usize = 32;

/// Newton division is only dispatched when *both* the divisor and the quotient
/// are at least this many words. Below this, the existing schoolbook /
/// Burnikel-Ziegler paths are faster.
///
/// The quotient must be large because the reciprocal itself costs `O(M(n))`; it
/// only pays off when it is amortized across many quotient blocks. Empirically
/// (see the `ubig_div_asymmetric` benchmark) the crossover sits at roughly
/// 6 000 words: below it Burnikel-Ziegler wins, above it the reciprocal based
/// path is faster (up to ~2× at very large sizes). This also keeps the block
/// multiplications in the NTT range where plain multiplications are cheapest.
const THRESHOLD_NEWTON: usize = 6000;

/// Environment-variable override for the Newton division threshold.
///
/// When the `tuning` feature is active the user may set
/// `DASHU_THRESHOLD_NEWTON_DIV` to override the compile-time default (e.g. set
/// it to a very large value to force the Burnikel-Ziegler path for comparison).
#[inline]
pub(crate) fn threshold() -> usize {
    #[cfg(feature = "tuning")]
    {
        if let Ok(s) = std::env::var("DASHU_THRESHOLD_NEWTON_DIV") {
            if let Ok(v) = s.parse() {
                return v;
            }
        }
    }
    THRESHOLD_NEWTON
}

const WORD_BYTES: usize = core::mem::size_of::<Word>();

#[inline]
fn mul_scratch_words(total: usize, smaller: usize) -> usize {
    // ceil(size / WORD_BYTES); written without `div_ceil` to respect the MSRV.
    let size = mul::memory_requirement_up_to(total, smaller).size();
    (size + WORD_BYTES - 1) / WORD_BYTES
}

/// Scratch words needed to compute the reciprocal of an `n`-word divisor.
fn recip_memory_words(n: usize) -> usize {
    if n <= RECIP_BASE {
        // dividend B^(2n) lives in a (2n+1)-word buffer; the base-case
        // schoolbook division needs no extra scratch.
        return 2 * n + 1;
    }
    let ell = (n - 1) / 2;
    let h = n - ell;
    // Buffers used while refining from `h` words to `n` words. They are carved
    // sequentially from the scratch region (each from the previous remainder),
    // so we keep a generous additive bound.
    let t_m = h + 1; // copy of the high part of T
    let t = n + h + 1; // T = D * V_h
    let u = 2 * h + 2; // U = T_m * V_h
    let mul1 = mul_scratch_words(n + h + 1, h + 1); // for D * V_h
    let mul2 = mul_scratch_words(2 * h + 2, h + 1); // for T_m * V_h
    let child = recip_memory_words(h);
    t_m + t + u + mul1.max(mul2) + child
}

/// Scratch words needed by the quotient-block routines, for an `n`-word
/// divisor. Covers both the full `2n/n` block and the trailing `m/n` block.
fn block_memory_words(n: usize) -> usize {
    // divrem_2n_by_n: UhV = U_h * V (2n+1 words) kept alive across the
    // in-place `win -= q*D` step.
    let uhv = 2 * n + 1;
    let mul1 = mul_scratch_words(2 * n + 1, n); // for U_h * V
    let mul2 = mul_scratch_words(2 * n, n); // for the in-place q*D subtract
                                            // divrem_m_by_n additionally needs a remainder copy buffer (up to 2n+1).
    let rbuf = 2 * n + 1;
    uhv + mul1.max(mul2) + rbuf
}

/// Memory requirement for Newton division.
pub fn memory_requirement_exact(lhs_len: usize, rhs_len: usize) -> Layout {
    let n = rhs_len;
    // V (the reciprocal) is allocated first and stays alive for the whole call.
    let v = n + 1;
    // The reciprocal computation and the block loop run sequentially and reuse
    // the same scratch region. The quotient-block buffers depend on the divisor
    // length only (the trailing m/n block has m < 2n), so `lhs_len` does not
    // affect the scratch requirement.
    let _ = lhs_len;
    let scratch = recip_memory_words(n).max(block_memory_words(n));
    array_layout::<Word>(v + scratch)
}

/// Divide lhs by rhs, replacing the top words of lhs by the quotient and the
/// bottom words of lhs by the remainder.
///
/// `lhs = [lhs % rhs, lhs / rhs]`
///
/// `rhs` must have at least 2 words and be normalized (the top bit must be 1).
/// `lhs` must be pre-applied the shift from fast_div_rhs_top.
///
/// Returns carry in the quotient (at most 1 because rhs is normalized).
#[must_use]
pub(crate) fn div_rem_in_place(
    lhs: &mut [Word],
    rhs: &[Word],
    _fast_div_rhs_top: FastDivideNormalized2,
    memory: &mut Memory,
) -> bool {
    let n = rhs.len();
    let m = lhs.len();
    debug_assert!(m >= n && n >= 2);

    // The Newton path needs more scratch than Burnikel-Ziegler (it computes a
    // full reciprocal of the divisor). Some callers size their scratch for a
    // different — smaller — divisor length (e.g. extended GCD post-processing).
    // When the provided scratch is too small for Newton, transparently fall
    // back to the divide-and-conquer algorithm, whose memory need is smaller.
    if memory.size_bytes() < memory_requirement_exact(m, n).size() {
        return crate::div::divide_conquer::div_rem_in_place(lhs, rhs, _fast_div_rhs_top, memory);
    }

    // Compute the reciprocal V of rhs once; it is reused for every quotient block.
    let (v, mut memory) = memory.allocate_slice_fill::<Word>(n + 1, 0);
    reciprocal(v, rhs, &mut memory);

    let mut overflow = false;
    let mut mm = m;
    while mm >= 2 * n {
        let o = divrem_2n_by_n(&mut lhs[mm - 2 * n..mm], rhs, v, &mut memory);
        if o {
            overflow = true;
        }
        mm -= n;
    }
    if mm > n {
        let o = divrem_m_by_n(&mut lhs[..mm], rhs, v, &mut memory);
        if o {
            overflow = true;
        }
    }
    overflow
}

/// Compute the approximate reciprocal `V` of a normalized `n`-word divisor `d`,
/// writing `n+1` words into `out`.
///
/// Guarantees `d * V < B^(2n) < d * (V + 2)` (V is an underestimate of
/// `B^(2n)/d` within 2).
fn reciprocal(out: &mut [Word], d: &[Word], memory: &mut Memory) {
    let n = d.len();
    debug_assert_eq!(out.len(), n + 1);
    debug_assert!(n >= 1);
    debug_assert!(d[n - 1] >> (WORD_BITS - 1) != 0, "divisor must be normalized");

    if n <= RECIP_BASE {
        recip_base(out, d, memory);
        #[cfg(debug_assertions)]
        debug_assert_newton_recip(out, d);
        return;
    }

    // Split: recurse on the high `h` words, then refine to full precision.
    let ell = (n - 1) / 2;
    let h = n - ell;
    let d_hi = &d[ell..]; // high h words (still normalized)

    // V_h = reciprocal of the high h words, stored in out[ell..n+1] (h+1 words).
    let (out_lo, out_hi) = out.split_at_mut(ell);
    debug_assert_eq!(out_hi.len(), h + 1);
    let _ = out_lo;
    reciprocal(out_hi, d_hi, memory);

    // T = d * V_h  (n words times h+1 words -> n + h + 1 words).
    let t_len = n + h + 1;
    let (t_m, mut memory) = memory.allocate_slice_fill::<Word>(h + 1, 0);
    let (t, mut memory) = memory.allocate_slice_fill::<Word>(t_len, 0);
    mul::multiply(t, d, out_hi, &mut memory);

    // Normalize so that T < B^(n+h): while T >= B^(n+h) { V_h -= 1; T -= d }.
    // T >= B^(n+h) iff word t[n+h] is nonzero. Runs at most twice.
    while t[n + h] != 0 {
        let underflow = add::sub_one_in_place(out_hi);
        debug_assert!(!underflow);
        debug_assert_zero!(add::sub_in_place(t, d));
    }

    // T = B^(n+h) - T  (now 0 < T < 2*d <= 2*B^n, since T>0 after the loop).
    // Computed as the two's complement of the low n+h words.
    negate_in_place(&mut t[..n + h]);

    // T_m = floor(T / B^ell), the high h+1 words of T.
    t_m.copy_from_slice(&t[ell..ell + h + 1]);

    // U = T_m * V_h  (2h+2 words).
    let u_len = 2 * h + 2;
    let (u, mut memory) = memory.allocate_slice_fill::<Word>(u_len, 0);
    mul::multiply(u, t_m, out_hi, &mut memory);

    // Assemble V = V_h * B^ell + floor(U / B^(2h-ell)).
    // The low `ell` words come from U[(2h-ell)..2h]; the high h+1 words are V_h.
    let x_lo = &u[(2 * h - ell)..2 * h];
    debug_assert_eq!(x_lo.len(), ell);
    let (lo, hi) = out.split_at_mut(ell);
    lo.copy_from_slice(x_lo);
    debug_assert_eq!(hi.len(), h + 1);
    // hi currently holds V_h; after the copy above it is unchanged, so V is set.
    let _ = hi;

    #[cfg(debug_assertions)]
    debug_assert_newton_recip(out, d);
}

/// Compute `out = floor((B^(2n) - 1) / d)` for small `n`.
///
/// This is a *strict* underestimate of `B^(2n) / d` (so `d * V < B^(2n)`
/// always), which is the contract the Newton refinement relies on. It matches
/// the base case of Brent & Zimmermann's ApproximateReciprocal
/// (`ceil(B^(2n)/d) - 1`).
fn recip_base(out: &mut [Word], d: &[Word], memory: &mut Memory) {
    let n = d.len();
    debug_assert!(n <= RECIP_BASE);
    debug_assert_eq!(out.len(), n + 1);

    if n == 1 {
        // B^2 - 1 as two all-ones words; divide by the single-word divisor.
        let (buf, _mem) = memory.allocate_slice_fill::<Word>(2, Word::MAX);
        let _rem = crate::div::div_by_word_in_place(buf, d[0]);
        out[0] = buf[0];
        out[1] = buf[1];
        return;
    }

    // dividend = B^(2n) - 1: low 2n words all-ones, top word zero (2n+1 words
    // total so the quotient has room for n+1 words).
    let (buf, _mem) = memory.allocate_slice_fill::<Word>(2 * n + 1, Word::MAX);
    buf[2 * n] = 0;
    // d is normalized (top bit set), so the normalization shift is 0 and the
    // schoolbook division can be applied directly.
    let fast_top = FastDivideNormalized2::new(highest_dword(d));
    let carry = crate::div::simple::div_rem_in_place(buf, d, fast_top);
    // floor((B^(2n)-1)/d) < B^(n+1) since d >= B^n/2, so no carry.
    debug_assert!(!carry);
    out.copy_from_slice(&buf[n..2 * n + 1]);
}

/// Divide a `2n`-word window by an `n`-word divisor using the precomputed
/// reciprocal `v`.
///
/// On return `win[0..n]` holds the remainder and `win[n..2n]` the quotient.
/// Returns the quotient carry (0 or 1).
fn divrem_2n_by_n(win: &mut [Word], d: &[Word], v: &[Word], memory: &mut Memory) -> bool {
    let n = d.len();
    debug_assert_eq!(win.len(), 2 * n);
    debug_assert_eq!(v.len(), n + 1);

    // If the high half is already >= d, the quotient has a leading carry.
    let mut carry = false;
    if cmp::cmp_same_len(&win[n..2 * n], d).is_ge() {
        debug_assert_zero!(add::sub_same_len_in_place(&mut win[n..2 * n], d));
        carry = true;
    }

    // q = floor(U_h * V / B^n) where U_h = win[n..2n].
    // U_h * V has 2n+1 words; q is the top n words (the 2n-th must be zero).
    let (uhv, mut mem) = memory.allocate_slice_fill::<Word>(2 * n + 1, 0);
    mul::multiply(uhv, &win[n..2 * n], v, &mut mem);
    debug_assert!(uhv[2 * n] == 0);
    let q = &mut uhv[n..2 * n]; // n-word quotient estimate

    // win -= q * d  (in place over all 2n words).
    debug_assert_zero!(mul::add_signed_mul(win, Sign::Negative, q, d, &mut mem));

    // Correct: while remainder >= d, subtract d and increment q.
    // The remainder (win[0..n+1]) may carry a few extra words at win[n].
    correct_remainder(&mut win[..n + 1], d, q);

    // Place the quotient in the high half (win[n] was cleared by correction).
    debug_assert!(win[n] == 0);
    win[n..2 * n].copy_from_slice(q);
    carry
}

/// Divide an `m`-word window (`n < m < 2n`) by an `n`-word divisor using the
/// precomputed reciprocal `v`.
///
/// On return `win[0..n]` holds the remainder and `win[n..m]` the quotient.
/// Returns the quotient carry (0 or 1).
fn divrem_m_by_n(win: &mut [Word], d: &[Word], v: &[Word], memory: &mut Memory) -> bool {
    let n = d.len();
    let m = win.len();
    debug_assert!(m > n && m < 2 * n);
    debug_assert_eq!(v.len(), n + 1);
    let qn = m - n; // quotient length (without carry), qn < n

    // U_h = win[n..m] (qn words). Since qn < n, U_h < d always holds, so there
    // is no leading-carry reduction here; the only carry is the top quotient word.

    // q = floor(U_h * V / B^n): product has m+1 words; q is the top qn+1 words.
    let (uhv, mut mem) = memory.allocate_slice_fill::<Word>(m + 1, 0);
    mul::multiply(uhv, &win[n..m], v, &mut mem);
    let q = &mut uhv[n..m + 1]; // qn+1 words; the leading word is the carry

    // rbuf = [win | 0] (m+1 words), then rbuf -= q * d.
    // Allocated from the remainder after `uhv` so that `q` (which borrows `uhv`)
    // and `rbuf` live in disjoint regions.
    let (rbuf, mut mem) = mem.allocate_slice_fill::<Word>(m + 1, 0);
    rbuf[..m].copy_from_slice(win);
    debug_assert_zero!(mul::add_signed_mul(rbuf, Sign::Negative, q, d, &mut mem));

    // Correct the remainder.
    correct_remainder(rbuf, d, q);

    // Write back: remainder into win[0..n], quotient (low qn words) into win[n..m].
    win[..n].copy_from_slice(&rbuf[..n]);
    win[n..m].copy_from_slice(&q[..qn]);
    debug_assert!(rbuf[n..].iter().all(|&x| x == 0));
    q[qn] != 0
}

/// While `rem >= d`, subtract `d` from `rem` and add 1 to `q`.
///
/// `rem` is a buffer whose value is the current remainder (possibly with a
/// small carry in `rem[n..]`); `d` has `n` words and `rem.len() >= n`. `q` is
/// incremented in lockstep. Because the quotient estimate never overshoots, the
/// remainder is always non-negative and only additive corrections are needed.
fn correct_remainder(rem: &mut [Word], d: &[Word], q: &mut [Word]) {
    let n = d.len();
    debug_assert!(rem.len() >= n);
    loop {
        let ge = if rem[n..].iter().any(|&x| x != 0) {
            true
        } else {
            cmp::cmp_same_len(&rem[..n], d).is_ge()
        };
        if !ge {
            break;
        }
        // rem -= d
        let borrow = add::sub_same_len_in_place(&mut rem[..n], d);
        if borrow {
            // propagate into the high words (guaranteed not to underflow overall)
            debug_assert_zero!(add::sub_one_in_place(&mut rem[n..]));
        }
        // q += 1
        let overflow = add::add_one_in_place(q);
        debug_assert!(!overflow);
    }
}

/// Two's-complement negate of a word slice in place (computes B^len - value,
/// assuming value != 0 so the result fits in `len` words).
fn negate_in_place(words: &mut [Word]) {
    let mut carry = 1u64 as Word; // add 1 after the bitwise complement
    for w in words.iter_mut() {
        let (val, b) = (!*w).overflowing_add(carry);
        *w = val;
        carry = if b { 1 } else { 0 };
    }
    debug_assert!(carry == 0);
}

#[cfg(debug_assertions)]
fn debug_assert_newton_recip(v: &[Word], d: &[Word]) {
    // Verify d * V < B^(2n) < d*(V+2) with exact UBig arithmetic.
    use crate::UBig;
    let n = d.len();
    if n > 4096 {
        return; // skip on huge inputs to keep debug builds responsive
    }
    let dv = UBig::from_words(d) * UBig::from_words(v);
    let b2n = UBig::ONE << (2 * n * WORD_BITS as usize);
    debug_assert!(dv < b2n, "reciprocal is not an underestimate (d*V >= B^2n)");
    let dv2 = &dv + UBig::from_words(d) * UBig::from(2u32);
    debug_assert!(b2n < dv2, "reciprocal bound B^2n < d*(V+2) violated");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{memory::MemoryAllocation, primitive::WORD_BITS, UBig};
    use rand_v08::{Rng, SeedableRng};

    fn random_normalized_words(rng: &mut impl Rng, n: usize) -> Vec<Word> {
        let mut w: Vec<Word> = (0..n).map(|_| rng.gen()).collect();
        w[n - 1] |= 1u64 << (WORD_BITS - 1); // ensure normalized (top bit set)
        w
    }

    /// Random non-zero words with a controllable number of leading zero bits in
    /// the top word, so that `normalize` applies a non-trivial shift.
    fn random_shifted_words(rng: &mut impl Rng, n: usize, top_zero_bits: u32) -> Vec<Word> {
        debug_assert!(top_zero_bits < WORD_BITS && n >= 1);
        let mut w: Vec<Word> = (0..n).map(|_| rng.gen()).collect();
        // clear the top `top_zero_bits` bits but keep the next bit set (nonzero)
        let mask = (1u64 << (WORD_BITS - top_zero_bits)) - 1;
        let hi = (w[n - 1] & mask) | (1u64 << (WORD_BITS - top_zero_bits - 1));
        w[n - 1] = hi;
        w
    }

    /// Exact reference reciprocal: floor(B^(2n) / d).
    fn ref_recip(d: &[Word]) -> UBig {
        let n = d.len();
        let b2n = UBig::ONE << (2 * n * WORD_BITS as usize);
        &b2n / UBig::from_words(d)
    }

    #[test]
    fn recip_bound_and_accuracy() {
        let mut rng = rand_v08::rngs::StdRng::seed_from_u64(1234);
        for &n in &[
            1usize, 2, 3, 4, 5, 7, 8, 16, 17, 31, 32, 33, 50, 63, 64, 65, 100, 200, 300,
        ] {
            for _ in 0..16 {
                let d = random_normalized_words(&mut rng, n);
                let mem_words = recip_memory_words(n);
                let mut alloc = MemoryAllocation::new(array_layout::<Word>(mem_words));
                let mut mem = alloc.memory();
                let mut v = vec![0u64 as Word; n + 1];
                reciprocal(&mut v, &d, &mut mem);

                let d_ubig = UBig::from_words(&d);
                let v_ubig = UBig::from_words(&v);
                let exact = ref_recip(&d);
                // V is an underestimate of the exact floor, within 2.
                assert!(v_ubig <= exact, "n={n}: V > floor(B^2n/d)");
                let diff = &exact - &v_ubig;
                assert!(diff <= UBig::from(2u32), "n={n}: floor(B^2n/d) - V > 2");
                // d * V < B^(2n) < d*(V+2)
                let b2n = UBig::ONE << (2 * n * WORD_BITS as usize);
                assert!(&d_ubig * &v_ubig < b2n, "n={n}: d*V >= B^2n");
                assert!(b2n < &d_ubig * &(&v_ubig + UBig::from(2u32)), "n={n}: B^2n >= d*(V+2)");
            }
        }
    }

    /// Run Newton's in-place division replicating the full public pipeline,
    /// including the normalization shift and the leading carry-word step.
    fn run_newton(dividend: &[Word], divisor: &[Word]) -> (UBig, UBig) {
        let n = divisor.len();
        let mut d = divisor.to_vec();
        let (shift, fast_top) = crate::div::normalize(&mut d);

        let mut lhs = dividend.to_vec();
        let lhs_carry = crate::shift::shl_in_place(&mut lhs, shift);
        let mut q_top: Word = if lhs_carry > 0 {
            crate::div::simple::div_rem_highest_word(lhs_carry, &mut lhs, &d, fast_top)
        } else {
            0
        };

        let layout = memory_requirement_exact(lhs.len(), n);
        let mut alloc = MemoryAllocation::new(layout);
        let mut mem = alloc.memory();
        let overflow = div_rem_in_place(&mut lhs, &d, fast_top, &mut mem);
        q_top += overflow as Word;

        let mut q = lhs[n..].to_vec();
        if q_top > 0 {
            q.push(q_top);
        }
        let mut r = lhs[..n].to_vec();
        let rb = crate::shift::shr_in_place(&mut r, shift);
        debug_assert_eq!(rb, 0);
        (UBig::from_words(&q), UBig::from_words(&r))
    }

    #[test]
    fn div_rem_correctness() {
        let mut rng = rand_v08::rngs::StdRng::seed_from_u64(987);
        for &(n, extra) in &[
            (160usize, 1usize),
            (160, 160),
            (200, 5),
            (200, 200),
            (300, 1),
            (300, 300),
            (300, 600),
            (500, 1),
            (500, 500),
            (500, 1000),
            (1000, 1),
            (1000, 1000),
            (1000, 2500),
        ] {
            for _ in 0..3 {
                let d = random_normalized_words(&mut rng, n);
                let m = n + extra;
                let dividend: Vec<Word> = (0..m).map(|_| rng.gen()).collect();

                let expected_q = &UBig::from_words(&dividend) / &UBig::from_words(&d);
                let expected_r = &UBig::from_words(&dividend) % &UBig::from_words(&d);

                let (q, r) = run_newton(&dividend, &d);
                assert_eq!(q, expected_q, "n={n} m={m}: quotient mismatch");
                assert_eq!(r, expected_r, "n={n} m={m}: remainder mismatch");
            }
        }
    }

    /// Edge cases: divisor close to B^n/2 (worst case for the reciprocal) and
    /// dividends that produce a leading carry.
    #[test]
    fn div_rem_edge_cases() {
        let n = 200usize;
        // divisor = B^n/2 exactly (smallest normalized value)
        let mut half = vec![0u64 as Word; n];
        half[n - 1] = 1u64 << (WORD_BITS - 1);

        let mut rng = rand_v08::rngs::StdRng::seed_from_u64(7);
        for &extra in &[1usize, 200, 400] {
            for _ in 0..3 {
                let m = n + extra;
                let dividend: Vec<Word> = (0..m).map(|_| rng.gen()).collect();
                let expected_q = &UBig::from_words(&dividend) / &UBig::from_words(&half);
                let expected_r = &UBig::from_words(&dividend) % &UBig::from_words(&half);
                let (q, r) = run_newton(&dividend, &half);
                assert_eq!(q, expected_q, "half-divisor n={n} m={m}: quotient");
                assert_eq!(r, expected_r, "half-divisor n={n} m={m}: remainder");
            }
        }
    }

    #[test]
    fn divrem_m_by_n_large_quotient() {
        let mut rng = rand_v08::rngs::StdRng::seed_from_u64(55);
        // m < 2n with qn = m-n close to n-1 (exercises the m/n block heavily)
        for &(n, qn) in &[
            (200usize, 1usize),
            (200, 100),
            (200, 199),
            (201, 200),
            (300, 1),
            (300, 299),
            (250, 249),
        ] {
            for _ in 0..4 {
                let d = random_normalized_words(&mut rng, n);
                let m = n + qn;
                let dividend: Vec<Word> = (0..m).map(|_| rng.gen()).collect();
                let expected_q = &UBig::from_words(&dividend) / &UBig::from_words(&d);
                let expected_r = &UBig::from_words(&dividend) % &UBig::from_words(&d);
                let (q, r) = run_newton(&dividend, &d);
                assert_eq!(q, expected_q, "n={n} qn={qn}: quotient mismatch");
                assert_eq!(r, expected_r, "n={n} qn={qn}: remainder mismatch");
            }
        }
    }

    #[test]
    fn div_rem_known_quotient() {
        // Dividend = a*b + c divided by a: the quotient must be exactly b and the
        // remainder exactly c. The divisor has a small top word, so normalize
        // applies a large shift (exercises the full unshifted pipeline).
        for &nw in &[161usize, 200, 250] {
            let bits = nw * 64;
            let a = (UBig::from(3u32) << (bits - 1)) | UBig::from(0x123456789u64);
            let b = (UBig::from(5u32) << (bits - 1)) | UBig::from(0xabcdef12u64);
            let c = UBig::from(7u32);
            let dividend = &(&a * &b) + &c;

            let a_w: Vec<Word> = a.as_words().to_vec();
            let n_w: Vec<Word> = dividend.as_words().to_vec();
            let (q, r) = run_newton(&n_w, &a_w);
            assert_eq!(q, b, "repro nw={nw}: quotient should be b");
            assert_eq!(r, c, "repro nw={nw}: remainder should be c");
        }
    }

    #[test]
    fn div_rem_with_shift() {
        // the full div_rem_unshifted_in_place pipeline including the leading
        // carry word from shl_in_place.
        let mut rng = rand_v08::rngs::StdRng::seed_from_u64(2024);
        for &(n, extra, shift_bits) in &[
            (162usize, 161usize, 1u32),
            (162, 161, 17),
            (162, 161, 40),
            (162, 161, 63),
            (200, 200, 63),
            (250, 250, 30),
            (300, 100, 63),
            (300, 600, 63),
        ] {
            for _ in 0..3 {
                let d = random_shifted_words(&mut rng, n, shift_bits);
                let m = n + extra;
                let dividend: Vec<Word> = (0..m).map(|_| rng.gen()).collect();
                let expected_q = &UBig::from_words(&dividend) / &UBig::from_words(&d);
                let expected_r = &UBig::from_words(&dividend) % &UBig::from_words(&d);
                let (q, r) = run_newton(&dividend, &d);
                assert_eq!(q, expected_q, "shift n={n} extra={extra} s={shift_bits}: quotient");
                assert_eq!(r, expected_r, "shift n={n} extra={extra} s={shift_bits}: remainder");
            }
        }
    }
}
