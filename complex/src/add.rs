//! Complex addition and subtraction.
//!
//! Mirroring `dashu-float`'s `add.rs`, the four ref/val combinations are written out explicitly
//! (grouped into `add_val_val` / `add_val_ref` / `add_ref_val` / `add_ref_ref` — and the same for
//! `sub`). Subtraction is componentwise negation of the right operand, so its kernel functions are
//! separate from addition's (no shared `signed_add`). The `Assign` forms forward through
//! [`core::mem::take`] to the by-value operator.

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

// --- Convenience-layer kernel functions, mirroring FBig's add_val_val / add_val_ref / etc. ---

#[inline]
fn add_val_val<R: Round, const B: Word>(lhs: CBig<R, B>, rhs: CBig<R, B>) -> CBig<R, B> {
    let ctx = Context::max(lhs.context(), rhs.context());
    ctx.unwrap_cfp(ctx.add(&lhs, &rhs))
}

#[inline]
fn add_val_ref<R: Round, const B: Word>(lhs: CBig<R, B>, rhs: &CBig<R, B>) -> CBig<R, B> {
    let ctx = Context::max(lhs.context(), rhs.context());
    ctx.unwrap_cfp(ctx.add(&lhs, rhs))
}

#[inline]
fn add_ref_val<R: Round, const B: Word>(lhs: &CBig<R, B>, rhs: CBig<R, B>) -> CBig<R, B> {
    let ctx = Context::max(lhs.context(), rhs.context());
    ctx.unwrap_cfp(ctx.add(lhs, &rhs))
}

#[inline]
fn add_ref_ref<R: Round, const B: Word>(lhs: &CBig<R, B>, rhs: &CBig<R, B>) -> CBig<R, B> {
    let ctx = Context::max(lhs.context(), rhs.context());
    ctx.unwrap_cfp(ctx.add(lhs, rhs))
}

#[inline]
fn sub_val_val<R: Round, const B: Word>(lhs: CBig<R, B>, rhs: CBig<R, B>) -> CBig<R, B> {
    let ctx = Context::max(lhs.context(), rhs.context());
    ctx.unwrap_cfp(ctx.sub(&lhs, &rhs))
}

#[inline]
fn sub_val_ref<R: Round, const B: Word>(lhs: CBig<R, B>, rhs: &CBig<R, B>) -> CBig<R, B> {
    let ctx = Context::max(lhs.context(), rhs.context());
    ctx.unwrap_cfp(ctx.sub(&lhs, rhs))
}

#[inline]
fn sub_ref_val<R: Round, const B: Word>(lhs: &CBig<R, B>, rhs: CBig<R, B>) -> CBig<R, B> {
    let ctx = Context::max(lhs.context(), rhs.context());
    ctx.unwrap_cfp(ctx.sub(lhs, &rhs))
}

#[inline]
fn sub_ref_ref<R: Round, const B: Word>(lhs: &CBig<R, B>, rhs: &CBig<R, B>) -> CBig<R, B> {
    let ctx = Context::max(lhs.context(), rhs.context());
    ctx.unwrap_cfp(ctx.sub(lhs, rhs))
}

// --- Add: all four ref/val combinations ---
impl<R: Round, const B: Word> Add for CBig<R, B> {
    type Output = CBig<R, B>;
    #[inline]
    fn add(self, rhs: CBig<R, B>) -> CBig<R, B> {
        add_val_val(self, rhs)
    }
}

impl<R: Round, const B: Word> Add<&CBig<R, B>> for CBig<R, B> {
    type Output = CBig<R, B>;
    #[inline]
    fn add(self, rhs: &CBig<R, B>) -> CBig<R, B> {
        add_val_ref(self, rhs)
    }
}

impl<R: Round, const B: Word> Add<CBig<R, B>> for &CBig<R, B> {
    type Output = CBig<R, B>;
    #[inline]
    fn add(self, rhs: CBig<R, B>) -> CBig<R, B> {
        add_ref_val(self, rhs)
    }
}

impl<R: Round, const B: Word> Add<&CBig<R, B>> for &CBig<R, B> {
    type Output = CBig<R, B>;
    #[inline]
    fn add(self, rhs: &CBig<R, B>) -> CBig<R, B> {
        add_ref_ref(self, rhs)
    }
}

// --- Sub: all four ref/val combinations ---
impl<R: Round, const B: Word> Sub for CBig<R, B> {
    type Output = CBig<R, B>;
    #[inline]
    fn sub(self, rhs: CBig<R, B>) -> CBig<R, B> {
        sub_val_val(self, rhs)
    }
}

impl<R: Round, const B: Word> Sub<&CBig<R, B>> for CBig<R, B> {
    type Output = CBig<R, B>;
    #[inline]
    fn sub(self, rhs: &CBig<R, B>) -> CBig<R, B> {
        sub_val_ref(self, rhs)
    }
}

impl<R: Round, const B: Word> Sub<CBig<R, B>> for &CBig<R, B> {
    type Output = CBig<R, B>;
    #[inline]
    fn sub(self, rhs: CBig<R, B>) -> CBig<R, B> {
        sub_ref_val(self, rhs)
    }
}

impl<R: Round, const B: Word> Sub<&CBig<R, B>> for &CBig<R, B> {
    type Output = CBig<R, B>;
    #[inline]
    fn sub(self, rhs: &CBig<R, B>) -> CBig<R, B> {
        sub_ref_ref(self, rhs)
    }
}

// --- AddAssign / SubAssign: forward through `mem::take` to the by-value operator ---
crate::helper_macros::impl_binop_assign_by_taking!(impl AddAssign<Self>, add_assign, add);
crate::helper_macros::impl_binop_assign_by_taking!(impl SubAssign<Self>, sub_assign, sub);

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
