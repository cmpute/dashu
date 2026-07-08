//! A cached complex number — [`CBig`] with a shared constant cache attached.
//!
//! This is the complex twin of [`dashu_float::CachedFBig`]: the in-memory value is a [`CBig`], plus
//! an [`Rc<RefCell<ConstCache>>`] handle shared across a computation chain. The complex
//! transcendentals are built entirely from real [`FBig`] operations, so the *same* [`ConstCache`]
//! (π, ln2, ln10, …) is reused unchanged — there are no complex-specific constants to store.

use alloc::rc::Rc;
use core::cell::RefCell;
use core::cmp::Ordering;
use core::str::FromStr;

use dashu_base::{AbsOrd, ConversionError, ParseError};
use dashu_float::round::{mode, Round};
use dashu_float::{CachedFBig, ConstCache, FBig, Repr};
use dashu_int::{IBig, UBig, Word};

use crate::cbig::CBig;
use crate::repr::Context;

/// A complex number that carries a shared handle to a [`ConstCache`].
///
/// It is functionally a [`CBig`]: same in-memory representation (`cbig`),
/// plus an [`Rc<RefCell<ConstCache>>`] handle. The difference is that the
/// transcendental operations (`ln`, `exp`, `sin`, `cos`, …) thread that handle
/// into the underlying [`Context`] methods, so they reuse and progressively
/// extend the cached exact binary-splitting state for the real constants
/// (π, ln2, ln10, …) instead of recomputing them from scratch on every call.
///
/// `Context`/`CBig` themselves stay `Copy` + `Send` + `Sync` + `no_std` (so
/// `static_cbig!` keeps producing a `CBig`); only this cached wrapper is
/// `!Send + !Sync`, because it shares state through an `Rc<RefCell<..>>`.
/// To share one cache across threads, build an analogous type over
/// `Arc<Mutex<ConstCache>>` instead (the [`Context`] methods accept
/// `Option<&mut ConstCache>`, independent of the container).
///
/// Every value-producing operation returns a `CachedCBig` that preserves the
/// handle, so `(z + w).ln().exp()` stays cached throughout — no silent cache
/// loss. When two `CachedCBig` values with different cache handles interact in
/// a binary operation, the LHS (left-hand-side) cache is preserved in the
/// result. For `CBig op CachedCBig` (and `FBig op CachedCBig`), the
/// `CachedCBig` operand's cache is preserved.
///
/// # Examples
///
/// ```
/// use core::cell::RefCell;
/// use std::rc::Rc;
/// use dashu_cmplx::{CBig, CachedCBig, ConstCache};
/// use dashu_float::{FBig, round::mode::HalfAway};
///
/// type C = CBig<HalfAway, 10>;
/// // build a cached 1+1i from a plain CBig
/// let z = CachedCBig::from(C::from_parts(FBig::from(1), FBig::from(1)));
///
/// // ln / exp reuse the same shared cache handle
/// let _ = z.clone().ln().exp();
/// ```
pub struct CachedCBig<R: Round = mode::Zero, const B: Word = 2> {
    pub(crate) cbig: CBig<R, B>,
    pub(crate) cache: Rc<RefCell<ConstCache>>,
}

impl<R: Round, const B: Word> CachedCBig<R, B> {
    /// Wrap a [`CBig`], sharing the given cache handle.
    #[inline]
    pub fn new(value: CBig<R, B>, cache: Rc<RefCell<ConstCache>>) -> Self {
        Self { cbig: value, cache }
    }

    /// Create a [`CachedCBig`] from its real and imaginary parts, attaching a *fresh* cache.
    /// Drop-in for [`CBig::from_parts`].
    #[inline]
    pub fn from_parts(re: FBig<R, B>, im: FBig<R, B>) -> Self {
        Self::new(CBig::from_parts(re, im), Rc::new(RefCell::new(ConstCache::new())))
    }

    /// Build a `CachedCBig` from a [`CBig`] result, re-attaching this value's
    /// shared cache handle (cloned cheaply via `Rc`).
    #[inline]
    pub(crate) fn from_cbig(cbig: CBig<R, B>, cache: &Rc<RefCell<ConstCache>>) -> Self {
        Self {
            cbig,
            cache: Rc::clone(cache),
        }
    }

    /// Borrow the inner [`CBig`].
    #[inline]
    pub fn as_cbig(&self) -> &CBig<R, B> {
        &self.cbig
    }

    /// Drop the cache handle and return the underlying [`CBig`].
    #[inline]
    pub fn into_cbig(self) -> CBig<R, B> {
        self.cbig
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

    // ----- accessors (delegate to the inner CBig) -----

    /// Get the shared [`Context`] of the complex number (see [`CBig::context`]).
    #[inline]
    pub const fn context(&self) -> Context<R> {
        self.cbig.context
    }

    /// Get the precision limit (`0` = unlimited); both parts share it (see [`CBig::precision`]).
    #[inline]
    pub const fn precision(&self) -> usize {
        self.cbig.context.precision()
    }

    /// Get a reference to the real part's raw representation (see [`CBig::re`]).
    #[inline]
    pub const fn re(&self) -> &Repr<B> {
        &self.cbig.re
    }

    /// Get a reference to the imaginary part's raw representation (see [`CBig::im`]).
    #[inline]
    pub const fn im(&self) -> &Repr<B> {
        &self.cbig.im
    }

    /// Decompose into the real and imaginary parts as [`CachedFBig`]s, **sharing this value's
    /// cache handle** so transcendentals on either part stay cached.
    ///
    /// This is an intentional divergence from [`CBig::into_parts`], which returns `(FBig, FBig)`:
    /// here both parts carry the original shared cache (cheaply, via `Rc`), preserving the
    /// accumulated constant state across the decomposition.
    #[inline]
    pub fn into_parts(self) -> (CachedFBig<R, B>, CachedFBig<R, B>) {
        let (re, im) = self.cbig.into_parts();
        (CachedFBig::new(re, Rc::clone(&self.cache)), CachedFBig::new(im, self.cache))
    }

    /// Determine if the complex number is numerically zero (both parts `±0`) (see [`CBig::is_zero`]).
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.cbig.is_zero()
    }

    /// Determine if either part is infinite (see [`CBig::is_infinite`]).
    #[inline]
    pub fn is_infinite(&self) -> bool {
        self.cbig.is_infinite()
    }

    /// Determine if the complex number is finite (neither part infinite) (see [`CBig::is_finite`]).
    #[inline]
    pub fn is_finite(&self) -> bool {
        self.cbig.is_finite()
    }
}

// ---------------------------------------------------------------------------
// From / Into
// ---------------------------------------------------------------------------

impl<R: Round, const B: Word> From<CBig<R, B>> for CachedCBig<R, B> {
    #[inline]
    fn from(cbig: CBig<R, B>) -> Self {
        Self::new(cbig, Rc::new(RefCell::new(ConstCache::new())))
    }
}

impl<R: Round, const B: Word> From<CachedCBig<R, B>> for CBig<R, B> {
    #[inline]
    fn from(cached: CachedCBig<R, B>) -> Self {
        cached.into_cbig()
    }
}

impl<R: Round, const B: Word> CBig<R, B> {
    /// Attach a shared cache handle, turning this [`CBig`] into a [`CachedCBig`].
    #[inline]
    pub fn into_cached(self, cache: Rc<RefCell<ConstCache>>) -> CachedCBig<R, B> {
        CachedCBig::new(self, cache)
    }
}

// ---------------------------------------------------------------------------
// FromStr / From / TryFrom
//
// Construction from an external value (string, FBig, integer, primitive float) attaches
// a *fresh* cache, exactly like `From<CBig>` above — there is no existing handle to share.
// ---------------------------------------------------------------------------

impl<R: Round, const B: Word> From<FBig<R, B>> for CachedCBig<R, B> {
    #[inline]
    fn from(re: FBig<R, B>) -> Self {
        CBig::from(re).into()
    }
}

impl<R: Round, const B: Word> From<UBig> for CachedCBig<R, B> {
    #[inline]
    fn from(v: UBig) -> Self {
        CBig::from(v).into()
    }
}

impl<R: Round, const B: Word> From<IBig> for CachedCBig<R, B> {
    #[inline]
    fn from(v: IBig) -> Self {
        CBig::from(v).into()
    }
}

macro_rules! impl_from_int_for_cached_cbig {
    ($($t:ty)*) => {$(
        impl<R: Round, const B: Word> From<$t> for CachedCBig<R, B> {
            #[inline]
            fn from(value: $t) -> Self {
                CBig::from(value).into()
            }
        }
    )*};
}
impl_from_int_for_cached_cbig!(u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize);

impl<R: Round> TryFrom<f32> for CachedCBig<R, 2> {
    type Error = ConversionError;

    #[inline]
    fn try_from(value: f32) -> Result<Self, Self::Error> {
        CBig::try_from(value).map(Self::from)
    }
}

impl<R: Round> TryFrom<f64> for CachedCBig<R, 2> {
    type Error = ConversionError;

    #[inline]
    fn try_from(value: f64) -> Result<Self, Self::Error> {
        CBig::try_from(value).map(Self::from)
    }
}

impl<R: Round, const B: Word> FromStr for CachedCBig<R, B> {
    type Err = ParseError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, ParseError> {
        Ok(CBig::from_str(s)?.into())
    }
}

macro_rules! impl_try_from_cached_cbig_for_int {
    ($($t:ty)*) => {$(
        impl<R: Round, const B: Word> TryFrom<CachedCBig<R, B>> for $t {
            type Error = ConversionError;

            #[inline]
            fn try_from(value: CachedCBig<R, B>) -> Result<Self, Self::Error> {
                value.cbig.try_into()
            }
        }
    )*};
}
impl_try_from_cached_cbig_for_int!(
    u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize UBig IBig
);

impl<R: Round, const B: Word> TryFrom<CachedCBig<R, B>> for FBig<R, B> {
    type Error = ConversionError;

    #[inline]
    fn try_from(value: CachedCBig<R, B>) -> Result<Self, Self::Error> {
        value.cbig.try_into()
    }
}

impl<R: Round> TryFrom<CachedCBig<R, 2>> for f32 {
    type Error = ConversionError;

    #[inline]
    fn try_from(value: CachedCBig<R, 2>) -> Result<Self, Self::Error> {
        value.cbig.try_into()
    }
}

impl<R: Round> TryFrom<CachedCBig<R, 2>> for f64 {
    type Error = ConversionError;

    #[inline]
    fn try_from(value: CachedCBig<R, 2>) -> Result<Self, Self::Error> {
        value.cbig.try_into()
    }
}

// ---------------------------------------------------------------------------
// Clone / Default / Debug / Display
// ---------------------------------------------------------------------------

impl<R: Round, const B: Word> Clone for CachedCBig<R, B> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            cbig: self.cbig.clone(),
            cache: Rc::clone(&self.cache),
        }
    }
}

impl<R: Round, const B: Word> Default for CachedCBig<R, B> {
    /// Default value: `0 + 0i` with a fresh cache.
    #[inline]
    fn default() -> Self {
        Self::from(CBig::default())
    }
}

impl<R: Round, const B: Word> core::fmt::Debug for CachedCBig<R, B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&self.cbig, f)
    }
}

// Display delegates to the inner CBig so the rendered string is identical (algebraic `a+bi`).
impl<R: Round, const B: Word> core::fmt::Display for CachedCBig<R, B> {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(&self.cbig, f)
    }
}

// ---------------------------------------------------------------------------
// PartialEq / Eq / Ord / AbsOrd (delegate to the inner CBig — value semantics)
// ---------------------------------------------------------------------------

impl<R1: Round, R2: Round, const B: Word> PartialEq<CachedCBig<R2, B>> for CachedCBig<R1, B> {
    #[inline]
    fn eq(&self, other: &CachedCBig<R2, B>) -> bool {
        // value equality, mirroring CBig (compares the representations only).
        self.cbig.re == other.cbig.re && self.cbig.im == other.cbig.im
    }
}

impl<R: Round, const B: Word> Eq for CachedCBig<R, B> {}

impl<R1: Round, R2: Round, const B: Word> PartialOrd<CachedCBig<R2, B>> for CachedCBig<R1, B> {
    #[inline]
    fn partial_cmp(&self, other: &CachedCBig<R2, B>) -> Option<Ordering> {
        self.cbig.partial_cmp(&other.cbig)
    }
}

impl<R: Round, const B: Word> Ord for CachedCBig<R, B> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.cbig.cmp(&other.cbig)
    }
}

impl<R: Round, const B: Word> AbsOrd for CachedCBig<R, B> {
    #[inline]
    fn abs_cmp(&self, other: &Self) -> Ordering {
        AbsOrd::abs_cmp(&self.cbig, &other.cbig)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;
    use dashu_float::round::mode;

    type F = FBig<mode::HalfAway, 10>;
    type C = CBig<mode::HalfAway, 10>;
    type CC = CachedCBig<mode::HalfAway, 10>;

    fn handle() -> Rc<RefCell<ConstCache>> {
        Rc::new(RefCell::new(ConstCache::new()))
    }

    /// A cached `re+im·i` at precision 53 sharing `h`.
    fn cached(re: i32, im: i32, h: &Rc<RefCell<ConstCache>>) -> CC {
        let mk = |v: i32| -> F { F::from(v).with_precision(53).value() };
        CachedCBig::new(CBig::from_parts(mk(re), mk(im)), h.clone())
    }

    /// The matching plain CBig (precision 53) for value comparison.
    fn c(re: i32, im: i32) -> C {
        let mk = |v: i32| -> F { F::from(v).with_precision(53).value() };
        CBig::from_parts(mk(re), mk(im))
    }

    #[test]
    fn constructors_and_accessors() {
        let h = handle();
        let z = cached(3, 4, &h);
        assert_eq!(z.as_cbig().re().significand(), &3.into());
        assert_eq!(z.as_cbig().im().significand(), &4.into());
        assert_eq!(z.precision(), 53);
        assert_eq!(z.context().precision(), 53);
        assert!(!z.is_zero());
        assert!(!z.is_infinite());
        assert!(z.is_finite());

        // Default = 0+0i; From<CBig> / into_cbig round-trip; from_parts attaches a fresh cache.
        assert!(CC::default().is_zero());
        let plain = c(3, 4);
        assert_eq!(CC::from(plain.clone()).into_cbig(), plain);
        let fp = CC::from_parts(F::from(3), F::from(4));
        assert_eq!(fp.as_cbig().re().significand(), &3.into());
    }

    #[test]
    fn fmt_matches_cbig() {
        for &(re, im) in &[
            (0, 0),
            (5, 0),
            (0, 1),
            (0, -1),
            (1, 2),
            (-3, -4),
            (1, 1),
            (2, -1),
        ] {
            let cval = c(re, im);
            let ccval = cached(re, im, &handle());
            assert_eq!(format!("{}", ccval), format!("{}", cval), "Display {re}+{im}i");
            assert_eq!(format!("{:?}", ccval), format!("{:?}", cval), "Debug {re}+{im}i");
        }
    }

    #[test]
    fn fromstr_matches_cbig() {
        for s in &["0", "1", "i", "-i", "3+4i", "-3-4i", "1+i", "2-i"] {
            assert_eq!(CC::from_str(s).unwrap().as_cbig(), &C::from_str(s).unwrap());
        }
    }

    #[test]
    fn ordering_matches_cbig() {
        let (a, b) = (c(1, 9), c(2, 0));
        let (ca, cb) = (cached(1, 9, &handle()), cached(2, 0, &handle()));
        assert_eq!(ca.cmp(&cb), a.cmp(&b));
        assert_eq!(ca.partial_cmp(&cb), a.partial_cmp(&b));
        assert_eq!(AbsOrd::abs_cmp(&ca, &cb), AbsOrd::abs_cmp(&a, &b));
        // PartialEq / Eq
        assert!(cached(3, 4, &handle()) == cached(3, 4, &handle()));
        assert!(cached(3, 4, &handle()) != cached(3, 5, &handle()));
    }

    #[test]
    fn conversions_match_cbig() {
        // From<FBig> / From<UBig> / From<IBig> / From<primitive>
        assert_eq!(CC::from(F::from(7)).as_cbig(), &C::from(F::from(7)));
        assert_eq!(CC::from(UBig::from(123u32)).as_cbig(), &C::from(UBig::from(123u32)));
        assert_eq!(CC::from(IBig::from(-456)).as_cbig(), &C::from(IBig::from(-456)));
        assert_eq!(CC::from(7u8).as_cbig(), &C::from(7u8));
        assert_eq!(CC::from(-9i32).as_cbig(), &C::from(-9i32));

        // TryFrom<CachedCBig> for ints / FBig delegates to the inner CBig.
        let ccval = cached(9, 0, &handle());
        let cval = c(9, 0);
        assert_eq!(IBig::try_from(ccval.clone()).ok(), IBig::try_from(cval.clone()).ok());
        assert_eq!(i32::try_from(ccval.clone()).ok(), i32::try_from(cval.clone()).ok());
        assert_eq!(F::try_from(ccval).ok(), F::try_from(cval).ok());
        // nonzero imaginary → LossOfPrecision on both.
        assert_eq!(F::try_from(cached(9, 1, &handle())).err(), F::try_from(c(9, 1)).err());
    }

    #[test]
    fn float_conversions_match_cbig() {
        // TryFrom<f64> / TryFrom<CachedCBig<R,2>> for f64 (base 2 only).
        type C2 = CBig<mode::HalfAway, 2>;
        type CC2 = CachedCBig<mode::HalfAway, 2>;
        let cv = C2::try_from(2.5f64).unwrap();
        let ccv = CC2::try_from(2.5f64).unwrap();
        assert_eq!(ccv.as_cbig(), &cv);
        assert_eq!(f64::try_from(ccv).unwrap(), 2.5f64);
        assert!(CC2::try_from(f32::NAN).is_err());
    }

    #[test]
    fn into_parts_shares_cache() {
        let h = handle();
        let z = cached(2, 3, &h);
        // populate the cache, then decompose — both parts must carry the shared handle
        let _ = z.clone().ln();
        let (re, im) = z.into_parts();
        assert!(re.cache().total_terms() > 0);
        assert!(im.cache().total_terms() > 0);
        // and the parts' values match CBig::into_parts
        let (cre, cim) = c(2, 3).into_parts();
        assert_eq!(re.as_fbig(), &cre);
        assert_eq!(im.as_fbig(), &cim);
    }

    #[test]
    fn cache_clear_zeros_and_recomputes() {
        let h = handle();
        let z = cached(2, 0, &h);
        let before = z.ln().into_cbig();
        assert!(z.cache().total_terms() > 0);

        z.clear_cache();
        assert_eq!(z.cache().total_terms(), 0);
        assert_eq!(z.cache().total_words(), 0);

        // After clearing, recomputation still produces the same result.
        assert_eq!(z.ln().into_cbig(), before);
    }
}
