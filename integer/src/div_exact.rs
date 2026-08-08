//! Exact division — the quotient of `self / other`, or `None` when the division is not exact.
//!
//! The [`DivExact`] / [`DivExactAssign`] traits (from `dashu-base`) compute `self / other` as
//! `Some(q)` when `other | self`, `None` otherwise. For a single-word divisor `UBig` uses **Hensel
//! (2-adic) division** ([`hensel_div_odd_in_place`]), which replaces the normalization + reciprocal
//! setup of the general division with a low-to-high loop of multiplies and subtracts: each quotient
//! limb is `(word − carry) · d^{-1} mod 2^WORD_BITS` for the precomputed modular inverse `d^{-1}`.
//! The consuming [`UBig::div_exact_word`] runs it in place on the dividend's own buffer, so no
//! quotient scratch or re-allocation is needed. Multi-word divisors fall back to the general
//! division and check the remainder.

use dashu_base::{DivExact, DivExactAssign, DivRem, Sign, UnsignedAbs};

use crate::{
    arch::word::DoubleWord,
    div,
    ibig::IBig,
    math::inv_mod_pow2,
    primitive::{extend_word, shrink_dword, WORD_BITS},
    repr::{Repr, TypedReprRef},
    shift,
    ubig::UBig,
    Word,
};

impl UBig {
    /// Divide by a single word exactly: `Some(self / d)` when `d | self`, else `None`.
    ///
    /// Consumes `self`: the Hensel (2-adic) division ([`hensel_div_odd_in_place`]) writes the
    /// quotient directly into `self`'s buffer — no scratch allocation — and the divisor's
    /// power-of-two part is handled by an exact in-place shift. The exactness of the odd part comes
    /// from the Hensel top-carry test.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashu_int::UBig;
    ///
    /// let a = UBig::from(10u32).pow(8) * 7u32; // 700000000
    /// assert_eq!(a.div_exact_word(10), Some(UBig::from(70000000u32)));
    /// assert_eq!(UBig::from(7u8).div_exact_word(3), None);
    /// ```
    pub fn div_exact_word(self, d: Word) -> Option<UBig> {
        let mut this = self;
        if this.div_exact_assign_word(d) {
            Some(this)
        } else {
            None
        }
    }

    /// In-place exact division by a single word: `self` becomes `self / d` when `d | self`, and is
    /// left unchanged otherwise. Returns whether the division was exact.
    ///
    /// A read-only divisibility probe runs before the in-place Hensel division, so a failed division
    /// (including a zero divisor) leaves `self` untouched, and the in-place division is guaranteed to
    /// succeed afterwards — no scratch or re-allocation.
    fn div_exact_assign_word(&mut self, d: Word) -> bool {
        if d == 0 {
            return false; // 0 is not a divisor
        }
        if self.is_zero() || d == 1 {
            return true; // 0 / d = 0, self / 1 = self
        }
        let trailing = d.trailing_zeros();
        let d_odd = d >> trailing;
        if d_odd == 1 {
            // d is a power of two: exact iff the 2-valuation supplies enough twos.
            if self.trailing_zeros().unwrap() >= trailing as usize {
                *self >>= trailing as usize;
                return true;
            }
            return false;
        }
        if trailing > 0 && self.trailing_zeros().unwrap() < trailing as usize {
            return false;
        }
        // Read-only divisibility probe: a failed probe returns `false` with `self` untouched, and it
        // guarantees the in-place Hensel below succeeds (so `self` is never left corrupted).
        if !self.is_multiple_of_word(d_odd) {
            return false;
        }
        let di = inv_mod_pow2(extend_word(d_odd), WORD_BITS) as Word;
        let mut buffer = core::mem::take(self).0.into_buffer();
        let ok = hensel_div_odd_in_place(&mut buffer, d_odd, di);
        debug_assert!(ok, "div_exact_assign_word: the probe passed, so the division must be exact");
        if trailing > 0 {
            shift::shr_in_place(&mut buffer, trailing);
        }
        self.0 = Repr::from_buffer(buffer);
        true
    }

    /// Is `self` a multiple of the single word `d`? Uses the read-only Hensel divisibility test
    /// ([`hensel_is_multiple_of`]) for the odd part — the same top-carry test the exact-division
    /// kernel uses — rather than computing a full remainder.
    pub(crate) fn is_multiple_of_word(&self, d: Word) -> bool {
        assert!(d != 0, "division by zero");
        if self.is_zero() || d == 1 {
            return true; // 0 and every number are multiples of 1
        }
        let trailing = d.trailing_zeros();
        let d_odd = d >> trailing;
        if d_odd == 1 {
            // d is a power of two: exact iff the 2-valuation supplies enough twos.
            return self.trailing_zeros().unwrap() >= trailing as usize;
        }
        if trailing > 0 && self.trailing_zeros().unwrap() < trailing as usize {
            return false;
        }
        let di = inv_mod_pow2(extend_word(d_odd), WORD_BITS) as Word;
        hensel_is_multiple_of(self.as_words(), d_odd, di)
    }
}

/// A `const`-capable divisibility test for a `DoubleWord` divisor (the backend of
/// [`UBig::is_multiple_of_const`] / [`IBig::is_multiple_of_const`]).
#[rustversion::since(1.64)]
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

impl UBig {
    /// Determine whether the integer is perfectly divisible by the divisor.
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
        // A single-word divisor uses the read-only Hensel divisibility test (multiply-based, no
        // remainder computation); larger divisors fall back to the remainder.
        if let TypedReprRef::RefSmall(dword) = divisor.repr() {
            if let Some(word) = shrink_dword(dword) {
                return self.is_multiple_of_word(word);
            }
        }
        (self % divisor).is_zero()
    }

    /// A const version of [UBig::is_multiple_of], but only accepts [DoubleWord][crate::DoubleWord]
    /// divisors.
    ///
    /// # Availability
    ///
    /// Since Rust 1.64
    #[rustversion::since(1.64)]
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
        (self % divisor).is_zero()
    }

    /// A const version of [IBig::is_multiple_of], but only accepts [DoubleWord][crate::DoubleWord]
    /// divisors.
    ///
    /// # Availability
    ///
    /// Since Rust 1.64
    #[rustversion::since(1.64)]
    #[inline]
    pub const fn is_multiple_of_const(&self, divisor: DoubleWord) -> bool {
        let (_, repr) = self.as_sign_repr();
        repr.is_multiple_of_dword(divisor)
    }
}

/// Trait-based exact division: `DivExact` / `DivExactAssign` from `dashu-base`.
///
/// The `UBig` divisor dispatches to the single-word Hensel path when it fits in a word; the
/// primitive `u8..u128`/`usize` divisors forward to it after a width check (a value that overflows
/// `Word` falls back to the `UBig` divisor path).
impl DivExact<UBig> for UBig {
    type Output = UBig;

    #[inline]
    fn div_exact(self, rhs: UBig) -> Option<UBig> {
        if let TypedReprRef::RefSmall(dword) = rhs.repr() {
            if let Some(word) = shrink_dword(dword) {
                return self.div_exact_word(word);
            }
        }
        let (q, r) = self.div_rem(&rhs);
        if r.is_zero() {
            Some(q)
        } else {
            None
        }
    }
}

impl DivExact<UBig> for &UBig {
    type Output = UBig;

    #[inline]
    fn div_exact(self, rhs: UBig) -> Option<UBig> {
        self.clone().div_exact(rhs)
    }
}

impl DivExactAssign<UBig> for UBig {
    #[inline]
    fn div_exact_assign(&mut self, rhs: UBig) -> bool {
        if let TypedReprRef::RefSmall(dword) = rhs.repr() {
            if let Some(word) = shrink_dword(dword) {
                return self.div_exact_assign_word(word);
            }
        }
        let (q, r) = (&*self).div_rem(&rhs);
        if r.is_zero() {
            *self = q;
            true
        } else {
            false
        }
    }
}

macro_rules! impl_div_exact_ubig_with_prim {
    ($($T:ty)*) => {$(
        impl DivExact<$T> for UBig {
            type Output = UBig;
            #[inline]
            fn div_exact(self, rhs: $T) -> Option<UBig> {
                match Word::try_from(rhs) {
                    Ok(word) => self.div_exact_word(word),
                    Err(_) => DivExact::<UBig>::div_exact(self, UBig::from(rhs)),
                }
            }
        }
        impl DivExactAssign<$T> for UBig {
            #[inline]
            fn div_exact_assign(&mut self, rhs: $T) -> bool {
                match Word::try_from(rhs) {
                    Ok(word) => self.div_exact_assign_word(word),
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
impl DivExact<IBig> for IBig {
    type Output = IBig;

    fn div_exact(self, rhs: IBig) -> Option<IBig> {
        let (sign_self, mag_self) = self.into_parts();
        let (sign_rhs, mag_rhs) = rhs.into_parts();
        let q_mag = mag_self.div_exact(mag_rhs)?;
        Some(IBig::from_parts(sign_self * sign_rhs, q_mag))
    }
}

impl DivExactAssign<IBig> for IBig {
    fn div_exact_assign(&mut self, rhs: IBig) -> bool {
        if let Some(q) = self.clone().div_exact(rhs) {
            *self = q;
            true
        } else {
            false
        }
    }
}

impl DivExact<IBig> for &IBig {
    type Output = IBig;

    #[inline]
    fn div_exact(self, rhs: IBig) -> Option<IBig> {
        self.clone().div_exact(rhs)
    }
}

macro_rules! impl_div_exact_ibig_with_prim {
    ($($T:ty)*) => {$(
        impl DivExact<$T> for IBig {
            type Output = IBig;
            #[inline]
            fn div_exact(self, rhs: $T) -> Option<IBig> {
                let sign = self.sign();
                let q_mag = self.unsigned_abs().div_exact(rhs)?;
                Some(IBig::from_parts(sign, q_mag))
            }
        }
        impl DivExactAssign<$T> for IBig {
            #[inline]
            fn div_exact_assign(&mut self, rhs: $T) -> bool {
                if let Some(q) = self.clone().div_exact(rhs) {
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
        impl DivExact<$T> for IBig {
            type Output = IBig;
            #[inline]
            fn div_exact(self, rhs: $T) -> Option<IBig> {
                let sign = if (self.sign() == Sign::Negative) != (rhs < 0) {
                    Sign::Negative
                } else {
                    Sign::Positive
                };
                let q_mag = self.unsigned_abs().div_exact(rhs.unsigned_abs())?;
                Some(IBig::from_parts(sign, q_mag))
            }
        }
        impl DivExactAssign<$T> for IBig {
            #[inline]
            fn div_exact_assign(&mut self, rhs: $T) -> bool {
                if let Some(q) = self.clone().div_exact(rhs) {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// `div_exact_word` must agree with the general division: `Some(q)` when `d | n` (here
    /// `n = d^i·rest` with `i ≥ 1`), `None` otherwise.
    #[test]
    fn test_div_exact_word_matches_div() {
        for d in [2u16, 3, 5, 7, 10, 12, 16, 25, 255, 1001] {
            let d = d as Word;
            for i in 1..10usize {
                for rest in [1u8, 5, 7, 11] {
                    let n = UBig::from(d).pow(i) * rest;
                    let want = &n / UBig::from_word(d);
                    assert_eq!(n.div_exact_word(d), Some(want), "d={d} i={i} rest={rest}");
                }
            }
            // a value not divisible by d (and not a multiple of its prime factors) → None
            let n = UBig::from(d).pow(2) + 1u8;
            assert_eq!(n.div_exact_word(d), None, "d={d}");
        }
    }

    /// `div_exact` must agree with `div_rem` for both single- and multi-word divisors, and return
    /// `None` for non-divisible cases.
    #[test]
    fn test_div_exact_matches_div() {
        let big = UBig::from(10u8).pow(50); // multi-word divisor
        for (a, b) in [
            (UBig::from(10u8).pow(80) * 7u8, UBig::from(10u8).pow(80)),
            (big.clone() * UBig::from(13u8), big.clone()),
            (UBig::from(2u8).pow(300) * 3u8, UBig::from(8u8)),
        ] {
            let (q, r) = (&a).div_rem(&b);
            assert_eq!(a.div_exact(b), if r.is_zero() { Some(q) } else { None });
        }
        // not divisible → None (single- and multi-word divisors)
        assert_eq!(UBig::from(7u8).div_exact_word(3), None);
        assert_eq!(UBig::from(7u8).div_exact(UBig::from(3u8)), None);
        assert_eq!(UBig::from(7u8).div_exact(big), None);
    }

    /// The `DivExact`/`DivExactAssign` trait impls: primitive divisors (including one wider than
    /// `Word`, which falls back to the `UBig` divisor path) and the in-place assign form.
    #[test]
    fn test_div_exact_trait_impls() {
        use dashu_base::{DivExact, DivExactAssign};

        // UBig ÷ UBig
        let a = UBig::from(10u8).pow(8) * 7u8;
        assert_eq!(a.clone().div_exact(UBig::from(10u8).pow(8)), Some(UBig::from(7u8)));
        assert_eq!(a.div_exact(UBig::from(3u8)), None);

        // UBig ÷ primitives — any width, including one that overflows Word (u128 on 64-bit Word)
        assert_eq!(UBig::from(10u8).pow(8).div_exact(10u8), Some(UBig::from(10u8).pow(7)));
        assert_eq!(UBig::from(10u8).pow(8).div_exact(10u32), Some(UBig::from(10u8).pow(7)));
        assert_eq!(UBig::from(10u8).pow(8).div_exact(10u128), Some(UBig::from(10u8).pow(7)));
        let wide = 1u128 << 100; // > Word::MAX on any current platform
        assert_eq!(UBig::from(10u8).pow(8).div_exact(wide), None);
        assert_eq!(UBig::from(wide).div_exact(1u128), Some(UBig::from(wide)));

        // DivExactAssign with a primitive (in place)
        let mut b = UBig::from(10u8).pow(8) * 7u8;
        assert!(b.div_exact_assign(10u8));
        assert_eq!(b, UBig::from(10u8).pow(7) * 7u8);
        assert!(!b.div_exact_assign(3u8)); // not divisible → unchanged
        assert_eq!(b, UBig::from(10u8).pow(7) * 7u8);

        // reference receiver keeps the dividend borrowable
        let ref_a = UBig::from(10u8).pow(8) * 7u8;
        assert_eq!((&ref_a).div_exact(UBig::from(10u8).pow(8)), Some(UBig::from(7u8)));
        assert_eq!((&ref_a).div_exact(UBig::from(3u8)), None);
        assert_eq!(ref_a, UBig::from(10u8).pow(8) * 7u8); // unchanged

        // DivExactAssign with a UBig divisor
        let mut c = UBig::from(10u8).pow(8) * 7u8;
        assert!(c.div_exact_assign(UBig::from(10u8).pow(8)));
        assert_eq!(c, UBig::from(7u8));
        assert!(!c.div_exact_assign(UBig::from(3u8)));
        assert_eq!(c, UBig::from(7u8));
    }

    /// The `DivExact`/`DivExactAssign` trait impls for `IBig`: sign-aware exact division, primitive
    /// divisors (unsigned and signed), and the in-place form.
    #[test]
    fn test_div_exact_ibig() {
        use dashu_base::{DivExact, DivExactAssign};

        // IBig ÷ IBig
        let a = IBig::from(10u8).pow(8) * 7u8;
        assert_eq!(a.clone().div_exact(IBig::from(10u8).pow(8)), Some(IBig::from(7u8)));
        assert_eq!(a.div_exact(IBig::from(3u8)), None);
        // signs: quotient sign is the product of the operands' signs
        assert_eq!(IBig::from(-14i32).div_exact(IBig::from(7i32)), Some(IBig::from(-2i32)));
        assert_eq!(IBig::from(14i32).div_exact(IBig::from(-7i32)), Some(IBig::from(-2i32)));
        assert_eq!(IBig::from(-14i32).div_exact(IBig::from(-7i32)), Some(IBig::from(2i32)));

        // reference receiver keeps the dividend borrowable
        let ref_a = IBig::from(10u8).pow(8) * 7u8;
        assert_eq!((&ref_a).div_exact(IBig::from(10u8).pow(8)), Some(IBig::from(7u8)));
        assert_eq!((&ref_a).div_exact(IBig::from(3u8)), None);
        assert_eq!(ref_a, IBig::from(10u8).pow(8) * 7u8); // unchanged

        // primitive divisors
        assert_eq!(IBig::from(10u8).pow(8).div_exact(10u8), Some(IBig::from(10u8).pow(7)));
        assert_eq!(IBig::from(-20i32).div_exact(5i32), Some(IBig::from(-4i32)));
        assert_eq!(IBig::from(20i32).div_exact(-5i32), Some(IBig::from(-4i32)));
        assert_eq!(IBig::from(20i32).div_exact(7i32), None);

        // DivExactAssign
        let mut b = IBig::from(10u8).pow(8) * 7u8;
        assert!(b.div_exact_assign(IBig::from(10u8).pow(8)));
        assert_eq!(b, IBig::from(7u8));
        assert!(!b.div_exact_assign(3u8)); // unchanged on failure
        assert_eq!(b, IBig::from(7u8));
        assert!(b.div_exact_assign(-7i32));
        assert_eq!(b, IBig::from(-1i32));
    }

    /// `is_multiple_of` on a single-word divisor (via the read-only Hensel test) must agree with the
    /// remainder check, for odd, even, and power-of-two divisors.
    #[test]
    fn test_is_multiple_of_word_matches_rem() {
        for d in [2u16, 3, 5, 7, 10, 12, 16, 25, 255] {
            let d = d as Word;
            for i in 1..10usize {
                for rest in [1u8, 5, 7, 11] {
                    let n = UBig::from(d).pow(i) * rest;
                    let want = (&n % UBig::from_word(d)).is_zero();
                    assert_eq!(n.is_multiple_of_word(d), want, "d={d} i={i} rest={rest}");
                    // public is_multiple_of dispatches through the same word test
                    assert_eq!(
                        n.is_multiple_of(&UBig::from_word(d)),
                        want,
                        "d={d} i={i} rest={rest}"
                    );
                }
            }
        }
    }
}
