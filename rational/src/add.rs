#![allow(clippy::suspicious_arithmetic_impl)] // Clippy doesn't like that add/sub is implemented with mul.

use core::ops::{Add, AddAssign, Sub, SubAssign};
use dashu_base::Gcd;
use dashu_int::{IBig, UBig};

use crate::{
    helper_macros::{impl_binop_assign_by_taking, impl_binop_with_int, impl_binop_with_macro},
    rbig::{RBig, Relaxed},
    repr::Repr,
};

macro_rules! impl_add_or_sub_with_rbig {
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
impl_binop_with_macro!(impl Add, add, impl_add_or_sub_with_rbig);
impl_binop_with_macro!(impl Sub, sub, impl_add_or_sub_with_rbig);
impl_binop_assign_by_taking!(impl AddAssign for RBig, add_assign, add);
impl_binop_assign_by_taking!(impl SubAssign for RBig, sub_assign, sub);

macro_rules! impl_addsub_with_relaxed {
    (
        $a:ident, $b:ident, $c:ident, $d:ident,
        $ra:ident, $rb:ident, $rc:ident, $rd:ident, $method:ident
    ) => {{
        let _unused = ($ra, $rc);
        Relaxed::from_parts(($a * $rd).$method($c * $rb), $b * $d)
    }};
}
impl_binop_with_macro!(impl Add for Relaxed, add, impl_addsub_with_relaxed);
impl_binop_with_macro!(impl Sub for Relaxed, sub, impl_addsub_with_relaxed);
impl_binop_assign_by_taking!(impl AddAssign for Relaxed, add_assign, add);
impl_binop_assign_by_taking!(impl SubAssign for Relaxed, sub_assign, sub);

macro_rules! impl_addsub_int_with_rbig {
    (
        $a:ident, $b:ident, $i:ident,
        $ra:ident, $rb:ident, $ri:ident, $method:ident
    ) => {{
        let _unused = ($ra, $ri);
        RBig(Repr {
            numerator: $a.$method($rb * $i),
            denominator: $b,
        })
    }};
}
macro_rules! impl_int_sub_rbig {
    // sub is not commutative
    (
        $a:ident, $b:ident, $i:ident,
        $ra:ident, $rb:ident, $ri:ident, $method:ident
    ) => {{
        let _unused = ($ra, $ri);
        RBig(Repr {
            numerator: ($rb * $i).$method($a),
            denominator: $b,
        })
    }};
}
impl_binop_with_int!(impl Add<UBig>, add, impl_addsub_int_with_rbig);
impl_binop_with_int!(impl Add<IBig>, add, impl_addsub_int_with_rbig);
impl_binop_with_int!(impl Sub<UBig>, sub, impl_addsub_int_with_rbig);
impl_binop_with_int!(impl Sub<IBig>, sub, impl_addsub_int_with_rbig);
impl_binop_with_int!(impl Add for UBig, add, impl_addsub_int_with_rbig);
impl_binop_with_int!(impl Add for IBig, add, impl_addsub_int_with_rbig);
impl_binop_with_int!(impl Sub for UBig, sub, impl_int_sub_rbig);
impl_binop_with_int!(impl Sub for IBig, sub, impl_int_sub_rbig);

macro_rules! impl_addsub_int_with_relaxed {
    (
        $a:ident, $b:ident, $i:ident,
        $ra:ident, $rb:ident, $ri:ident, $method:ident
    ) => {{
        let _unused = ($ra, $ri);
        Relaxed(Repr {
            numerator: $a.$method($rb * $i),
            denominator: $b,
        })
    }};
}
macro_rules! impl_int_sub_relaxed {
    (
        $a:ident, $b:ident, $i:ident,
        $ra:ident, $rb:ident, $ri:ident, $method:ident
    ) => {{
        let _unused = ($ra, $ri);
        Relaxed(Repr {
            numerator: ($rb * $i).$method($a),
            denominator: $b,
        })
    }};
}
impl_binop_with_int!(impl Add<UBig>, add, Relaxed, impl_addsub_int_with_relaxed);
impl_binop_with_int!(impl Add<IBig>, add, Relaxed, impl_addsub_int_with_relaxed);
impl_binop_with_int!(impl Sub<UBig>, sub, Relaxed, impl_addsub_int_with_relaxed);
impl_binop_with_int!(impl Sub<IBig>, sub, Relaxed, impl_addsub_int_with_relaxed);
impl_binop_with_int!(impl Add for UBig, add, Relaxed, impl_addsub_int_with_relaxed);
impl_binop_with_int!(impl Add for IBig, add, Relaxed, impl_addsub_int_with_relaxed);
impl_binop_with_int!(impl Sub for UBig, sub, Relaxed, impl_int_sub_relaxed);
impl_binop_with_int!(impl Sub for IBig, sub, Relaxed, impl_int_sub_relaxed);
