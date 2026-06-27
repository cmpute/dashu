//! Complex subtraction.

use crate::cbig::CBig;
use crate::context::{combine_parts, CfpResult, Context};
use core::ops::{Sub, SubAssign};
use dashu_float::round::Round;
use dashu_int::Word;

impl<R: Round> Context<R> {
    /// Subtract two complex numbers under this context (context layer).
    pub fn sub<const B: Word>(&self, z: &CBig<R, B>, w: &CBig<R, B>) -> CfpResult<R, B> {
        let re = self.float().sub(z.re(), w.re())?;
        let im = self.float().sub(z.imag(), w.imag())?;
        Ok(combine_parts(re, im))
    }
}

crate::helper_macros::impl_cbig_binop!(Sub, sub, SubAssign, sub_assign);

#[cfg(test)]
mod tests {
    use super::*;
    use dashu_float::round::mode;

    type C = CBig<mode::HalfAway, 10>;

    #[test]
    fn sub_componentwise() {
        let z = C::from_parts(3.into(), 4.into());
        let w = C::from_parts(1.into(), 2.into());
        let r = &z - &w; // Sub operator (no inherent sub method)
        assert_eq!(r.re().significand(), &2.into());
        assert_eq!(r.imag().significand(), &2.into());
    }

    #[test]
    fn z_minus_z_is_zero() {
        let z = C::from_parts(7.into(), 9.into());
        let r = &z - &z;
        assert!(r.is_zero());
    }
}
