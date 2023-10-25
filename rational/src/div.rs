#![allow(clippy::suspicious_arithmetic_impl)] // Clippy doesn't like that div is implemented with mul.

use core::ops::{Div, DivAssign};
use dashu_base::{Gcd, UnsignedAbs};
use dashu_int::{IBig, UBig};

use crate::{
    error::panic_divide_by_0,
    helper_macros::{impl_binop_assign_by_taking, impl_binop_with_int, impl_binop_with_macro},
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
        Relaxed::from_parts($a * $d * $c.sign(), $b * $c.unsigned_abs())
    }};
}

impl_binop_with_macro!(Div, div, Relaxed, impl_div_with_relaxed);
impl_binop_assign_by_taking!(impl DivAssign<Relaxed> for Relaxed, div_assign, div);

macro_rules! impl_rbig_div_ubig {
    (
        $a:ident, $b:ident, $i:ident,
        $ra:ident, $rb:ident, $ri:ident, $method:ident
    ) => {{
        if $ri.is_zero() {
            panic_divide_by_0()
        }

        let _unused = $rb;
        let g = $ra.gcd($ri);
        RBig(Repr {
            numerator: $a / &g,
            denominator: ($b / g) * $i,
        })
    }};
}
impl_binop_with_int!(impl Div<UBig>, div, RBig, impl_rbig_div_ubig);

macro_rules! impl_rbig_div_ibig {
    (
        $a:ident, $b:ident, $i:ident,
        $ra:ident, $rb:ident, $ri:ident, $method:ident
    ) => {{
        if $ri.is_zero() {
            panic_divide_by_0()
        }

        let _unused = $rb;
        let g = $ra.gcd($ri);
        RBig(Repr {
            numerator: $a / &g * $i.sign(),
            denominator: ($b / g) * $i.unsigned_abs(),
        })
    }};
}
impl_binop_with_int!(impl Div<IBig>, div, RBig, impl_rbig_div_ibig);

macro_rules! impl_relaxed_div_ibig {
    (
        $a:ident, $b:ident, $i:ident,
        $ra:ident, $rb:ident, $ri:ident, $method:ident
    ) => {{
        if $ri.is_zero() {
            panic_divide_by_0()
        }

        let _unused = ($ra, $rb);
        Relaxed::from_parts($a * $i.sign(), $b * $i.unsigned_abs())
    }};
}
impl_binop_with_int!(impl Div<IBig>, div, Relaxed, impl_relaxed_div_ibig);

macro_rules! impl_relaxed_div_ubig {
    (
        $a:ident, $b:ident, $i:ident,
        $ra:ident, $rb:ident, $ri:ident, $method:ident
    ) => {{
        if $ri.is_zero() {
            panic_divide_by_0()
        }

        let _unused = ($ra, $rb);
        Relaxed::from_parts($a, $b * $i)
    }};
}
impl_binop_with_int!(impl Div<UBig>, div, Relaxed, impl_relaxed_div_ubig);

// TODO: implement div_euclid, rem_euclid, div_rem_euclid
