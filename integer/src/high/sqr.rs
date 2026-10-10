//! High-part squaring kernel.

#[cfg(test)]
use crate::primitive::WORD_BITS_USIZE;
use crate::{
    add,
    arch::word::Word,
    buffer::Buffer,
    helper_macros::debug_assert_zero,
    memory::{self, Memory, MemoryAllocation},
    primitive::locate_top_word_plus_one,
    repr::TypedReprRef::{RefLarge, RefSmall},
    sqr,
    ubig::UBig,
};
use alloc::alloc::Layout;
use static_assertions::const_assert;

/// If the window length is at or below this, the multiplication column sweep is
/// used directly (a dedicated symmetric base case is a future tuning item).
///
/// Tuned together with the multiplication kernel's threshold: the recursion
/// only pays off once its exact high block reaches the Karatsuba band.
const THRESHOLD_SIMPLE_DEFAULT: usize = 96;
const_assert!(THRESHOLD_SIMPLE_DEFAULT >= 8);

/// Environment-variable override for the base-case threshold.
///
/// When the `tuning` feature is active the user may set
/// `DASHU_THRESHOLD_SQRHIGH_SIMPLE` to override the compile-time default.
mod threshold {
    #[inline]
    pub fn simple() -> usize {
        #[cfg(feature = "tuning")]
        {
            if let Ok(s) = std::env::var("DASHU_THRESHOLD_SQRHIGH_SIMPLE") {
                if let Ok(v) = s.parse() {
                    return v;
                }
            }
        }
        super::THRESHOLD_SIMPLE_DEFAULT
    }
}

/// Temporary memory required by [`sqr_high`] for a window of `n` words.
pub(crate) fn memory_requirement_up_to(n: usize) -> Layout {
    // Same layout as the multiplication kernel, except the exact high block is
    // squared (sqr scratch) and the two cross products collapse into one
    // doubled cross (multiplication scratch at the smaller size).
    if n <= threshold::simple() {
        memory::zero_layout()
    } else {
        let l = n / 4;
        let k = n - l;
        let block = memory::add_layout(
            memory::array_layout::<Word>(2 * k),
            sqr::memory_requirement_exact(k),
        );
        let cross = memory::add_layout(
            memory::array_layout::<Word>(l + 3),
            super::mul::memory_requirement_up_to(l),
        );
        memory::max_layout(block, cross)
    }
}

/// Accumulate the high window of `ap * ap` (`n` words) into `t` (`n + 3` words
/// holding the product columns `n-2 ..= 2n`). Returns whether any contribution
/// below the window (columns `< n`) is nonzero. See [`super::mul`] for the
/// shared layout and error story.
pub(super) fn sqr_high_into(t: &mut [Word], ap: &[Word], memory: &mut Memory) -> bool {
    let n = ap.len();
    debug_assert!(t.len() == n + 3);

    if n <= threshold::simple() {
        return super::mul::mul_high_basecase(t, ap, ap, n);
    }

    // Square split with a large high part: l = n/4, k = n - l. Then
    //   * the exact block ap[l..]^2 covers columns 2l..2n-1,
    //   * the cross term 2 * ap * ap[..l] reaches the window only through the
    //     high l words of ap[k..] * ap[..l] (columns n..n+l-1); the middle part
    //     ap[l..k] * ap[..l] spans columns l..n-1, entirely below the window,
    //   * the low block ap[..l]^2 spans columns 0..2l-2, below column n-2.
    let l = n / 4;
    let k = n - l;
    debug_assert!(l >= 1 && 2 * l + 2 <= n);

    let mut sticky = false;

    {
        // Exact high block at columns 2l..2n-1.
        let (block, mut block_memory) = memory.allocate_slice_fill::<Word>(2 * k, 0);
        sqr::sqr(block, &ap[l..], &mut block_memory);
        let off = n - 2 - 2 * l;
        let overflow = add::add_in_place(&mut t[..n + 2], &block[off..]);
        debug_assert_zero!(overflow);
        sticky |= block[..off].iter().any(|&w| w != 0);
    }
    {
        // The window-reaching cross term, doubled (2 * ap[k..] * ap[..l]).
        let (x, mut cross_memory) = memory.allocate_slice_fill::<Word>(l + 3, 0);
        sticky |= super::mul::mul_high_into(x, &ap[k..], &ap[..l], &mut cross_memory);
        for _ in 0..2 {
            if add::add_same_len_in_place(&mut t[..l + 3], x) {
                debug_assert_zero!(add::add_word_in_place(&mut t[l + 3..], 1));
            }
        }
    }

    // Dropped low square (and the below-window cross parts): all of them are
    // nonzero only through the low block ap[..l] — except the exact block's
    // boundary words and the carries onto the two guard columns below the
    // window, which are dropped from the returned value like any other
    // contribution below it.
    sticky |= ap[..l].iter().any(|&w| w != 0);
    sticky |= t[0] != 0;
    sticky |= t[1] != 0;
    sticky
}

/// Compute the high `out_words` words of the square `a * a`, with a certified
/// one-sided error bound. See the [module documentation](super) for the exact
/// contract.
///
/// `out_words` is clamped to `words(a) + 2` (as on [`super::mul_high`], a
/// window up to two words beyond the operand keeps it whole); a window
/// covering the entire square returns it exactly with a `false` sticky flag.
///
/// # Examples
///
/// ```
/// use dashu_int::{high, UBig, Word};
/// use core::str::FromStr;
///
/// let a = UBig::from_str_radix("fffffffffffffffffffffffffffffff1fffffffffffffffd", 16).unwrap();
/// let (v, sticky) = high::sqr_high(&a, 3);
/// let full = &a * &a;
/// let s = (2 * a.as_words().len() - 3) * Word::BITS as usize;
/// let top = &full >> s;
///
/// assert!(v <= top);
/// assert!(&top - &v <= UBig::from(5u32));
/// assert_eq!(sticky, &full != &(&top << s));
/// ```
#[must_use]
pub fn sqr_high(a: &UBig, out_words: usize) -> (UBig, bool) {
    let wa = a.repr().len();

    if wa == 0 {
        return (UBig::ZERO, false);
    }
    if out_words >= 2 * wa {
        return (a * a, false);
    }
    if out_words == 0 {
        return (UBig::ZERO, true);
    }

    // Dispatch on the representation like the other kernels: a small (inline)
    // operand squares inside a double word exactly; a large one runs on the
    // word slice.
    match a.repr() {
        RefSmall(dword) => super::mul::mul_high_dword(dword, dword, out_words),
        RefLarge(words) => sqr_high_words(words, out_words),
    }
}

/// Word-level core of [`sqr_high`]. The slice is normalized (no trailing zero
/// words, by the `Repr` invariant). The caller has already handled the zero
/// operand, the empty window and windows covering the entire square.
fn sqr_high_words(aw: &[Word], out_words: usize) -> (UBig, bool) {
    let wa = aw.len();
    debug_assert_eq!(wa, locate_top_word_plus_one(aw));
    debug_assert!(out_words >= 1 && out_words < 2 * wa);

    let n = out_words.min(wa + 2);

    let (ap, sticky, square_frame);
    if n <= wa {
        // Truncating the operand to its top n words drops a value worth less
        // than one unit of the window's least significant word.
        square_frame = true;
        ap = &aw[wa - n..];
        sticky = wa > n && aw[..wa - n].iter().any(|&w| w != 0);
    } else {
        // Extended window: the operand stays whole.
        square_frame = false;
        ap = &aw[..wa];
        sticky = false;
    }

    let mut buffer = Buffer::allocate(n + 3);
    buffer.push_zeros(n + 3);
    let sticky_core = if square_frame {
        let mut allocation = MemoryAllocation::new(memory_requirement_up_to(n));
        sqr_high_into(&mut buffer, ap, &mut allocation.memory())
    } else {
        super::mul::mul_high_basecase(&mut buffer, ap, ap, n)
    };

    // The window may carry zero high words when the square is short; UBig
    // words are little-endian, so trim from the top.
    let hi = &buffer[2..n + 2];
    let hi_len = locate_top_word_plus_one(hi);
    (UBig::from_words(&hi[..hi_len]), sticky || sticky_core)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{memory::MemoryAllocation, UBig};
    use alloc::vec;
    use alloc::vec::Vec;

    /// Fixed dense operand pattern (no generator state): the i-th word is
    /// `(i+1)·step + i`, wrapping — distinct, carry-rich words, identical on
    /// every run (and sharing their low bits across word sizes, since the
    /// arithmetic is modular). The top word is kept nonzero so the operand
    /// really spans `len` words.
    fn pattern_words(len: usize, step: u64) -> Vec<Word> {
        (0..len)
            .map(|i| {
                let w = (i as u64 + 1).wrapping_mul(step).wrapping_add(i as u64) as Word;
                if i + 1 == len && w == 0 {
                    1
                } else {
                    w
                }
            })
            .collect()
    }

    /// Two fixed dense steps with mixed bit runs (see [`pattern_words`]).
    const CASES: [u64; 2] = [0x0123_4567_89ab_cdef, 0x1357_9bdf_2468_ace1];

    /// Check the full public contract against a schoolbook full square.
    fn check_case(a: &[Word], n: usize) {
        // Apply the same clamping as the public contract.
        let n = n.min(a.len() + 2).min(2 * a.len());
        let a_u = UBig::from_words(a);
        let (v, sticky) = sqr_high(&a_u, n);
        let full = &a_u * &a_u;
        let s = WORD_BITS_USIZE * (2 * a.len() - n);
        let top = &full >> s;

        assert!(v <= top, "one-sidedness violated at n={n}");
        let diff = &top - &v;
        assert!(
            diff <= UBig::from(n as u64 + 2),
            "error bound violated: diff={diff:?} n={n} a_len={}",
            a.len()
        );
        assert_eq!(sticky, full != (&top << s), "sticky mismatch at n={n}");
    }

    #[test]
    fn test_sqr_high_basecase_range() {
        for n in 1..=THRESHOLD_SIMPLE_DEFAULT {
            for &step in &CASES {
                check_case(&pattern_words(n, step), n);
            }
        }
    }

    #[test]
    fn test_sqr_high_recursive_range() {
        for &n in &[97usize, 100, 130, 200, 300, 400] {
            for &step in &CASES {
                check_case(&pattern_words(n, step), n);
                check_case(&pattern_words(n, step), n / 2 + 1);
            }
        }
    }

    #[test]
    fn test_sqr_high_extended_windows() {
        // Windows reaching one and two words beyond the operand (the
        // equal-precision floating-point shape).
        for n in 1..=40usize {
            for &step in &CASES {
                check_case(&pattern_words(n, step), n + 1);
                check_case(&pattern_words(n, step), n + 2);
            }
        }
    }

    #[test]
    fn test_sqr_high_recursive_guard_sticky() {
        // As in the multiplication kernel's guard-sticky test: sparse
        // operands depositing the exact high block's boundary word on the
        // guard column below the window. n = 100: l = 25; the word at
        // 2l - 1 = 49 squares onto column n - 2 and every other drop site
        // stays zero, so only the guard content proves the drop.
        let mut a = vec![0 as Word; 100];
        a[49] = 1; // 2l - 1
        a[99] = 1;
        let a_u = UBig::from_words(&a);
        let (v, sticky) = sqr_high(&a_u, 100);
        let full = &a_u * &a_u;
        let s = WORD_BITS_USIZE * (2 * 100 - 100);
        assert_eq!(v, &full >> s, "window value mismatch");
        assert!(sticky, "content dropped below the window must set sticky");
    }

    #[test]
    fn test_sqr_high_edge_patterns() {
        for n in &[3usize, 40, 96, 97, 130] {
            check_case(&vec![Word::MAX; *n], *n);
        }
        let mut sparse = vec![0 as Word; 33];
        sparse[32] = 1;
        check_case(&sparse, 17);

        // Exact window with trailing zero words.
        let mut a = vec![0 as Word; 7];
        a[6] = Word::MAX;
        let (v, sticky) = sqr_high(&UBig::from_words(&a), 4);
        assert!(!sticky);
        let full = UBig::from_words(&a) * UBig::from_words(&a);
        assert_eq!(v, &full >> (WORD_BITS_USIZE * (14 - 4)));

        let small = UBig::from_words(&[1, 2, 3]);
        let (v, sticky) = sqr_high(&small, 8);
        assert_eq!(v, &small * &small);
        assert!(!sticky);

        // Small (inline) operands: the exact double-word fast path.
        for &step in &CASES {
            for len in 1..=2usize {
                let a = pattern_words(len, step);
                for n in 1..(2 * len) {
                    check_case(&a, n);
                }
                // A window covering the whole square is exact.
                let a_u = UBig::from_words(&a);
                let (v, sticky) = sqr_high(&a_u, 2 * len);
                assert_eq!(v, &a_u * &a_u);
                assert!(!sticky);
            }
        }
    }

    #[test]
    fn test_sqr_high_memory_layout_sound() {
        for &n in &[97usize, 130, 200, 400] {
            let a = pattern_words(n, 0x7e57_7e57_7e57_7e57);
            let mut buffer = Buffer::allocate(n + 3);
            buffer.push_zeros(n + 3);
            let mut allocation = MemoryAllocation::new(memory_requirement_up_to(n));
            let _ = sqr_high_into(&mut buffer, &a, &mut allocation.memory());
        }
    }
}
