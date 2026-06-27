//! Complex addition.

use crate::cbig::CBig;
use crate::context::{combine_parts, CfpResult, Context};
use core::ops::{Add, AddAssign};
use dashu_float::round::Round;
use dashu_int::Word;

impl<R: Round> Context<R> {
    /// Add two complex numbers under this context (context layer).
    ///
    /// Returns a [`CfpResult`] carrying each part's inexactness. Addition is componentwise, so each
    /// part is a single correctly-rounded real addition.
    pub fn add<const B: Word>(&self, z: &CBig<R, B>, w: &CBig<R, B>) -> CfpResult<R, B> {
        let re = self.float().add(z.re(), w.re())?;
        let im = self.float().add(z.imag(), w.imag())?;
        Ok(combine_parts(re, im))
    }
}

crate::helper_macros::impl_cbig_binop!(Add, add, AddAssign, add_assign);

#[cfg(test)]
mod tests {
    use super::*;
    use dashu_float::round::mode;

    type C = CBig<mode::HalfAway, 10>;

    #[test]
    fn add_componentwise() {
        let z = C::from_parts(3.into(), 4.into());
        let w = C::from_parts(1.into(), 2.into());
        let r = &z + &w;
        assert_eq!(r.re().significand(), &4.into());
        assert_eq!(r.imag().significand(), &6.into());
    }

    #[test]
    fn add_operator() {
        let z = C::from_parts(1.into(), 2.into());
        let w = C::from_parts(3.into(), 4.into());
        let r = z.clone() + w.clone();
        assert_eq!(r.re().significand(), &4.into());
        assert_eq!(r.imag().significand(), &6.into());

        let mut acc = z.clone();
        acc += &w;
        assert_eq!(acc.re().significand(), &4.into());
    }
}
