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

// TODO: disable these in 0.1.0 release. Release a new version after implementing a Lehmer's GCD version

impl UBig {
    /// Compute the greatest common divisor between self and the other operand
    ///
    /// # Example
    /// ```
    /// # use dashu_int::ubig;
    /// assert_eq!(ubig!(12).gcd(&ubig!(18)), ubig!(6));
    /// ```
    ///
    /// Panics if two oprands are both zero.
    #[inline]
    pub fn gcd(&self, rhs: &UBig) -> UBig {
        ubig::gcd_repr_ref_ref(self.repr(), rhs.repr())
    }

    /// Compute the greatest common divisor between self and the other operand, and return
    /// both the common divisor `g` and the Bézout coefficients.
    ///
    /// # Example
    /// ```
    /// # use dashu_int::{ibig, ubig};
    /// assert_eq!(ubig!(12).extended_gcd(&ubig!(18)), (ubig!(6), ibig!(-1), ibig!(1)));
    /// ```
    ///
    /// Panics if two oprands are both zero.
    #[inline]
    pub fn extended_gcd(&self, rhs: &UBig) -> (UBig, IBig, IBig) {
        ubig::xgcd_repr_val_val(self.clone().into_repr(), rhs.clone().into_repr())
    }
}

mod ubig {
    use super::*;
    use crate::{
        primitive::shrink_dword,
        repr::{TypedRepr, TypedReprRef},
    };

    pub(crate) fn gcd_repr_ref_ref(lhs: TypedReprRef, rhs: TypedReprRef) -> UBig {
        match (lhs, rhs) {
            (RefSmall(dword0), RefSmall(dword1)) => dword0.gcd(dword1).into(),
            (RefSmall(dword0), RefLarge(buffer1)) => gcd_large_dword(buffer1, dword0),
            (RefLarge(buffer0), RefSmall(dword1)) => gcd_large_dword(buffer0, dword1),
            (RefLarge(buffer0), RefLarge(buffer1)) => gcd_large(buffer0.into(), buffer1.into()),
        }
    }

    pub(crate) fn xgcd_repr_val_val(lhs: TypedRepr, rhs: TypedRepr) -> (UBig, IBig, IBig) {
        match (lhs, rhs) {
            (Small(dword0), Small(dword1)) => {
                let (g, s, t) = dword0.gcd_ext(dword1);
                (UBig::from(g), s.into(), t.into())
            }
            (Large(buffer0), Small(dword1)) => ubig::extended_gcd_large_dword(buffer0, dword1),
            (Small(dword0), Large(buffer1)) => {
                let (g, s, t) = ubig::extended_gcd_large_dword(buffer1, dword0);
                (g, t, s)
            }
            (Large(buffer0), Large(buffer1)) => ubig::extended_gcd_large(buffer0, buffer1),
        }
    }

    /// Perform gcd on a large number with a `Word`.
    #[inline]
    fn gcd_large_dword(buffer: &[Word], rhs: DoubleWord) -> UBig {
        if rhs == 0 {
            let clone = Buffer::from(buffer);
            clone.into()
        } else if let Some(word) = shrink_dword(rhs) {
            // reduce the large number by single word rhs
            let rem = div::rem_by_word(buffer, word);
            if rem == 0 {
                word.into()
            } else {
                rem.gcd(word).into()
            }
        } else {
            // reduce the large number by double word rhs
            let rem = div::rem_by_dword(buffer, rhs);
            if rem == 0 {
                rhs.into()
            } else {
                rem.gcd(rhs).into()
            }
        }
    }

    /// Perform extended gcd on a large number with a `Word`.
    #[inline]
    fn extended_gcd_large_dword(mut buffer: Buffer, rhs: DoubleWord) -> (UBig, IBig, IBig) {
        if rhs == 0 {
            (buffer.into(), IBig::one(), IBig::zero())
        } else if let Some(word) = shrink_dword(rhs) {
            // reduce the large number by single word rhs
            let rem = div::div_by_word_in_place(&mut buffer, word);
            if rem == 0 {
                (UBig::from(word), IBig::zero(), IBig::one())
            } else {
                let (r, s, t) = word.gcd_ext(rem);
                let new_t = -t * IBig::from(UBig::from(buffer)) + s;
                (UBig::from(r), IBig::from(t), new_t)
            }
        } else {
            // reduce the large number by double word rhs
            let rem = div::div_by_dword_in_place(&mut buffer, rhs);
            if rem == 0 {
                (UBig::from(rhs), IBig::zero(), IBig::one())
            } else {
                let (r, s, t) = rhs.gcd_ext(rem);
                let new_t = -t * IBig::from(UBig::from(buffer)) + s;
                (UBig::from(r), IBig::from(t), new_t)
            }
        }
    }

    /// Perform gcd on two large numbers.
    #[inline]
    fn gcd_large(mut lhs: Buffer, mut rhs: Buffer) -> UBig {
        let len = gcd::gcd_in_place(&mut lhs, &mut rhs);
        lhs.truncate(len);
        lhs.into()
    }

    /// Perform extended gcd on two large numbers.
    #[inline]
    fn extended_gcd_large(mut lhs: Buffer, mut rhs: Buffer) -> (UBig, IBig, IBig) {
        let res_len = lhs.len().min(rhs.len());
        let mut buffer = Buffer::allocate(res_len);
        buffer.push_zeros(res_len);

        let mut allocation =
            MemoryAllocation::new(gcd::memory_requirement_exact(lhs.len(), rhs.len()));
        let mut memory = allocation.memory();

        let (lhs_sign, rhs_sign) =
            gcd::xgcd_in_place(&mut lhs, &mut rhs, &mut buffer, false, &mut memory);
        (
            buffer.into(),
            IBig::from_sign_magnitude(lhs_sign, rhs.into()),
            IBig::from_sign_magnitude(rhs_sign, lhs.into()),
        )
    }
}
