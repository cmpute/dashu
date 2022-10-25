use core::ops::{Mul, MulAssign};

use dashu_base::Gcd;

use crate::{
    helper_macros::{impl_binop_assign_by_taking, impl_binop_with_macro},
    rbig::{RBig, Relaxed},
    repr::Repr,
};

impl RBig {
    #[inline]
    pub fn square(&self) -> Self {
        Self(Repr {
            numerator: self.numerator().square(),
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
    #[inline]
    pub fn square(&self) -> Self {
        Self(Repr {
            numerator: self.numerator().square(),
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
