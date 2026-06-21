//! Montgomery addition and subtraction.

use super::repr::{Montgomery, MontgomeryInner, MontgomeryLargeRepr, MontgomeryLargeVal};
use crate::{
    add, cmp, error::panic_different_rings, helper_macros::forward_modular_binop_to_assign,
    helper_macros::impl_modular_binop_ref_ref_by_clone,
    helper_macros::impl_modular_commutative_op_for_ref, shift,
};
use core::ops::{Add, AddAssign, Neg, Sub, SubAssign};
use num_modular::Reducer;

impl<'a> Neg for Montgomery<'a> {
    type Output = Montgomery<'a>;

    #[inline]
    fn neg(self) -> Montgomery<'a> {
        match self.into_repr() {
            MontgomeryInner::Single(raw, ring) => Montgomery::from_single(ring.0.neg(raw), ring),
            MontgomeryInner::Double(raw, ring) => Montgomery::from_double(ring.0.neg(raw), ring),
            MontgomeryInner::Large(mut raw, ring) => {
                negate_in_place_large(ring, &mut raw);
                Montgomery::from_large(raw, ring)
            }
        }
    }
}

impl<'a> Neg for &Montgomery<'a> {
    type Output = Montgomery<'a>;

    #[inline]
    fn neg(self) -> Montgomery<'a> {
        self.clone().neg()
    }
}

forward_modular_binop_to_assign!(impl Add, add, AddAssign, add_assign for Montgomery);
impl_modular_commutative_op_for_ref!(impl Add, add for Montgomery);
impl_modular_binop_ref_ref_by_clone!(impl Add, add for Montgomery);

impl<'a> AddAssign<&Montgomery<'a>> for Montgomery<'a> {
    #[inline]
    fn add_assign(&mut self, rhs: &Montgomery<'a>) {
        match (self.repr_mut(), rhs.repr()) {
            (MontgomeryInner::Single(raw0, ring), MontgomeryInner::Single(raw1, ring1)) => {
                Montgomery::check_same_ring_single(ring, ring1);
                ring.0.add_in_place(raw0, raw1);
            }
            (MontgomeryInner::Double(raw0, ring), MontgomeryInner::Double(raw1, ring1)) => {
                Montgomery::check_same_ring_double(ring, ring1);
                ring.0.add_in_place(raw0, raw1);
            }
            (MontgomeryInner::Large(raw0, ring), MontgomeryInner::Large(raw1, ring1)) => {
                Montgomery::check_same_ring_large(ring, ring1);
                add_in_place_large(ring, raw0, raw1);
            }
            _ => panic_different_rings(),
        }
    }
}

forward_modular_binop_to_assign!(impl Sub, sub, SubAssign, sub_assign for Montgomery);
impl_modular_binop_ref_ref_by_clone!(impl Sub, sub for Montgomery);

impl<'a> Sub<Montgomery<'a>> for &Montgomery<'a> {
    type Output = Montgomery<'a>;

    #[inline]
    fn sub(self, mut rhs: Montgomery<'a>) -> Montgomery<'a> {
        match (self.repr(), rhs.repr_mut()) {
            (MontgomeryInner::Single(raw0, ring), MontgomeryInner::Single(raw1, ring1)) => {
                Montgomery::check_same_ring_single(ring, ring1);
                *raw1 = ring.0.sub(raw0, raw1);
            }
            (MontgomeryInner::Double(raw0, ring), MontgomeryInner::Double(raw1, ring1)) => {
                Montgomery::check_same_ring_double(ring, ring1);
                *raw1 = ring.0.sub(raw0, raw1);
            }
            (MontgomeryInner::Large(raw0, ring), MontgomeryInner::Large(raw1, ring1)) => {
                Montgomery::check_same_ring_large(ring, ring1);
                sub_in_place_large_swap(ring, raw0, raw1);
            }
            _ => panic_different_rings(),
        }
        rhs
    }
}

impl<'a> SubAssign<&Montgomery<'a>> for Montgomery<'a> {
    #[inline]
    fn sub_assign(&mut self, rhs: &Montgomery<'a>) {
        match (self.repr_mut(), rhs.repr()) {
            (MontgomeryInner::Single(raw0, ring), MontgomeryInner::Single(raw1, ring1)) => {
                Montgomery::check_same_ring_single(ring, ring1);
                ring.0.sub_in_place(raw0, raw1);
            }
            (MontgomeryInner::Double(raw0, ring), MontgomeryInner::Double(raw1, ring1)) => {
                Montgomery::check_same_ring_double(ring, ring1);
                ring.0.sub_in_place(raw0, raw1);
            }
            (MontgomeryInner::Large(raw0, ring), MontgomeryInner::Large(raw1, ring1)) => {
                Montgomery::check_same_ring_large(ring, ring1);
                sub_in_place_large(ring, raw0, raw1);
            }
            _ => panic_different_rings(),
        }
    }
}

impl<'a> Montgomery<'a> {
    /// Calculate 2*target mod m in Montgomery form.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::{monty::MontgomeryRepr, UBig};
    /// let ring = MontgomeryRepr::new(UBig::from(0x1234_5679u32));
    /// let a = ring.reduce(4000u32);
    /// assert_eq!(a.dbl(), ring.reduce(4000u32 + 4000u32));
    /// ```
    pub fn dbl(self) -> Self {
        match self.into_repr() {
            MontgomeryInner::Single(raw, ring) => Montgomery::from_single(ring.0.dbl(raw), ring),
            MontgomeryInner::Double(raw, ring) => Montgomery::from_double(ring.0.dbl(raw), ring),
            MontgomeryInner::Large(mut raw, ring) => {
                dbl_in_place_large(ring, &mut raw);
                Montgomery::from_large(raw, ring)
            }
        }
    }
}

pub(crate) fn negate_in_place_large(ring: &MontgomeryLargeRepr, raw: &mut MontgomeryLargeVal) {
    let modulus = &ring.modulus;
    if !raw.0.iter().all(|w| *w == 0) {
        let overflow = add::sub_same_len_in_place_swap(modulus, &mut raw.0);
        debug_assert!(!overflow);
    }
}

fn add_in_place_large(
    ring: &MontgomeryLargeRepr,
    lhs: &mut MontgomeryLargeVal,
    rhs: &MontgomeryLargeVal,
) {
    let modulus = &ring.modulus;
    let overflow = add::add_same_len_in_place(&mut lhs.0, &rhs.0);
    if overflow || cmp::cmp_same_len(&lhs.0, modulus).is_ge() {
        let overflow2 = add::sub_same_len_in_place(&mut lhs.0, modulus);
        debug_assert_eq!(overflow, overflow2);
    }
}

fn dbl_in_place_large(ring: &MontgomeryLargeRepr, raw: &mut MontgomeryLargeVal) {
    let modulus = &ring.modulus;
    let overflow = shift::shl_in_place(&mut raw.0, 1) > 0;
    if overflow || cmp::cmp_same_len(&raw.0, modulus).is_ge() {
        let overflow2 = add::sub_same_len_in_place(&mut raw.0, modulus);
        debug_assert_eq!(overflow, overflow2);
    }
}

fn sub_in_place_large(
    ring: &MontgomeryLargeRepr,
    lhs: &mut MontgomeryLargeVal,
    rhs: &MontgomeryLargeVal,
) {
    let modulus = &ring.modulus;
    let overflow = add::sub_same_len_in_place(&mut lhs.0, &rhs.0);
    if overflow {
        let overflow2 = add::add_same_len_in_place(&mut lhs.0, modulus);
        debug_assert!(overflow2);
    }
}

/// rhs = self - rhs
fn sub_in_place_large_swap(
    ring: &MontgomeryLargeRepr,
    lhs: &MontgomeryLargeVal,
    rhs: &mut MontgomeryLargeVal,
) {
    let modulus = &ring.modulus;
    let overflow = add::sub_same_len_in_place_swap(&lhs.0, &mut rhs.0);
    if overflow {
        let overflow2 = add::add_same_len_in_place(&mut rhs.0, modulus);
        debug_assert!(overflow2);
    }
}
