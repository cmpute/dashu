use core::ops::{Mul, MulAssign};

use dashu_base::Gcd;
use dashu_int::{IBig, UBig};

use crate::{
    helper_macros::{impl_binop_assign_by_taking, impl_binop_with_int, impl_binop_with_macro},
    rbig::{RBig, Relaxed},
    repr::Repr,
};

impl Repr {
    #[inline]
    fn sqr(&self) -> Self {
        Self {
            numerator: self.numerator.sqr().into(),
            denominator: self.denominator.sqr(),
        }
    }

    #[inline]
    fn cubic(&self) -> Self {
        Self {
            numerator: self.numerator.cubic(),
            denominator: self.denominator.cubic(),
        }
    }

    /// `self^n` for any `n` in `isize`.
    ///
    /// For a negative exponent the value is reciprocated first: `x.pow(-n) ==
    /// 1 / x.pow(n)` for a nonzero `x`. A zero base raised to a negative power is
    /// a division by zero and panics, matching the rest of the rationals API.
    #[inline]
    fn pow(&self, n: isize) -> Self {
        if n >= 0 {
            Self {
                numerator: self.numerator.pow(n as usize),
                denominator: self.denominator.pow(n as usize),
            }
        } else {
            // self^n = (denominator / numerator)^|n|. Strip the numerator's
            // sign onto the new numerator so the denominator stays a positive
            // UBig. `into_parts` yields (Positive, ZERO) for a zero numerator,
            // which we reject before inverting it.
            let exp = n.unsigned_abs();
            let (sign, numerator_mag) = self.numerator.clone().into_parts();
            if numerator_mag.is_zero() {
                crate::error::panic_divide_by_0();
            }
            // sign^exp collapses to Positive for an even exponent.
            let result_sign = if exp % 2 == 0 {
                dashu_base::Sign::Positive
            } else {
                sign
            };
            Self {
                numerator: result_sign * self.denominator.pow(exp),
                denominator: numerator_mag.pow(exp),
            }
        }
    }
}

impl RBig {
    /// Compute the square of the number (`self * self`).
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_ratio::RBig;
    /// let a = RBig::from_parts(2.into(), 3u8.into());
    /// let a2 = RBig::from_parts(4.into(), 9u8.into());
    /// assert_eq!(a.sqr(), a2);
    /// ```
    #[inline]
    pub fn sqr(&self) -> Self {
        Self(self.0.sqr())
    }

    /// Compute the cubic of the number (`self * self * self`).
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_ratio::RBig;
    /// let a = RBig::from_parts(2.into(), 3u8.into());
    /// let a3 = RBig::from_parts(8.into(), 27u8.into());
    /// assert_eq!(a.cubic(), a3);
    /// ```
    #[inline]
    pub fn cubic(&self) -> Self {
        Self(self.0.cubic())
    }

    /// Raise this number to a power of `n`.
    ///
    /// A negative exponent reciprocates first: `x.pow(-n) == 1 / x.pow(n)` for a
    /// nonzero `x` (matching `powf` on the floats). A zero base raised to a
    /// negative power is a division by zero and panics. Non-negative exponents
    /// behave exactly as before.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_ratio::RBig;
    /// let a = RBig::from_parts(2.into(), 3u8.into());
    /// let a5 = RBig::from_parts(32.into(), 243u8.into());
    /// assert_eq!(a.pow(5), a5);
    /// let a_inv = RBig::from_parts(3.into(), 2u8.into());
    /// assert_eq!(a.pow(-1), a_inv);
    /// ```
    #[inline]
    pub fn pow(&self, n: isize) -> Self {
        Self(self.0.pow(n))
    }
}

macro_rules! impl_mul_with_rbig {
    (
        $a:ident, $b:ident, $c:ident, $d:ident,
        $ra:ident, $rb:ident, $rc:ident, $rd:ident, $method:ident
    ) => {{
        // a/b * c/d = (ac)/gcd(a,d)/gcd(b,c)/(bd)
        let g_ad = $ra.gcd($rd);
        let g_bc = $rb.gcd($rc);
        RBig(Repr {
            numerator: ($a / &g_ad).$method($c / &g_bc),
            denominator: ($b / g_bc).$method($d / g_ad),
        })
    }};
}

impl_binop_with_macro!(impl Mul, mul, impl_mul_with_rbig);
impl_binop_assign_by_taking!(impl MulAssign for RBig, mul_assign, mul);

impl Relaxed {
    /// Compute the square of the number (`self * self`).
    ///
    /// See [RBig::sqr] for details.
    #[inline]
    pub fn sqr(&self) -> Self {
        Self(self.0.sqr())
    }

    /// Compute the cubic of the number (`self * self * self`).
    ///
    /// See [RBig::cubic] for details.
    #[inline]
    pub fn cubic(&self) -> Self {
        Self(self.0.cubic())
    }

    /// Raise this number to a power of `n`.
    ///
    /// See [RBig::pow] for details.
    #[inline]
    pub fn pow(&self, n: isize) -> Self {
        Self(self.0.pow(n))
    }
}

macro_rules! impl_mul_with_relaxed {
    (
        $a:ident, $b:ident, $c:ident, $d:ident,
        $ra:ident, $rb:ident, $rc:ident, $rd:ident, $method:ident
    ) => {{
        let _unused = ($ra, $rb, $rc, $rd);
        Relaxed::from_parts($a.$method($c), $b.$method($d))
    }};
}
impl_binop_with_macro!(impl Mul for Relaxed, mul, impl_mul_with_relaxed);
impl_binop_assign_by_taking!(impl MulAssign for Relaxed, mul_assign, mul);

macro_rules! impl_mul_int_with_rbig {
    (
        $a:ident, $b:ident, $i:ident,
        $ra:ident, $rb:ident, $ri:ident, $method:ident
    ) => {{
        let _unused = ($ra, $rb, $ri);
        let g = $rb.gcd($ri);
        RBig(Repr {
            numerator: $a.$method($i / &g),
            denominator: $b / g,
        })
    }};
}
impl_binop_with_int!(impl Mul<UBig>, mul, impl_mul_int_with_rbig);
impl_binop_with_int!(impl Mul<IBig>, mul, impl_mul_int_with_rbig);
impl_binop_with_int!(impl Mul for UBig, mul, impl_mul_int_with_rbig);
impl_binop_with_int!(impl Mul for IBig, mul, impl_mul_int_with_rbig);

macro_rules! impl_mul_int_with_relaxed {
    (
        $a:ident, $b:ident, $i:ident,
        $ra:ident, $rb:ident, $ri:ident, $method:ident
    ) => {{
        let _unused = ($ra, $rb, $ri);
        Relaxed::from_parts($a.$method($i), $b)
    }};
}
impl_binop_with_int!(impl Mul<UBig>, mul, Relaxed, impl_mul_int_with_relaxed);
impl_binop_with_int!(impl Mul<IBig>, mul, Relaxed, impl_mul_int_with_relaxed);
impl_binop_with_int!(impl Mul for UBig, mul, Relaxed, impl_mul_int_with_relaxed);
impl_binop_with_int!(impl Mul for IBig, mul, Relaxed, impl_mul_int_with_relaxed);

#[cfg(test)]
mod tests {
    use super::*;

    fn r(n: i64, d: u64) -> RBig {
        RBig::from_parts(IBig::from(n), UBig::from(d))
    }

    #[test]
    fn pow_non_negative() {
        let a = r(2, 3);
        assert_eq!(a.pow(0), RBig::ONE);
        assert_eq!(a.pow(1), a);
        assert_eq!(a.pow(5), r(32, 243));
        // zero base with a non-negative exponent is well defined.
        assert_eq!(RBig::ZERO.pow(0), RBig::ONE);
        assert_eq!(RBig::ZERO.pow(5), RBig::ZERO);
    }

    #[test]
    fn pow_negative_positive_base() {
        // (2/3)^-1 = 3/2,  (2/3)^-3 = 27/8
        let a = r(2, 3);
        assert_eq!(a.pow(-1), r(3, 2));
        assert_eq!(a.pow(-3), r(27, 8));
        // integer-valued base: 5^-1 = 1/5, 5^-2 = 1/25
        let five = r(5, 1);
        assert_eq!(five.pow(-1), r(1, 5));
        assert_eq!(five.pow(-2), r(1, 25));
    }

    #[test]
    fn pow_negative_base() {
        let a = r(-2, 3);
        // odd exponents keep the sign: (-2/3)^-1 = -3/2,  (-2/3)^-3 = -27/8
        assert_eq!(a.pow(-1), r(-3, 2));
        assert_eq!(a.pow(-3), r(-27, 8));
        // even exponents collapse to positive: (-2/3)^-2 = 9/4
        assert_eq!(a.pow(-2), r(9, 4));
        // reciprocal of an even unsigned power: (-6/35)^-2 = 1225/36, ^-4 = 1500625/1296
        let b = r(-6, 35);
        assert_eq!(b.pow(-2), r(1225, 36));
        assert_eq!(b.pow(-4), r(1_500_625, 1296));
    }

    #[test]
    fn pow_relaxed_matches_rbig() {
        let a = r(-2, 3);
        for n in [-3isize, -2, -1, 0, 1, 2, 3] {
            assert_eq!(a.clone().relax().pow(n).canonicalize(), a.pow(n));
        }
        // a non-canonical Relaxed (common factor 3) still yields the same value
        // after canonicalization.
        let relaxed = Relaxed::from_parts(IBig::from(6), UBig::from(9u8));
        assert_eq!(relaxed.pow(-1).canonicalize(), r(3, 2));
    }

    #[test]
    #[should_panic(expected = "Divisor or denominator must not be zero")]
    fn pow_zero_base_negative_panics() {
        let _ = RBig::ZERO.pow(-1);
    }

    #[test]
    #[should_panic(expected = "Divisor or denominator must not be zero")]
    fn pow_zero_base_negative_even_panics() {
        let _ = RBig::ZERO.pow(-2);
    }
}
