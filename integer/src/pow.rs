//! Exponentiation.

use dashu_base::UnsignedAbs;

use crate::{
    repr::TypedReprRef::*, ibig::IBig, primitive::PrimitiveUnsigned, sign::Sign::*, ubig::UBig,
};

impl UBig {
    /// Raises self to the power of `exp`.
    ///
    /// # Example
    ///
    /// ```
    /// # use dashu_int::ubig;
    /// assert_eq!(ubig!(3).pow(3), ubig!(27));
    /// ```
    #[inline]
    pub fn pow(&self, exp: usize) -> UBig {
        match exp {
            0 => return UBig::one(),
            1 => return self.clone(),
            2 => return self * self,
            _ => {}
        }
        match self.repr() {
            RefSmall(0) => return UBig::zero(),
            RefSmall(1) => return UBig::one(),
            RefSmall(2) => {
                let mut x = UBig::zero();
                x.set_bit(exp);
                return x;
            }
            _ => {}
        }
        let mut p = usize::BIT_SIZE - 2 - exp.leading_zeros();
        let mut res = self * self;
        loop {
            if exp & (1 << p) != 0 {
                res *= self;
            }
            if p == 0 {
                break;
            }
            p -= 1;
            res = &res * &res;
        }
        res
    }
}

impl IBig {
    /// Raises self to the power of `exp`.
    ///
    /// # Example
    ///
    /// ```
    /// # use dashu_int::ibig;
    /// assert_eq!(ibig!(-3).pow(3), ibig!(-27));
    /// ```
    #[inline]
    pub fn pow(&self, exp: usize) -> IBig {
        let sign = if self.sign() == Negative && exp % 2 == 1 {
            Negative
        } else {
            Positive
        };
        IBig::from_sign_magnitude(sign, self.unsigned_abs().pow(exp))
    }
}
