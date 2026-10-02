//! High-part squaring kernel.

#[cfg(test)]
use crate::primitive::WORD_BITS_USIZE;
use crate::{
    add,
    arch::word::Word,
    buffer::Buffer,
    memory::{self, Memory, MemoryAllocation},
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
        debug_assert!(!overflow);
        sticky |= block[..off].iter().any(|&w| w != 0);
    }
    {
        // The window-reaching cross term, doubled (2 * ap[k..] * ap[..l]).
        let (x, mut cross_memory) = memory.allocate_slice_fill::<Word>(l + 3, 0);
        sticky |= super::mul::mul_high_into(x, &ap[k..], &ap[..l], &mut cross_memory);
        for _ in 0..2 {
            if add::add_same_len_in_place(&mut t[..l + 3], x) {
                debug_assert!(!add::add_word_in_place(&mut t[l + 3..], 1));
            }
        }
    }

    // Dropped low square (and the below-window cross parts): all of them are
    // nonzero only through the low block ap[..l].
    sticky |= ap[..l].iter().any(|&w| w != 0);
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
    let aw = a.as_words();
    let wa = super::mul::trim_words(aw);

    if wa == 0 {
        return (UBig::ZERO, false);
    }
    if out_words >= 2 * wa {
        return (a * a, false);
    }
    if out_words == 0 {
        return (UBig::ZERO, true);
    }
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

    let hi = &buffer[2..n + 2];
    let hi_len = super::mul::trim_words(hi);
    (UBig::from_words(&hi[..hi_len]), sticky || sticky_core)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{memory::MemoryAllocation, UBig};

    fn lcg_words(seed: u64, len: usize) -> Vec<Word> {
        let mut s = seed | 1;
        (0..len)
            .map(|i| {
                s = s
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                // Fold both halves of the u64 state so the helper works for
                // every word size.
                let w = ((s >> 32) ^ s) as Word;
                if i + 1 == len && w == 0 {
                    1
                } else {
                    w
                }
            })
            .collect()
    }

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
            for seed in 1..=2u64 {
                check_case(&lcg_words(seed * 53, n), n);
            }
        }
    }

    #[test]
    fn test_sqr_high_recursive_range() {
        for &n in &[97usize, 100, 130, 200, 300, 400] {
            for seed in 1..=2u64 {
                check_case(&lcg_words(seed * 71, n), n);
                check_case(&lcg_words(seed * 97, n), n / 2 + 1);
            }
        }
    }

    #[test]
    fn test_sqr_high_extended_windows() {
        // Windows reaching one and two words beyond the operand (the
        // equal-precision floating-point shape).
        for n in 1..=40usize {
            for seed in 1..=2u64 {
                check_case(&lcg_words(seed * 67, n), n + 1);
                check_case(&lcg_words(seed * 73, n), n + 2);
            }
        }
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
    }

    #[test]
    fn test_sqr_high_memory_layout_sound() {
        for &n in &[97usize, 130, 200, 400] {
            let a = lcg_words(0x7e57, n);
            let mut buffer = Buffer::allocate(n + 3);
            buffer.push_zeros(n + 3);
            let mut allocation = MemoryAllocation::new(memory_requirement_up_to(n));
            let _ = sqr_high_into(&mut buffer, &a, &mut allocation.memory());
        }
    }
}
