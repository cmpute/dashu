//! High-part multiplication kernel.

#[cfg(test)]
use crate::primitive::WORD_BITS_USIZE;
use crate::{
    add,
    arch::word::Word,
    buffer::Buffer,
    helper_macros::debug_assert_zero,
    memory::{self, Memory, MemoryAllocation},
    mul,
    primitive::double_word,
    ubig::UBig,
    Sign::Positive,
};
use alloc::alloc::Layout;
use static_assertions::const_assert;

/// If the window length is at or below this, the windowed column sweep is used directly.
///
/// The sweep beats the recursive composition up to ~96 words: below that the
/// recursion's exact high block is a full base-case multiplication, which
/// costs more than everything the sweep saves. At 96 words the block reaches
/// the Karatsuba band (k = 72) and the recursion starts winning (measured
/// 0.90x at 128 words, 0.67x at 320; see the `crossover_mulhigh` test).
const THRESHOLD_SIMPLE_DEFAULT: usize = 96;
// The recursive split needs l = n/4 >= 1 and 2*l + 2 <= n, which holds for every
// n >= 6; the margin below keeps the recursion domain comfortably above that.
const_assert!(THRESHOLD_SIMPLE_DEFAULT >= 8);

/// Environment-variable override for the base-case threshold.
///
/// When the `tuning` feature is active the user may set
/// `DASHU_THRESHOLD_MULHIGH_SIMPLE` to override the compile-time default.
mod threshold {
    #[inline]
    pub fn simple() -> usize {
        #[cfg(feature = "tuning")]
        {
            if let Ok(s) = std::env::var("DASHU_THRESHOLD_MULHIGH_SIMPLE") {
                if let Ok(v) = s.parse() {
                    return v;
                }
            }
        }
        super::THRESHOLD_SIMPLE_DEFAULT
    }
}

/// Temporary memory required by [`mul_high`] for a window of `n` words.
pub(crate) fn memory_requirement_up_to(n: usize) -> Layout {
    // Recursion layout (l = n/4, k = n - l); the bump allocator reuses the same
    // region across the sequential scopes, so only the larger scope matters:
    //   exact high block:  2k words + mul::memory_requirement_up_to(2k, k)
    //   cross products:    (l + 3) words + memory_requirement_up_to(l)
    // The max is computed exactly instead of a closed formula so that tuning
    // either this threshold or the multiplication thresholds stays sound.
    if n <= threshold::simple() {
        memory::zero_layout()
    } else {
        let l = n / 4;
        let k = n - l;
        let block = memory::add_layout(
            memory::array_layout::<Word>(2 * k),
            mul::memory_requirement_up_to(2 * k, k),
        );
        let cross =
            memory::add_layout(memory::array_layout::<Word>(l + 3), memory_requirement_up_to(l));
        memory::max_layout(block, cross)
    }
}

/// Windowed column sweep: accumulate into `t` every product term that reaches
/// the window's boundary column or above. The window is the top `window` words
/// of the `ap * bp` product (`na = ap.len() >= nb = bp.len()`, `window >= na`),
/// and `t` (`window + 3` words) holds the product columns from two below the
/// window up through its top.
///
/// The multiplier words of `bp` are consumed two at a time by the shared
/// double-word kernel; each pair sweeps only the suffix of `ap` whose products
/// reach the accumulator, aligned at the pair's lowest kept product. Every
/// neglected slice is a prefix of `ap` times one multiplier word, worth less
/// than one unit of the accumulator's bottom column; summed over all sweeps
/// the neglected value stays below `nb/2 + 2` units of the window's least
/// significant word — within the certified `window + 2` bound.
///
/// Returns whether any contribution below the window is nonzero.
pub(super) fn mul_high_basecase(t: &mut [Word], ap: &[Word], bp: &[Word], window: usize) -> bool {
    let (na, nb) = (ap.len(), bp.len());
    debug_assert!(nb <= na && na <= window && window < na + nb && t.len() == window + 3);

    // Product column held by t[0]: the window starts at column base_col, and
    // the accumulator reaches two columns further down.
    let base_col = na + nb - window;

    // Index of the first nonzero word of ap: a neglected prefix ap[..i0] is
    // nonzero exactly when i0 exceeds it.
    let first_nonzero = ap.iter().position(|&w| w != 0).unwrap_or(na);

    let mut sticky = false;

    let mut j = 0;
    let mut pairs = bp.chunks_exact(2);
    for pair in &mut pairs {
        // Products a_i·b_j reach t[0]'s column when i+j >= base_col - 2; sweep
        // from the first such i. The sweep lands at t[off..off+len].
        let i0 = base_col.saturating_sub(2).saturating_sub(j).min(na);
        let len = na - i0;
        let off = i0 + j + 2 - base_col;
        let (carry_lo, carry_hi) = mul::add_mul_dword_same_len_in_place(
            &mut t[off..off + len],
            &ap[i0..],
            pair[0],
            pair[1],
        );
        let overflow =
            add::add_dword_in_place(&mut t[off + len..], double_word(carry_lo, carry_hi));
        debug_assert!(!overflow);
        sticky |= i0 > first_nonzero && (pair[0] != 0 || pair[1] != 0);
        j += 2;
    }
    // The leftover odd multiplier word (j = nb-1) sweeps the same way.
    if let &[m] = pairs.remainder() {
        let jj = nb - 1;
        let i0 = base_col.saturating_sub(2).saturating_sub(jj).min(na);
        let len = na - i0;
        let off = i0 + jj + 2 - base_col;
        let carry = mul::add_mul_word_same_len_in_place(&mut t[off..off + len], m, &ap[i0..]);
        let overflow = add::add_word_in_place(&mut t[off + len..], carry);
        debug_assert!(!overflow);
        sticky |= i0 > first_nonzero && m != 0;
    }

    // The two accumulator words below the window are dropped from the value;
    // any content there means the true product is strictly above the window.
    sticky |= t[0] != 0;
    sticky |= t[1] != 0;
    sticky
}

/// Accumulate the high window of `ap * bp` (both `n` words) into `t` (`n + 3`
/// words holding the product columns `n-2 ..= 2n`). Returns whether any
/// contribution below the window (columns `< n`) is nonzero.
///
/// The value accumulated into `t[2..n+2]` (the window columns `n..2n-1`) is
/// the window of the product with the error bound stated on [`mul_high`]; the
/// additions are all into `t`, so `t` need not start zeroed for the value to
/// be correct — callers use that to skip zeroing on reuse.
pub(super) fn mul_high_into(t: &mut [Word], ap: &[Word], bp: &[Word], memory: &mut Memory) -> bool {
    let n = ap.len();
    debug_assert!(bp.len() == n && t.len() == n + 3);

    if n <= threshold::simple() {
        return mul_high_basecase(t, ap, bp, n);
    }

    // Split with a large high part: l = n/4, k = n - l. Then
    //   * the exact block ap[l..] * bp[l..] covers columns 2l..2n-1,
    //   * the cross products ap[k..] * bp[..l] and ap[..l] * bp[k..] span
    //     columns k..n+l-1, so only their high l words (columns n..n+l-1)
    //     reach the window, and
    //   * the low-low block ap[..l] * bp[..l] spans columns 0..2l-2, which
    //     lies entirely below column n-2 (2l + 2 <= n), so it is dropped.
    let l = n / 4;
    let k = n - l;
    debug_assert!(l >= 1 && 2 * l + 2 <= n);

    let mut sticky = false;

    {
        // Exact high block at columns 2l..2n-1. Only its words from column n-2
        // up are added into t; the words below only matter through their
        // bounded carry, so they are dropped and flagged.
        let (block, mut block_memory) = memory.allocate_slice_fill::<Word>(2 * k, 0);
        debug_assert_zero!(mul::add_signed_mul_same_len(
            block,
            Positive,
            &ap[l..],
            &bp[l..],
            &mut block_memory
        ));
        let off = n - 2 - 2 * l;
        let overflow = add::add_in_place(&mut t[..n + 2], &block[off..]);
        debug_assert!(!overflow);
        sticky |= block[..off].iter().any(|&w| w != 0);
    }
    {
        // Cross products. A recursive call on (top l words, bottom l words)
        // produces exactly the frame t[0..l+3] here: its columns l-2..2l are
        // our columns n-2..n+l (k + l = n), so it aligns with t directly.
        let (x, mut cross_memory) = memory.allocate_slice_fill::<Word>(l + 3, 0);
        let crosses: [(&[Word], &[Word]); 2] = [(&ap[k..], &bp[..l]), (&ap[..l], &bp[k..])];
        for (u, v) in crosses {
            sticky |= mul_high_into(x, u, v, &mut cross_memory);
            if add::add_same_len_in_place(&mut t[..l + 3], x) {
                debug_assert!(!add::add_word_in_place(&mut t[l + 3..], 1));
            }
            x.fill(0);
        }
    }

    // Dropped low-low block.
    sticky |= ap[..l].iter().any(|&w| w != 0) && bp[..l].iter().any(|&w| w != 0);
    sticky
}

/// Number of words up to and including the last nonzero word.
pub(super) fn trim_words(words: &[Word]) -> usize {
    words.iter().rposition(|&w| w != 0).map_or(0, |i| i + 1)
}

/// Compute the high `out_words` words of the product `a * b`, with a certified
/// one-sided error bound. See the [module documentation](super) for the exact
/// contract.
///
/// `out_words` is clamped to `min(words(a), words(b)) + 2`: a window wider
/// than the smaller operand by one or two words keeps the smaller operand
/// whole (the sweep then runs on the (nearly) full operands), and anything
/// wider falls back to that boundary. A window covering the entire product
/// returns it exactly with a `false` sticky flag.
///
/// # Examples
///
/// ```
/// use dashu_int::{high, UBig, Word};
/// use core::str::FromStr;
///
/// let a = UBig::from_str_radix("fffffffffffffffffffffffffffffffeffffffffffffffff", 16).unwrap();
/// let b = UBig::from_str_radix("100020003000400050006000700080009000a000b000c000d", 16).unwrap();
///
/// let (v, sticky) = high::mul_high(&a, &b, 2);
/// let full = &a * &b;
/// let s = (a.as_words().len() + b.as_words().len() - 2) * Word::BITS as usize;
/// let top = &full >> s;
///
/// // v under-estimates the truncated product by less than out_words + 2 ulps ...
/// assert!(v <= top);
/// assert!(&top - &v <= UBig::from(4u32));
/// // ... and sticky reports whether anything was dropped.
/// assert_eq!(sticky, &full != &(&top << s));
/// ```
#[must_use]
pub fn mul_high(a: &UBig, b: &UBig, out_words: usize) -> (UBig, bool) {
    let aw = a.as_words();
    let bw = b.as_words();
    // Normalized values never carry empty words, but trim defensively so the
    // word counts below always describe real content.
    let wa = trim_words(aw);
    let wb = trim_words(bw);

    if wa == 0 || wb == 0 {
        // A zero operand: the product and every window of it are zero.
        return (UBig::ZERO, false);
    }
    if out_words >= wa + wb {
        // The window covers the entire product.
        return (a * b, false);
    }
    if out_words == 0 {
        return (UBig::ZERO, true);
    }
    // The window may extend up to two words beyond the smaller operand: the
    // windowed sweep then runs directly on (nearly) full operands. Beyond
    // that, clamp to the smaller operand so the recursive path keeps its
    // square shape.
    let n = out_words.min(wa.min(wb) + 2);
    debug_assert!(n >= 1 && n < wa + wb);

    let (ap, bp, sticky, square_frame);
    if n <= wa.min(wb) {
        // Square frame: truncate both operands to their top n words. Each
        // truncation drops a value worth less than one unit of the window's
        // least significant word, absorbed by the certified bound.
        square_frame = true;
        ap = &aw[wa - n..];
        bp = &bw[wb - n..];
        sticky = (wa > n && aw[..wa - n].iter().any(|&w| w != 0))
            || (wb > n && bw[..wb - n].iter().any(|&w| w != 0));
    } else {
        // Extended window: keep the smaller operand whole and take as many
        // words of the larger one as the window allows. Only the larger
        // operand is truncated (again below one unit of the window's last
        // word); the extended window itself adds no further error.
        square_frame = false;
        let (lo_full, hi_full) = if wa >= wb { (bw, aw) } else { (aw, bw) };
        let hi_len = hi_full.len().min(n);
        let (hi_words, lo_words) = (&hi_full[hi_full.len() - hi_len..], lo_full);
        ap = hi_words;
        bp = lo_words;
        sticky =
            hi_len < hi_full.len() && hi_full[..hi_full.len() - hi_len].iter().any(|&w| w != 0);
    }

    let mut buffer = Buffer::allocate(n + 3);
    buffer.push_zeros(n + 3);
    // The recursive path needs scratch memory; the windowed sweep needs none.
    let sticky_core = if square_frame {
        let mut allocation = MemoryAllocation::new(memory_requirement_up_to(n));
        mul_high_into(&mut buffer, ap, bp, &mut allocation.memory())
    } else {
        // Extended window: run the sweep directly (correct at any size; the
        // cost is the caller's choice).
        mul_high_basecase(&mut buffer, ap, bp, n)
    };

    // The window may carry zero high words when the product is short; UBig
    // words are little-endian, so trim from the top.
    let hi = &buffer[2..n + 2];
    let hi_len = trim_words(hi);
    (UBig::from_words(&hi[..hi_len]), sticky || sticky_core)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{memory::MemoryAllocation, UBig};

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

    /// Check the full public contract against a schoolbook full product.
    fn check_case(a: &[Word], b: &[Word], n: usize) {
        // Apply the same clamping as the public contract.
        let n = n.min(a.len().min(b.len()) + 2).min(a.len() + b.len());
        let (a_u, b_u) = (UBig::from_words(a), UBig::from_words(b));
        let (v, sticky) = mul_high(&a_u, &b_u, n);
        let full = &a_u * &b_u;
        let s = WORD_BITS_USIZE * (a.len() + b.len() - n);
        let top = &full >> s;

        assert!(v <= top, "one-sidedness violated at n={n}");
        let diff = &top - &v;
        assert!(
            diff <= UBig::from(n as u64 + 2),
            "error bound violated: diff={diff:?} n={n} a_len={} b_len={}",
            a.len(),
            b.len()
        );
        assert_eq!(sticky, full != (&top << s), "sticky mismatch at n={n}");
    }

    #[test]
    fn test_mul_high_basecase_range() {
        for n in 1..=THRESHOLD_SIMPLE_DEFAULT {
            for seed in 1..=2u64 {
                let a = lcg_words(seed * 31, n);
                let b = lcg_words(seed * 47 + 5, n);
                check_case(&a, &b, n);
            }
        }
    }

    #[test]
    fn test_mul_high_extended_windows() {
        // Windows reaching one and two words beyond the smaller operand: the
        // sweep then covers (nearly) full operands. This is the shape the
        // floating-point layer uses for equal-precision multiplication.
        for n in 1..=40usize {
            for seed in 1..=2u64 {
                let a = lcg_words(seed * 59, n);
                let b = lcg_words(seed * 61 + 3, n);
                check_case(&a, &b, n + 1);
                check_case(&a, &b, n + 2);
                check_case(&b, &a, n + 1);
                check_case(&b, &a, n + 2);
            }
            // Rectangular operands (smaller by a few words).
            let a = lcg_words(0x0dd, n + 5);
            let b = lcg_words(0x0ee, n);
            check_case(&a, &b, n + 2);
            check_case(&a, &b, (n + 5).min(n + 2));
        }
    }

    #[test]
    fn test_mul_high_recursive_range() {
        // Sizes spanning the first and second recursion levels.
        for &n in &[97usize, 100, 120, 160, 200, 260, 300, 385, 400] {
            for seed in 1..=2u64 {
                let a = lcg_words(seed * 101, n);
                let b = lcg_words(seed * 137 + 11, n);
                check_case(&a, &b, n);
            }
        }
    }

    #[test]
    fn test_mul_high_unbalanced() {
        for &(wa, wb) in &[(128usize, 40usize), (300, 100), (120, 120), (500, 130)] {
            let a = lcg_words(0x1234_5678, wa);
            let b = lcg_words(0x9abc_def0, wb);
            let n = wb.min(wa).min(100);
            check_case(&a, &b, n);
            check_case(&b, &a, n);
        }
    }

    #[test]
    fn test_mul_high_edge_patterns() {
        // All-ones words: maximal carry chains.
        let ones = |len: usize| vec![Word::MAX; len];
        for n in &[3usize, 12, 40, 96, 97, 130] {
            check_case(&ones(*n), &ones(*n), *n);
            check_case(&ones(*n), &ones(*n), *n / 2 + 1);
        }
        // Single high bit: sparse products with long zero runs.
        let mut sparse = vec![0 as Word; 33];
        sparse[32] = 1;
        check_case(&sparse, &sparse.clone(), 17);
        check_case(&sparse, &lcg_words(7, 33), 20);

        // Trailing zero words: the dropped part is exactly zero, so the window
        // must be exact with a false sticky flag.
        let mut a = vec![0 as Word; 7];
        a[6] = Word::MAX;
        let mut b = vec![0 as Word; 7];
        b[6] = 3;
        let (v, sticky) = mul_high(&UBig::from_words(&a), &UBig::from_words(&b), 4);
        assert!(!sticky);
        let full = UBig::from_words(&a) * UBig::from_words(&b);
        assert_eq!(v, &full >> (WORD_BITS_USIZE * (7 + 7 - 4)));

        // Window covering the whole product, empty window, zero operands.
        let small = UBig::from_words(&[1, 2, 3]);
        let (v, sticky) = mul_high(&small, &small, 8);
        assert_eq!(v, &small * &small);
        assert!(!sticky);
        let (v, sticky) = mul_high(&small, &small, 0);
        assert_eq!(v, UBig::ZERO);
        assert!(sticky);
        let (v, sticky) = mul_high(&UBig::ZERO, &small, 2);
        assert_eq!(v, UBig::ZERO);
        assert!(!sticky);
    }

    #[test]
    fn test_mul_high_memory_layout_sound() {
        // Every allocation the recursion makes must fit the advertised layout.
        for &n in &[97usize, 120, 200, 400] {
            let a = lcg_words(0x0f0f, n);
            let b = lcg_words(0xf0f0, n);
            let mut buffer = Buffer::allocate(n + 3);
            buffer.push_zeros(n + 3);
            let mut allocation = MemoryAllocation::new(memory_requirement_up_to(n));
            let _ = mul_high_into(&mut buffer, &a, &b, &mut allocation.memory());
        }
    }

    /// Compare the windowed sweep against the recursive composition to find the
    /// base-case threshold. Run with:
    ///   cargo test -p dashu-int --release -- high::mul::tests::crossover_mulhigh --ignored --nocapture
    #[test]
    #[ignore]
    fn crossover_mulhigh() {
        use std::time::Instant;

        let sizes: &[usize] = &[80, 96, 112, 128, 160, 200, 256, 320, 384];
        println!("{:>8} {:>14} {:>14} {:>10}", "words", "sweep(µs)", "recurse(µs)", "ratio");
        println!("{}", "-".repeat(52));

        for &n in sizes {
            let a = lcg_words(0x51ed, n);
            let b = lcg_words(0x270d, n);
            let layout = memory_requirement_up_to(n);
            let warmup = 5;
            let iters = 100;

            let time = |recursive: bool| {
                let mut best = f64::MAX;
                for _ in 0..warmup {
                    let mut allocation = MemoryAllocation::new(layout);
                    let mut t = Buffer::allocate(n + 3);
                    t.push_zeros(n + 3);
                    if recursive {
                        let _ = mul_high_into(&mut t, &a, &b, &mut allocation.memory());
                    } else {
                        let _ = mul_high_basecase(&mut t, &a, &b, n);
                    }
                }
                for _ in 0..iters {
                    let mut allocation = MemoryAllocation::new(layout);
                    let mut t = Buffer::allocate(n + 3);
                    t.push_zeros(n + 3);
                    let start = Instant::now();
                    if recursive {
                        let _ = mul_high_into(&mut t, &a, &b, &mut allocation.memory());
                    } else {
                        let _ = mul_high_basecase(&mut t, &a, &b, n);
                    }
                    best = best.min(start.elapsed().as_secs_f64() * 1e6);
                }
                best
            };

            let t_sweep = time(false);
            let t_recurse = time(true);
            println!(
                "{:>8} {:>14.2} {:>14.2} {:>9.2}x",
                n,
                t_sweep,
                t_recurse,
                t_recurse / t_sweep
            );
        }
    }
}
