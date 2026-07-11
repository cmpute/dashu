//! Differential property tests for the `_dword` fast paths.
//!
//! Per `AGENTS.md`, double-word operations are a "first-class citizen" in this
//! crate. The internal `_dword` / `_large_dword` code paths are taken whenever a
//! `RefLarge` (≥3-word) `UBig` meets a `RefSmall` (dword-fitting) `UBig` — see the
//! dispatch in `add_ops::repr`. These tests drive those paths by pairing a Large
//! operand with a Small one (including the boundary values `0`, `1`, `Word::MAX`,
//! `DoubleWord::MAX`) and checking mathematical laws that must hold regardless of
//! the chosen path.
//!
//! Portable across `Word` = u16/u32/u64 (the `force_bits` CI matrix): no widths or
//! integer literals are hardcoded — everything goes through `Word`/`DoubleWord`.

use dashu_int::ops::Gcd;
use dashu_int::{DoubleWord, UBig, Word};
use proptest::prelude::*;

/// A `DoubleWord` built from two random words (portable across Word widths).
fn dword() -> impl Strategy<Value = DoubleWord> {
    (any::<Word>(), any::<Word>()).prop_map(|(hi, lo)| {
        let lo = lo as DoubleWord;
        let hi = (hi as DoubleWord) << Word::BITS;
        hi | lo
    })
}

/// A `UBig` whose magnitude spans ≥3 words (so it is `RefLarge`), forcing the
/// `_large_dword` paths when combined with a Small operand.
fn large_ubig() -> impl Strategy<Value = UBig> {
    prop::collection::vec(any::<Word>(), 3..8).prop_map(|mut words| {
        // Keep the most-significant word nonzero so normalization can't shrink the
        // magnitude back below 3 words.
        if words.last() == Some(&Word::MIN) {
            *words.last_mut().unwrap() = Word::MAX;
        }
        UBig::from_words(&words)
    })
}

/// The canonical Small (dword-fitting) `UBig`, plus boundary values that stress the
/// word/double-word seam.
fn small_ubig() -> impl Strategy<Value = UBig> {
    prop_oneof![
        dword().prop_map(UBig::from_dword),
        Just(UBig::ZERO),
        Just(UBig::ONE),
        Just(UBig::from_word(Word::MAX)),
        Just(UBig::from_dword(DoubleWord::MAX)),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, ..Default::default() })]

    /// (a + s) - a == s,  (a + s) - s == a,  and commutativity across the two paths.
    #[test]
    fn add_sub_dword_roundtrip(a in large_ubig(), s in small_ubig()) {
        let sum = a.clone() + s.clone();
        prop_assert_eq!(sum.clone() - a.clone(), s.clone());
        prop_assert_eq!(sum.clone() - s.clone(), a.clone());
        prop_assert_eq!(a.clone() + s.clone(), s.clone() + a.clone());
    }

    /// (a * s) / s == a  and  (a * s) % s == 0   (s != 0).
    #[test]
    fn mul_div_dword_roundtrip(
        a in large_ubig(),
        s in small_ubig().prop_filter("nonzero", |s| !s.is_zero())
    ) {
        let prod = a.clone() * s.clone();
        prop_assert_eq!(prod.clone() / s.clone(), a.clone());
        prop_assert_eq!(prod.clone() % s.clone(), UBig::ZERO);
        prop_assert_eq!(a.clone() * s.clone(), s.clone() * a.clone());
    }

    /// (a / s) * s + (a % s) == a,  with  (a % s) < s   (s != 0).
    #[test]
    fn div_rem_dword_identity(
        a in large_ubig(),
        s in small_ubig().prop_filter("nonzero", |s| !s.is_zero())
    ) {
        let q = a.clone() / s.clone();
        let r = a.clone() % s.clone();
        prop_assert_eq!(q.clone() * s.clone() + r.clone(), a.clone());
        prop_assert!(r < s.clone());
    }

    /// gcd(a, s) divides both operands; Euclid step gcd(a, s) == gcd(s, a % s).
    #[test]
    fn gcd_dword_properties(
        a in large_ubig(),
        s in small_ubig().prop_filter("nonzero", |s| !s.is_zero())
    ) {
        let g = (&a).gcd(&s);
        prop_assert_eq!(a.clone() % g.clone(), UBig::ZERO);
        prop_assert_eq!(s.clone() % g.clone(), UBig::ZERO);
        prop_assert_eq!((&a).gcd(&s), (&s).gcd(&(a.clone() % s.clone())));
    }

    /// Bitwise `_dword` paths: xor involution, and/or absorption, commutativity.
    #[test]
    fn bitwise_dword_laws(a in large_ubig(), s in small_ubig()) {
        prop_assert_eq!(a.clone() ^ s.clone() ^ s.clone(), a.clone());      // xor involution
        prop_assert_eq!((a.clone() | s.clone()) & a.clone(), a.clone());    // a ⊆ (a | s)
        prop_assert_eq!((a.clone() & s.clone()) | a.clone(), a.clone());    // (a & s) ⊆ a
        prop_assert_eq!(a.clone() | s.clone(), s.clone() | a.clone());
        prop_assert_eq!(a.clone() & s.clone(), s.clone() & a.clone());
    }

    /// Shifting a Small value matches multiplication/division by 2^n.
    #[test]
    fn shift_dword_matches_mul(s in small_ubig(), n in 0usize..(4 * Word::BITS as usize)) {
        let pow2 = UBig::ONE << n;
        prop_assert_eq!(s.clone() << n, s.clone() * pow2.clone());
        prop_assert_eq!(s.clone() >> n, s.clone() / pow2.clone());
    }
}
