//! A cached floating-point number — [`FBig`] with a shared constant cache attached.

use alloc::rc::Rc;
use core::cell::RefCell;

use dashu_base::Sign;

use crate::error::panic_unlimited_precision;
use crate::fbig::FBig;
use crate::math::cache::ConstCache;
use crate::repr::{Context, Repr, Word};
use crate::round::{mode, Round, Rounded};
use crate::utils::digit_len;

/// A floating-point number that carries a shared handle to a [`ConstCache`].
///
/// It is functionally an [`FBig`]: same in-memory representation (`fbig`),
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
/// When two `CachedFBig` values with different cache handles interact in a binary
/// operation, the LHS (left-hand-side) cache is preserved in the result. For
/// `FBig op CachedFBig`, the `CachedFBig` operand's cache is preserved.
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
    pub(crate) fbig: FBig<R, B>,
    pub(crate) cache: Rc<RefCell<ConstCache>>,
}

impl<R: Round, const B: Word> CachedFBig<R, B> {
    /// Wrap an [`FBig`], sharing the given cache handle.
    #[inline]
    pub fn new(value: FBig<R, B>, cache: Rc<RefCell<ConstCache>>) -> Self {
        Self { fbig: value, cache }
    }

    /// Build from raw parts, sharing the given cache handle.
    #[inline]
    pub fn from_repr(repr: Repr<B>, context: Context<R>, cache: Rc<RefCell<ConstCache>>) -> Self {
        Self {
            fbig: FBig::new(repr, context),
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
    pub(crate) fn from_fbig(fbig: FBig<R, B>, cache: &Rc<RefCell<ConstCache>>) -> Self {
        Self {
            fbig,
            cache: Rc::clone(cache),
        }
    }

    /// Borrow the inner [`FBig`].
    #[inline]
    pub fn as_fbig(&self) -> &FBig<R, B> {
        &self.fbig
    }

    /// Drop the cache handle and return the underlying [`FBig`].
    #[inline]
    pub fn into_fbig(self) -> FBig<R, B> {
        self.fbig
    }

    /// Borrow the shared constant cache immutably.
    ///
    /// Use this to inspect cache state, e.g. `cached.cache().total_terms()`.
    #[inline]
    pub fn cache(&self) -> impl core::ops::Deref<Target = ConstCache> + '_ {
        self.cache.borrow()
    }

    /// Clear all cached constant state, freeing the underlying memory.
    ///
    /// The next transcendental operation will recompute constants from scratch.
    #[inline]
    pub fn clear_cache(&self) {
        self.cache.borrow_mut().clear();
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
        self.fbig.context.precision
    }

    /// Number of significant digits (see [`FBig::digits`]).
    #[inline]
    pub fn digits(&self) -> usize {
        self.fbig.repr.digits()
    }

    /// The associated context.
    #[inline]
    pub const fn context(&self) -> Context<R> {
        self.fbig.context
    }

    /// The underlying representation.
    #[inline]
    pub const fn repr(&self) -> &Repr<B> {
        &self.fbig.repr
    }

    /// Consume and return the underlying representation.
    #[inline]
    pub fn into_repr(self) -> Repr<B> {
        self.fbig.repr
    }

    /// Sign of the number (see [`FBig::sign`]).
    #[inline]
    pub const fn sign(&self) -> Sign {
        self.fbig.repr.sign()
    }

    /// Change precision, preserving the handle (see [`FBig::with_precision`]).
    pub fn with_precision(&self, precision: usize) -> Rounded<Self> {
        self.fbig
            .clone()
            .with_precision(precision)
            .map(|f| Self::from_fbig(f, &self.cache))
    }

    /// Change rounding mode, preserving the handle (see [`FBig::with_rounding`]).
    pub fn with_rounding<NewR: Round>(&self) -> CachedFBig<NewR, B> {
        CachedFBig::from_fbig(self.fbig.clone().with_rounding::<NewR>(), &self.cache)
    }
}

impl<R: Round, const B: Word> CachedFBig<R, B> {
    /// ULP of the number (see [`FBig::ulp`]).
    pub fn ulp(&self) -> Self {
        if self.fbig.context.precision == 0 {
            panic_unlimited_precision();
        }
        let repr = Repr {
            significand: dashu_int::IBig::ONE,
            exponent: self.fbig.repr.exponent + self.fbig.repr.digits() as isize
                - self.fbig.context.precision as isize,
        };
        Self::from_repr(repr, self.fbig.context, Rc::clone(&self.cache))
    }

    /// Convert to an integer (see [`FBig::to_int`]).
    pub fn to_int(&self) -> Rounded<dashu_int::IBig> {
        self.fbig.clone().to_int()
    }

    /// Convert to `f32` (see [`FBig::to_f32`]).
    pub fn to_f32(&self) -> Rounded<f32> {
        self.fbig.clone().to_f32()
    }

    /// Convert to `f64` (see [`FBig::to_f64`]).
    pub fn to_f64(&self) -> Rounded<f64> {
        self.fbig.clone().to_f64()
    }

    /// Construct from significand + exponent, with a fresh cache (see [`FBig::from_parts`]).
    pub fn from_parts(significand: dashu_int::IBig, exponent: isize) -> Self {
        let precision = digit_len::<B>(&significand).max(1);
        let repr = Repr::new(significand, exponent);
        Self::with_cache(repr, Context::new(precision))
    }
}

// ---------------------------------------------------------------------------
// From / Into
// ---------------------------------------------------------------------------

impl<R: Round, const B: Word> From<FBig<R, B>> for CachedFBig<R, B> {
    #[inline]
    fn from(fbig: FBig<R, B>) -> Self {
        Self::new(fbig, Rc::new(RefCell::new(ConstCache::new())))
    }
}

impl<R: Round, const B: Word> From<CachedFBig<R, B>> for FBig<R, B> {
    #[inline]
    fn from(cached: CachedFBig<R, B>) -> Self {
        cached.into_fbig()
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
// Clone / Default / Debug / comparisons
// ---------------------------------------------------------------------------

impl<R: Round, const B: Word> Clone for CachedFBig<R, B> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            fbig: self.fbig.clone(),
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
            .field("repr", &self.fbig.repr)
            .field("precision", &self.fbig.context.precision)
            .finish()
    }
}

impl<R1: Round, R2: Round, const B: Word> PartialEq<CachedFBig<R2, B>> for CachedFBig<R1, B> {
    #[inline]
    fn eq(&self, other: &CachedFBig<R2, B>) -> bool {
        // value equality, mirroring FBig (compares the representation only).
        self.fbig.repr == other.fbig.repr
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
            let cached = CachedFBig::<mode::HalfAway, 10>::pi(precision, &h).into_fbig();
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

        assert_eq!(x.clone().ln().into_fbig(), y.clone().ln());
        assert_eq!(x.clone().exp().into_fbig(), y.clone().exp());
        assert_eq!(x.clone().sin().into_fbig(), y.clone().sin());
        assert_eq!(x.clone().cos().into_fbig(), y.clone().cos());
        assert_eq!(x.clone().exp_m1().into_fbig(), y.clone().exp_m1());
        assert_eq!(x.clone().ln_1p().into_fbig(), y.clone().ln_1p());
        assert_eq!(x.powf(&x.clone()).into_fbig(), y.clone().powf(&y));
    }

    #[test]
    fn test_cache_extension_matches_scratch() {
        // Extending π 100 -> 1000 through one shared handle must equal a from-scratch compute.
        let h = handle();
        let _pi_100 = CachedFBig::<mode::HalfAway, 10>::pi(100, &h);
        let pi_1000 = CachedFBig::<mode::HalfAway, 10>::pi(1000, &h).into_fbig();
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
        let sum_ln = (a.clone() + b.clone()).ln().into_fbig();
        let expected = (fbig(2, 30) + fbig(3, 30)).ln();
        assert_eq!(sum_ln, expected);
    }

    #[test]
    fn test_arithmetic_matches_fbig() {
        let a =
            CachedFBig::<mode::HalfAway, 10>::with_cache(Repr::new(2.into(), 0), Context::new(20));
        let b =
            CachedFBig::<mode::HalfAway, 10>::with_cache(Repr::new(3.into(), 0), Context::new(20));

        assert_eq!((a.clone() + b.clone()).into_fbig(), fbig(2, 20) + fbig(3, 20));
        assert_eq!((a.clone() - b.clone()).into_fbig(), fbig(2, 20) - fbig(3, 20));
        assert_eq!((a.clone() * b.clone()).into_fbig(), fbig(2, 20) * fbig(3, 20));
        assert_eq!((a.clone() / b.clone()).into_fbig(), fbig(2, 20) / fbig(3, 20));
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

    #[test]
    fn test_arithmetic_with_fbig() {
        let a =
            CachedFBig::<mode::HalfAway, 10>::with_cache(Repr::new(2.into(), 0), Context::new(20));
        let b = fbig(3, 20);

        // CachedFBig op FBig — cache preserved (LHS)
        let c = a.clone() + b.clone();
        assert_eq!(c.into_fbig(), fbig(2, 20) + fbig(3, 20));

        // FBig op CachedFBig — cache preserved (RHS)
        let d = b.clone() + a.clone();
        assert_eq!(d.into_fbig(), fbig(3, 20) + fbig(2, 20));

        // Sub, Mul, Div
        assert_eq!((a.clone() - b.clone()).into_fbig(), fbig(2, 20) - fbig(3, 20));
        assert_eq!((a.clone() * b.clone()).into_fbig(), fbig(2, 20) * fbig(3, 20));
        assert_eq!((a.clone() / b.clone()).into_fbig(), fbig(2, 20) / fbig(3, 20));
    }

    #[test]
    fn test_arithmetic_with_primitives() {
        let a =
            CachedFBig::<mode::HalfAway, 10>::with_cache(Repr::new(2.into(), 0), Context::new(20));

        // CachedFBig op primitive
        assert_eq!((a.clone() + 3u8).into_fbig(), fbig(2, 20) + 3u8);
        assert_eq!((a.clone() - 3i32).into_fbig(), fbig(2, 20) - 3i32);
        assert_eq!((a.clone() * 4u64).into_fbig(), fbig(2, 20) * 4u64);

        // Primitive op CachedFBig
        assert_eq!((3u8 + a.clone()).into_fbig(), 3u8 + fbig(2, 20));
        assert_eq!((10i32 - a.clone()).into_fbig(), 10i32 - fbig(2, 20));
    }

    #[test]
    fn test_cache_size() {
        let x = CachedFBig::<mode::HalfAway, 10>::with_cache(
            Repr::new(1234.into(), -3),
            Context::new(50),
        );
        let _ = x.ln();
        // After computing ln(1.234), the cache should have some state
        assert!(x.cache().total_terms() > 0);
        assert!(x.cache().total_words() > 0);
    }

    #[test]
    fn test_cache_clear() {
        let x = CachedFBig::<mode::HalfAway, 10>::with_cache(
            Repr::new(1234.into(), -3),
            Context::new(50),
        );
        let before_clear = x.ln().into_fbig();
        assert!(x.cache().total_terms() > 0);

        x.clear_cache();
        assert_eq!(x.cache().total_terms(), 0);
        assert_eq!(x.cache().total_words(), 0);

        // After clearing, recomputation still produces the same result
        let after_clear = x.ln().into_fbig();
        assert_eq!(after_clear, before_clear);
    }
}
