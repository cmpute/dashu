//! Complex addition and subtraction.
//!
//! Addition and subtraction are componentwise perfectly-rounded real additions/subtractions on each
//! part, forwarded through the shared [`impl_cbig_binop!`] macro (same pattern as `Mul` / `Div`).

use crate::cbig::CBig;
use crate::repr::{combine_parts, CfpResult, Context};
use core::ops::{Add, AddAssign, Sub, SubAssign};
use dashu_float::round::Round;
use dashu_int::Word;

impl<R: Round> Context<R> {
    /// Add two complex numbers under this context (context layer).
    ///
    /// Returns a [`CfpResult`] carrying each part's inexactness. Addition is componentwise, so each
    /// part is a single correctly-rounded real addition.
    pub fn add<const B: Word>(&self, z: &CBig<R, B>, w: &CBig<R, B>) -> CfpResult<R, B> {
        let re = self.float().add(z.re(), w.re())?;
        let im = self.float().add(z.im(), w.im())?;
        Ok(combine_parts(re, im))
    }

    /// Subtract two complex numbers under this context (context layer).
    ///
    /// Returns a [`CfpResult`] carrying each part's inexactness. Subtraction is componentwise, so each
    /// part is a single correctly-rounded real subtraction.
    pub fn sub<const B: Word>(&self, z: &CBig<R, B>, w: &CBig<R, B>) -> CfpResult<R, B> {
        let re = self.float().sub(z.re(), w.re())?;
        let im = self.float().sub(z.im(), w.im())?;
        Ok(combine_parts(re, im))
    }
}

// --- Add: all four ref/val combinations, plus Assign, via the shared macro ---
crate::helper_macros::impl_cbig_binop!(Add, add, AddAssign, add_assign);

// --- Sub: all four ref/val combinations, plus Assign, via the shared macro ---
crate::helper_macros::impl_cbig_binop!(Sub, sub, SubAssign, sub_assign);

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
        assert_eq!(r.im().significand(), &6.into());
    }

    #[test]
    fn add_all_ref_val_combinations() {
        let z = C::from_parts(1.into(), 2.into());
        let w = C::from_parts(3.into(), 4.into());
        // val + val, val + ref, ref + val, ref + ref
        assert_eq!((z.clone() + w.clone()).im().significand(), &6.into());
        assert_eq!((z.clone() + &w).im().significand(), &6.into());
        assert_eq!((&z + w.clone()).im().significand(), &6.into());
        assert_eq!((&z + &w).im().significand(), &6.into());
    }

    #[test]
    fn add_assign_val_and_ref() {
        let z = C::from_parts(1.into(), 2.into());
        let w = C::from_parts(3.into(), 4.into());

        let mut acc = z.clone();
        acc += w.clone();
        assert_eq!(acc.re().significand(), &4.into());
        assert_eq!(acc.im().significand(), &6.into());

        let mut acc = z.clone();
        acc += &w;
        assert_eq!(acc.re().significand(), &4.into());
    }

    #[test]
    fn sub_componentwise() {
        let z = C::from_parts(3.into(), 4.into());
        let w = C::from_parts(1.into(), 2.into());
        let r = &z - &w;
        assert_eq!(r.re().significand(), &2.into());
        assert_eq!(r.im().significand(), &2.into());
    }

    #[test]
    fn sub_all_ref_val_combinations() {
        let z = C::from_parts(5.into(), 6.into());
        let w = C::from_parts(2.into(), 1.into());
        assert_eq!((z.clone() - w.clone()).re().significand(), &3.into());
        assert_eq!((z.clone() - &w).re().significand(), &3.into());
        assert_eq!((&z - w.clone()).re().significand(), &3.into());
        assert_eq!((&z - &w).re().significand(), &3.into());
    }

    #[test]
    fn z_minus_z_is_zero() {
        let z = C::from_parts(7.into(), 9.into());
        let r = &z - &z;
        assert!(r.is_zero());
    }

    #[test]
    fn sub_assign_val_and_ref() {
        let z = C::from_parts(5.into(), 6.into());
        let w = C::from_parts(2.into(), 1.into());

        let mut acc = z.clone();
        acc -= w.clone();
        assert_eq!(acc.re().significand(), &3.into());
        assert_eq!(acc.im().significand(), &5.into());

        let mut acc = z.clone();
        acc -= &w;
        assert_eq!(acc.re().significand(), &3.into());
    }
}
