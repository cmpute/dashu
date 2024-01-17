//! Number-theoretic multiplication algorithm.

// TODO(0.5): try the implementation of concrete-ntt

use crate::arch::{
    ntt::{MAX_ORDER, PRIMES},
    word::Word,
};
const _UNUSED: u32 = MAX_ORDER; // TODO: implement Schönhage-Strassen multiplication

type ModuloWord = num_modular::Normalized2by1Divisor<Word>;

/// The number of prime factors in the ring.
pub const NUM_PRIMES: usize = 3;

/// A prime to be used for the number-theoretic transform.
pub struct Prime {
    /// A prime of the form k * 2^MAX_ORDER + 1.
    pub(crate) prime: Word,
    /// max_order_root has order 2^MAX_ORDER.
    pub(crate) max_order_root: Word,
}

/// Factor fields of the three-prime ring.
const FIELDS: [ModuloWord; NUM_PRIMES] = [
    ModuloWord::new(PRIMES[0].prime),
    ModuloWord::new(PRIMES[1].prime),
    ModuloWord::new(PRIMES[2].prime),
];

// /// An element of the three-prime ring.
// #[derive(Clone, Copy, Debug, Eq, PartialEq)]
// struct RingElement {
//     val: [ModuloSingleRaw; NUM_PRIMES],
// }

// impl From<Word> for RingElement {
//     /// Convert a `Word` to `RingElement`.
//     fn from(x: Word) -> RingElement {
//         RingElement {
//             val: [
//                 ModuloSingleRaw::from_word(x, &FIELDS[0]),
//                 ModuloSingleRaw::from_word(x, &FIELDS[1]),
//                 ModuloSingleRaw::from_word(x, &FIELDS[2]),
//             ],
//         }
//     }
// }

// impl RingElement {
//     const fn zero() -> RingElement {
//         RingElement {
//             val: [
//                 ModuloSingleRaw::from_word(0, &FIELDS[0]),
//                 ModuloSingleRaw::from_word(0, &FIELDS[1]),
//                 ModuloSingleRaw::from_word(0, &FIELDS[2]),
//             ],
//         }
//     }

//     const fn mul(self, rhs: RingElement) -> RingElement {
//         RingElement {
//             val: [
//                 FIELDS[0].mul(self.val[0], rhs.val[0]),
//                 FIELDS[1].mul(self.val[1], rhs.val[1]),
//                 FIELDS[2].mul(self.val[2], rhs.val[2]),
//             ],
//         }
//     }

//     const fn inverse(self) -> RingElement {
//         RingElement {
//             val: [
//                 FIELDS[0].pow_word(self.val[0], PRIMES[0].prime - 2),
//                 FIELDS[1].pow_word(self.val[1], PRIMES[1].prime - 2),
//                 FIELDS[2].pow_word(self.val[2], PRIMES[2].prime - 2),
//             ],
//         }
//     }
// }

// const MAX_ORDER_ROOT: RingElement = RingElement {
//     val: [
//         ModuloSingleRaw::from_word(PRIMES[0].max_order_root, &FIELDS[0]),
//         ModuloSingleRaw::from_word(PRIMES[1].max_order_root, &FIELDS[1]),
//         ModuloSingleRaw::from_word(PRIMES[2].max_order_root, &FIELDS[2]),
//     ],
// };

// type RootTable = [RingElement; MAX_ORDER as usize + 1];

// // ROOTS[order]^(2^order) = 1
// #[allow(dead_code)]
// static ROOTS: RootTable = generate_roots(MAX_ORDER_ROOT);

// // INVERSE_ROOTS[order]^(2^order) = 1
// #[allow(dead_code)]
// static INVERSE_ROOTS: RootTable = generate_roots(MAX_ORDER_ROOT.inverse());

// const fn generate_roots(max_order_root: RingElement) -> RootTable {
//     let mut table = [RingElement::zero(); MAX_ORDER as usize + 1];
//     let mut order = MAX_ORDER as usize;
//     table[order] = max_order_root;
//     while order > 0 {
//         table[order - 1] = table[order].mul(table[order]);
//         order -= 1;
//     }
//     table
// }

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_inverse() {
//         let one: Word = 1;
//         let one = RingElement::from(one);
//         assert_eq!(MAX_ORDER_ROOT.inverse().mul(MAX_ORDER_ROOT), one);
//         assert_eq!(MAX_ORDER_ROOT.inverse().inverse(), MAX_ORDER_ROOT);
//     }

//     #[test]
//     fn test_roots() {
//         let one: Word = 1;
//         let one = RingElement::from(one);
//         assert_eq!(ROOTS[0], one);
//         assert_ne!(ROOTS[1], one);
//         assert_eq!(INVERSE_ROOTS[0], one);
//         assert_ne!(INVERSE_ROOTS[1], one);
//     }
// }
