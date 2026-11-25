use crate::{
    error::{assert_finite, FpError},
    fbig::FBig,
    repr::Word,
    round::Round,
};
use core::ops::{Shl, ShlAssign, Shr, ShrAssign};

impl<R: Round, const B: Word> FBig<R, B> {
    /// Multiply by `B^delta` in place. The scaling is exact, but the exponent has to stay
    /// within the finite range: a result at or beyond the `isize::MAX` / `isize::MIN`
    /// sentinel exponents overflows / underflows exactly like a multiplication by `B^delta`
    /// would (see [`Context::unwrap_fp`](crate::Context::unwrap_fp)), instead of leaving a
    /// nonzero significand at a sentinel exponent (or wrapping around).
    ///
    /// A shift by zero is the identity for every finite value, also for the saturation
    /// endpoints themselves (the largest finite value `(B^p − 1)·B^isize::MAX` and the
    /// smallest `B^isize::MIN` have a nonzero significand at a sentinel exponent): nothing
    /// is scaled, so nothing overflows.
    fn scale_exponent(&mut self, delta: i128) {
        assert_finite(&self.repr);
        if delta == 0 || self.repr.significand.is_zero() {
            return; // unchanged (±0 has no scale)
        }
        let exponent = self.repr.exponent as i128 + delta;
        if exponent >= isize::MAX as i128 {
            let overflow = Err(FpError::Overflow(self.repr.sign()));
            *self = self.context.unwrap_fp(overflow);
        } else if exponent <= isize::MIN as i128 {
            let underflow = Err(FpError::Underflow(self.repr.sign()));
            *self = self.context.unwrap_fp(underflow);
        } else {
            self.repr.exponent = exponent as isize;
        }
    }
}

impl<R: Round, const B: Word> Shl<isize> for FBig<R, B> {
    type Output = Self;
    #[inline]
    fn shl(mut self, rhs: isize) -> Self::Output {
        self.scale_exponent(rhs as i128);
        self
    }
}

impl<R: Round, const B: Word> ShlAssign<isize> for FBig<R, B> {
    #[inline]
    fn shl_assign(&mut self, rhs: isize) {
        self.scale_exponent(rhs as i128);
    }
}

impl<R: Round, const B: Word> Shr<isize> for FBig<R, B> {
    type Output = Self;
    #[inline]
    fn shr(mut self, rhs: isize) -> Self::Output {
        self.scale_exponent(-(rhs as i128));
        self
    }
}

impl<R: Round, const B: Word> ShrAssign<isize> for FBig<R, B> {
    #[inline]
    fn shr_assign(&mut self, rhs: isize) {
        self.scale_exponent(-(rhs as i128));
    }
}
