use core::ops::{Div, DivAssign};
use dashu_base::{Gcd, UnsignedAbs};

use crate::{
    error::panic_divide_by_0,
    helper_macros::{impl_binop_assign_by_taking, impl_binop_with_macro},
    rbig::{RBig, Relaxed},
    repr::Repr,
};

macro_rules! impl_div_with_rbig {
    (
        $a:ident, $b:ident, $c:ident, $d:ident,
        $ra:ident, $rb:ident, $rc:ident, $rd:ident, $method:ident
    ) => {{
        if $rc.is_zero() {
            panic_divide_by_0()
        }

        // a/b / c/d = (ad)/gcd(a,c)/gcd(b,d)/(bc)
        let g_ac = $ra.gcd($rc);
        let g_bd = $rb.gcd($rd);
        RBig(Repr {
            numerator: ($a / &g_ac) * ($d / &g_bd) * $c.sign(),
            denominator: ($b / g_bd) * ($c.unsigned_abs() / g_ac),
        })
    }};
}

impl_binop_with_macro!(Div, div, impl_div_with_rbig);
impl_binop_assign_by_taking!(impl DivAssign<RBig> for RBig, div_assign, div);

macro_rules! impl_div_with_relaxed {
    (
        $a:ident, $b:ident, $c:ident, $d:ident,
        $ra:ident, $rb:ident, $rc:ident, $rd:ident, $method:ident
    ) => {{
        if $rc.is_zero() {
            panic_divide_by_0()
        }

        let _unused = ($ra, $rb, $rd);
        Relaxed(Repr {
            numerator: $ra * $rd * $c.sign(),
            denominator: $rb * $rc.unsigned_abs(),
        })
    }};
}

impl_binop_with_macro!(Div, div, Relaxed, impl_div_with_relaxed);
impl_binop_assign_by_taking!(impl DivAssign<Relaxed> for Relaxed, div_assign, div);
