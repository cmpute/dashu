use core::ops::{Add, AddAssign, Sub, SubAssign};
use dashu_base::Gcd;

use crate::{
    helper_macros::{impl_binop_assign_by_taking, impl_binop_with_macro},
    rbig::{RBig, Relaxed},
    repr::Repr,
};

macro_rules! impl_addsub_with_rbig {
    (
        $a:ident, $b:ident, $c:ident, $d:ident,
        $ra:ident, $rb:ident, $rc:ident, $rd:ident, $method:ident
    ) => {{
        let _unused = ($ra, $rc);
        let g_bd = Gcd::gcd($rb, $rd);

        // a/b ± c/d = (ad ± bc)/bd
        let repr = if g_bd.is_one() {
            let left = $a * $rd;
            let right = $c * $rb;
            Repr {
                numerator: left.$method(right),
                denominator: $b * $d,
            }
        } else {
            let ddg = $d / &g_bd;
            let left = &ddg * $a;
            let right = $rb / &g_bd * $c;
            Repr {
                numerator: left.$method(right),
                denominator: $b * ddg,
            }
            .reduce_with_hint(g_bd)
        };

        RBig(repr)
    }};
}
impl_binop_with_macro!(Add, add, impl_addsub_with_rbig);
impl_binop_with_macro!(Sub, sub, impl_addsub_with_rbig);
impl_binop_assign_by_taking!(impl AddAssign<RBig> for RBig, add_assign, add);
impl_binop_assign_by_taking!(impl SubAssign<RBig> for RBig, sub_assign, sub);

macro_rules! impl_addsub_with_relaxed {
    (
        $a:ident, $b:ident, $c:ident, $d:ident,
        $ra:ident, $rb:ident, $rc:ident, $rd:ident, $method:ident
    ) => {{
        let _unused = ($ra, $rc);
        Relaxed::from_parts(($a * $rd).$method($c * $rb), $b * $d)
    }};
}
impl_binop_with_macro!(Add, add, Relaxed, impl_addsub_with_relaxed);
impl_binop_with_macro!(Sub, sub, Relaxed, impl_addsub_with_relaxed);
impl_binop_assign_by_taking!(impl AddAssign<Relaxed> for Relaxed, add_assign, add);
impl_binop_assign_by_taking!(impl SubAssign<Relaxed> for Relaxed, sub_assign, sub);
