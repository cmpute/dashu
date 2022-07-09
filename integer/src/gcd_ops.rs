//! Operators for finding greatest common divisor.

use crate::{
    arch::word::{DoubleWord, Word},
    div, gcd,
    ibig::IBig,
    memory::MemoryAllocation,
    repr::{Buffer, TypedRepr::*, TypedReprRef::*},
    ubig::UBig,
};
use dashu_base::ring::{ExtendedGcd, Gcd};

impl UBig {
    /// Compute the greatest common divisor between self and the other operand
    ///
    /// # Example
    /// ```
    /// # use dashu_int::ubig;
    /// // assert_eq!(ubig!(12).gcd(&ubig!(18)), ubig!(6));
    /// ```
    ///
    /// Panics if two oprands are both zero.
    #[inline]
    #[allow(unused)] // enable after 0.1.0
    pub(crate) fn gcd(&self, rhs: &UBig) -> UBig {
        UBig(self.repr().gcd(rhs.repr()))
    }

    /// Compute the greatest common divisor between self and the other operand, and return
    /// both the common divisor `g` and the Bézout coefficients.
    ///
    /// # Example
    /// ```
    /// # use dashu_int::{ibig, ubig};
    /// // assert_eq!(ubig!(12).gcd_ext(&ubig!(18)), (ubig!(6), ibig!(-1), ibig!(1)));
    /// ```
    ///
    /// Panics if two oprands are both zero.
    #[inline]
    #[allow(unused)] // enable after 0.1.0
    pub(crate) fn gcd_ext(&self, rhs: &UBig) -> (UBig, IBig, IBig) {
        let (r, s, t) = self.clone().into_repr().gcd_ext(rhs.clone().into_repr());
        (UBig(r), IBig(s), IBig(t))
    }
}

mod repr {
    use super::*;
    use crate::{
        primitive::{shrink_dword, PrimitiveSigned},
        repr::{Repr, TypedRepr, TypedReprRef},
    };

    impl<'l, 'r> Gcd<TypedReprRef<'r>> for TypedReprRef<'l> {
        type Output = Repr;

        fn gcd(self, rhs: TypedReprRef) -> Repr {
            match (self, rhs) {
                (RefSmall(dword0), RefSmall(dword1)) => Repr::from_dword(dword0.gcd(dword1)),
                (RefSmall(dword0), RefLarge(buffer1)) => gcd_large_dword(buffer1, dword0),
                (RefLarge(buffer0), RefSmall(dword1)) => gcd_large_dword(buffer0, dword1),
                (RefLarge(buffer0), RefLarge(buffer1)) => gcd_large(buffer0.into(), buffer1.into()),
            }
        }
    }

    /// Perform gcd on a large number with a `Word`.
    #[inline]
    fn gcd_large_dword(buffer: &[Word], rhs: DoubleWord) -> Repr {
        if rhs == 0 {
            Repr::from_buffer(buffer.into())
        } else if let Some(word) = shrink_dword(rhs) {
            // reduce the large number by single word rhs
            let rem = div::rem_by_word(buffer, word);
            if rem == 0 {
                Repr::from_word(word)
            } else {
                Repr::from_word(rem.gcd(word))
            }
        } else {
            // reduce the large number by double word rhs
            let rem = div::rem_by_dword(buffer, rhs);
            if rem == 0 {
                Repr::from_dword(rhs)
            } else {
                Repr::from_dword(rem.gcd(rhs))
            }
        }
    }

    /// Perform gcd on two large numbers.
    #[inline]
    fn gcd_large(mut lhs: Buffer, mut rhs: Buffer) -> Repr {
        let len = gcd::gcd_in_place(&mut lhs, &mut rhs);
        lhs.truncate(len);
        Repr::from_buffer(lhs)
    }

    impl ExtendedGcd<TypedRepr> for TypedRepr {
        type OutputCoeff = Repr;
        type OutputGcd = Repr;

        fn gcd_ext(self, rhs: TypedRepr) -> (Repr, Repr, Repr) {
            match (self, rhs) {
                (Small(dword0), Small(dword1)) => {
                    let (g, s, t) = dword0.gcd_ext(dword1);
                    let (s_sign, s_mag) = s.to_sign_magnitude();
                    let (t_sign, t_mag) = t.to_sign_magnitude();
                    (
                        Repr::from_dword(g),
                        Repr::from_dword(s_mag).with_sign(s_sign),
                        Repr::from_dword(t_mag).with_sign(t_sign),
                    )
                }
                (Large(buffer0), Small(dword1)) => gcd_ext_large_dword(buffer0, dword1),
                (Small(dword0), Large(buffer1)) => {
                    let (g, s, t) = gcd_ext_large_dword(buffer1, dword0);
                    (g, t, s)
                }
                (Large(buffer0), Large(buffer1)) => gcd_ext_large(buffer0, buffer1),
            }
        }
    }

    /// Perform extended gcd on a large number with a `Word`.
    #[inline]
    fn gcd_ext_large_dword(mut buffer: Buffer, rhs: DoubleWord) -> (Repr, Repr, Repr) {
        if rhs == 0 {
            (Repr::from_buffer(buffer), Repr::one(), Repr::zero())
        } else if let Some(word) = shrink_dword(rhs) {
            // reduce the large number by single word rhs
            let rem = div::div_by_word_in_place(&mut buffer, word);
            if rem == 0 {
                (Repr::from_word(word), Repr::zero(), Repr::one())
            } else {
                let (r, s, t) = word.gcd_ext(rem);
                let (t_sign, t_mag) = t.to_sign_magnitude();
                let new_t = s - t * IBig(Repr::from_buffer(buffer));
                (
                    Repr::from_word(r),
                    Repr::from_word(t_mag).with_sign(t_sign),
                    new_t.0,
                )
            }
        } else {
            // reduce the large number by double word rhs
            let rem = div::div_by_dword_in_place(&mut buffer, rhs);
            if rem == 0 {
                (Repr::from_dword(rhs), Repr::zero(), Repr::one())
            } else {
                let (r, s, t) = rhs.gcd_ext(rem);
                let (t_sign, t_mag) = t.to_sign_magnitude();
                let new_t = s - t * IBig(Repr::from_buffer(buffer));
                (
                    Repr::from_dword(r),
                    Repr::from_dword(t_mag).with_sign(t_sign),
                    new_t.0,
                )
            }
        }
    }

    /// Perform extended gcd on two large numbers.
    #[inline]
    fn gcd_ext_large(mut lhs: Buffer, mut rhs: Buffer) -> (Repr, Repr, Repr) {
        let res_len = lhs.len().min(rhs.len());
        let mut buffer = Buffer::allocate(res_len);
        buffer.push_zeros(res_len);

        let mut allocation =
            MemoryAllocation::new(gcd::memory_requirement_exact(lhs.len(), rhs.len()));
        let mut memory = allocation.memory();

        let (lhs_sign, rhs_sign) =
            gcd::gcd_ext_in_place(&mut lhs, &mut rhs, &mut buffer, false, &mut memory);
        (
            Repr::from_buffer(buffer),
            Repr::from_buffer(rhs).with_sign(lhs_sign),
            Repr::from_buffer(lhs).with_sign(rhs_sign),
        )
    }
}
