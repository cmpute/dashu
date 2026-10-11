//! Short (truncated) division kernel.

use super::mul;
#[cfg(test)]
use crate::primitive::WORD_BITS_USIZE;
use crate::{
    add,
    arch::word::Word,
    buffer::Buffer,
    div,
    math::FastDivideNormalized2,
    memory::{self, Memory, MemoryAllocation},
    primitive::{highest_dword, locate_top_word_plus_one, WORD_BITS},
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

/// Smallest window length such that the recursive split satisfies its
/// constraints for every length at or above it (the split needs
/// `k >= (n+4)/2` with `k = n - (n - 2·⌊n/3⌋)`, which fails for n = 13, 14).
/// Used to clamp runtime tuning values.
const MIN_SPLIT_LEN: usize = 15;

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

/// Core short division: `out` (n words) := high part of the quotient of
/// `words` (2n words, clobbered) by `d` (n words, most significant bit set),
/// returning the quotient's carry word (0 or 1).
///
/// The value `carry·B^n + {out, n}` approximates `words/d` with the two-sided
/// error bound stated on [`div_high`]: the exact base case is off by less than
/// one ulp, and each recursive level adds the slack of one short product
/// window plus the block-quotient correction — under the split constraint
/// `k >= (n+4)/2` that is at most a couple of ulps per level, summing
/// geometrically (over `l = n/3`) to well under `2n` overall.
fn div_high_core(out: &mut [Word], words: &mut [Word], d: &[Word], memory: &mut Memory) -> bool {
    let n = d.len();
    debug_assert!(out.len() == n && words.len() == 2 * n && n >= 2);
    debug_assert!(d[n - 1] >> (WORD_BITS - 1) != 0);

    if n <= threshold::simple() {
        // Exact division of the full window: the truncated quotient is the
        // best possible approximation (error below one ulp, one-sided).
        let fast_top = FastDivideNormalized2::new(highest_dword(d));
        let carry = div::div_rem_in_place(words, d, fast_top, memory);
        out.copy_from_slice(&words[n..]);
        return carry;
    }

    // Split with a large high part: k = 2n/3, l = n/3. The high k words of
    // the quotient come from an exact division of the top 2k words of the
    // dividend by the top k words of `d`; their interaction with d's low l
    // words is removed through the high window of an l×l short product; the
    // low l words come from a recursive short division of the remaining
    // region by d's low l words.
    let l = n - 2 * (n / 3);
    let k = n - l;
    debug_assert!(l >= 2 && k >= (n + 4) / 2);

    // Exact high block: the division writes its quotient into the top of the
    // dividend slice (copied out to out[l..n]) and the remainder just below,
    // so that afterwards the region words[..n+l] holds the dividend minus the
    // block quotient times d's high part.
    let mut qh: Word = {
        let fast_top = FastDivideNormalized2::new(highest_dword(&d[l..]));
        let carry = div::div_rem_in_place(&mut words[2 * l..], &d[l..], fast_top, memory);
        out[l..].copy_from_slice(&words[n + l..]);
        carry as Word
    };

    // Cross block: the low l words of the block quotient interact with d's
    // low l words at the region's top. Subtract the certified lower bound of
    // that product (a short-product window) plus the block carry's copy of
    // the low divisor words; everything neglected stays inside the error
    // band. A borrow past the region means the block quotient was too large:
    // step it down and add `d` back until the region is restored (at most a
    // couple of rounds — the split constraint bounds the overshoot).
    {
        let (tp, mut cross_memory) = memory.allocate_slice_fill::<Word>(l + 3, 0);
        // The short product's sticky flag (content dropped below its window)
        // needs no action: any neglected contribution leaves the region too
        // large, which the certified two-sided band of the final quotient
        // already covers — unlike a `debug_assert_zero!` case, `true` is an
        // expected, benign outcome on this path.
        mul::mul_high_into(tp, &out[k..], &d[..l], &mut cross_memory);

        let mut cy: u32 = add::sub_same_len_in_place(&mut words[n..n + l], &tp[2..l + 2]) as u32;
        if qh != 0 {
            cy += add::sub_same_len_in_place(&mut words[n..n + l], &d[..l]) as u32;
        }
        while cy > 0 {
            if add::sub_one_in_place(&mut out[l..]) {
                // a borrow out of the k block words can only come from qh
                debug_assert!(qh > 0);
                qh -= 1;
            }
            cy -= add::add_same_len_in_place(&mut words[l..n + l], d) as u32;
        }
    }

    // Recursive short division of the region's top 2l words by d's low l
    // words; its carry joins the block quotient.
    let carry = div_high_core(&mut out[..l], &mut words[k..k + 2 * l], &d[k..], memory);
    if carry && add::add_word_in_place(&mut out[l..], 1) {
        qh += 1;
    }
    debug_assert!(qh <= 1);
    qh != 0
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
/// // a window exactly as wide as the denominator (which satisfies the sizing
/// // precondition at any word size), with the numerator scaled to it
/// let n = denom.as_words().len();
/// let numer = (&denom << (n * Word::BITS as usize)) + UBig::from(7u32);
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
    // The word slices of a UBig are normalized (no trailing zero words, by
    // dashu-int's representation invariant), so the counts are the lengths.
    let (wn, wd) = (nw.len(), dw.len());

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
    let numer_shift = (2 * n) as isize * word_bits as isize - bn as isize;
    let denom_shift = n * word_bits - bd; // nonnegative by the checks above

    let mut buffer = Buffer::allocate(3 * n);
    buffer.push_zeros(3 * n);
    let (out, words) = buffer[..].split_at_mut(n);

    if numer_shift >= 0 {
        shift::shl_words_into(words, &nw[..wn], numer_shift as usize);
    } else {
        // The numerator's value overflows the window by at most two bits (the
        // band and `bd <= n·WORD_BITS` bound it). Shift those bits off
        // word-wise, so a numerator one word longer than the window still
        // feeds the loop; any bits leaving at the bottom are worth far less
        // than one ulp of the quotient.
        let j = (-numer_shift) as u32; // 1 or 2
        for i in 0..words.len().min(wn) {
            words[i] = nw[i] >> j;
            if i + 1 < wn {
                words[i] |= nw[i + 1] << (WORD_BITS - j);
            }
        }
    }

    let mut denom_buffer = Buffer::allocate(n);
    denom_buffer.push_zeros(n);
    shift::shl_words_into(&mut denom_buffer, &dw[..wd], denom_shift);

    let mut allocation = MemoryAllocation::new(memory_requirement_up_to(n));
    let carry = div_high_core(out, words, &denom_buffer, &mut allocation.memory());

    if carry {
        // the carry word sits on top of the full n-word window (it is nonzero,
        // so the buffer is already normalized)
        let mut buffer = Buffer::allocate(n + 1);
        buffer.push_zeros(n + 1);
        buffer[..n].copy_from_slice(out);
        buffer[n] = 1;
        Some(UBig::from_words(&buffer))
    } else {
        // the quotient's top word can be zero when the window over-covers it
        let qlen = locate_top_word_plus_one(out);
        Some(UBig::from_words(&out[..qlen]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{test_utils::pattern_words, IBig, UBig};
    use alloc::vec;
    use alloc::vec::Vec;
    use dashu_base::Abs;

    /// Fixed (denominator, numerator) step pairs: complementary patterns, so
    /// the two operands of a case never align.
    const CASES: [(u64, u64); 3] = [
        (0x0123_4567_89ab_cdef, 0x3333_3333_3333_3335),
        (0xfedc_ba98_7654_3211, 0x5eed_5eed_5eed_5eed),
        (0x1357_9bdf_2468_ace1, 0x0f0f_0f0f_0f0f_0f0f),
    ];

    /// Check the full public contract with an exact integer comparison:
    /// `|q·denom·2^down − numer·2^up| <= E·denom·2^down` is precisely the
    /// documented band `|q − (numer/denom)·2^sigma| <= E` scaled to integers
    /// (multiplying by `denom·2^down`), with `up = max(sigma, 0)` and
    /// `down = max(−sigma, 0)`.
    fn check_case(numer: &[Word], denom: &[Word], n: usize) -> (IBig, IBig) {
        let (numer, denom) = (UBig::from_words(numer), UBig::from_words(denom));
        let q = div_high(&numer, &denom, n).expect("in-band inputs must be accepted");

        let sigma = n as isize * WORD_BITS_USIZE as isize + denom.bit_len() as isize
            - numer.bit_len() as isize;
        let (up, down) = (sigma.max(0) as usize, (-sigma).max(0) as usize);
        let bound = IBig::from(2 * n as u64 + 2) * IBig::from(&denom << down);
        let diff = (IBig::from((&q * &denom) << down) - IBig::from(&numer << up)).abs();
        assert!(diff <= bound, "error bound violated: |diff|={diff:?} n={n} sigma={sigma}");
        (diff, bound)
    }

    /// A numerator pinned to a band edge: [`in_band_numer`] shifted so that
    /// its bit-length offset from the denominator is exactly
    /// `n·WORD_BITS − sigma`, i.e. `sigma ∈ [-2, 2]` covers the whole band.
    fn band_edge_numer(step: u64, denom: &[Word], n: usize, sigma: isize) -> Vec<Word> {
        let base = UBig::from_words(&in_band_numer(step, denom, n));
        let numer = if sigma >= 0 {
            &base >> sigma as usize
        } else {
            &base << (-sigma) as usize
        };
        numer.as_words().to_vec()
    }

    /// A numerator whose bit length exceeds the denominator's by exactly
    /// `n·WORD_BITS`: `n` pattern words on top of the divisor words, with the
    /// top word adjusted to carry the same bit length as the divisor's top
    /// word (so the band holds for any word size).
    fn in_band_numer(step: u64, denom: &[Word], n: usize) -> Vec<Word> {
        let d_top = *denom.last().unwrap();
        let d_bits = WORD_BITS - d_top.leading_zeros();
        let mut numer = pattern_words(n, step);
        numer[n - 1] = (d_top >> 1) | (1 << (d_bits - 1));
        numer.extend_from_slice(denom);
        numer
    }

    #[test]
    fn test_div_high_exact_band() {
        // The base-case band (n <= threshold) and the first recursion levels,
        // with the numerator pinned to every band edge (sigma ∈ [-2, 2]).
        for &n in &[2, 3, 5, 8, 12, 15, 16, 17, 20, 24, 30, 45, 60, 90, 130, 200] {
            for &(dstep, nstep) in &CASES {
                let denom = pattern_words((n / 2 + 2).min(n), dstep);
                for sigma in -2..=2isize {
                    check_case(&band_edge_numer(nstep, &denom, n, sigma), &denom, n);
                }
            }
        }
    }

    #[test]
    fn test_div_high_full_width_denominators() {
        // Denominators with a full top word exercise the normalization shifts
        // in the kernel (and the `numer_shift < wn·WORD_BITS` copy path —
        // reached on the negative band edges, where the numerator grows past
        // 2n words).
        for &n in &[6, 16, 24, 40, 70] {
            let mut denom = pattern_words(n / 2 + 2, 0x5eed + n as u64);
            let last = denom.len() - 1;
            denom[last] |= 1 << (WORD_BITS - 1);
            for sigma in -2..=2isize {
                check_case(&band_edge_numer(0x7000 + n as u64, &denom, n, sigma), &denom, n);
            }
        }
    }

    #[test]
    fn test_div_high_carry_quotient() {
        // Numerators at and above the block-quotient carry edge: the exact
        // block sees the top of its dividend at least as large as the top of
        // the divisor.
        for &n in &[4, 15, 16, 24, 40, 70] {
            let mut denom = pattern_words(n / 2 + 2, 0x5eed);
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
        // Eight complementary step pairs, all distinct from `CASES`.
        const HEADROOM_STEPS: [(u64, u64); 8] = [
            (0x0fed_cba9_8765_4321, 0x7777_7777_7777_7777),
            (0x5eed_5eed_5eed_5eed, 0x1357_9bdf_2468_ace1),
            (0x3333_3333_3333_3335, 0x0123_4567_89ab_cdef),
            (0x0f0f_0f0f_0f0f_0f0f, 0xfedc_ba98_7654_3211),
            (0x7777_7777_7777_7777, 0x0fed_cba9_8765_4321),
            (0x1357_9bdf_2468_ace1, 0x5eed_5eed_5eed_5eed),
            (0x0123_4567_89ab_cdef, 0x3333_3333_3333_3335),
            (0x2468_ace0_2468_ace1, 0x9999_9999_9999_9999),
        ];
        let mut worst = IBig::ZERO;
        for &n in &[16, 24, 33, 48, 64, 96] {
            for &(dstep, nstep) in &HEADROOM_STEPS {
                let denom = pattern_words(n / 2 + 2, dstep);
                let (diff, bound) = check_case(&in_band_numer(nstep, &denom, n), &denom, n);
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
