//! Comparison traits for [`CBig`].
//!
//! [`CBig`] mirrors [`dashu_float::FBig`]'s comparison surface rather than MPC's "complex has no
//! order" stance: a lexicographic total [`Ord`] (by real part, then imaginary), an [`AbsOrd`]
//! magnitude comparison via `|z|²`, and (behind `num-order`) `NumOrd`/`NumHash`.

use crate::cbig::CBig;
use crate::repr::Context;
use core::cmp::Ordering;
use dashu_base::AbsOrd;
use dashu_float::round::Round;
use dashu_float::Repr;
use dashu_int::Word;

/// Lexicographic comparison by `(re, then im)` using the value-based [`Repr`] order. This is a
/// well-defined total order (usable for `BTreeMap`/sorting), not an algebraic one.
pub(crate) fn lex_cmp<const B: Word>(
    re1: &Repr<B>,
    im1: &Repr<B>,
    re2: &Repr<B>,
    im2: &Repr<B>,
) -> Ordering {
    match re1.cmp(re2) {
        Ordering::Equal => im1.cmp(im2),
        ord => ord,
    }
}

impl<R1: Round, R2: Round, const B: Word> PartialEq<CBig<R2, B>> for CBig<R1, B> {
    /// Componentwise exact equality. `+0 == -0` per component (matching `FBig`); the context is
    /// ignored.
    #[inline]
    fn eq(&self, other: &CBig<R2, B>) -> bool {
        self.re == other.re && self.im == other.im
    }
}
impl<R: Round, const B: Word> Eq for CBig<R, B> {}

impl<R1: Round, R2: Round, const B: Word> PartialOrd<CBig<R2, B>> for CBig<R1, B> {
    #[inline]
    fn partial_cmp(&self, other: &CBig<R2, B>) -> Option<Ordering> {
        Some(lex_cmp(&self.re, &self.im, &other.re, &other.im))
    }
}

impl<R: Round, const B: Word> Ord for CBig<R, B> {
    /// Lexicographic total order by `(re, then im)`. Special values are placed consistently with
    /// `FBig` (`-∞ < finite < +∞` per component).
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        lex_cmp(&self.re, &self.im, &other.re, &other.im)
    }
}

impl<R: Round, const B: Word> AbsOrd for CBig<R, B> {
    /// Magnitude comparison by `|z|`. Compared through `|z|²` (the exact squared modulus — both
    /// sides are non-negative, so the order is preserved) to avoid the `sqrt`/`hypot` of [`CBig::abs`].
    #[inline]
    fn abs_cmp(&self, other: &Self) -> Ordering {
        // Exact squared magnitudes at unlimited precision (no rounding, no overflow).
        let unlim = Context::<R>::new(0);
        let f = unlim.float();
        let n1 = f.unwrap_fp(unlim.norm(self));
        let n2 = f.unwrap_fp(unlim.norm(other));
        n1.cmp(&n2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashu_float::round::mode;

    type C = CBig<mode::HalfAway, 10>;

    #[test]
    fn eq_componentwise() {
        let a = C::from_parts(3.into(), 4.into());
        let b = C::from_parts(3.into(), 4.into());
        assert!(a == b);
        let c = C::from_parts(3.into(), 5.into());
        assert!(a != c);
    }

    #[test]
    fn signed_zero_eq() {
        let p = C::from_parts(3.into(), 0.into());
        let n = C::new(Repr::new(3.into(), 0), Repr::neg_zero(), Context::new(0));
        // +0 == -0 on the imaginary part
        assert!(p == n);
    }

    #[test]
    fn ord_lexicographic() {
        let a = C::from_parts(1.into(), 9.into());
        let b = C::from_parts(2.into(), 0.into());
        assert!(a < b); // real part dominates
        let c = C::from_parts(1.into(), 10.into());
        assert!(a < c); // equal real, larger imag
    }

    #[test]
    fn absord_by_magnitude() {
        let a = C::from_parts(3.into(), 4.into()); // |z| = 5
        let b = C::from_parts(5.into(), 0.into()); // |z| = 5
        assert!(a.abs_cmp(&b).is_eq());
        let c = C::from_parts(1.into(), 1.into()); // |z|² = 2
        assert!(c.abs_cmp(&a).is_lt());
    }
}
