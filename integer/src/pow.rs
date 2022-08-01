//! Exponentiation.

use dashu_base::UnsignedAbs;

use crate::{
    ibig::IBig, primitive::PrimitiveUnsigned, repr::TypedReprRef::*, sign::Sign::*, ubig::UBig,
};

impl UBig {
    /// Raises self to the power of `exp`.
    ///
    /// # Example
    ///
    /// ```
    /// # use dashu_int::UBig;
    /// assert_eq!(UBig::from(3u8).pow(3), 27);
    /// ```
    #[inline]
    pub fn pow(&self, exp: usize) -> UBig {
        // TODO: remove trailing zeros before powering
        // TODO: lift the base to fill a Word / DoubleWord before actual powering
        match exp {
            0 => return UBig::ONE,
            1 => return self.clone(),
            2 => return self * self,
            _ => {}
        }
        match self.repr() {
            RefSmall(0) => return UBig::ZERO,
            RefSmall(1) => return UBig::ONE,
            RefSmall(2) => {
                let mut x = UBig::ZERO;
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
            // TODO: use specialized square function
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
    /// # use dashu_int::IBig;
    /// assert_eq!(IBig::from(-3).pow(3), -27);
    /// ```
    #[inline]
    pub fn pow(&self, exp: usize) -> IBig {
        let sign = if self.sign() == Negative && exp % 2 == 1 {
            Negative
        } else {
            Positive
        };
        // TODO(next): prevent copy by unsigned_abs
        IBig(self.unsigned_abs().pow(exp).0.with_sign(sign))
    }
}
