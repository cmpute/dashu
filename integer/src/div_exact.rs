//! Exact division — the quotient of `self / other`, or `None` when the division is not exact.
//!
//! The [`DivExact`] / [`DivExactAssign`] traits (re-exported through `dashu-base` from
//! `num-modular`, with the empty precomputation `()`) compute `self / other` as `Some(q)` when
//! `other | self`, `None` otherwise.
//!
//! Exact division uses **Hensel (2-adic) division**: the modular inverse of the (odd part of the)
//! divisor is precomputed by Newton iteration, and each quotient limb is then `(word − carry) ·
//! d^{-1} mod 2^WORD_BITS` — a low-to-high loop of multiplies and subtracts with no normalization
//! and no reciprocal precomputation, which makes it roughly twice as fast as a general division.
//! The quotient is written in place into the dividend's own buffer, so no scratch is allocated.
//! The dividend's own buffer is used for the quotient, so `div_exact` consumes its dividend (the
//! assigning forms take a read-only divisibility probe — or a backup clone — to leave the dividend
//! untouched on failure).
//!
//! Three divisor widths are supported by dedicated kernels:
//!
//! - a **single word** ([`hensel_div_odd_in_place`]): each step subtracts one `q · d` product;
//! - a **double word** ([`hensel_div_odd_dword_in_place`]): each step subtracts `q · d` over a
//!   two-word window via the double-word multiply kernel;
//! - **multi word** ([`hensel_div_exact_large`]): each step subtracts `q · D` over a `D`-length
//!   window.
//!
//! Divisors are stripped of their factors of 2 first (the quotient is then shifted back), so the
//! kernels only ever see an odd divisor. The divisibility test ([`UBig::is_multiple_of`]) reuses
//! the same kernels: a read-only probe for single-word divisors, and the exactness test of the
//! division itself (on a scratch copy) for wider divisors.
//!
//! The multi-word kernel is schoolbook (O(n·m)), so for divisors beyond
//! [`THRESHOLD_DIV_EXACT_DEFAULT`] words — where the general division's sub-quadratic
//! divide-and-conquer algorithm is faster — exact division falls back to the general division plus
//! a remainder check.

use dashu_base::{DivRem, Sign, UnsignedAbs};
use num_modular::{DivExact, DivExactAssign};

use crate::{
    add,
    arch::word::{DoubleWord, Word},
    ibig::IBig,
    math::inv_mod_pow2,
    mul::{sub_mul_dword_same_len_in_place, sub_mul_word_same_len_in_place},
    primitive::{extend_word, shrink_dword, WORD_BITS},
    repr::{TypedRepr, TypedReprRef},
    ubig::UBig,
};

/// If the divisor length (in words) exceeds this, exact division falls back to the general
/// division — the schoolbook Hensel loop (O(n·m)) loses to the sub-quadratic divide-and-conquer
/// division at that size. The crossover is a heuristic (it depends on the size ratio as well as
/// the absolute divisor size); 180 words matches the observed crossover for balanced operands.
const THRESHOLD_DIV_EXACT_DEFAULT: usize = 180;

/// Environment-variable override for the exact-division threshold.
///
/// When the `tuning` feature is active the user may set `DASHU_THRESHOLD_DIV_EXACT` to override
/// the compile-time default.
mod threshold {
    #[inline]
    pub fn div_exact() -> usize {
        #[cfg(feature = "tuning")]
        {
            if let Ok(s) = std::env::var("DASHU_THRESHOLD_DIV_EXACT") {
                if let Ok(v) = s.parse::<usize>() {
                    return v;
                }
            }
        }
        super::THRESHOLD_DIV_EXACT_DEFAULT
    }
}

impl UBig {
    /// In-place exact division by a fixed `DoubleWord` divisor: `self` becomes `self / divisor`
    /// when `divisor | self`, and is left unchanged otherwise. Returns whether the division was
    /// exact.
    ///
    /// The in-place backend of [`DivExactAssign`] for a `DoubleWord` divisor (mirroring
    /// [`UBig::is_multiple_of_const`], the `const` divisor form). A single-word divisor is probed
    /// by the read-only Hensel test first (so a failure leaves `self` untouched) and then divided
    /// in place; a double-word divisor backs up `self` with an `O(len)` clone (its probe is as
    /// expensive as the division itself).
    fn div_exact_assign_dword(&mut self, divisor: DoubleWord) -> bool {
        if divisor == 0 {
            return false; // 0 is not a divisor
        }
        if self.is_zero() || divisor == 1 {
            return true; // 0 / d = 0, self / 1 = self
        }
        if shrink_dword(divisor).is_some() {
            // A single-word divisor: probe first (cheap, read-only), so the in-place division
            // below is guaranteed to succeed and `self` can be consumed without a backup.
            if !self.repr().is_multiple_of(TypedReprRef::RefSmall(divisor)) {
                return false;
            }
            let taken = core::mem::take(self);
            let q = taken
                .into_repr()
                .div_exact(TypedRepr::Small(divisor), &())
                .expect("the probe passed, so the division is exact");
            *self = UBig(q);
            return true;
        }
        // A double-word divisor: back up `self` (an O(len) clone) so a failed division can
        // restore it.
        let backup = self.clone();
        let taken = core::mem::take(self);
        match taken.into_repr().div_exact(TypedRepr::Small(divisor), &()) {
            Some(q) => {
                *self = UBig(q);
                true
            }
            None => {
                *self = backup;
                false
            }
        }
    }
}

/// Ops for `TypedRepr` / `TypedReprRef` — the four ownership combinations, mirroring
/// [`div_ops`](crate::div_ops), together with the `Buffer`-level helpers they dispatch to (the
/// Hensel kernels live at the top level).
pub(crate) mod repr {
    use super::*;
    use crate::{
        arch::word::{DoubleWord, Word},
        buffer::Buffer,
        div,
        math::inv_mod_pow2,
        primitive::{extend_word, shrink_dword, split_dword, WORD_BITS, WORD_BITS_USIZE},
        repr::{Repr, TypedRepr, TypedReprRef},
        shift,
        ubig::UBig,
    };

    impl DivExact<TypedRepr, ()> for TypedRepr {
        type Output = Repr;

        #[inline]
        fn div_exact(self, rhs: TypedRepr, _: &()) -> Option<Repr> {
            match (self, rhs) {
                (TypedRepr::Small(dword0), TypedRepr::Small(dword1)) => {
                    div_exact_dword(dword0, dword1)
                }
                (TypedRepr::Small(_), TypedRepr::Large(_)) => None, // small < large, cannot divide
                (TypedRepr::Large(buffer0), TypedRepr::Small(dword1)) => {
                    if let Some(word) = shrink_dword(dword1) {
                        div_exact_large_word(buffer0, word)
                    } else {
                        div_exact_large_dword(buffer0, dword1)
                    }
                }
                (TypedRepr::Large(buffer0), TypedRepr::Large(buffer1)) => {
                    div_exact_large(buffer0, buffer1)
                }
            }
        }
    }

    impl<'l> DivExact<TypedRepr, ()> for TypedReprRef<'l> {
        type Output = Repr;

        #[inline]
        fn div_exact(self, rhs: TypedRepr, _: &()) -> Option<Repr> {
            match (self, rhs) {
                (TypedReprRef::RefSmall(dword0), TypedRepr::Small(dword1)) => {
                    div_exact_dword(dword0, dword1)
                }
                (TypedReprRef::RefSmall(_), TypedRepr::Large(_)) => None,
                (TypedReprRef::RefLarge(words0), TypedRepr::Small(dword1)) => {
                    if let Some(word) = shrink_dword(dword1) {
                        div_exact_large_word(words0.into(), word)
                    } else {
                        div_exact_large_dword(words0.into(), dword1)
                    }
                }
                (TypedReprRef::RefLarge(words0), TypedRepr::Large(buffer1)) => {
                    div_exact_large(words0.into(), buffer1)
                }
            }
        }
    }

    impl<'r> DivExact<TypedReprRef<'r>, ()> for TypedRepr {
        type Output = Repr;

        #[inline]
        fn div_exact(self, rhs: TypedReprRef, _: &()) -> Option<Repr> {
            match (self, rhs) {
                (TypedRepr::Small(dword0), TypedReprRef::RefSmall(dword1)) => {
                    div_exact_dword(dword0, dword1)
                }
                (TypedRepr::Small(_), TypedReprRef::RefLarge(_)) => None,
                (TypedRepr::Large(buffer0), TypedReprRef::RefSmall(dword1)) => {
                    if let Some(word) = shrink_dword(dword1) {
                        div_exact_large_word(buffer0, word)
                    } else {
                        div_exact_large_dword(buffer0, dword1)
                    }
                }
                (TypedRepr::Large(buffer0), TypedReprRef::RefLarge(words1)) => {
                    div_exact_large(buffer0, words1.into())
                }
            }
        }
    }

    impl<'l, 'r> DivExact<TypedReprRef<'r>, ()> for TypedReprRef<'l> {
        type Output = Repr;

        #[inline]
        fn div_exact(self, rhs: TypedReprRef, _: &()) -> Option<Repr> {
            match (self, rhs) {
                (TypedReprRef::RefSmall(dword0), TypedReprRef::RefSmall(dword1)) => {
                    div_exact_dword(dword0, dword1)
                }
                (TypedReprRef::RefSmall(_), TypedReprRef::RefLarge(_)) => None,
                (TypedReprRef::RefLarge(words0), TypedReprRef::RefSmall(dword1)) => {
                    if let Some(word) = shrink_dword(dword1) {
                        div_exact_large_word(words0.into(), word)
                    } else {
                        div_exact_large_dword(words0.into(), dword1)
                    }
                }
                (TypedReprRef::RefLarge(words0), TypedReprRef::RefLarge(words1)) => {
                    div_exact_large(words0.into(), words1.into())
                }
            }
        }
    }

    /// Both operands fit in a `DoubleWord`: the division is trivial.
    #[inline]
    fn div_exact_dword(lhs: DoubleWord, rhs: DoubleWord) -> Option<Repr> {
        if rhs == 0 {
            None
        } else if rhs == 1 {
            Some(Repr::from_dword(lhs))
        } else if lhs % rhs == 0 {
            Some(Repr::from_dword(lhs / rhs))
        } else {
            None
        }
    }

    /// In-place exact division of the `Buffer` by a single word.
    ///
    /// The dividend buffer is consumed and replaced by the quotient on success (the caller owns the
    /// buffer, so a failed division simply drops the modified buffer). The divisor's power-of-two part
    /// is stripped by the 2-valuation, and the odd part is divided out by the Hensel kernel.
    fn div_exact_large_word(mut buffer: Buffer, d: Word) -> Option<Repr> {
        if d == 0 {
            return None; // 0 is not a divisor
        }
        if d == 1 {
            return Some(Repr::from_buffer(buffer));
        }
        let trailing = d.trailing_zeros();
        let d_odd = d >> trailing;
        if d_odd == 1 {
            // d is a power of two: exact iff the 2-valuation supplies enough twos.
            if trailing_zeros(&buffer) >= trailing as usize {
                shift::shr_in_place(&mut buffer, trailing);
                return Some(Repr::from_buffer(buffer));
            }
            return None;
        }
        if trailing > 0 && trailing_zeros(&buffer) < trailing as usize {
            return None;
        }
        let di = inv_mod_pow2(extend_word(d_odd), WORD_BITS) as Word;
        if !hensel_div_odd_in_place(&mut buffer, d_odd, di) {
            return None;
        }
        if trailing > 0 {
            shift::shr_in_place(&mut buffer, trailing);
        }
        Some(Repr::from_buffer(buffer))
    }

    /// In-place exact division of the `Buffer` by a double word.
    ///
    /// Like [`div_exact_large_word`], but for a divisor that needs two words: the odd part is divided
    /// by the double-word Hensel kernel ([`hensel_div_odd_dword_in_place`]), or by the single-word
    /// kernel when the odd part fits in a word (a divisor with a large power-of-two part).
    fn div_exact_large_dword(mut buffer: Buffer, d: DoubleWord) -> Option<Repr> {
        debug_assert!(shrink_dword(d).is_none()); // the caller dispatches on the width
        let trailing = d.trailing_zeros();
        let d_odd = d >> trailing;
        if d_odd == 1 {
            // d is a power of two: exact iff the 2-valuation supplies enough twos.
            if trailing_zeros(&buffer) >= trailing as usize {
                shr_erase_front(&mut buffer, trailing as usize);
                return Some(Repr::from_buffer(buffer));
            }
            return None;
        }
        if trailing > 0 && trailing_zeros(&buffer) < trailing as usize {
            return None;
        }
        if let Some(word) = shrink_dword(d_odd) {
            let di = inv_mod_pow2(extend_word(word), WORD_BITS) as Word;
            if !hensel_div_odd_in_place(&mut buffer, word, di) {
                return None;
            }
        } else {
            let (d_lo, d_hi) = split_dword(d_odd);
            let di = inv_mod_pow2(extend_word(d_lo), WORD_BITS) as Word;
            if !hensel_div_odd_dword_in_place(&mut buffer, d_lo, d_hi, di) {
                return None;
            }
        }
        if trailing > 0 {
            shr_erase_front(&mut buffer, trailing as usize);
        }
        Some(Repr::from_buffer(buffer))
    }

    /// In-place exact division of the `Buffer` by a multi-word divisor.
    ///
    /// For a divisor within [`THRESHOLD_DIV_EXACT_DEFAULT`] words, the common factors of 2 are
    /// stripped first (both buffers are shifted and trimmed), so the Hensel kernel sees an odd divisor;
    /// the quotient needs no post-shift because the dividend was shifted before the division. A divisor
    /// that collapses to one or two words after the strip is handed to the matching narrower kernel.
    /// For a larger divisor the schoolbook Hensel loop (O(n·m)) loses to the general division — which
    /// switches to a sub-quadratic divide-and-conquer algorithm at large sizes — so the general
    /// division is used instead.
    fn div_exact_large(mut dividend: Buffer, mut divisor: Buffer) -> Option<Repr> {
        if dividend.len() < divisor.len() {
            return None; // dividend is smaller than the divisor
        }
        if divisor.len() > super::threshold::div_exact() {
            // General division + remainder check; same result, faster for large divisors.
            let (q, r) =
                UBig(Repr::from_buffer(dividend)).div_rem(UBig(Repr::from_buffer(divisor)));
            return if r.is_zero() { Some(q.0) } else { None };
        }
        let s = trailing_zeros(&divisor);
        if s > 0 {
            if trailing_zeros(&dividend) < s {
                return None; // not enough factors of 2 in the dividend
            }
            shr_erase_front(&mut dividend, s);
            shr_erase_front(&mut divisor, s);
            divisor.pop_zeros();
            dividend.pop_zeros();
        }
        if dividend.len() < divisor.len() {
            return None; // dividend is smaller than the divisor
        }
        match divisor.len() {
            1 => {
                // A divisor like 2^s·3 with a large power-of-two part collapses to one word.
                let d = divisor[0];
                debug_assert!(d & 1 == 1, "the common factors of 2 were already stripped");
                let di = inv_mod_pow2(extend_word(d), WORD_BITS) as Word;
                if !hensel_div_odd_in_place(&mut dividend, d, di) {
                    return None;
                }
            }
            2 => {
                let (d_lo, d_hi) = (divisor[0], divisor[1]);
                debug_assert!(d_lo & 1 == 1, "the common factors of 2 were already stripped");
                let di = inv_mod_pow2(extend_word(d_lo), WORD_BITS) as Word;
                if !hensel_div_odd_dword_in_place(&mut dividend, d_lo, d_hi, di) {
                    return None;
                }
            }
            _ => {
                if !hensel_div_exact_large(&mut dividend, &divisor) {
                    return None;
                }
            }
        }
        Some(Repr::from_buffer(dividend))
    }

    impl TypedReprRef<'_> {
        /// Determine whether `self` is a multiple of `rhs` (non-const; the const counterpart is
        /// [`TypedReprRef::is_multiple_of_dword`]).
        ///
        /// A single-word divisor uses the read-only Hensel divisibility test (multiply-based, no
        /// remainder computation); wider divisors reuse the exactness test of the Hensel division
        /// itself on a scratch copy.
        pub(crate) fn is_multiple_of(&self, rhs: TypedReprRef) -> bool {
            match (self, rhs) {
                (TypedReprRef::RefSmall(dword0), TypedReprRef::RefSmall(dword1)) => {
                    dword1 != 0 && dword0 % dword1 == 0
                }
                (TypedReprRef::RefSmall(_), TypedReprRef::RefLarge(_)) => false,
                (TypedReprRef::RefLarge(words0), TypedReprRef::RefSmall(dword1)) => {
                    is_multiple_of_dword(words0, dword1)
                }
                (TypedReprRef::RefLarge(words0), TypedReprRef::RefLarge(words1)) => {
                    is_multiple_of_large(words0, words1)
                }
            }
        }
    }

    /// Is `words` a multiple of the single word `d`?
    fn is_multiple_of_word(words: &[Word], d: Word) -> bool {
        if d == 0 {
            return false; // 0 is not a divisor
        }
        let trailing = d.trailing_zeros();
        let d_odd = d >> trailing;
        if d_odd == 1 {
            // d is a power of two: exact iff the 2-valuation supplies enough twos.
            return trailing_zeros(words) >= trailing as usize;
        }
        if trailing > 0 && trailing_zeros(words) < trailing as usize {
            return false;
        }
        let di = inv_mod_pow2(extend_word(d_odd), WORD_BITS) as Word;
        hensel_is_multiple_of(words, d_odd, di)
    }

    /// Is `words` a multiple of the double word `d`? A divisor that fits in a word is delegated to
    /// [`is_multiple_of_word`]; a full double word runs the read-only divisibility test via the
    /// exactness check of the double-word Hensel division on a scratch copy.
    fn is_multiple_of_dword(words: &[Word], d: DoubleWord) -> bool {
        if d == 0 {
            return false; // 0 is not a divisor
        }
        if let Some(word) = shrink_dword(d) {
            return is_multiple_of_word(words, word);
        }
        let trailing = d.trailing_zeros();
        let d_odd = d >> trailing;
        if d_odd == 1 {
            return trailing_zeros(words) >= trailing as usize;
        }
        if trailing > 0 && trailing_zeros(words) < trailing as usize {
            return false;
        }
        let mut buffer = words.to_vec();
        if let Some(word) = shrink_dword(d_odd) {
            // The odd part fits in a word (e.g. 5·2^70): the divisor is effectively a single word,
            // so the single-word kernel is required (the double-word kernel would produce a quotient
            // that is one word short).
            let di = inv_mod_pow2(extend_word(word), WORD_BITS) as Word;
            hensel_div_odd_in_place(&mut buffer, word, di)
        } else {
            let (d_lo, d_hi) = split_dword(d_odd);
            let di = inv_mod_pow2(extend_word(d_lo), WORD_BITS) as Word;
            hensel_div_odd_dword_in_place(&mut buffer, d_lo, d_hi, di)
        }
    }

    /// Is `words` a multiple of the multi-word `divisor`?
    ///
    /// The exactness test of the Hensel division on a scratch copy IS the divisibility test; sharing
    /// [`div_exact_large`] also shares the common-factor stripping.
    fn is_multiple_of_large(words: &[Word], divisor: &[Word]) -> bool {
        div_exact_large(Buffer::from(words), Buffer::from(divisor)).is_some()
    }

    /// The number of trailing zero bits of the value stored in `words` (little-endian). Returns
    /// `usize::MAX` for the all-zero value (every number divides it).
    fn trailing_zeros(words: &[Word]) -> usize {
        for (i, &w) in words.iter().enumerate() {
            if w != 0 {
                return i * WORD_BITS_USIZE + w.trailing_zeros() as usize;
            }
        }
        usize::MAX
    }

    /// Right-shift the buffer by `shift` bits in place, erasing the whole words that fall out.
    ///
    /// [`shift::shr_in_place`] only handles shifts within one word, so larger shifts (a divisor with
    /// whole words of trailing zeros) first erase the low words, then shift the remainder.
    fn shr_erase_front(buffer: &mut Buffer, shift: usize) {
        buffer.erase_front(shift / WORD_BITS_USIZE);
        if shift % WORD_BITS_USIZE != 0 {
            shift::shr_in_place(buffer, (shift % WORD_BITS_USIZE) as u32);
        }
    }

    /// A `const`-capable divisibility test for a `DoubleWord` divisor (the backend of
    /// [`UBig::is_multiple_of_const`] / [`IBig::is_multiple_of_const`]). This is the remainder-based
    /// test, kept separate from the non-const Hensel-based [`TypedReprRef::is_multiple_of`] because
    /// `const fn`s cannot allocate or call non-const kernels.
    impl<'a> TypedReprRef<'a> {
        pub(crate) const fn is_multiple_of_dword(self, divisor: DoubleWord) -> bool {
            if let Some(w) = shrink_dword(divisor) {
                match self {
                    TypedReprRef::RefSmall(dword) => dword % extend_word(w) == 0,
                    TypedReprRef::RefLarge(words) => div::rem_by_word(words, w) == 0,
                }
            } else {
                match self {
                    TypedReprRef::RefSmall(dword) => dword % divisor == 0,
                    TypedReprRef::RefLarge(words) => div::rem_by_dword(words, divisor) == 0,
                }
            }
        }
    }
}

/// Hensel (2-adic) division of `words` by the odd single-word `d` **in place**, using the
/// precomputed inverse `di = d^{-1} mod 2^WORD_BITS`. Returns whether the division is exact; on
/// success `words` holds the exact quotient.
///
/// Each quotient limb is `(words[i] − carry) · di mod 2^W`, computed low-to-high with only
/// multiplies and subtracts — no normalization, no division. The computation is naturally in place:
/// each input limb is read before its output limb is written. The exactness test comes from the top
/// carry: the computation maintains `u = q·d + T·2^(W·n)` with `T = c + high(q[n-1]·d) ≥ 0`, so
/// `T = 0` — i.e. `d | u`, with `q` the exact quotient — iff `c == 0` and the final high product is
/// zero.
pub(crate) fn hensel_div_odd_in_place(words: &mut [Word], d: Word, di: Word) -> bool {
    let mut c: Word = 0;
    let mut q_last = words[0].wrapping_mul(di);
    words[0] = q_last;
    for word in words.iter_mut().skip(1) {
        let h = ((extend_word(q_last) * extend_word(d)) >> WORD_BITS) as Word;
        c = c.wrapping_add(h);
        let (l, borrow) = word.overflowing_sub(c);
        c = borrow as Word;
        q_last = l.wrapping_mul(di);
        *word = q_last;
    }
    let h = ((extend_word(q_last) * extend_word(d)) >> WORD_BITS) as Word;
    c == 0 && h == 0
}

/// Hensel (2-adic) divisibility test: does the odd single-word `d` divide `words`?
///
/// The read-only version of [`hensel_div_odd_in_place`] — the same low-to-high loop of multiplies
/// and subtracts (each input limb read before any write), but the quotient limbs are not written
/// out. The top-carry test is identical: `d | words` iff `c == 0` and the final high product is
/// zero.
pub(crate) fn hensel_is_multiple_of(words: &[Word], d: Word, di: Word) -> bool {
    let mut c: Word = 0;
    let mut q_last = words[0].wrapping_mul(di);
    for word in words.iter().skip(1) {
        let h = ((extend_word(q_last) * extend_word(d)) >> WORD_BITS) as Word;
        c = c.wrapping_add(h);
        let (l, borrow) = word.overflowing_sub(c);
        c = borrow as Word;
        q_last = l.wrapping_mul(di);
    }
    let h = ((extend_word(q_last) * extend_word(d)) >> WORD_BITS) as Word;
    c == 0 && h == 0
}

/// Hensel (2-adic) division of `words` by the odd double-word `d = d_lo + d_hi·B` **in place**,
/// using the precomputed inverse `di = d^{-1} mod 2^WORD_BITS` of the low word. Returns whether the
/// division is exact; on success `words[..n-1]` holds the exact quotient and `words[n-1]` is zero.
///
/// Each step subtracts `q · d` over a two-word window via the double-word multiply kernel; the
/// quotient limb is still a single word. The exactness test is the same as
/// [`hensel_div_odd_in_place`]: the high part (the last word) must be zero, with no outstanding
/// borrow.
pub(crate) fn hensel_div_odd_dword_in_place(
    words: &mut [Word],
    d_lo: Word,
    d_hi: Word,
    di: Word,
) -> bool {
    let n = words.len();
    debug_assert!(n >= 2 && d_lo & 1 == 1);
    for i in 0..n - 1 {
        let q = words[i].wrapping_mul(di);
        // Subtract q·d from the 2-word window; the product q·d has at most 3 words, so the total
        // borrow is at most one word (carry_hi == 0), spilled into the words beyond the window.
        let (borrow_lo, borrow_hi) =
            sub_mul_dword_same_len_in_place(&mut words[i..i + 2], &[d_lo, d_hi], q, 0);
        debug_assert!(borrow_hi == 0, "the total borrow of q·d is at most one word");
        if borrow_lo != 0 && (i + 2 >= n || add::sub_word_in_place(&mut words[i + 2..], borrow_lo))
        {
            return false; // the borrow ran off the end: the division is not exact
        }
        words[i] = q;
    }
    words[n - 1] == 0
}

/// Hensel (2-adic) exact division of `dividend` by the odd multi-word `divisor`, **in place**.
/// Returns whether the division is exact; on success `dividend[..qn]` holds the exact quotient
/// (`qn = dividend.len() - divisor.len() + 1`) and `dividend[qn..]` is zero.
///
/// The generalisation of [`hensel_div_odd_in_place`] to a multi-word divisor: each step subtracts
/// `q · divisor` over a `divisor`-length window with [`sub_mul_word_same_len_in_place`], and the
/// (at most one-word) borrow is propagated through the remaining words. An outstanding borrow at the
/// end means the dividend underflowed — the division cannot be exact. For exact division the
/// corrections cancel against `dividend[qn..]` exactly, so that suffix tests zero.
pub(crate) fn hensel_div_exact_large(dividend: &mut [Word], divisor: &[Word]) -> bool {
    let n = dividend.len();
    let m = divisor.len();
    debug_assert!(n >= m && m >= 2 && divisor[0] & 1 == 1);
    let qn = n - m + 1;
    let di = inv_mod_pow2(extend_word(divisor[0]), WORD_BITS) as Word;

    for i in 0..qn {
        let q = dividend[i].wrapping_mul(di);
        // Subtract q·divisor from the window, then propagate the borrow (at most one word, since
        // q·divisor < B^(m+1)) through the remaining words. For exact division this borrow is
        // absorbed by the high words; if it runs off the end, the dividend underflowed.
        let mut borrow = sub_mul_word_same_len_in_place(&mut dividend[i..i + m], q, divisor);
        if borrow != 0 {
            for w in dividend[i + m..].iter_mut() {
                let (l, b) = w.overflowing_sub(borrow);
                *w = l;
                borrow = b as Word;
                if borrow == 0 {
                    break;
                }
            }
        }
        if borrow != 0 {
            return false; // the borrow ran off the end: the division is not exact
        }
        dividend[i] = q;
    }
    dividend[qn..].iter().all(|&w| w == 0)
}

impl UBig {
    /// Determine whether the integer is perfectly divisible by the divisor.
    ///
    /// A divisor that fits in a single word uses the read-only Hensel divisibility test; wider
    /// divisors reuse the exactness test of the Hensel division.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::UBig;
    /// let a = UBig::from(24u8);
    /// let b = UBig::from(6u8);
    /// assert!(a.is_multiple_of(&b));
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the divisor is zero.
    #[inline]
    pub fn is_multiple_of(&self, divisor: &Self) -> bool {
        assert!(!divisor.is_zero(), "division by zero");
        self.repr().is_multiple_of(divisor.repr())
    }

    /// A const version of [UBig::is_multiple_of], but only accepts [DoubleWord][crate::DoubleWord]
    /// divisors.
    ///
    #[inline]
    pub const fn is_multiple_of_const(&self, divisor: DoubleWord) -> bool {
        self.repr().is_multiple_of_dword(divisor)
    }
}

impl IBig {
    /// Determine whether the integer is perfectly divisible by the divisor.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::IBig;
    /// let a = IBig::from(24);
    /// let b = IBig::from(-6);
    /// assert!(a.is_multiple_of(&b));
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the divisor is zero.
    #[inline]
    pub fn is_multiple_of(&self, divisor: &Self) -> bool {
        self.unsigned_abs().is_multiple_of(&divisor.unsigned_abs())
    }

    /// A const version of [IBig::is_multiple_of], but only accepts [DoubleWord][crate::DoubleWord]
    /// divisors.
    ///
    #[inline]
    pub const fn is_multiple_of_const(&self, divisor: DoubleWord) -> bool {
        let (_, repr) = self.as_sign_repr();
        repr.is_multiple_of_dword(divisor)
    }
}

/// Trait-based exact division: the [`DivExact`] / [`DivExactAssign`] traits, re-exported through
/// `dashu-base` from `num-modular` with the empty precomputation `()` (call sites pass `&()`).
///
/// The `UBig` divisor delegates to the `TypedRepr`-level `DivExact` impls (single-word Hensel,
/// double-word Hensel, and multi-word Hensel). A primitive `u8..u128`/`usize` divisor that fits in
/// a `DoubleWord` is divided in place by the same kernels; a wider one falls back to the `UBig`
/// divisor path.
impl DivExact<UBig, ()> for UBig {
    type Output = UBig;

    #[inline]
    fn div_exact(self, rhs: UBig, _: &()) -> Option<UBig> {
        self.into_repr().div_exact(rhs.into_repr(), &()).map(UBig)
    }
}

impl DivExact<UBig, ()> for &UBig {
    type Output = UBig;

    #[inline]
    fn div_exact(self, rhs: UBig, _: &()) -> Option<UBig> {
        self.clone().div_exact(rhs, &())
    }
}

impl DivExactAssign<UBig, ()> for UBig {
    #[inline]
    fn div_exact_assign(&mut self, rhs: UBig, _: &()) -> bool {
        if let TypedReprRef::RefSmall(dword) = rhs.repr() {
            return self.div_exact_assign_dword(dword);
        }
        // A multi-word divisor: back up `self` (an O(len) clone) so a failed division can restore
        // it, then divide the taken buffer in place.
        let backup = self.clone();
        let taken = core::mem::take(self);
        match taken.into_repr().div_exact(rhs.into_repr(), &()) {
            Some(q) => {
                *self = UBig(q);
                true
            }
            None => {
                *self = backup;
                false
            }
        }
    }
}

macro_rules! impl_div_exact_ubig_with_prim {
    ($($T:ty)*) => {$(
        impl DivExact<$T, ()> for UBig {
            type Output = UBig;
            #[inline]
            fn div_exact(self, rhs: $T, _: &()) -> Option<UBig> {
                match DoubleWord::try_from(rhs) {
                    Ok(dword) => self.into_repr().div_exact(TypedRepr::Small(dword), &()).map(UBig),
                    Err(_) => DivExact::<UBig, ()>::div_exact(self, UBig::from(rhs), &()),
                }
            }
        }
        impl DivExactAssign<$T, ()> for UBig {
            #[inline]
            fn div_exact_assign(&mut self, rhs: $T, _: &()) -> bool {
                match DoubleWord::try_from(rhs) {
                    Ok(dword) => self.div_exact_assign_dword(dword),
                    Err(_) => {
                        let (q, r) = (&*self).div_rem(&UBig::from(rhs));
                        if r.is_zero() {
                            *self = q;
                            true
                        } else {
                            false
                        }
                    }
                }
            }
        }
    )*};
}
impl_div_exact_ubig_with_prim!(u8 u16 u32 u64 u128 usize);

/// `DivExact` / `DivExactAssign` for `IBig`: sign-aware exact division. The magnitudes are divided
/// by the `UBig` implementations, and the sign of the quotient is the product of the operands'
/// signs. The primitive divisor impls (unsigned and signed) divide the magnitudes and attach the
/// sign.
impl DivExact<IBig, ()> for IBig {
    type Output = IBig;

    fn div_exact(self, rhs: IBig, _: &()) -> Option<IBig> {
        let (sign_self, mag_self) = self.into_parts();
        let (sign_rhs, mag_rhs) = rhs.into_parts();
        let q_mag = mag_self.div_exact(mag_rhs, &())?;
        Some(IBig::from_parts(sign_self * sign_rhs, q_mag))
    }
}

impl DivExactAssign<IBig, ()> for IBig {
    fn div_exact_assign(&mut self, rhs: IBig, _: &()) -> bool {
        if let Some(q) = self.clone().div_exact(rhs, &()) {
            *self = q;
            true
        } else {
            false
        }
    }
}

impl DivExact<IBig, ()> for &IBig {
    type Output = IBig;

    #[inline]
    fn div_exact(self, rhs: IBig, _: &()) -> Option<IBig> {
        self.clone().div_exact(rhs, &())
    }
}

macro_rules! impl_div_exact_ibig_with_prim {
    ($($T:ty)*) => {$(
        impl DivExact<$T, ()> for IBig {
            type Output = IBig;
            #[inline]
            fn div_exact(self, rhs: $T, _: &()) -> Option<IBig> {
                let sign = self.sign();
                let q_mag = self.unsigned_abs().div_exact(rhs, &())?;
                Some(IBig::from_parts(sign, q_mag))
            }
        }
        impl DivExactAssign<$T, ()> for IBig {
            #[inline]
            fn div_exact_assign(&mut self, rhs: $T, _: &()) -> bool {
                if let Some(q) = self.clone().div_exact(rhs, &()) {
                    *self = q;
                    true
                } else {
                    false
                }
            }
        }
    )*};
}
impl_div_exact_ibig_with_prim!(u8 u16 u32 u64 u128 usize);

macro_rules! impl_div_exact_ibig_with_signed_prim {
    ($($T:ty)*) => {$(
        impl DivExact<$T, ()> for IBig {
            type Output = IBig;
            #[inline]
            fn div_exact(self, rhs: $T, _: &()) -> Option<IBig> {
                let sign = if (self.sign() == Sign::Negative) != (rhs < 0) {
                    Sign::Negative
                } else {
                    Sign::Positive
                };
                let q_mag = self.unsigned_abs().div_exact(rhs.unsigned_abs(), &())?;
                Some(IBig::from_parts(sign, q_mag))
            }
        }
        impl DivExactAssign<$T, ()> for IBig {
            #[inline]
            fn div_exact_assign(&mut self, rhs: $T, _: &()) -> bool {
                if let Some(q) = self.clone().div_exact(rhs, &()) {
                    *self = q;
                    true
                } else {
                    false
                }
            }
        }
    )*};
}
impl_div_exact_ibig_with_signed_prim!(i8 i16 i32 i64 i128 isize);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        arch::word::Word,
        primitive::{extend_word, WORD_BITS_USIZE},
    };

    /// `div_exact_assign` with a single-word divisor must agree with the general division: exact
    /// (with the quotient in `n`) when `d | n` (here `n = d^i·rest` with `i ≥ 1`), leaving `n`
    /// unchanged otherwise.
    #[test]
    fn test_div_exact_assign_matches_div() {
        use dashu_base::DivExactAssign;

        for d in [2u16, 3, 5, 7, 10, 12, 16, 25, 255, 1001] {
            let d = d as Word;
            for i in 1..10usize {
                for rest in [1u8, 5, 7, 11] {
                    let n = UBig::from(d).pow(i) * rest;
                    let want = &n / UBig::from_word(d);
                    let mut got = n;
                    assert!(got.div_exact_assign(extend_word(d), &()), "d={d} i={i} rest={rest}");
                    assert_eq!(got, want, "d={d} i={i} rest={rest}");
                }
            }
            // a value not divisible by d (and not a multiple of its prime factors) stays unchanged
            let mut n = UBig::from(d).pow(2) + 1u8;
            let before = n.clone();
            assert!(!n.div_exact_assign(extend_word(d), &()), "d={d}");
            assert_eq!(n, before, "d={d}");
        }
    }

    /// `div_exact` must agree with `div_rem` for single-word, double-word, and multi-word divisors
    /// (odd and even), and return `None` for non-divisible cases.
    #[test]
    fn test_div_exact_matches_div() {
        // single- and double-word divisors
        for d in [
            UBig::from(10u8).pow(8), // single word on 64-bit
            (UBig::ONE << 64) + 3u8, // double word on 64-bit (odd)
            (UBig::ONE << 70) * 5u8, // double word on 64-bit (even)
        ] {
            for i in 1..6usize {
                let n = d.clone().pow(i) * 7u8;
                let (q, r) = (&n).div_rem(&d);
                assert!(r.is_zero(), "d={d:?} i={i}");
                assert_eq!(n.clone().div_exact(d.clone(), &()), Some(q), "d={d:?} i={i}");
            }
            let n = d.clone().pow(2) + 1u8;
            assert_eq!(n.div_exact(d, &()), None, "d must not divide d^2+1");
        }

        // multi-word divisors
        let big = UBig::from(10u8).pow(50);
        for (a, b) in [
            (UBig::from(10u8).pow(80) * 7u8, UBig::from(10u8).pow(80)),
            (big.clone() * UBig::from(13u8), big.clone()),
            (UBig::from(2u8).pow(300) * 3u8, UBig::from(8u8)),
        ] {
            let (q, r) = (&a).div_rem(&b);
            assert_eq!(a.div_exact(b, &()), if r.is_zero() { Some(q) } else { None });
        }
        // not divisible → None (single- and multi-word divisors)
        assert_eq!(UBig::from(7u8).div_exact(3u8, &()), None);
        assert_eq!(UBig::from(7u8).div_exact(UBig::from(3u8), &()), None);
        assert_eq!(UBig::from(7u8).div_exact(big, &()), None);
    }

    /// The multi-word Hensel kernel must agree with `div_rem` on a sweep of odd divisors (the
    /// kernel only sees odd divisors; even ones are stripped by [`repr::div_exact_large`]).
    #[test]
    fn test_hensel_div_exact_large_matches_div() {
        for d_bits in [70usize, 100, 150, 300] {
            let d = (UBig::ONE << d_bits) + 1u8;
            let dw = d.as_words().to_vec();
            for i in 1..5usize {
                let n = d.clone().pow(i) * 12345u16;
                let want = &n / &d;
                let mut buf = n.as_words().to_vec();
                assert!(hensel_div_exact_large(&mut buf, &dw), "d={d_bits} i={i}");
                assert_eq!(UBig::from_words(&buf), want, "d={d_bits} i={i}");
            }
            // not divisible → false
            let n = d.clone().pow(2) + 2u8;
            let mut buf = n.as_words().to_vec();
            assert!(!hensel_div_exact_large(&mut buf, &dw), "d={d_bits}");
        }

        // even multi-word divisors exercise the 2-split inside `div_exact_large`
        for d_bits in [70usize, 130, 300] {
            let odd = (UBig::ONE << d_bits) + 5u8;
            let d = &odd * UBig::from(16u8);
            let n = d.clone().pow(3) * 77u8;
            let (q, r) = (&n).div_rem(&d);
            assert!(r.is_zero());
            assert_eq!(n.clone().div_exact(d.clone(), &()), Some(q));
            // a value whose odd part divides but whose 2-valuation is too low
            assert_eq!((odd * 7u8).div_exact(d, &()), None);
        }
    }

    /// `div_exact_assign` with a double-word divisor. The divisors are built relative to the word
    /// size so they need two words on every platform: `Word::MAX²` is just below
    /// `DoubleWord::MAX`, so it (and its neighbours) always span two words.
    #[test]
    fn test_div_exact_assign_dword() {
        use dashu_base::DivExactAssign;

        let base = extend_word(Word::MAX) * extend_word(Word::MAX); // Word::MAX² (odd)
        for d in [
            base + 2,                                       // odd double word
            base + 3,                                       // even double word
            (1 as DoubleWord) << (2 * WORD_BITS_USIZE - 1), // power of two
        ] {
            let d_ubig = UBig::from_dword(d);
            for i in 1..6usize {
                let n = d_ubig.clone().pow(i) * 7u8;
                let want = &n / &d_ubig;
                let mut got = n;
                assert!(got.div_exact_assign(d, &()), "d={d:?} i={i}");
                assert_eq!(got, want, "d={d:?} i={i}");
            }
            let mut n = d_ubig.clone().pow(2) + 1u8;
            let before = n.clone();
            assert!(!n.div_exact_assign(d, &()), "d={d:?}");
            assert_eq!(n, before, "d={d:?}");
        }
    }

    /// The `DivExact`/`DivExactAssign` trait impls: primitive divisors (including one wider than
    /// `Word`, which falls back to the `UBig` divisor path) and the in-place assign form.
    #[test]
    fn test_div_exact_trait_impls() {
        use dashu_base::{DivExact, DivExactAssign};

        // UBig ÷ UBig
        let a = UBig::from(10u8).pow(8) * 7u8;
        assert_eq!(a.clone().div_exact(UBig::from(10u8).pow(8), &()), Some(UBig::from(7u8)));
        assert_eq!(a.div_exact(UBig::from(3u8), &()), None);

        // UBig ÷ primitives — any width, including one that overflows Word (u128 on 64-bit Word)
        assert_eq!(UBig::from(10u8).pow(8).div_exact(10u8, &()), Some(UBig::from(10u8).pow(7)));
        assert_eq!(UBig::from(10u8).pow(8).div_exact(10u32, &()), Some(UBig::from(10u8).pow(7)));
        assert_eq!(UBig::from(10u8).pow(8).div_exact(10u128, &()), Some(UBig::from(10u8).pow(7)));
        let wide = 1u128 << 100; // > Word::MAX on any current platform
        assert_eq!(UBig::from(10u8).pow(8).div_exact(wide, &()), None);
        assert_eq!(UBig::from(wide).div_exact(1u128, &()), Some(UBig::from(wide)));

        // DivExactAssign with a primitive (in place)
        let mut b = UBig::from(10u8).pow(8) * 7u8;
        assert!(b.div_exact_assign(10u8, &()));
        assert_eq!(b, UBig::from(10u8).pow(7) * 7u8);
        assert!(!b.div_exact_assign(3u8, &())); // not divisible → unchanged
        assert_eq!(b, UBig::from(10u8).pow(7) * 7u8);

        // DivExactAssign with a multi-word divisor, unchanged on failure
        let d = UBig::from(10u8).pow(50);
        let mut c = d.clone().pow(2) * 7u8;
        assert!(c.div_exact_assign(d.clone(), &()));
        assert_eq!(c, &d * 7u8);
        let mut c = d.clone().pow(2) * 7u8;
        let before = c.clone();
        assert!(!c.div_exact_assign(d.clone() + 1u8, &()));
        assert_eq!(c, before);

        // reference receiver keeps the dividend borrowable
        let ref_a = UBig::from(10u8).pow(8) * 7u8;
        assert_eq!((&ref_a).div_exact(UBig::from(10u8).pow(8), &()), Some(UBig::from(7u8)));
        assert_eq!((&ref_a).div_exact(UBig::from(3u8), &()), None);
        assert_eq!(ref_a, UBig::from(10u8).pow(8) * 7u8); // unchanged
    }

    /// The `DivExact`/`DivExactAssign` trait impls for `IBig`: sign-aware exact division, primitive
    /// divisors (unsigned and signed), and the in-place form.
    #[test]
    fn test_div_exact_ibig() {
        use dashu_base::{DivExact, DivExactAssign};

        // IBig ÷ IBig
        let a = IBig::from(10u8).pow(8) * 7u8;
        assert_eq!(a.clone().div_exact(IBig::from(10u8).pow(8), &()), Some(IBig::from(7u8)));
        assert_eq!(a.div_exact(IBig::from(3u8), &()), None);
        // signs: quotient sign is the product of the operands' signs
        assert_eq!(IBig::from(-14i32).div_exact(IBig::from(7i32), &()), Some(IBig::from(-2i32)));
        assert_eq!(IBig::from(14i32).div_exact(IBig::from(-7i32), &()), Some(IBig::from(-2i32)));
        assert_eq!(IBig::from(-14i32).div_exact(IBig::from(-7i32), &()), Some(IBig::from(2i32)));

        // reference receiver keeps the dividend borrowable
        let ref_a = IBig::from(10u8).pow(8) * 7u8;
        assert_eq!((&ref_a).div_exact(IBig::from(10u8).pow(8), &()), Some(IBig::from(7u8)));
        assert_eq!((&ref_a).div_exact(IBig::from(3u8), &()), None);
        assert_eq!(ref_a, IBig::from(10u8).pow(8) * 7u8); // unchanged

        // primitive divisors
        assert_eq!(IBig::from(10u8).pow(8).div_exact(10u8, &()), Some(IBig::from(10u8).pow(7)));
        assert_eq!(IBig::from(-20i32).div_exact(5i32, &()), Some(IBig::from(-4i32)));
        assert_eq!(IBig::from(20i32).div_exact(-5i32, &()), Some(IBig::from(-4i32)));
        assert_eq!(IBig::from(20i32).div_exact(7i32, &()), None);

        // DivExactAssign
        let mut b = IBig::from(10u8).pow(8) * 7u8;
        assert!(b.div_exact_assign(IBig::from(10u8).pow(8), &()));
        assert_eq!(b, IBig::from(7u8));
        assert!(!b.div_exact_assign(3u8, &())); // unchanged on failure
        assert_eq!(b, IBig::from(7u8));
        assert!(b.div_exact_assign(-7i32, &()));
        assert_eq!(b, IBig::from(-1i32));
    }

    /// `is_multiple_of` must agree with the remainder check for single-word, double-word, and
    /// multi-word divisors (odd, even, and power-of-two).
    #[test]
    fn test_is_multiple_of_matches_rem() {
        // single-word divisors (via the read-only Hensel test)
        for d in [2u16, 3, 5, 7, 10, 12, 16, 25, 255] {
            let d = d as Word;
            for i in 1..10usize {
                for rest in [1u8, 5, 7, 11] {
                    let n = UBig::from(d).pow(i) * rest;
                    let want = (&n % UBig::from_word(d)).is_zero();
                    assert_eq!(
                        n.is_multiple_of(&UBig::from_word(d)),
                        want,
                        "d={d} i={i} rest={rest}"
                    );
                }
            }
        }

        // double-word divisors
        for d in [
            (UBig::ONE << 64) + 3u8,
            (UBig::ONE << 70) * 5u8,
            UBig::ONE << 100,
        ] {
            for i in 1..8usize {
                let n = d.clone().pow(i) * 7u8;
                let want = (&n % &d).is_zero();
                assert_eq!(n.is_multiple_of(&d), want, "d={d:?} i={i}");
            }
            let n = d.clone().pow(2) + 1u8;
            assert!(!n.is_multiple_of(&d));
        }

        // multi-word divisors
        let d = (UBig::ONE << 200) + 1u8;
        for i in 1..6usize {
            let n = d.clone().pow(i) * 11u8;
            let want = (&n % &d).is_zero();
            assert_eq!(n.is_multiple_of(&d), want, "i={i}");
        }
        assert!(!(d.clone().pow(2) + 2u8).is_multiple_of(&d));
    }

    /// The `const` divisibility test agrees with the remainder for both word and dword divisors.
    #[test]
    fn test_is_multiple_of_const_matches_rem() {
        for (n, d) in [
            (UBig::from(24u8), 6u8),
            (UBig::from(24u8), 7u8),
            (UBig::from(10u8).pow(8), 10u8),
            (UBig::from(10u8).pow(8), 3u8),
        ] {
            assert_eq!(
                n.is_multiple_of_const(d as DoubleWord),
                (&n % UBig::from_word(d as Word)).is_zero()
            );
        }
    }
}
