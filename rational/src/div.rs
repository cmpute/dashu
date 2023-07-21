#![allow(clippy::suspicious_arithmetic_impl)] // Clippy doesn't like that div is implemented with mul.

use core::ops::{Div, DivAssign, Rem, RemAssign};
use dashu_base::{DivEuclid, DivRemEuclid, Gcd, Inverse, RemEuclid, UnsignedAbs};
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

impl_binop_with_macro!(impl Div, div, impl_div_with_rbig);
impl_binop_with_macro!(impl Div for Relaxed, div, impl_div_with_relaxed);
impl_binop_assign_by_taking!(impl DivAssign for RBig, div_assign, div);
impl_binop_assign_by_taking!(impl DivAssign for Relaxed, div_assign, div);

// the strategy here for Rem is consistent with dashu_float::FBig
macro_rules! impl_rem_with_rbig {
    (
        $a:ident, $b:ident, $c:ident, $d:ident,
        $ra:ident, $rb:ident, $rc:ident, $rd:ident, $method:ident
    ) => {{
        let _unused = ($ra, $rc);
        let g_bd = Gcd::gcd($rb, $rd);

        // a/b % c/d = (ad % bc)/bd
        let ddg = $d / &g_bd;
        let left = &ddg * $a;
        let right = $rb / &g_bd * $c.unsigned_abs();

        let (sign, r1) = left.$method(&right).into_parts();
        let r2 = right - &r1;
        let rem = if r1 < r2 {
            IBig::from_parts(sign, r1)
        } else {
            IBig::from_parts(-sign, r2)
        };

        RBig::from_parts(rem, $b * ddg)
    }};
}
macro_rules! impl_rem_with_relaxed {
    (
        $a:ident, $b:ident, $c:ident, $d:ident,
        $ra:ident, $rb:ident, $rc:ident, $rd:ident, $method:ident
    ) => {{
        let _unused = ($ra, $rc);

        let (left, right) = ($a * $rd, $c.unsigned_abs() * $rb);
        let (sign, r1) = left.$method(&right).into_parts();
        let r2 = right - &r1;
        let rem = if r1 < r2 {
            IBig::from_parts(sign, r1)
        } else {
            IBig::from_parts(-sign, r2)
        };

        Relaxed::from_parts(rem, $b * $d)
    }};
}
impl_binop_with_macro!(impl Rem, rem, impl_rem_with_rbig);
impl_binop_with_macro!(impl Rem for Relaxed, rem, impl_rem_with_relaxed);
impl_binop_assign_by_taking!(impl RemAssign for RBig, rem_assign, rem);
impl_binop_assign_by_taking!(impl RemAssign for Relaxed, rem_assign, rem);

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
impl_binop_with_int!(impl Div<UBig>, div, RBig, impl_rbig_div_ubig);
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
impl_binop_with_int!(impl Div<IBig>, div, Relaxed, impl_relaxed_div_ibig);
impl_binop_with_int!(impl Div<UBig>, div, Relaxed, impl_relaxed_div_ubig);

macro_rules! impl_euclid_div {
    (
        $a:ident, $b:ident, $c:ident, $d:ident,
        $ra:ident, $rb:ident, $rc:ident, $rd:ident, $method:ident
    ) => {{
        if $rc.is_zero() {
            panic_divide_by_0()
        }

        let _unused = ($ra, $rb, $rd);
        ($a * $d).$method($b * $c)
    }};
}
impl_binop_with_macro!(impl DivEuclid, div_euclid -> IBig, impl_euclid_div);
impl_binop_with_macro!(impl DivEuclid for Relaxed, div_euclid -> IBig, impl_euclid_div);

macro_rules! impl_euclid_rem_with_rbig {
    (
        $a:ident, $b:ident, $c:ident, $d:ident,
        $ra:ident, $rb:ident, $rc:ident, $rd:ident, $method:ident
    ) => {{
        let _unused = ($ra, $rc);
        let g_bd = Gcd::gcd($rb, $rd);

        let ddg = $d / &g_bd;
        let left = &ddg * $a;
        let right = $rb / &g_bd * $c;
        RBig::from_parts(left.$method(right).into(), $b * ddg)
    }};
}
macro_rules! impl_euclid_rem_with_relaxed {
    (
        $a:ident, $b:ident, $c:ident, $d:ident,
        $ra:ident, $rb:ident, $rc:ident, $rd:ident, $method:ident
    ) => {{
        let _unused = ($ra, $rc);

        let (left, right) = ($a * $rd, $c * $rb);
        Relaxed::from_parts(left.$method(right).into(), $b * $d)
    }};
}
impl_binop_with_macro!(impl RemEuclid, rem_euclid, impl_euclid_rem_with_rbig);
impl_binop_with_macro!(impl RemEuclid for Relaxed, rem_euclid, impl_euclid_rem_with_relaxed);

macro_rules! impl_euclid_divrem_with_rbig {
    (
        $a:ident, $b:ident, $c:ident, $d:ident,
        $ra:ident, $rb:ident, $rc:ident, $rd:ident, $method:ident
    ) => {{
        let _unused = ($ra, $rc);
        let g_bd = Gcd::gcd($rb, $rd);

        let ddg = $d / &g_bd;
        let left = &ddg * $a;
        let right = $rb / &g_bd * $c;
        let (q, r) = left.$method(right).into();
        (q, RBig::from_parts(r.into(), $b * ddg))
    }};
}
macro_rules! impl_euclid_divrem_with_relaxed {
    (
        $a:ident, $b:ident, $c:ident, $d:ident,
        $ra:ident, $rb:ident, $rc:ident, $rd:ident, $method:ident
    ) => {{
        let _unused = ($ra, $rc);

        let (left, right) = ($a * $rd, $c * $rb);
        let (q, r) = left.$method(right).into();
        (q, Relaxed::from_parts(r.into(), $b * $d))
    }};
}
impl_binop_with_macro!(impl DivRemEuclid for RBig, div_rem_euclid, OutputDiv = IBig, OutputRem = RBig, impl_euclid_divrem_with_rbig);
impl_binop_with_macro!(impl DivRemEuclid for Relaxed, div_rem_euclid, OutputDiv = IBig, OutputRem = Relaxed, impl_euclid_divrem_with_relaxed);

impl Inverse for Repr {
    type Output = Repr;

    #[inline]
    fn inv(self) -> Repr {
        let (sign, num) = self.numerator.into_parts();
        Repr {
            numerator: IBig::from_parts(sign, self.denominator),
            denominator: num,
        }
    }
}

impl Inverse for RBig {
    type Output = RBig;
    #[inline]
    fn inv(self) -> RBig {
        RBig(self.0.inv())
    }
}

impl Inverse for &RBig {
    type Output = RBig;
    #[inline]
    fn inv(self) -> RBig {
        RBig(self.0.clone().inv())
    }
}

impl Inverse for Relaxed {
    type Output = Relaxed;
    #[inline]
    fn inv(self) -> Relaxed {
        Relaxed(self.0.inv())
    }
}

impl Inverse for &Relaxed {
    type Output = Relaxed;
    #[inline]
    fn inv(self) -> Relaxed {
        Relaxed(self.0.clone().inv())
    }
}

macro_rules! impl_eq_for_prim_ints {
    ($($t:ty)*) => {$(
        impl PartialEq<$t> for RBig {
            #[inline]
            fn eq(&self, rhs: &$t) -> bool {
                *self == RBig::from(*rhs)
            }
        }

        impl PartialEq<RBig> for $t {
            #[inline]
            fn eq(&self, rhs: &RBig) -> bool {
                *rhs == RBig::from(*self)
            }
        }
    )*};
}
impl_eq_for_prim_ints!(u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize);