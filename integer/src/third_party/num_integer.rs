//! Implement num-integer traits.

use crate::{ibig::IBig, ubig::UBig};
use dashu_base::{BitTest, CubicRoot, DivRem, ExtendedGcd, Gcd, Sign, SquareRoot};
use num_integer_v01 as num_integer;

impl num_integer::Integer for UBig {
    #[inline]
    fn div_floor(&self, other: &Self) -> Self {
        self / other
    }
    #[inline]
    fn div_rem(&self, other: &Self) -> (Self, Self) {
        DivRem::div_rem(self, other)
    }
    #[inline]
    fn mod_floor(&self, other: &Self) -> Self {
        self % other
    }
    #[inline]
    fn divides(&self, other: &Self) -> bool {
        (self % other).is_zero()
    }
    #[inline]
    fn is_multiple_of(&self, other: &Self) -> bool {
        UBig::is_multiple_of(self, other)
    }
    #[inline]
    fn is_even(&self) -> bool {
        !self.bit(0)
    }
    #[inline]
    fn is_odd(&self) -> bool {
        self.bit(0)
    }
    #[inline]
    fn gcd(&self, other: &Self) -> Self {
        Gcd::gcd(self, other)
    }
    #[inline]
    fn lcm(&self, other: &Self) -> Self {
        if self.is_zero() && other.is_zero() {
            UBig::ZERO
        } else {
            self / Gcd::gcd(self, other) * other
        }
    }
    #[inline]
    fn extended_gcd(&self, other: &Self) -> num_integer::ExtendedGcd<Self> {
        let (g, x, y) = ExtendedGcd::gcd_ext(self, other);
        num_integer::ExtendedGcd {
            gcd: g,
            x: x.try_into().unwrap(),
            y: y.try_into().unwrap(),
        }
    }
}

impl num_integer::Roots for UBig {
    #[inline]
    fn sqrt(&self) -> Self {
        SquareRoot::sqrt(self)
    }
    #[inline]
    fn cbrt(&self) -> Self {
        CubicRoot::cbrt(self)
    }
    #[inline]
    fn nth_root(&self, n: u32) -> Self {
        self.nth_root(n as usize)
    }
}

impl num_integer::Integer for IBig {
    #[inline]
    fn div_floor(&self, other: &Self) -> Self {
        let (q, r) = DivRem::div_rem(self, other);
        if !r.is_zero() && q.sign() == Sign::Negative {
            q - IBig::ONE
        } else {
            q
        }
    }
    #[inline]
    fn div_rem(&self, other: &Self) -> (Self, Self) {
        DivRem::div_rem(self, other)
    }
    #[inline]
    fn mod_floor(&self, other: &Self) -> Self {
        let r = self % other;
        if !r.is_zero() && self.sign() * other.sign() == Sign::Negative {
            other + r
        } else {
            r
        }
    }
    #[inline]
    fn divides(&self, other: &Self) -> bool {
        (self % other).is_zero()
    }
    #[inline]
    fn is_multiple_of(&self, other: &Self) -> bool {
        IBig::is_multiple_of(self, other)
    }
    #[inline]
    fn is_even(&self) -> bool {
        (self & IBig::ONE).is_zero()
    }
    #[inline]
    fn is_odd(&self) -> bool {
        (self & IBig::ONE).is_one()
    }
    #[inline]
    fn gcd(&self, other: &Self) -> Self {
        Gcd::gcd(self, other).into()
    }
    #[inline]
    fn lcm(&self, other: &Self) -> Self {
        if self.is_zero() && other.is_zero() {
            IBig::ZERO
        } else {
            self / Gcd::gcd(self, other) * other
        }
    }
    #[inline]
    fn extended_gcd(&self, other: &Self) -> num_integer::ExtendedGcd<Self> {
        let (g, x, y) = ExtendedGcd::gcd_ext(self, other);
        num_integer::ExtendedGcd {
            gcd: g.into(),
            x,
            y,
        }
    }
}

impl num_integer::Roots for IBig {
    #[inline]
    fn sqrt(&self) -> Self {
        SquareRoot::sqrt(self).into()
    }
    #[inline]
    fn cbrt(&self) -> Self {
        CubicRoot::cbrt(self)
    }
    #[inline]
    fn nth_root(&self, n: u32) -> Self {
        self.nth_root(n as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_integer::Integer;

    #[test]
    fn ubig_floor_division() {
        assert_eq!(UBig::from(5u8).div_floor(&UBig::from(3u8)), UBig::from(1u8));
        // mod_floor must be the remainder (2), not a bitwise operation (5 & 3 = 1)
        assert_eq!(UBig::from(5u8).mod_floor(&UBig::from(3u8)), UBig::from(2u8));
        assert_eq!(UBig::from(9u8).div_mod_floor(&UBig::from(3u8)), (UBig::from(3u8), UBig::ZERO));
    }

    #[test]
    fn ibig_floor_division() {
        assert_eq!(IBig::from(-5).div_floor(&IBig::from(3)), IBig::from(-2));
        assert_eq!(IBig::from(-5).mod_floor(&IBig::from(3)), IBig::from(1));
        assert_eq!(IBig::from(5).mod_floor(&IBig::from(-3)), IBig::from(-1));
        assert_eq!(IBig::from(-5).mod_floor(&IBig::from(-3)), IBig::from(-2));
    }
}
