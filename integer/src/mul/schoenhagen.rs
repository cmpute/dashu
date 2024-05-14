//! Number-theoretic multiplication algorithm.

// TODO(0.5): try the implementation of concrete-ntt

use crate::arch::{
    word::Word,
};

struct FermatReducer(usize);

enum FermatResidue<'a> {
    Minus1,
    Normal(&'a mut [Word])
}

impl FermatReducer {
    fn reduce(&self, value: &mut [Word]) -> FermatResidue<'_> {
        todo!()
    }

    /// Do butterfly operation in-place efficiently
    /// 
    /// (lhs, rhs) = ((lhs + rhs) % modulus, (lhs + rhs) % modulus)
    fn butterfly(&self, lhs: &mut FermatResidue<'_>, rhs: &mut FermatResidue<'_>) {
        todo!()
    }

    fn shl(&self) {
        todo!()
    }

    fn mul(&self) {
        todo!()
    }
}

struct FFTParams {
    m: usize, // bit size of the coefficients
    l: usize, // size of the polynomial
    n: usize // the modulus used in FFT is 2^n + 1
}

fn fft_forward() {

}

fn fft_reverse() {

}

fn memory_requirement_up_to() {
    
}

fn add_signed_mul_same_len() {

}

fn add_signed_mul() {

}

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
