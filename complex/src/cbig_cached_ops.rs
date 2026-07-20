//! Operators for [`CachedCBig`] — all binary/unary operators and the transcendental convenience
//! methods, with cache preservation.
//!
//! Structured to mirror `dashu-float`'s `fbig_cached_ops.rs`: every repeated operator family is a
//! macro, and the transcendentals split into cache-threading forwards (which bypass [`CBig`]'s
//! convenience layer — it hardcodes `None` — and call the complex [`Context`] with `Some(cache)`)
//! and plain delegations (no cache involved, just preserve the handle).

use alloc::rc::Rc;
use core::iter::{Product, Sum};
use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use dashu_base::Inverse;
use dashu_float::round::{ErrorBounds, Round};
use dashu_float::FBig;
use dashu_int::{IBig, Word};

use crate::cbig::CBig;
use crate::cbig_cached::CachedCBig;

// ===========================================================================
// CachedCBig op CachedCBig (preserves LHS cache) — owned + Assign
// ===========================================================================

macro_rules! impl_cached_cbig_binop {
    ($Op:ident, $op:ident) => {
        impl<R: Round, const B: Word> $Op<CachedCBig<R, B>> for CachedCBig<R, B> {
            type Output = CachedCBig<R, B>;
            #[inline]
            fn $op(self, rhs: CachedCBig<R, B>) -> Self::Output {
                CachedCBig::from_cbig($Op::$op(self.cbig, rhs.cbig), &self.cache)
            }
        }
    };
}
macro_rules! impl_cached_cbig_binop_assign {
    ($OpAssign:ident, $op_assign:ident, $Op:ident, $op:ident) => {
        impl<R: Round, const B: Word> $OpAssign<CachedCBig<R, B>> for CachedCBig<R, B> {
            #[inline]
            fn $op_assign(&mut self, rhs: CachedCBig<R, B>) {
                self.cbig = $Op::$op(self.cbig.clone(), rhs.cbig);
            }
        }
    };
}
impl_cached_cbig_binop!(Add, add);
impl_cached_cbig_binop!(Sub, sub);
impl_cached_cbig_binop!(Mul, mul);
impl_cached_cbig_binop!(Div, div);
impl_cached_cbig_binop_assign!(AddAssign, add_assign, Add, add);
impl_cached_cbig_binop_assign!(SubAssign, sub_assign, Sub, sub);
impl_cached_cbig_binop_assign!(MulAssign, mul_assign, Mul, mul);
impl_cached_cbig_binop_assign!(DivAssign, div_assign, Div, div);

// ===========================================================================
// CachedCBig op CBig and CBig op CachedCBig (cache preserved from CachedCBig side)
// ===========================================================================

macro_rules! impl_cached_binop_one_way_with_cbig {
    ($Op:ident, $op:ident) => {
        impl<R: Round, const B: Word> $Op<CBig<R, B>> for CachedCBig<R, B> {
            type Output = CachedCBig<R, B>;
            #[inline]
            fn $op(self, rhs: CBig<R, B>) -> Self::Output {
                CachedCBig::from_cbig($Op::$op(self.cbig, rhs), &self.cache)
            }
        }
        impl<'l, R: Round, const B: Word> $Op<CBig<R, B>> for &'l CachedCBig<R, B> {
            type Output = CachedCBig<R, B>;
            #[inline]
            fn $op(self, rhs: CBig<R, B>) -> Self::Output {
                CachedCBig::from_cbig($Op::$op(self.cbig.clone(), rhs), &self.cache)
            }
        }
        impl<'r, R: Round, const B: Word> $Op<&'r CBig<R, B>> for CachedCBig<R, B> {
            type Output = CachedCBig<R, B>;
            #[inline]
            fn $op(self, rhs: &CBig<R, B>) -> Self::Output {
                CachedCBig::from_cbig($Op::$op(self.cbig, rhs.clone()), &self.cache)
            }
        }
        impl<'l, 'r, R: Round, const B: Word> $Op<&'r CBig<R, B>> for &'l CachedCBig<R, B> {
            type Output = CachedCBig<R, B>;
            #[inline]
            fn $op(self, rhs: &CBig<R, B>) -> Self::Output {
                CachedCBig::from_cbig($Op::$op(self.cbig.clone(), rhs.clone()), &self.cache)
            }
        }
    };
}

macro_rules! impl_cached_binop_reverse_with_cbig {
    ($Op:ident, $op:ident) => {
        impl<R: Round, const B: Word> $Op<CachedCBig<R, B>> for CBig<R, B> {
            type Output = CachedCBig<R, B>;
            #[inline]
            fn $op(self, rhs: CachedCBig<R, B>) -> Self::Output {
                CachedCBig::from_cbig($Op::$op(self, rhs.cbig), &rhs.cache)
            }
        }
        impl<'l, R: Round, const B: Word> $Op<CachedCBig<R, B>> for &'l CBig<R, B> {
            type Output = CachedCBig<R, B>;
            #[inline]
            fn $op(self, rhs: CachedCBig<R, B>) -> Self::Output {
                CachedCBig::from_cbig($Op::$op(self.clone(), rhs.cbig), &rhs.cache)
            }
        }
        impl<'r, R: Round, const B: Word> $Op<&'r CachedCBig<R, B>> for CBig<R, B> {
            type Output = CachedCBig<R, B>;
            #[inline]
            fn $op(self, rhs: &CachedCBig<R, B>) -> Self::Output {
                CachedCBig::from_cbig($Op::$op(self, rhs.cbig.clone()), &rhs.cache)
            }
        }
        impl<'l, 'r, R: Round, const B: Word> $Op<&'r CachedCBig<R, B>> for &'l CBig<R, B> {
            type Output = CachedCBig<R, B>;
            #[inline]
            fn $op(self, rhs: &CachedCBig<R, B>) -> Self::Output {
                CachedCBig::from_cbig($Op::$op(self.clone(), rhs.cbig.clone()), &rhs.cache)
            }
        }
    };
}

// Assign: CachedCBig op= CBig
macro_rules! impl_cached_binop_assign_with_cbig {
    ($OpAssign:ident, $op_assign:ident, $Op:ident, $op:ident) => {
        impl<R: Round, const B: Word> $OpAssign<CBig<R, B>> for CachedCBig<R, B> {
            #[inline]
            fn $op_assign(&mut self, rhs: CBig<R, B>) {
                self.cbig = $Op::$op(self.cbig.clone(), rhs);
            }
        }
        impl<R: Round, const B: Word> $OpAssign<&CBig<R, B>> for CachedCBig<R, B> {
            #[inline]
            fn $op_assign(&mut self, rhs: &CBig<R, B>) {
                self.cbig = $Op::$op(self.cbig.clone(), rhs.clone());
            }
        }
    };
}

macro_rules! impl_cached_binop_with_cbig {
    ($Op:ident, $op:ident, $OpAssign:ident, $op_assign:ident) => {
        impl_cached_binop_one_way_with_cbig!($Op, $op);
        impl_cached_binop_reverse_with_cbig!($Op, $op);
        impl_cached_binop_assign_with_cbig!($OpAssign, $op_assign, $Op, $op);
    };
}

impl_cached_binop_with_cbig!(Add, add, AddAssign, add_assign);
impl_cached_binop_with_cbig!(Sub, sub, SubAssign, sub_assign);
impl_cached_binop_with_cbig!(Mul, mul, MulAssign, mul_assign);
impl_cached_binop_with_cbig!(Div, div, DivAssign, div_assign);

// ===========================================================================
// CachedCBig op FBig and FBig op CachedCBig (scalar Mul/Div only — matches CBig's surface)
// ===========================================================================

macro_rules! impl_cached_binop_one_way_with_fbig {
    ($Op:ident, $op:ident) => {
        impl<R: Round, const B: Word> $Op<FBig<R, B>> for CachedCBig<R, B> {
            type Output = CachedCBig<R, B>;
            #[inline]
            fn $op(self, rhs: FBig<R, B>) -> Self::Output {
                CachedCBig::from_cbig($Op::$op(self.cbig, rhs), &self.cache)
            }
        }
        impl<'l, R: Round, const B: Word> $Op<FBig<R, B>> for &'l CachedCBig<R, B> {
            type Output = CachedCBig<R, B>;
            #[inline]
            fn $op(self, rhs: FBig<R, B>) -> Self::Output {
                CachedCBig::from_cbig($Op::$op(self.cbig.clone(), rhs), &self.cache)
            }
        }
        impl<'r, R: Round, const B: Word> $Op<&'r FBig<R, B>> for CachedCBig<R, B> {
            type Output = CachedCBig<R, B>;
            #[inline]
            fn $op(self, rhs: &FBig<R, B>) -> Self::Output {
                CachedCBig::from_cbig($Op::$op(self.cbig, rhs.clone()), &self.cache)
            }
        }
        impl<'l, 'r, R: Round, const B: Word> $Op<&'r FBig<R, B>> for &'l CachedCBig<R, B> {
            type Output = CachedCBig<R, B>;
            #[inline]
            fn $op(self, rhs: &FBig<R, B>) -> Self::Output {
                CachedCBig::from_cbig($Op::$op(self.cbig.clone(), rhs.clone()), &self.cache)
            }
        }
    };
}

macro_rules! impl_cached_binop_reverse_with_fbig {
    ($Op:ident, $op:ident) => {
        impl<R: Round, const B: Word> $Op<CachedCBig<R, B>> for FBig<R, B> {
            type Output = CachedCBig<R, B>;
            #[inline]
            fn $op(self, rhs: CachedCBig<R, B>) -> Self::Output {
                CachedCBig::from_cbig($Op::$op(self, rhs.cbig), &rhs.cache)
            }
        }
        impl<'l, R: Round, const B: Word> $Op<CachedCBig<R, B>> for &'l FBig<R, B> {
            type Output = CachedCBig<R, B>;
            #[inline]
            fn $op(self, rhs: CachedCBig<R, B>) -> Self::Output {
                CachedCBig::from_cbig($Op::$op(self.clone(), rhs.cbig), &rhs.cache)
            }
        }
        impl<'r, R: Round, const B: Word> $Op<&'r CachedCBig<R, B>> for FBig<R, B> {
            type Output = CachedCBig<R, B>;
            #[inline]
            fn $op(self, rhs: &CachedCBig<R, B>) -> Self::Output {
                CachedCBig::from_cbig($Op::$op(self, rhs.cbig.clone()), &rhs.cache)
            }
        }
        impl<'l, 'r, R: Round, const B: Word> $Op<&'r CachedCBig<R, B>> for &'l FBig<R, B> {
            type Output = CachedCBig<R, B>;
            #[inline]
            fn $op(self, rhs: &CachedCBig<R, B>) -> Self::Output {
                CachedCBig::from_cbig($Op::$op(self.clone(), rhs.cbig.clone()), &rhs.cache)
            }
        }
    };
}

impl_cached_binop_one_way_with_fbig!(Mul, mul);
impl_cached_binop_reverse_with_fbig!(Mul, mul);
impl_cached_binop_one_way_with_fbig!(Div, div);
impl_cached_binop_reverse_with_fbig!(Div, div);

// ===========================================================================
// Unary operators
// ===========================================================================

impl<R: Round, const B: Word> Neg for CachedCBig<R, B> {
    type Output = CachedCBig<R, B>;
    #[inline]
    fn neg(self) -> Self::Output {
        CachedCBig::from_cbig(-self.cbig, &self.cache)
    }
}

impl<R: Round, const B: Word> Neg for &CachedCBig<R, B> {
    type Output = CachedCBig<R, B>;
    #[inline]
    fn neg(self) -> Self::Output {
        CachedCBig::from_cbig(-&self.cbig, &self.cache)
    }
}

impl<R: Round, const B: Word> Inverse for CachedCBig<R, B> {
    type Output = CachedCBig<R, B>;
    #[inline]
    fn inv(self) -> Self::Output {
        CachedCBig::from_cbig(Inverse::inv(self.cbig), &self.cache)
    }
}

impl<R: Round, const B: Word> Inverse for &CachedCBig<R, B> {
    type Output = CachedCBig<R, B>;
    #[inline]
    fn inv(self) -> Self::Output {
        CachedCBig::from_cbig(Inverse::inv(&self.cbig), &self.cache)
    }
}

// ===========================================================================
// Sum / Product
//
// The value computation delegates to CBig's impls (fold with +/*). The result keeps the first
// element's cache handle (empty iterator → Default).
// ===========================================================================

impl<R: Round, const B: Word> Sum for CachedCBig<R, B> {
    fn sum<I: Iterator<Item = Self>>(mut iter: I) -> Self {
        match iter.next() {
            Some(first) => {
                let cache = Rc::clone(&first.cache);
                let cbig: CBig<R, B> = core::iter::once(first.cbig)
                    .chain(iter.map(|c| c.cbig))
                    .sum();
                CachedCBig::from_cbig(cbig, &cache)
            }
            None => Self::default(),
        }
    }
}

impl<'a, R: Round, const B: Word> Sum<&'a CachedCBig<R, B>> for CachedCBig<R, B> {
    fn sum<I: Iterator<Item = &'a CachedCBig<R, B>>>(mut iter: I) -> Self {
        match iter.next() {
            Some(first) => {
                let cache = Rc::clone(&first.cache);
                let cbig: CBig<R, B> = core::iter::once(first.cbig.clone())
                    .chain(iter.map(|c| c.cbig.clone()))
                    .sum();
                CachedCBig::from_cbig(cbig, &cache)
            }
            None => Self::default(),
        }
    }
}

impl<R: Round, const B: Word> Product for CachedCBig<R, B> {
    fn product<I: Iterator<Item = Self>>(mut iter: I) -> Self {
        match iter.next() {
            Some(first) => {
                let cache = Rc::clone(&first.cache);
                let cbig: CBig<R, B> = core::iter::once(first.cbig)
                    .chain(iter.map(|c| c.cbig))
                    .product();
                CachedCBig::from_cbig(cbig, &cache)
            }
            None => Self::default(),
        }
    }
}

impl<'a, R: Round, const B: Word> Product<&'a CachedCBig<R, B>> for CachedCBig<R, B> {
    fn product<I: Iterator<Item = &'a CachedCBig<R, B>>>(mut iter: I) -> Self {
        match iter.next() {
            Some(first) => {
                let cache = Rc::clone(&first.cache);
                let cbig: CBig<R, B> = core::iter::once(first.cbig.clone())
                    .chain(iter.map(|c| c.cbig.clone()))
                    .product();
                CachedCBig::from_cbig(cbig, &cache)
            }
            None => Self::default(),
        }
    }
}

// ===========================================================================
// Math functions — macros are defined at module scope and invoked inside the impl block below
// (macro_rules! cannot be defined inside an `impl`).
// ===========================================================================

/// Forward a unary transcendental to the complex [`Context`](crate::repr::Context) method, threading
/// this value's shared cache and panicking on error (mirrors the convenience-layer unwrap).
macro_rules! forward_cached {
    ($name:ident => $ctx:ident) => {
        #[doc = concat!("See [`CBig::", stringify!($name), "`].")]
        #[inline]
        pub fn $name(&self) -> CachedCBig<R, B> {
            let ctx = self.cbig.context();
            let cbig = {
                let mut c = self.cache.borrow_mut();
                ctx.unwrap_cfp(ctx.$ctx::<B>(&self.cbig, Some(&mut *c)))
            };
            CachedCBig::from_cbig(cbig, &self.cache)
        }
    };
}

/// Delegate a unary method to the inner [`CBig`] (no cache involved — just preserve the handle).
macro_rules! delegate_to_cbig {
    ($name:ident) => {
        #[doc = concat!("See [`CBig::", stringify!($name), "`].")]
        #[inline]
        pub fn $name(&self) -> CachedCBig<R, B> {
            CachedCBig::from_cbig(self.cbig.$name(), &self.cache)
        }
    };
    ($name:ident($arg:ident: $arg_ty:ty)) => {
        #[doc = concat!("See [`CBig::", stringify!($name), "`].")]
        #[inline]
        pub fn $name(&self, $arg: $arg_ty) -> CachedCBig<R, B> {
            CachedCBig::from_cbig(self.cbig.$name($arg), &self.cache)
        }
    };
}

impl<R: Round, const B: Word> CachedCBig<R, B> {
    // non-cache delegations
    delegate_to_cbig!(sqr);
    delegate_to_cbig!(conj);
    delegate_to_cbig!(proj);
    delegate_to_cbig!(mul_i(negative: bool));
    delegate_to_cbig!(powi(exp: IBig));

    /// The squared modulus `re² + im²` (a real [`FBig`]) (see [`CBig::norm`]).
    #[inline]
    pub fn norm(&self) -> FBig<R, B> {
        self.cbig.norm()
    }
}

// Transcendentals route through the Ziv-backed (or Ziv-dependent) real/complex methods, which
// require `R: ErrorBounds`.
impl<R: ErrorBounds, const B: Word> CachedCBig<R, B> {
    // cache-threading transcendentals
    forward_cached!(ln => log);
    forward_cached!(exp => exp);
    forward_cached!(sin => sin);
    forward_cached!(cos => cos);
    forward_cached!(tan => tan);
    forward_cached!(asin => asin);
    forward_cached!(acos => acos);
    forward_cached!(atan => atan);

    // `sqrt` and `abs` route through `Context::hypot`, which is now Ziv-backed (`R: ErrorBounds`).
    delegate_to_cbig!(sqrt);

    /// The modulus `|z|` (a real [`FBig`]) (see [`CBig::abs`]).
    #[inline]
    pub fn abs(&self) -> FBig<R, B> {
        self.cbig.abs()
    }

    /// The argument `atan2(im, re)` (a real [`FBig`]); threads the cache into `atan2`
    /// (see [`CBig::arg`]).
    #[inline]
    pub fn arg(&self) -> FBig<R, B> {
        let ctx = self.cbig.context();
        let mut c = self.cache.borrow_mut();
        ctx.float()
            .unwrap_fp(ctx.arg::<B>(&self.cbig, Some(&mut *c)))
    }

    /// Complex power `self^w` (see [`CBig::powf`]). Threads the cache into the inner `log`/`exp`.
    #[inline]
    pub fn powf(&self, w: &Self) -> Self {
        let ctx = self.cbig.context();
        let cbig = {
            let mut c = self.cache.borrow_mut();
            ctx.unwrap_cfp(ctx.powf::<B>(&self.cbig, &w.cbig, Some(&mut *c)))
        };
        CachedCBig::from_cbig(cbig, &self.cache)
    }

    /// Sine and cosine together (see [`CBig::sin_cos`]). Threads the cache into the real
    /// `sin_cos`/`sinh_cosh`.
    #[inline]
    pub fn sin_cos(&self) -> (Self, Self) {
        let ctx = self.cbig.context();
        let (s, c) = {
            let mut guard = self.cache.borrow_mut();
            ctx.sin_cos::<B>(&self.cbig, Some(&mut *guard))
        };
        (
            CachedCBig::from_cbig(ctx.unwrap_cfp(s), &self.cache),
            CachedCBig::from_cbig(ctx.unwrap_cfp(c), &self.cache),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;
    use core::cell::RefCell;
    use dashu_float::round::mode;
    use dashu_float::ConstCache;

    type F = FBig<mode::HalfAway, 10>;
    type C = CBig<mode::HalfAway, 10>;
    type CC = CachedCBig<mode::HalfAway, 10>;

    fn c(re: i32, im: i32) -> C {
        let mk = |v: i32| -> F { F::from(v).with_precision(53).value() };
        CBig::from_parts(mk(re), mk(im))
    }
    fn cc(re: i32, im: i32) -> CC {
        CachedCBig::from(c(re, im))
    }
    /// Assert a CachedCBig equals the matching CBig by value.
    fn eq(ccval: CC, expected: &C) {
        assert_eq!(ccval.as_cbig(), expected);
    }

    #[test]
    fn arithmetic_matches_cbig() {
        let (a, b) = (c(2, 1), c(3, 4));
        let (ca, cb) = (cc(2, 1), cc(3, 4));
        eq(ca.clone() + cb.clone(), &(&a + &b));
        eq(ca.clone() - cb.clone(), &(&a - &b));
        eq(ca.clone() * cb.clone(), &(&a * &b));
        eq(ca.clone() / cb.clone(), &(&a / &b));

        // Cross-type with CBig (both directions): ca == a by value, so ca + a == a + a.
        eq(ca.clone() + a.clone(), &(&a + &a));
        eq(a.clone() + ca.clone(), &(&a + &a));

        // Scalar Mul/Div with FBig.
        let s = F::from(2);
        eq(ca.clone() * s.clone(), &(&a * &s));
        eq(s.clone() * ca.clone(), &(&s * &a));
        eq(ca.clone() / s.clone(), &(&a / &s));
    }

    #[test]
    fn neg_inverse_matches_cbig() {
        let (a, ca) = (c(2, 1), cc(2, 1));
        eq(-ca.clone(), &(-&a));
        eq(Inverse::inv(ca.clone()), &Inverse::inv(&a));
    }

    #[test]
    fn sum_product_matches_cbig() {
        let vals = [(1, 2), (3, 4), (-1, 1)];
        let cvals: Vec<C> = vals.iter().map(|&(r, i)| c(r, i)).collect();
        let ccvals: Vec<CC> = vals.iter().map(|&(r, i)| cc(r, i)).collect();

        eq(ccvals.iter().sum::<CC>(), &cvals.iter().sum::<C>());
        eq(ccvals.clone().into_iter().sum::<CC>(), &cvals.clone().into_iter().sum::<C>());
        eq(ccvals.iter().product::<CC>(), &cvals.iter().product::<C>());
        eq(
            ccvals.clone().into_iter().product::<CC>(),
            &cvals.clone().into_iter().product::<C>(),
        );
        // Empty iterator → Default (0+0i).
        eq(core::iter::empty::<CC>().sum::<CC>(), &C::default());
    }

    #[test]
    fn transcendentals_match_cbig() {
        let (a, ca) = (c(2, 1), cc(2, 1));
        // cache-threading transcendentals
        eq(ca.clone().ln(), &a.clone().ln());
        eq(ca.clone().exp(), &a.clone().exp());
        eq(ca.clone().sin(), &a.clone().sin());
        eq(ca.clone().cos(), &a.clone().cos());
        eq(ca.clone().tan(), &a.clone().tan());
        eq(ca.clone().asin(), &a.clone().asin());
        eq(ca.clone().acos(), &a.clone().acos());
        eq(ca.clone().atan(), &a.clone().atan());
        eq(ca.clone().powf(&ca.clone()), &a.clone().powf(&a));

        let (s_c, c_c) = ca.clone().sin_cos();
        let (s, c) = a.clone().sin_cos();
        eq(s_c, &s);
        eq(c_c, &c);

        // non-cache delegations
        eq(ca.clone().sqr(), &a.clone().sqr());
        eq(ca.clone().sqrt(), &a.clone().sqrt());
        eq(ca.clone().conj(), &a.clone().conj());
        eq(ca.clone().proj(), &a.clone().proj());
        eq(ca.clone().powi(3.into()), &a.clone().powi(3.into()));

        // real-valued decompositions (return FBig, match CBig exactly)
        assert_eq!(ca.norm(), a.norm());
        assert_eq!(ca.abs(), a.abs());
        assert_eq!(ca.arg(), a.arg());
    }

    #[test]
    fn cache_populated_and_survives_arithmetic() {
        // Transcendentals populate the shared cache (the whole point of the wrapper).
        let z = cc(2, 1);
        assert_eq!(z.cache().total_terms(), 0);
        let _ = z.clone().ln();
        assert!(z.cache().total_terms() > 0);
        assert!(z.cache().total_words() > 0);

        // a and b share one handle; the sum keeps it so the subsequent ln() reuses it,
        // and the value still matches the CBig computation.
        let mk = |v: i32| -> F { F::from(v).with_precision(53).value() };
        let h = Rc::new(RefCell::new(ConstCache::new()));
        let a = CC::new(C::from_parts(mk(2), mk(0)), h.clone());
        let b = CC::new(C::from_parts(mk(3), mk(0)), h.clone());
        let sum_ln = (a.clone() + b.clone()).ln().into_cbig();
        let expected = (C::from_parts(mk(2), F::ZERO) + C::from_parts(mk(3), F::ZERO)).ln();
        assert_eq!(sum_ln, expected);
        assert!(h.borrow().total_terms() > 0);
    }
}
