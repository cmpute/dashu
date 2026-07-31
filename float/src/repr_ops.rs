//! Core trait impls for [`Repr`]: [`PartialEq`]/[`Eq`] (value equality — `+0`/`-0` and same-sign
//! infinities compare equal), [`Neg`], and exact [`Add`]/[`Sub`]/[`Mul`].
//!
//! A [`Repr`] carries no precision limit, so the arithmetic ops are lossless (no rounding). These
//! are the shared primitives the crate reaches for whenever it needs an exact intermediate — the
//! Ziv containment test, the correctly-rounded `Sum`, and the `FBig` multiply path.
//!
//! Each arithmetic operator's logic lives in the `&Repr`-by-`&Repr` primary impl; the val/ref
//! forwarders delegate to it without cloning (a `Repr`'s significand is heap-backed, so the ref/ref
//! form is the no-extra-allocation path). [`Mul`] saturates exponent overflow/underflow to the
//! signed infinity/zero sentinels so the operator is infallible; the precision-limited
//! [`Context`](crate::Context) multiply re-derives the [`FpError`] it needs from that saturated
//! result.

use core::cmp::Ordering;
use core::ops::{Add, Mul, Neg, Sub};

use dashu_base::Sign;

use crate::error::FpError;
use crate::repr::{Repr, Word};
use crate::utils::shl_digits;

impl<const B: Word> PartialEq for Repr<B> {
    /// Two representations are equal when they denote the same value. In particular `+0`
    /// and `-0` compare equal, as do two infinities of the same sign.
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        if self.significand.is_zero() && other.significand.is_zero() {
            let (self_inf, other_inf) = (self.is_infinite(), other.is_infinite());
            match (self_inf, other_inf) {
                (true, true) => self.sign() == other.sign(),
                (false, false) => true, // both are ±0
                _ => false,             // one is zero, the other is infinite
            }
        } else {
            self.significand == other.significand && self.exponent == other.exponent
        }
    }
}

impl<const B: Word> Eq for Repr<B> {}

impl<const B: Word> Neg for Repr<B> {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self::Output {
        Repr::neg(self)
    }
}

impl<const B: Word> Add<&Repr<B>> for &Repr<B> {
    type Output = Repr<B>;

    #[inline]
    fn add(self, rhs: &Repr<B>) -> Repr<B> {
        debug_assert!(self.is_finite());
        debug_assert!(rhs.is_finite());

        // Zero operands short-circuit so a `-0` operand's sentinel exponent (-1) can't bleed into
        // the result exponent — `precise_sum` (the `Sum` impl) relies on this. The aligned path
        // below would compute the same value but would rebuild via `Repr::new`, normalizing away
        // the operand's own representation.
        if self.significand.is_zero() {
            return rhs.clone();
        }
        if rhs.significand.is_zero() {
            return self.clone();
        }

        // Result exponent is min(lhs, rhs); shift the larger-exponent significand up (appending
        // trailing base-`B` zero-digits is lossless) and add. `IBig` add handles opposite signs.
        match self.exponent.cmp(&rhs.exponent) {
            Ordering::Equal => Repr::new(&self.significand + &rhs.significand, self.exponent),
            Ordering::Greater => Repr::new(
                shl_digits::<B>(&self.significand, (self.exponent - rhs.exponent) as usize)
                    + &rhs.significand,
                rhs.exponent,
            ),
            Ordering::Less => Repr::new(
                &self.significand
                    + shl_digits::<B>(&rhs.significand, (rhs.exponent - self.exponent) as usize),
                self.exponent,
            ),
        }
    }
}

impl<const B: Word> Add<&Repr<B>> for Repr<B> {
    type Output = Repr<B>;
    #[inline]
    fn add(self, rhs: &Repr<B>) -> Repr<B> {
        (&self) + rhs
    }
}
impl<const B: Word> Add<Repr<B>> for &Repr<B> {
    type Output = Repr<B>;
    #[inline]
    fn add(self, rhs: Repr<B>) -> Repr<B> {
        self + &rhs
    }
}
impl<const B: Word> Add<Repr<B>> for Repr<B> {
    type Output = Repr<B>;
    #[inline]
    fn add(self, rhs: Repr<B>) -> Repr<B> {
        (&self) + &rhs
    }
}

impl<const B: Word> Sub<&Repr<B>> for &Repr<B> {
    type Output = Repr<B>;

    #[inline]
    fn sub(self, rhs: &Repr<B>) -> Repr<B> {
        debug_assert!(self.is_finite());
        debug_assert!(rhs.is_finite());

        // Zero short-circuits: `x - 0 = x` (keep x), `0 - x = -x` (negate x). As with `Add`, this
        // keeps a zero operand's own representation rather than rebuilding via `Repr::new`.
        if rhs.significand.is_zero() {
            return self.clone();
        }
        if self.significand.is_zero() {
            return rhs.clone().neg();
        }

        // Mirror `Add`: align to the smaller exponent, then subtract significands.
        match self.exponent.cmp(&rhs.exponent) {
            Ordering::Equal => Repr::new(&self.significand - &rhs.significand, self.exponent),
            Ordering::Greater => Repr::new(
                shl_digits::<B>(&self.significand, (self.exponent - rhs.exponent) as usize)
                    - &rhs.significand,
                rhs.exponent,
            ),
            Ordering::Less => Repr::new(
                &self.significand
                    - shl_digits::<B>(&rhs.significand, (rhs.exponent - self.exponent) as usize),
                self.exponent,
            ),
        }
    }
}

impl<const B: Word> Sub<&Repr<B>> for Repr<B> {
    type Output = Repr<B>;
    #[inline]
    fn sub(self, rhs: &Repr<B>) -> Repr<B> {
        (&self) - rhs
    }
}
impl<const B: Word> Sub<Repr<B>> for &Repr<B> {
    type Output = Repr<B>;
    #[inline]
    fn sub(self, rhs: Repr<B>) -> Repr<B> {
        self - &rhs
    }
}
impl<const B: Word> Sub<Repr<B>> for Repr<B> {
    type Output = Repr<B>;
    #[inline]
    fn sub(self, rhs: Repr<B>) -> Repr<B> {
        (&self) - &rhs
    }
}

impl<const B: Word> Mul<&Repr<B>> for &Repr<B> {
    type Output = Repr<B>;

    #[inline]
    fn mul(self, rhs: &Repr<B>) -> Repr<B> {
        debug_assert!(self.is_finite());
        debug_assert!(rhs.is_finite());

        let significand = &self.significand * &rhs.significand;
        if significand.is_zero() {
            // The product significand is `+0`; attach the XOR sign of the operands.
            return if self.sign() != rhs.sign() {
                Repr::neg_zero()
            } else {
                Repr::zero()
            };
        }
        let sign = if self.sign() != rhs.sign() {
            Sign::Negative
        } else {
            Sign::Positive
        };
        // Exponent = lhs + rhs; saturate an `isize` overflow to the signed infinity/zero sentinel
        // so the operator stays infallible (unreachable for real inputs — it needs operands with
        // exponents ~±2^62). `Context::mul` re-derives the `FpError` from this saturated result.
        let exponent = match self.exponent.checked_add(rhs.exponent) {
            Some(e) => e,
            None => {
                debug_assert!(
                    self.exponent.is_positive() == rhs.exponent.is_positive(),
                    "checked_add overflow with mixed-sign exponents is impossible"
                );
                return if self.exponent > 0 {
                    Repr::infinity_with_sign(sign)
                } else {
                    Repr::zero_with_sign(sign)
                };
            }
        };
        match Repr::new(significand, exponent).check_finite_exponent() {
            Ok(r) => r,
            Err(FpError::Overflow(s)) => Repr::infinity_with_sign(s),
            Err(FpError::Underflow(s)) => Repr::zero_with_sign(s),
            Err(_) => unreachable!(),
        }
    }
}

impl<const B: Word> Mul<&Repr<B>> for Repr<B> {
    type Output = Repr<B>;
    #[inline]
    fn mul(self, rhs: &Repr<B>) -> Repr<B> {
        (&self) * rhs
    }
}
impl<const B: Word> Mul<Repr<B>> for &Repr<B> {
    type Output = Repr<B>;
    #[inline]
    fn mul(self, rhs: Repr<B>) -> Repr<B> {
        self * &rhs
    }
}
impl<const B: Word> Mul<Repr<B>> for Repr<B> {
    type Output = Repr<B>;
    #[inline]
    fn mul(self, rhs: Repr<B>) -> Repr<B> {
        (&self) * &rhs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashu_int::IBig;

    fn r<const B: Word>(sig: i128, exp: isize) -> Repr<B> {
        Repr::new(IBig::from(sig), exp)
    }

    #[test]
    fn add_same_exponent() {
        assert_eq!(&r::<10>(3, 0) + &r::<10>(4, 0), r::<10>(7, 0));
    }

    #[test]
    fn add_aligns_exponents() {
        // 3 + 0.4 = 3.4
        assert_eq!(&r::<10>(3, 0) + &r::<10>(4, -1), r::<10>(34, -1));
        // 1 + 0.00001 = 1.00001 (exact; the small operand's digits are all retained)
        assert_eq!(&r::<10>(1, 0) + &r::<10>(1, -5), r::<10>(100001, -5));
    }

    #[test]
    fn add_neg_zero_is_identity() {
        let nz = Repr::<10>::neg_zero();
        let x = r::<10>(5, -2);
        assert_eq!(&nz + &x, x);
        assert_eq!(&x + &nz, x);
        // -0 + -0 = -0: the zero short-circuit returns the other operand unchanged.
        assert_eq!(&nz + &nz, Repr::<10>::neg_zero());
    }

    #[test]
    fn add_cancellation_is_positive_zero() {
        let z = &r::<10>(1, 0) + &r::<10>(-1, 0);
        assert_eq!(z, Repr::<10>::zero());
        assert!(z.is_pos_zero());
    }

    #[test]
    fn sub_basic() {
        assert_eq!(&r::<10>(5, 0) - &r::<10>(3, 0), r::<10>(2, 0));
        assert_eq!(&r::<10>(3, 0) - &r::<10>(5, 0), r::<10>(-2, 0));
        // x - 0 = x (subtraction is a + (-0); the zero short-circuit keeps x's representation)
        assert_eq!(&r::<10>(5, 0) - &Repr::<10>::zero(), r::<10>(5, 0));
    }

    #[test]
    fn mul_basic() {
        assert_eq!(&r::<10>(3, 0) * &r::<10>(4, 0), r::<10>(12, 0));
        // significands multiply, exponents add: 3e2 * 2e-1 = 6e1
        assert_eq!(&r::<10>(3, 2) * &r::<10>(2, -1), r::<10>(6, 1));
        assert_eq!(&r::<2>(3, 0) * &r::<2>(3, 0), r::<2>(9, 0));
    }

    #[test]
    fn mul_zero_product_sign() {
        // (+0) * (+5) = +0
        assert_eq!(&Repr::<10>::zero() * &r::<10>(5, 0), Repr::<10>::zero());
        // (+0) * (-5) = -0 (the XOR sign of the operands is attached to the zero product)
        let prod = &Repr::<10>::zero() * &r::<10>(-5, 0);
        assert!(prod.is_neg_zero());
    }

    #[test]
    fn ref_val_combos() {
        let a = r::<10>(2, 0);
        let b = r::<10>(3, 0);
        assert_eq!(&a + &b, r::<10>(5, 0));
        assert_eq!(a.clone() + &b, r::<10>(5, 0));
        assert_eq!(&a + b.clone(), r::<10>(5, 0));
        assert_eq!(a.clone() + b.clone(), r::<10>(5, 0));

        let c = r::<10>(7, 0);
        let d = r::<10>(4, 0);
        assert_eq!(&c - &d, r::<10>(3, 0));
        assert_eq!(c.clone() - &d, r::<10>(3, 0));
        assert_eq!(&c - d.clone(), r::<10>(3, 0));
        assert_eq!(c.clone() - d.clone(), r::<10>(3, 0));

        let e = r::<10>(6, 0);
        let f = r::<10>(7, 0);
        assert_eq!(&e * &f, r::<10>(42, 0));
        assert_eq!(e.clone() * &f, r::<10>(42, 0));
        assert_eq!(&e * f.clone(), r::<10>(42, 0));
        assert_eq!(e.clone() * f.clone(), r::<10>(42, 0));
    }
}
