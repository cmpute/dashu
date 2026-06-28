//! `NumOrd` / `NumHash` for [`CBig`] (behind the `num-order` feature), mirroring `FBig`'s surface.
//!
//! `NumOrd` agrees with the lexicographic [`Ord`](crate::cbig::CBig) (by real, then imaginary).
//! `NumHash` mirrors the `num-order` crate's `Complex<f32>`/`Complex<f64>` hashing: the number is
//! treated as `a + b·i` and the per-part residues are combined algebraically into a single field
//! element `a + bterm` (where `bterm = ∓PROOT²·b²`, sign of `b`), rather than hashing the parts
//! sequentially. This keeps a `CBig` and a `num-complex` `Complex` of the same value in sync.
//! Consistency relies on `dashu-float`'s `Repr` residue equalling num-order's `f64` `fhash` for the
//! same finite value.

use crate::cbig::CBig;
use crate::cmp::lex_cmp;
use _num_modular::{FixedMersenneInt, ModularInteger};
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
    fn num_hash<H: Hasher>(&self, state: &mut H) {
        // Mirror num-order's `Complex<f64>` NumHash: z = a + b·i, hash(a + bterm).
        type MInt = FixedMersenneInt<127, 1>;
        const M127U: u128 = i128::MAX as u128;
        const PROOT: u128 = i32::MAX as u128;

        let a = self.re().num_hash_residue();
        let b = self.imag().num_hash_residue();

        let bterm = if b >= 0 {
            let pb = MInt::new(b as u128, &M127U) * PROOT;
            -((pb * pb).residue() as i128)
        } else {
            let pb = MInt::new((-b) as u128, &M127U) * PROOT;
            (pb * pb).residue() as i128
        };
        a.wrapping_add(bterm).num_hash(state)
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

    /// i128 residue a `NumHash` impl writes (captured via `Hasher::write_i128`).
    fn residue<T: NumHash>(v: &T) -> i128 {
        struct Collector(i128);
        impl core::hash::Hasher for Collector {
            fn write_i128(&mut self, v: i128) {
                self.0 = v;
            }
            fn write(&mut self, _: &[u8]) {}
            fn finish(&self) -> u64 {
                0
            }
        }
        let mut c = Collector(0);
        v.num_hash(&mut c);
        c.0
    }

    #[test]
    fn cbig_num_hash_matches_num_complex() {
        // The base-2 CBig residue must equal num-order's `Complex<f64>` formula, transcribed from
        // num-order/src/hash.rs (Case 4): hash(a + bterm), bterm = ∓PROOT²·b² (sign of b), with a, b
        // the f64 residues. (Relies on dashu-float's Repr residue equalling f64's `fhash` — see
        // float's `test_fbig_num_hash_matches_f64`.)
        use dashu_float::FBig;
        type CF = CBig<mode::Zero, 2>;
        type MInt = FixedMersenneInt<127, 1>;
        const M127U: u128 = i128::MAX as u128;
        const PROOT: u128 = i32::MAX as u128;

        fn num_complex_expected(re: f64, im: f64) -> i128 {
            let a = residue(&re);
            let b = residue(&im);
            let bterm = if b >= 0 {
                let pb = MInt::new(b as u128, &M127U) * PROOT;
                -((pb * pb).residue() as i128)
            } else {
                let pb = MInt::new((-b) as u128, &M127U) * PROOT;
                (pb * pb).residue() as i128
            };
            residue(&(a.wrapping_add(bterm)))
        }

        for (re, im) in [
            (3.0_f64, 4.0),
            (1.0, 0.0),
            (0.0, 1.0),
            (-2.0, 0.5),
            (0.0, 0.0),
            (1.5, -2.25),
            (100.0, -0.0625),
        ] {
            let z = CF::from_parts(FBig::try_from(re).unwrap(), FBig::try_from(im).unwrap());
            assert_eq!(
                residue(&z),
                num_complex_expected(re, im),
                "CBig num_hash disagrees with num-complex formula for {re}+{im}i"
            );
        }
    }
}
