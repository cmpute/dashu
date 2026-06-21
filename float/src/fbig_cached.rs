//! A cached floating-point number — [`FBig`] with a shared constant cache attached.

use alloc::rc::Rc;
use core::cell::RefCell;
use core::ops::{
    Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Sub, SubAssign,
};

use dashu_base::{Abs, Sign};

use crate::error::panic_unlimited_precision;
use crate::fbig::FBig;
use crate::math::cache::ConstCache;
use crate::math::FpResult;
use crate::repr::{Context, Repr, Word};
use crate::round::{mode, Round, Rounded};
use crate::utils::digit_len;

/// A floating-point number that carries a shared handle to a [`ConstCache`].
///
/// It is functionally an [`FBig`]: same in-memory representation (`repr` + `context`),
/// plus an [`Rc<RefCell<ConstCache>>`] handle. The difference is that the
/// transcendental operations (`ln`, `exp`, `sin`, `cos`, …, `pi`, base conversion)
/// thread that handle into the underlying [`Context`] methods, so they reuse and
/// progressively extend the cached exact binary-splitting state instead of
/// recomputing constants from scratch on every call.
///
/// `Context`/`FBig` themselves stay `Copy` + `Send` + `Sync` + `no_std` (so
/// [`static_fbig!`](dashu_macros::static_fbig!) keeps working); only this cached
/// wrapper is `!Send + !Sync`, because it shares state through an `Rc<RefCell<..>>`.
/// To share one cache across threads, build an analogous type over
/// `Arc<Mutex<ConstCache>>` instead (the [`Context`] methods accept
/// `Option<&mut ConstCache>`, independent of the container).
///
/// Every value-producing operation returns a `CachedFBig` that preserves the
/// handle, so `(a + b).ln().exp()` stays cached throughout — no silent cache loss.
///
/// # Examples
///
/// ```
/// use core::cell::RefCell;
/// use core::str::FromStr;
/// use dashu_float::{CachedFBig, ConstCache, Context};
/// use dashu_float::round::mode::HalfAway;
/// use std::rc::Rc;
///
/// let cache = Rc::new(RefCell::new(ConstCache::new()));
/// // build a cached decimal number 1.234
/// let x = CachedFBig::<HalfAway, 10>::with_cache(
///     dashu_float::Repr::new(1234.into(), -3),
///     Context::new(50),
/// );
///
/// // ln / exp reuse the same shared cache handle
/// let _ = x.clone().ln().exp();
/// ```
pub struct CachedFBig<R: Round = mode::Zero, const B: Word = 2> {
    pub(crate) repr: Repr<B>,
    pub(crate) context: Context<R>,
    pub(crate) cache: Rc<RefCell<ConstCache>>,
}

impl<R: Round, const B: Word> CachedFBig<R, B> {
    /// Wrap an [`FBig`], sharing the given cache handle.
    #[inline]
    pub fn new(value: FBig<R, B>, cache: Rc<RefCell<ConstCache>>) -> Self {
        let FBig { repr, context } = value;
        Self {
            repr,
            context,
            cache,
        }
    }

    /// Build from raw parts, sharing the given cache handle.
    #[inline]
    pub fn from_repr(repr: Repr<B>, context: Context<R>, cache: Rc<RefCell<ConstCache>>) -> Self {
        Self {
            repr,
            context,
            cache,
        }
    }

    /// Build from raw parts with a fresh, exclusive cache.
    #[inline]
    pub fn with_cache(repr: Repr<B>, context: Context<R>) -> Self {
        Self::from_repr(repr, context, Rc::new(RefCell::new(ConstCache::new())))
    }

    /// Build a `CachedFBig` from an [`FBig`] result, re-attaching this value's
    /// shared cache handle (cloned cheaply via `Rc`).
    #[inline]
    fn from_fbig(fbig: FBig<R, B>, cache: &Rc<RefCell<ConstCache>>) -> Self {
        let FBig { repr, context } = fbig;
        Self {
            repr,
            context,
            cache: Rc::clone(cache),
        }
    }

    /// Drop the cache handle and return the underlying [`FBig`].
    #[inline]
    pub fn into_uncached(self) -> FBig<R, B> {
        FBig::new(self.repr, self.context)
    }

    /// π at `precision` base-`B` digits, reusing/extending `cache`.
    pub fn pi(precision: usize, cache: &Rc<RefCell<ConstCache>>) -> Self {
        let fbig = {
            let mut c = cache.borrow_mut();
            Context::<R>::new(precision).pi::<B>(Some(&mut *c)).value()
        };
        Self::from_fbig(fbig, cache)
    }

    // ----- accessors -----

    /// Maximum precision set for the number (see [`FBig::precision`]).
    #[inline]
    pub const fn precision(&self) -> usize {
        self.context.precision
    }

    /// Number of significant digits (see [`FBig::digits`]).
    #[inline]
    pub fn digits(&self) -> usize {
        self.repr.digits()
    }

    /// The associated context.
    #[inline]
    pub const fn context(&self) -> Context<R> {
        self.context
    }

    /// The underlying representation.
    #[inline]
    pub const fn repr(&self) -> &Repr<B> {
        &self.repr
    }

    /// Consume and return the underlying representation.
    #[inline]
    pub fn into_repr(self) -> Repr<B> {
        self.repr
    }

    /// Sign of the number (see [`FBig::sign`]).
    #[inline]
    pub const fn sign(&self) -> Sign {
        self.repr.sign()
    }

    // ----- transcendentals (consult/extend the shared cache) -----

    /// Natural logarithm (see [`FBig::ln`]).
    pub fn ln(&self) -> Self {
        let fbig = {
            let mut c = self.cache.borrow_mut();
            self.context.ln::<B>(&self.repr, Some(&mut *c)).value()
        };
        Self::from_fbig(fbig, &self.cache)
    }

    /// `log(x+1)` (see [`FBig::ln_1p`]).
    pub fn ln_1p(&self) -> Self {
        let fbig = {
            let mut c = self.cache.borrow_mut();
            self.context.ln_1p::<B>(&self.repr, Some(&mut *c)).value()
        };
        Self::from_fbig(fbig, &self.cache)
    }

    /// Exponential `eˣ` (see [`FBig::exp`]).
    pub fn exp(&self) -> Self {
        let fbig = {
            let mut c = self.cache.borrow_mut();
            self.context.exp::<B>(&self.repr, Some(&mut *c)).value()
        };
        Self::from_fbig(fbig, &self.cache)
    }

    /// `eˣ-1` (see [`FBig::exp_m1`]).
    pub fn exp_m1(&self) -> Self {
        let fbig = {
            let mut c = self.cache.borrow_mut();
            self.context.exp_m1::<B>(&self.repr, Some(&mut *c)).value()
        };
        Self::from_fbig(fbig, &self.cache)
    }

    /// `self^exp` (see [`FBig::powf`]).
    pub fn powf(&self, exp: &Self) -> Self {
        let context = Context::max(self.context, exp.context);
        let fbig = {
            let mut c = self.cache.borrow_mut();
            context
                .powf::<B>(&self.repr, &exp.repr, Some(&mut *c))
                .value()
        };
        Self::from_fbig(fbig, &self.cache)
    }

    /// Sine (see [`FBig::sin`]).
    pub fn sin(&self) -> Self {
        let fbig = {
            let mut c = self.cache.borrow_mut();
            self.context
                .sin::<B>(&self.repr, Some(&mut *c))
                .value(&self.context)
        };
        Self::from_fbig(fbig, &self.cache)
    }

    /// Cosine (see [`FBig::cos`]).
    pub fn cos(&self) -> Self {
        let fbig = {
            let mut c = self.cache.borrow_mut();
            self.context
                .cos::<B>(&self.repr, Some(&mut *c))
                .value(&self.context)
        };
        Self::from_fbig(fbig, &self.cache)
    }

    /// Sine and cosine together (see [`FBig::sin_cos`]).
    pub fn sin_cos(&self) -> (Self, Self) {
        let (s, c) = {
            let mut guard = self.cache.borrow_mut();
            let cache = Some(&mut *guard);
            let (s, c) = self.context.sin_cos::<B>(&self.repr, cache);
            (s.value(&self.context), c.value(&self.context))
        };
        (Self::from_fbig(s, &self.cache), Self::from_fbig(c, &self.cache))
    }

    /// Tangent (see [`FBig::tan`]).
    pub fn tan(&self) -> FpResult<B> {
        let mut c = self.cache.borrow_mut();
        self.context.tan::<B>(&self.repr, Some(&mut *c))
    }

    /// Arcsine (see [`FBig::asin`]).
    pub fn asin(&self) -> FpResult<B> {
        let mut c = self.cache.borrow_mut();
        self.context.asin::<B>(&self.repr, Some(&mut *c))
    }

    /// Arccosine (see [`FBig::acos`]).
    pub fn acos(&self) -> FpResult<B> {
        let mut c = self.cache.borrow_mut();
        self.context.acos::<B>(&self.repr, Some(&mut *c))
    }

    /// Arctangent (see [`FBig::atan`]).
    pub fn atan(&self) -> Self {
        let fbig = {
            let mut c = self.cache.borrow_mut();
            self.context
                .atan::<B>(&self.repr, Some(&mut *c))
                .value(&self.context)
        };
        Self::from_fbig(fbig, &self.cache)
    }

    /// `atan2(y, x)` (see [`FBig::atan2`]).
    pub fn atan2(&self, x: &Self) -> FpResult<B> {
        let mut c = self.cache.borrow_mut();
        self.context.atan2::<B>(&self.repr, &x.repr, Some(&mut *c))
    }

    // ----- pure ops (no constants; delegate to FBig, preserve handle) -----

    /// Integer power (see [`FBig::powi`]).
    pub fn powi(&self, exp: dashu_int::IBig) -> Self {
        let fbig = FBig::new(self.repr.clone(), self.context).powi(exp);
        Self::from_fbig(fbig, &self.cache)
    }

    /// Square (see [`FBig::sqr`]).
    pub fn sqr(&self) -> Self {
        let fbig = FBig::new(self.repr.clone(), self.context).sqr();
        Self::from_fbig(fbig, &self.cache)
    }

    /// Cube (see [`FBig::cubic`]).
    pub fn cubic(&self) -> Self {
        let fbig = FBig::new(self.repr.clone(), self.context).cubic();
        Self::from_fbig(fbig, &self.cache)
    }

    /// Square root (see [`Context::sqrt`]).
    pub fn sqrt(&self) -> Self {
        let fbig = self.context.sqrt::<B>(&self.repr).value();
        Self::from_fbig(fbig, &self.cache)
    }

    /// Multiplicative inverse (see [`Context::inv`]).
    pub fn inv(&self) -> Self {
        let fbig = self.context.inv::<B>(&self.repr).value();
        Self::from_fbig(fbig, &self.cache)
    }

    /// Reciprocal `1/x` — alias for [`Self::inv`].
    pub fn reciprocal(&self) -> Self {
        self.inv()
    }

    /// Change precision, preserving the handle (see [`FBig::with_precision`]).
    pub fn with_precision(&self, precision: usize) -> Rounded<Self> {
        let fbig = FBig::new(self.repr.clone(), self.context).with_precision(precision);
        fbig.map(|f| Self::from_fbig(f, &self.cache))
    }

    /// Change rounding mode, preserving the handle (see [`FBig::with_rounding`]).
    pub fn with_rounding<NewR: Round>(&self) -> CachedFBig<NewR, B> {
        let fbig = FBig::new(self.repr.clone(), self.context).with_rounding::<NewR>();
        CachedFBig::from_fbig(fbig, &self.cache)
    }
}

impl<R: Round, const B: Word> CachedFBig<R, B> {
    /// ULP of the number (see [`FBig::ulp`]).
    pub fn ulp(&self) -> Self {
        if self.context.precision == 0 {
            panic_unlimited_precision();
        }
        let repr = Repr {
            significand: dashu_int::IBig::ONE,
            exponent: self.repr.exponent + self.repr.digits() as isize
                - self.context.precision as isize,
        };
        Self::from_repr(repr, self.context, Rc::clone(&self.cache))
    }

    /// Convert to an integer (see [`FBig::to_int`]).
    pub fn to_int(&self) -> Rounded<dashu_int::IBig> {
        FBig::new(self.repr.clone(), self.context).to_int()
    }

    /// Convert to `f32` (see [`FBig::to_f32`]).
    pub fn to_f32(&self) -> Rounded<f32> {
        FBig::new(self.repr.clone(), self.context).to_f32()
    }

    /// Convert to `f64` (see [`FBig::to_f64`]).
    pub fn to_f64(&self) -> Rounded<f64> {
        FBig::new(self.repr.clone(), self.context).to_f64()
    }

    /// Construct from significand + exponent, with a fresh cache (see [`FBig::from_parts`]).
    pub fn from_parts(significand: dashu_int::IBig, exponent: isize) -> Self {
        let precision = digit_len::<B>(&significand).max(1);
        let repr = Repr::new(significand, exponent);
        Self::with_cache(repr, Context::new(precision))
    }
}

impl<R: Round, const B: Word> FBig<R, B> {
    /// Attach a shared cache handle, turning this [`FBig`] into a [`CachedFBig`].
    #[inline]
    pub fn into_cached(self, cache: Rc<RefCell<ConstCache>>) -> CachedFBig<R, B> {
        CachedFBig::new(self, cache)
    }
}

// ---------------------------------------------------------------------------
// operators (owned operands, preserve the cache handle)
// ---------------------------------------------------------------------------

macro_rules! impl_cached_binop {
    ($Op:ident, $op:ident) => {
        impl<R: Round, const B: Word> $Op<CachedFBig<R, B>> for CachedFBig<R, B> {
            type Output = CachedFBig<R, B>;
            #[inline]
            fn $op(self, rhs: CachedFBig<R, B>) -> Self::Output {
                let lf = FBig::new(self.repr, self.context);
                let rf = FBig::new(rhs.repr, rhs.context);
                CachedFBig::from_fbig($Op::$op(lf, rf), &self.cache)
            }
        }
    };
}
impl_cached_binop!(Add, add);
impl_cached_binop!(Sub, sub);
impl_cached_binop!(Mul, mul);
impl_cached_binop!(Div, div);
impl_cached_binop!(Rem, rem);

macro_rules! impl_cached_binop_assign {
    ($OpAssign:ident, $op_assign:ident, $Op:ident, $op:ident) => {
        impl<R: Round, const B: Word> $OpAssign<CachedFBig<R, B>> for CachedFBig<R, B> {
            #[inline]
            fn $op_assign(&mut self, rhs: CachedFBig<R, B>) {
                let lf = FBig::new(self.repr.clone(), self.context);
                let rf = FBig::new(rhs.repr, rhs.context);
                let res = $Op::$op(lf, rf);
                let FBig { repr, context } = res;
                self.repr = repr;
                self.context = context;
            }
        }
    };
}
impl_cached_binop_assign!(AddAssign, add_assign, Add, add);
impl_cached_binop_assign!(SubAssign, sub_assign, Sub, sub);
impl_cached_binop_assign!(MulAssign, mul_assign, Mul, mul);
impl_cached_binop_assign!(DivAssign, div_assign, Div, div);
impl_cached_binop_assign!(RemAssign, rem_assign, Rem, rem);

impl<R: Round, const B: Word> Neg for CachedFBig<R, B> {
    type Output = CachedFBig<R, B>;
    #[inline]
    fn neg(self) -> Self::Output {
        let lf = FBig::new(self.repr, self.context);
        CachedFBig::from_fbig(-lf, &self.cache)
    }
}

impl<R: Round, const B: Word> Abs for CachedFBig<R, B> {
    type Output = CachedFBig<R, B>;
    #[inline]
    fn abs(self) -> Self::Output {
        let lf = FBig::new(self.repr, self.context);
        CachedFBig::from_fbig(Abs::abs(lf), &self.cache)
    }
}

// ---------------------------------------------------------------------------
// Clone / Default / comparisons
// ---------------------------------------------------------------------------

impl<R: Round, const B: Word> Clone for CachedFBig<R, B> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            repr: self.repr.clone(),
            context: self.context,
            cache: Rc::clone(&self.cache),
        }
    }
}

impl<R: Round, const B: Word> Default for CachedFBig<R, B> {
    /// Default value: 0 with a fresh cache.
    #[inline]
    fn default() -> Self {
        Self::with_cache(Repr::zero(), Context::new(0))
    }
}

impl<R: Round, const B: Word> core::fmt::Debug for CachedFBig<R, B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CachedFBig")
            .field("repr", &self.repr)
            .field("precision", &self.context.precision)
            .finish()
    }
}

impl<R1: Round, R2: Round, const B: Word> PartialEq<CachedFBig<R2, B>> for CachedFBig<R1, B> {
    #[inline]
    fn eq(&self, other: &CachedFBig<R2, B>) -> bool {
        // value equality, mirroring FBig (compares the representation only).
        self.repr == other.repr
    }
}

impl<R: Round, const B: Word> Eq for CachedFBig<R, B> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::round::mode;
    use alloc::format;

    fn handle() -> Rc<RefCell<ConstCache>> {
        Rc::new(RefCell::new(ConstCache::new()))
    }

    /// An `FBig` with value `n` at the given precision (so inexact results match the
    /// `CachedFBig` operands built at the same precision).
    fn fbig(n: i32, prec: usize) -> FBig<mode::HalfAway, 10> {
        FBig::from_repr(Repr::new(n.into(), 0), Context::new(prec))
    }

    #[test]
    fn test_pi_matches_fbig() {
        for &precision in &[10usize, 50, 100] {
            let h = handle();
            let cached = CachedFBig::<mode::HalfAway, 10>::pi(precision, &h).into_uncached();
            let direct = FBig::<mode::HalfAway, 10>::pi(precision);
            assert_eq!(cached, direct, "pi mismatch at precision {precision}");
        }
    }

    #[test]
    fn test_transcendentals_match_fbig() {
        let x = CachedFBig::<mode::HalfAway, 10>::with_cache(
            Repr::new(1234.into(), -3), // 1.234
            Context::new(50),
        );
        let y = FBig::<mode::HalfAway, 10>::from_repr(Repr::new(1234.into(), -3), Context::new(50));

        assert_eq!(x.clone().ln().into_uncached(), y.clone().ln());
        assert_eq!(x.clone().exp().into_uncached(), y.clone().exp());
        assert_eq!(x.clone().sin().into_uncached(), y.clone().sin());
        assert_eq!(x.clone().cos().into_uncached(), y.clone().cos());
        assert_eq!(x.clone().exp_m1().into_uncached(), y.clone().exp_m1());
        assert_eq!(x.clone().ln_1p().into_uncached(), y.clone().ln_1p());
        assert_eq!(x.powf(&x.clone()).into_uncached(), y.clone().powf(&y));
    }

    #[test]
    fn test_cache_extension_matches_scratch() {
        // Extending π 100 -> 1000 through one shared handle must equal a from-scratch compute.
        let h = handle();
        let _pi_100 = CachedFBig::<mode::HalfAway, 10>::pi(100, &h);
        let pi_1000 = CachedFBig::<mode::HalfAway, 10>::pi(1000, &h).into_uncached();
        let direct = Context::<mode::HalfAway>::new(1000).pi::<10>(None).value();
        assert_eq!(pi_1000, direct);
    }

    #[test]
    fn test_cache_survives_arithmetic() {
        // a and b share one cache handle; the sum must keep it so the subsequent
        // ln() reuses the same shared cache.
        let h = handle();
        let a = CachedFBig::<mode::HalfAway, 10>::from_repr(
            Repr::new(2.into(), 0),
            Context::new(30),
            h.clone(),
        );
        let b = CachedFBig::<mode::HalfAway, 10>::from_repr(
            Repr::new(3.into(), 0),
            Context::new(30),
            h.clone(),
        );
        let sum_ln = (a.clone() + b.clone()).ln().into_uncached();
        let expected = (fbig(2, 30) + fbig(3, 30)).ln();
        assert_eq!(sum_ln, expected);
    }

    #[test]
    fn test_arithmetic_matches_fbig() {
        let a =
            CachedFBig::<mode::HalfAway, 10>::with_cache(Repr::new(2.into(), 0), Context::new(20));
        let b =
            CachedFBig::<mode::HalfAway, 10>::with_cache(Repr::new(3.into(), 0), Context::new(20));

        assert_eq!((a.clone() + b.clone()).into_uncached(), fbig(2, 20) + fbig(3, 20));
        assert_eq!((a.clone() - b.clone()).into_uncached(), fbig(2, 20) - fbig(3, 20));
        assert_eq!((a.clone() * b.clone()).into_uncached(), fbig(2, 20) * fbig(3, 20));
        assert_eq!((a.clone() / b.clone()).into_uncached(), fbig(2, 20) / fbig(3, 20));
    }

    #[test]
    fn test_debug_compiles() {
        let x = CachedFBig::<mode::HalfAway, 10>::with_cache(
            Repr::new(1234.into(), -3),
            Context::new(50),
        );
        let s = format!("{:?}", x);
        assert!(s.contains("CachedFBig"));
    }
}
