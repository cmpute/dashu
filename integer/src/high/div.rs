//! Short (truncated) division kernel.

use super::mul;
#[cfg(test)]
use crate::primitive::WORD_BITS_USIZE;
use crate::{
    add,
    arch::word::Word,
    buffer::Buffer,
    div,
    helper_macros::debug_assert_zero,
    math::FastDivideNormalized2,
    memory::{self, Memory, MemoryAllocation},
    primitive::{highest_dword, WORD_BITS},
    shift,
    ubig::UBig,
};
use alloc::alloc::Layout;
use dashu_base::BitTest;
use static_assertions::const_assert;

/// If the window length is at or below this, the window is divided exactly
/// instead of split.
///
/// The recursive split takes `k = 2n/3` and `l = n - k`; its error analysis
/// needs `k >= (n+4)/2` and `l >= 2`, which first both hold for `n = 15`
/// (`k = 10`, `l = 5`). Below that the exact base-case division costs the
/// same order as any split, with the smallest possible error.
const THRESHOLD_SIMPLE_DEFAULT: usize = 15;
const_assert!(THRESHOLD_SIMPLE_DEFAULT >= MIN_SPLIT_LEN);

/// Smallest window length for which the recursive split satisfies its
/// constraints, for any `n % 3`. Used to clamp runtime tuning values.
const MIN_SPLIT_LEN: usize = 12;

/// Environment-variable override for the base-case threshold.
///
/// When the `tuning` feature is active the user may set
/// `DASHU_THRESHOLD_DIVHIGH_SIMPLE` to override the compile-time default.
/// Values that would break the split constraint are clamped to [`MIN_SPLIT_LEN`].
mod threshold {
    #[inline]
    pub fn simple() -> usize {
        #[cfg(feature = "tuning")]
        {
            if let Ok(s) = std::env::var("DASHU_THRESHOLD_DIVHIGH_SIMPLE") {
                if let Ok(v) = s.parse::<usize>() {
                    return v.max(super::MIN_SPLIT_LEN);
                }
            }
        }
        super::THRESHOLD_SIMPLE_DEFAULT
    }
}

/// Temporary memory required by [`div_high`] for a window of `n` words.
///
/// A recursive level splits `n = k + l` (`k = 2n/3`, `l = n/3`) and uses, in
/// sequential scopes: the scratch of an exact `2k`-by-`k` division; then a
/// short-product buffer of `l + 3` words together with the short-product
/// scratch; and finally the whole requirement of the recursive `l`-level
/// call. The bump allocator reuse closed scopes, so the requirement is the
/// maximum over the three.
pub(crate) fn memory_requirement_up_to(n: usize) -> Layout {
    if n <= threshold::simple() {
        div::memory_requirement_exact(2 * n, n)
    } else {
        let l = n - 2 * (n / 3);
        let k = n - l;
        let exact = div::memory_requirement_exact(2 * k, k);
        let cross = memory::add_layout(
            memory::array_layout::<Word>(l + 3),
            mul::memory_requirement_up_to(l),
        );
        memory::max_layout(exact, memory::max_layout(cross, memory_requirement_up_to(l)))
    }
}

/// Core short division: `q` (n words) := high part of the quotient of `np`
/// (2n words, clobbered) by `dp` (n words, most significant bit set), returning
/// the quotient's carry word (0 or 1).
///
/// The value `carry·B^n + {q, n}` approximates `np/dp` with the two-sided
/// error bound stated on [`div_high`]: the exact base case is off by less than
/// one ulp, and each recursive level adds the slack of one short product
/// window plus the block-quotient correction — under the split constraint
/// `k >= (n+4)/2` that is at most a couple of ulps per level, summing
/// geometrically (over `l = n/3`) to well under `2n` overall.
fn div_high_core(q: &mut [Word], np: &mut [Word], dp: &[Word], memory: &mut Memory) -> bool {
    let n = dp.len();
    debug_assert!(q.len() == n && np.len() == 2 * n && n >= 2);
    debug_assert!(dp[n - 1] >> (WORD_BITS - 1) != 0);

    if n <= threshold::simple() {
        // Exact division of the full window: the truncated quotient is the
        // best possible approximation (error below one ulp, one-sided).
        let fast_top = FastDivideNormalized2::new(highest_dword(dp));
        let carry = div::div_rem_in_place(np, dp, fast_top, memory);
        q.copy_from_slice(&np[n..]);
        return carry;
    }

    // Split with a large high part: k = 2n/3, l = n/3. The high k words of
    // the quotient come from an exact division of the top 2k words of np by
    // the top k words of dp; their interaction with dp's low l words is
    // removed through the high window of an l×l short product; the low l
    // words come from a recursive short division of the remaining region by
    // dp's low l words.
    let l = n - 2 * (n / 3);
    let k = n - l;
    debug_assert!(l >= 2 && k >= (n + 4) / 2);

    // Exact high block: the division writes its quotient into the top of the
    // np slice (copied out to q[l..n]) and the remainder just below, so that
    // afterwards the region np[..n+l] holds np minus the block quotient times
    // dp's high part.
    let mut qh: Word = {
        let fast_top = FastDivideNormalized2::new(highest_dword(&dp[l..]));
        let carry = div::div_rem_in_place(&mut np[2 * l..], &dp[l..], fast_top, memory);
        q[l..].copy_from_slice(&np[n + l..]);
        carry as Word
    };

    // Cross block: the low l words of the block quotient interact with dp's
    // low l words at the region's top. Subtract the certified lower bound of
    // that product (a short-product window) plus the block carry's copy of
    // the low divisor words; everything neglected stays inside the error
    // band. A borrow past the region means the block quotient was too large:
    // step it down and add dp back until the region is restored (at most a
    // couple of rounds — the split constraint bounds the overshoot).
    {
        let (tp, mut cross_memory) = memory.allocate_slice_fill::<Word>(l + 3, 0);
        let _neglected = mul::mul_high_into(tp, &q[k..], &dp[..l], &mut cross_memory);

        let mut cy: u32 = add::sub_same_len_in_place(&mut np[n..n + l], &tp[2..l + 2]) as u32;
        if qh != 0 {
            cy += add::sub_same_len_in_place(&mut np[n..n + l], &dp[..l]) as u32;
        }
        while cy > 0 {
            if add::sub_one_in_place(&mut q[l..]) {
                // a borrow out of the k block words can only come from qh
                debug_assert!(qh > 0);
                qh -= 1;
            }
            cy -= add::add_same_len_in_place(&mut np[l..n + l], dp) as u32;
        }
    }

    // Recursive short division of the region's top 2l words by dp's low l
    // words; its carry joins the block quotient.
    let carry = div_high_core(&mut q[..l], &mut np[k..k + 2 * l], &dp[k..], memory);
    if carry && add::add_word_in_place(&mut q[l..], 1) {
        qh += 1;
    }
    debug_assert!(qh <= 1);
    qh != 0
}

/// Write `value << shift` into `words` (assumed longer than the result), which
/// is fully overwritten. The bit part of the shift must not push anything out
/// of the top of `words`.
fn shift_words_into(words: &mut [Word], value: &[Word], shift: usize) {
    let (word_shift, bit_shift) = (shift / WORD_BITS as usize, shift % WORD_BITS as usize);
    debug_assert!(words.len() >= value.len() + word_shift);
    words.fill(0);
    words[word_shift..word_shift + value.len()].copy_from_slice(value);
    if bit_shift > 0 {
        debug_assert_zero!(shift::shl_in_place(words, bit_shift as u32));
    }
}

/// Short (truncated) division: computes the high `out_words + 1` words of the
/// quotient `numer / denom`, with a certified two-sided error bound. See the
/// [module documentation](super) for the exact contract.
///
/// Let `n = out_words`, `bn = numer.bit_len()`, `bd = denom.bit_len()`. The
/// call returns `None` unless `n >= 2`, the denominator has at least two and
/// at most `n` words, and the quotient is sized for the window
/// (`bn − bd ∈ [n·WORD_BITS − 2, n·WORD_BITS + 2]`) — pad the numerator with
/// zero digits (or split it) to fit. Otherwise the returned value `q`
/// satisfies
///
/// ```text
/// q − E  <=  (numer / denom) · 2^sigma  <  q + E
/// ```
///
/// with `sigma = n·WORD_BITS + bd − bn` (a small number, `−2..=2`, under the
/// sizing precondition) and `E = 2·n + 2` units in the last place of `q`.
/// Unlike the product functions no exactness flag is returned: the caller must
/// treat the result as inexact within the band.
///
/// # Examples
///
/// ```
/// use core::str::FromStr;
/// use dashu_base::BitTest;
/// use dashu_base::Abs;
/// use dashu_int::{high, IBig, UBig, Word};
///
/// let denom = UBig::from_str_radix("53210fed89abcdef13579bdf00d2c4a6", 16).unwrap();
/// let numer = (&denom << (4 * Word::BITS as usize)) + UBig::from(7u32);
/// let n = 4;
/// let q = high::div_high(&numer, &denom, n).unwrap();
///
/// // |q·2^(-sigma) − numer/denom| <= 2n + 2, checked here in exact integers
/// let sigma = n as isize * Word::BITS as isize + denom.bit_len() as isize
///     - numer.bit_len() as isize;
/// let (up, down) = (sigma.max(0) as usize, (-sigma).max(0) as usize);
/// let diff = IBig::from((&q * &denom) << up) - IBig::from(&numer << down);
/// let bound = IBig::from(2 * n as u32 + 2) * IBig::from(&denom << down);
/// assert!(diff.abs() <= bound);
/// ```
#[must_use]
pub fn div_high(numer: &UBig, denom: &UBig, out_words: usize) -> Option<UBig> {
    let nw = numer.as_words();
    let dw = denom.as_words();
    // Normalized values never carry empty words, but trim defensively so the
    // word counts below always describe real content.
    let wn = mul::trim_words(nw);
    let wd = mul::trim_words(dw);

    let n = out_words;
    if n < 2 || wn == 0 || wd < 2 || wd > n {
        return None;
    }
    let (bn, bd) = (numer.bit_len(), denom.bit_len());
    if bd > n * WORD_BITS as usize {
        return None;
    }
    let band = bn.checked_sub(bd)?;
    if !((n * WORD_BITS as usize - 2)..=(n * WORD_BITS as usize + 2)).contains(&band) {
        return None;
    }

    // Normalize to the kernel's (2n)/n shape: the numerator is scaled to
    // exactly 2n words and the denominator to exactly n words with its most
    // significant bit set. Both scalings are exact powers of two, so the
    // quotient only moves by the documented factor 2^sigma. When the numerator
    // is up to two bits too long for the window the down-shift truncates it;
    // the dropped bits stay inside the certified band.
    let word_bits = WORD_BITS as usize;
    // band and `bd <= n·WORD_BITS` keep this in `[-2, 2n·WORD_BITS]`
    let s_np = (2 * n) as isize * word_bits as isize - bn as isize;
    let s_dp = n * word_bits - bd; // nonnegative by the checks above

    let mut buffer = Buffer::allocate(3 * n);
    buffer.push_zeros(3 * n);
    let (q, np) = buffer[..].split_at_mut(n);

    if s_np >= 0 {
        shift_words_into(np, &nw[..wn], s_np as usize);
    } else {
        // The numerator's value overflows the window by at most two bits (the
        // band and `bd <= n·WORD_BITS` bound it). Shift those bits off
        // word-wise, so a numerator one word longer than the window still
        // feeds the loop; any bits leaving at the bottom are worth far less
        // than one ulp of the quotient.
        let j = (-s_np) as u32; // 1 or 2
        for i in 0..np.len().min(wn) {
            np[i] = nw[i] >> j;
            if i + 1 < wn {
                np[i] |= nw[i + 1] << (WORD_BITS - j);
            }
        }
    }

    let mut denom_buffer = Buffer::allocate(n);
    denom_buffer.push_zeros(n);
    shift_words_into(&mut denom_buffer, &dw[..wd], s_dp);

    let mut allocation = MemoryAllocation::new(memory_requirement_up_to(n));
    let carry = div_high_core(q, np, &denom_buffer, &mut allocation.memory());

    if carry {
        // the carry word sits on top of the full n-word window; trim only after
        let mut words = Buffer::allocate(n + 1);
        words.push_zeros(n + 1);
        words[..n].copy_from_slice(q);
        words[n] = 1;
        let len = mul::trim_words(&words);
        Some(UBig::from_words(&words[..len]))
    } else {
        let qlen = mul::trim_words(q);
        Some(UBig::from_words(&q[..qlen]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{IBig, UBig};
    use dashu_base::Abs;

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
                // Keep every word (in particular the top one) nonzero.
                if i + 1 == len && w == 0 {
                    1
                } else {
                    w
                }
            })
            .collect()
    }

    /// Check the full public contract with an exact integer comparison:
    /// `|q·denom·2^up − numer·2^down| <= E·denom·2^down` is precisely the
    /// documented band scaled to integers, with `up = max(sigma, 0)` and
    /// `down = max(−sigma, 0)`.
    fn check_case(numer: &[Word], denom: &[Word], n: usize) -> (IBig, IBig) {
        let (numer, denom) = (UBig::from_words(numer), UBig::from_words(denom));
        let q = div_high(&numer, &denom, n).expect("in-band inputs must be accepted");

        let sigma = n as isize * WORD_BITS_USIZE as isize + denom.bit_len() as isize
            - numer.bit_len() as isize;
        let (up, down) = (sigma.max(0) as usize, (-sigma).max(0) as usize);
        let bound = IBig::from(2 * n as u64 + 2) * IBig::from(&denom << down);
        let diff = (IBig::from((&q * &denom) << up) - IBig::from(&numer << down)).abs();
        assert!(diff <= bound, "error bound violated: |diff|={diff:?} n={n} sigma={sigma}");
        (diff, bound)
    }

    /// A numerator whose bit length exceeds the denominator's by exactly
    /// `n·WORD_BITS`: `n` random words on top of the divisor words, with the
    /// top word adjusted to carry the same bit length as the divisor's top
    /// word (so the band holds for any word size).
    fn in_band_numer(seed: u64, denom: &[Word], n: usize) -> Vec<Word> {
        let d_top = *denom.last().unwrap();
        let d_bits = WORD_BITS - d_top.leading_zeros();
        let mut numer = lcg_words(seed, n);
        numer[n - 1] = (d_top >> 1) | (1 << (d_bits - 1));
        numer.extend_from_slice(denom);
        numer
    }

    #[test]
    fn test_div_high_exact_band() {
        // The base-case band (n <= threshold) and the first recursion levels.
        for &n in &[2, 3, 5, 8, 12, 15, 16, 17, 20, 24, 30, 45, 60, 90, 130, 200] {
            for seed in 1..=3u64 {
                let denom = lcg_words(seed * 101 + 7, (n / 2 + 2).min(n));
                check_case(&in_band_numer(seed * 61 + 3, &denom, n), &denom, n);
            }
        }
    }

    #[test]
    fn test_div_high_full_width_denominators() {
        // Denominators with a full top word exercise the normalization shifts
        // in the kernel (and the `s_np < wn·WORD_BITS` copy path).
        for &n in &[6, 16, 24, 40, 70] {
            let mut denom = lcg_words(0x5eed + n as u64, n / 2 + 2);
            let last = denom.len() - 1;
            denom[last] |= 1 << (WORD_BITS - 1);
            check_case(&in_band_numer(0x7000 + n as u64, &denom, n), &denom, n);
        }
    }

    #[test]
    fn test_div_high_carry_quotient() {
        // Numerators at and above the block-quotient carry edge: the exact
        // block sees the top of its dividend at least as large as the top of
        // the divisor.
        for &n in &[4, 15, 16, 24, 40, 70] {
            let mut denom = lcg_words(0x5eed, n / 2 + 2);
            let last = denom.len() - 1;
            denom[last] |= 1 << (WORD_BITS - 1);

            // A numerator extended to n extra words, with the top word full.
            let mut numer = denom.clone();
            numer.resize(n + denom.len(), 0);
            numer[n + denom.len() - 1] = Word::MAX;
            check_case(&numer, &denom, n);

            // The same frame with the smallest possible full-width top word.
            let mut numer = denom.clone();
            numer.resize(n + denom.len(), 0);
            numer[n + denom.len() - 1] = 1 << (WORD_BITS - 1);
            check_case(&numer, &denom, n);
        }
    }

    #[test]
    fn test_div_high_edge_patterns() {
        // All-ones words: maximal borrow and carry chains.
        let ones = |len: usize| vec![Word::MAX; len];
        for &n in &[3, 12, 16, 24, 48, 80] {
            check_case(&ones(n + n / 2 + 2), &ones(n / 2 + 2), n);
        }

        // A sparse denominator with a single high bit.
        let mut denom = vec![0 as Word; 9];
        denom[8] = 1;
        check_case(&in_band_numer(0x0dd, &denom, 12), &denom, 12);

        // Out-of-band and malformed inputs must be declined.
        let d = UBig::from_words(&[1, 2, 3]);
        let small = &d << 100;
        assert!(div_high(&small, &d, 2).is_none()); // window too small for the quotient
        assert!(div_high(&small, &d, 0).is_none());
        assert!(div_high(&d, &UBig::from(2u32), 4).is_none()); // single-word divisor
        assert!(div_high(&UBig::ZERO, &d, 4).is_none());
        assert!(div_high(&d, &UBig::ZERO, 4).is_none());

        // A divisor wider than the window is declined.
        let wide = &d << 300;
        assert!(div_high(&(&wide << 10), &wide, 2).is_none());
    }

    /// The empirical worst-case error must stay far inside the documented
    /// `2n + 2` band — the analysis on [`div_high_core`] predicts about half
    /// of it at most.
    #[test]
    fn test_div_high_empirical_error_headroom() {
        let mut worst = IBig::ZERO;
        for &n in &[16, 24, 33, 48, 64, 96] {
            for seed in 1..=8u64 {
                let denom = lcg_words(seed * 7919, n / 2 + 2);
                let (diff, bound) = check_case(&in_band_numer(seed * 104729, &denom, n), &denom, n);
                // fixed-point ratio in thousandths, for a readable maximum
                let ratio = (&diff * 1000) / &bound;
                if ratio > worst {
                    worst = ratio;
                }
            }
        }
        assert!(
            worst <= IBig::from(700),
            "empirical error crept above 70% of the bound: {worst} thousandths"
        );
    }
}
