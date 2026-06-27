//! `NumOrd` / `NumHash` for [`CBig`] (behind the `num-order` feature), mirroring `FBig`'s surface.
//!
//! `NumOrd` agrees with the lexicographic [`Ord`](crate::cbig::CBig) (by real, then imaginary);
//! `NumHash` is consistent with [`PartialEq`](crate::cbig::CBig) (componentwise value equality).

use crate::cbig::CBig;
use crate::cmp::lex_cmp;
use core::cmp::Ordering;
use core::hash::Hasher;
use dashu_float::round::Round;
use dashu_int::Word;
use num_order::{NumHash, NumOrd};

impl<R1: Round, R2: Round, const B: Word> NumOrd<CBig<R2, B>> for CBig<R1, B> {
    #[inline]
    fn num_cmp(&self, other: &CBig<R2, B>) -> Ordering {
        lex_cmp(&self.re, &self.im, &other.re, &other.im)
    }

    #[inline]
    fn num_partial_cmp(&self, other: &CBig<R2, B>) -> Option<Ordering> {
        Some(self.num_cmp(other))
    }
}

impl<R: Round, const B: Word> NumHash for CBig<R, B> {
    #[inline]
    fn num_hash<H: Hasher>(&self, state: &mut H) {
        self.re.num_hash(state);
        self.im.num_hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashu_float::round::mode;

    type C = CBig<mode::HalfAway, 10>;

    #[test]
    fn num_ord_agrees_with_ord() {
        let a = C::from_parts(1.into(), 9.into());
        let b = C::from_parts(2.into(), 0.into());
        assert_eq!(a.num_cmp(&b), a.cmp(&b));
        assert!(a.num_lt(&b));
    }

    #[test]
    fn num_hash_consistent_with_eq() {
        fn hash_of<T: NumHash>(v: &T) -> u64 {
            use std::hash::DefaultHasher;
            let mut h = DefaultHasher::new();
            v.num_hash(&mut h);
            std::hash::Hasher::finish(&h)
        }
        let a = C::from_parts(3.into(), 4.into());
        let b = C::from_parts(3.into(), 4.into());
        assert_eq!(hash_of(&a), hash_of(&b));
    }
}
