//! Exponentiation.

use crate::{
    ibig::IBig,
    primitive::PrimitiveUnsigned,
    sign::Sign::*,
    ubig::{Repr::*, UBig},
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
            0 => return UBig::from_word(1),
            1 => return self.clone(),
            2 => return self * self,
            _ => {}
        }
        match self.repr() {
            Small(0) => return UBig::from_word(0),
            Small(1) => return UBig::from_word(1),
            Small(2) => {
                let mut x = UBig::from_word(0);
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
        IBig::from_sign_magnitude(sign, self.magnitude().pow(exp))
    }
}
