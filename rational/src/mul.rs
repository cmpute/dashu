use core::ops::{Mul, MulAssign};

use dashu_base::Gcd;
use dashu_int::{IBig, UBig};

use crate::{
    helper_macros::{impl_binop_assign_by_taking, impl_binop_with_int, impl_binop_with_macro},
    rbig::{RBig, Relaxed},
    repr::Repr,
};

impl RBig {
    /// Compute the square of the number (`self * self`).
    /// 
    /// # Examples
    ///
    /// ```
    /// # use dashu_ratio::RBig;
    /// let a = RBig::from_parts(2.into(), 3u8.into());
    /// let a2 = RBig::from_parts(4.into(), 9u8.into());
    /// assert_eq!(a.square(), a2);
    /// ```
    #[inline]
    pub fn square(&self) -> Self {
        Self(Repr {
            numerator: self.numerator().square().into(),
            denominator: self.denominator().square(),
        })
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

impl_binop_with_macro!(Mul, mul, impl_mul_with_rbig);
impl_binop_assign_by_taking!(impl MulAssign<RBig> for RBig, mul_assign, mul);

impl Relaxed {
    /// Compute the square of the number (`self * self`).
    /// 
    /// See [RBig::square] for details.
    #[inline]
    pub fn square(&self) -> Self {
        Self(Repr {
            numerator: self.numerator().square().into(),
            denominator: self.denominator().square(),
        })
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
impl_binop_with_macro!(Mul, mul, Relaxed, impl_mul_with_relaxed);
impl_binop_assign_by_taking!(impl MulAssign<Relaxed> for Relaxed, mul_assign, mul);

macro_rules! impl_mul_int_with_rbig {
    (
        $a:ident, $b:ident, $i:ident,
        $ra:ident, $rb:ident, $ri:ident, $method:ident
    ) => {{
        let _unused = ($ra, $rb, $ri);
        let g = $rb.gcd($ri);
        RBig(Repr {
            numerator: ($a / &g).$method($i),
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
