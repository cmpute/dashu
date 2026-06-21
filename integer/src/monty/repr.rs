//! Modulus context and Montgomery-form values.

use crate::{
    arch::word::{DoubleWord, Word},
    buffer::Buffer,
    cmp,
    error::panic_different_rings,
    primitive::{double_word, shrink_dword, split_dword, DWORD_BITS, WORD_BITS_USIZE},
    repr::TypedReprRef,
    ubig::UBig,
};
use alloc::boxed::Box;
use core::ptr;
use num_modular::{Montgomery as NumMontgomery, Reducer};

/// Precomputed Montgomery constants for a fixed odd modulus.
///
/// This is the Montgomery analogue of [`ConstDivisor`](crate::fast_div::ConstDivisor):
/// it stores an odd modulus together with the values needed to perform fast modular
/// arithmetic in [Montgomery form](self). Create it once with [`MontgomeryRepr::new`]
/// and use [`MontgomeryRepr::reduce`] to convert values into the ring.
///
/// The modulus **must be odd and greater than 1**; this is enforced at construction
/// time (Montgomery reduction requires `m^{-1} mod 2^WORD_BITS` to exist).
///
/// # Examples
///
/// ```
/// # use dashu_int::{monty::MontgomeryRepr, UBig};
/// let ring = MontgomeryRepr::new(UBig::from(10001u32));
/// let x = ring.reduce(12345);
/// let y = ring.reduce(67890);
/// assert_eq!((x * y).residue(), UBig::from(12345u32 * 67890u32 % 10001));
/// ```
pub struct MontgomeryRepr(pub(crate) MontgomeryReprData);

pub(crate) enum MontgomeryReprData {
    Single(MontgomerySingleRepr),
    Double(MontgomeryDoubleRepr),
    Large(MontgomeryLargeRepr),
}

/// Montgomery context for a single-word modulus (delegates to [`num_modular::Montgomery`]).
#[derive(Clone, Copy, Debug)]
pub(crate) struct MontgomerySingleRepr(pub(crate) NumMontgomery<Word>);

/// Montgomery context for a double-word modulus (delegates to [`num_modular::Montgomery`]).
#[derive(Clone, Copy, Debug)]
pub(crate) struct MontgomeryDoubleRepr(pub(crate) NumMontgomery<DoubleWord>);

/// Montgomery context for a multi-word modulus.
#[derive(Debug)]
pub(crate) struct MontgomeryLargeRepr {
    /// The odd modulus as little-endian words (no normalization shift).
    pub(crate) modulus: Box<[Word]>,
    /// `-m^{-1} mod 2^(2*WORD_BITS)`, the double-word Montgomery constant. The single-word
    /// constant is just its low word (see [`MontgomeryLargeRepr::n0_word`]); REDC clears two
    /// words per iteration via the addmul_2 kernel.
    pub(crate) n0_dword: DoubleWord,
    /// `R^2 mod m` (where `R = 2^(WORD_BITS * s)`) — used to convert plain values into
    /// Montgomery form. The Montgomery form of 1 (`R mod m`) is derived from this on demand.
    pub(crate) r2_mod_m: Box<[Word]>,
}

/// A value in Montgomery form tied to a [`MontgomeryRepr`].
///
/// Analogous to [`Reduced`](crate::modular::Reduced), but stored in Montgomery form so
/// that multiplication uses Montgomery reduction instead of division. Values are kept
/// in the canonical range `[0, m)`.
///
/// Create one with [`MontgomeryRepr::reduce`].
pub struct Montgomery<'a>(MontgomeryInner<'a>);

pub(crate) enum MontgomeryInner<'a> {
    Single(Word, &'a MontgomerySingleRepr),
    Double(DoubleWord, &'a MontgomeryDoubleRepr),
    Large(MontgomeryLargeVal, &'a MontgomeryLargeRepr),
}

/// Multi-word value in Montgomery form. `self.0.len() == ring.modulus.len()`.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct MontgomeryLargeVal(pub(crate) Box<[Word]>);

impl MontgomeryRepr {
    /// Create a [`MontgomeryRepr`] for the given odd modulus `m`.
    ///
    /// # Panics
    ///
    /// Panics if `m` is not odd or `m <= 1` (Montgomery reduction requires an odd modulus).
    pub fn new(m: UBig) -> Self {
        match m.repr() {
            TypedReprRef::RefSmall(dword) => {
                assert!(
                    dword & 1 == 1 && dword > 1,
                    "Montgomery modulus must be odd and greater than 1"
                );
                if let Some(word) = shrink_dword(dword) {
                    Self(MontgomeryReprData::Single(MontgomerySingleRepr(
                        <NumMontgomery<Word> as Reducer<Word>>::new(&word),
                    )))
                } else {
                    Self(MontgomeryReprData::Double(MontgomeryDoubleRepr(<NumMontgomery<
                        DoubleWord,
                    > as Reducer<DoubleWord>>::new(
                        &dword
                    ))))
                }
            }
            TypedReprRef::RefLarge(words) => {
                assert!(words[0] & 1 == 1, "Montgomery modulus must be odd and greater than 1");
                Self(MontgomeryReprData::Large(MontgomeryLargeRepr::new(m)))
            }
        }
    }

    /// Get the inner data reference (for matching on the size class).
    #[inline]
    pub(crate) fn data(&self) -> &MontgomeryReprData {
        &self.0
    }
}

impl MontgomeryLargeRepr {
    fn new(m: UBig) -> Self {
        let words = m.as_words();
        let s = words.len();
        debug_assert!(s >= 3 && words[0] & 1 == 1);

        let modulus = Buffer::from(words).into_boxed_slice();
        let n0_dword = neginv_dword(double_word(modulus[0], modulus[1]));
        debug_assert_eq!(
            double_word(modulus[0], modulus[1])
                .wrapping_mul(n0_dword)
                .wrapping_add(1),
            0,
            "n0_dword is not the negated modular inverse of the modulus's low double word"
        );

        // R^2 mod m, computed once via UBig arithmetic.
        let r = UBig::ONE << (s * WORD_BITS_USIZE);
        let r2_mod_m = (&r * &r) % &m;

        Self {
            modulus,
            n0_dword,
            r2_mod_m: to_exact_words(&r2_mod_m, s),
        }
    }

    /// The single-word Montgomery constant `-m^{-1} mod 2^WORD_BITS` (the low word of
    /// [`Self::n0_dword`]), used by the single-word REDC path.
    #[inline]
    pub(crate) fn n0_word(&self) -> Word {
        split_dword(self.n0_dword).0
    }
}

/// Pack a `UBig` (which is `< m`, hence at most `s` words) into exactly `s` words.
pub(crate) fn to_exact_words(u: &UBig, s: usize) -> Box<[Word]> {
    let words = u.as_words();
    debug_assert!(words.len() <= s);
    let mut buffer = Buffer::allocate_exact(s);
    buffer.push_slice(words);
    buffer.push_zeros(s - words.len());
    buffer.into_boxed_slice()
}

/// Compute `-m^{-1} mod 2^(2*WORD_BITS)` for an odd `m` (given by its low double word) using
/// table-free Hensel lifting.
///
/// Since `m` is odd, `m^{-1} ≡ 1 (mod 2)`, so `i = 1` is a valid 1-bit seed. The Newton
/// step `i ← i·(2 − m·i)` (all wrapping) doubles the number of correct low bits each iteration:
/// 1 → 2 → 4 → … → `2*WORD_BITS`. The result is then negated.
const fn neginv_dword(m: DoubleWord) -> DoubleWord {
    let two: DoubleWord = 2;
    let mut i: DoubleWord = 1; // m^{-1} mod 2 (m is odd)
    let mut correct_bits: u32 = 1;
    while correct_bits < DWORD_BITS {
        i = i.wrapping_mul(two.wrapping_sub(m.wrapping_mul(i)));
        correct_bits <<= 1;
    }
    i.wrapping_neg()
}

impl<'a> Montgomery<'a> {
    /// Get representation.
    #[inline]
    pub(crate) fn repr(&self) -> &MontgomeryInner<'a> {
        &self.0
    }

    /// Get mutable representation.
    #[inline]
    pub(crate) fn repr_mut(&mut self) -> &mut MontgomeryInner<'a> {
        &mut self.0
    }

    #[inline]
    pub(crate) fn into_repr(self) -> MontgomeryInner<'a> {
        self.0
    }

    #[inline]
    pub(crate) const fn from_single(raw: Word, ring: &'a MontgomerySingleRepr) -> Self {
        Montgomery(MontgomeryInner::Single(raw, ring))
    }

    #[inline]
    pub(crate) const fn from_double(raw: DoubleWord, ring: &'a MontgomeryDoubleRepr) -> Self {
        Montgomery(MontgomeryInner::Double(raw, ring))
    }

    #[inline]
    pub(crate) fn from_large(raw: MontgomeryLargeVal, ring: &'a MontgomeryLargeRepr) -> Self {
        debug_assert!(raw.is_valid(ring));
        Montgomery(MontgomeryInner::Large(raw, ring))
    }

    #[inline]
    pub(crate) fn check_same_ring_single(lhs: &MontgomerySingleRepr, rhs: &MontgomerySingleRepr) {
        if !ptr::eq(lhs, rhs) {
            panic_different_rings();
        }
    }

    #[inline]
    pub(crate) fn check_same_ring_double(lhs: &MontgomeryDoubleRepr, rhs: &MontgomeryDoubleRepr) {
        if !ptr::eq(lhs, rhs) {
            panic_different_rings();
        }
    }

    #[inline]
    pub(crate) fn check_same_ring_large(lhs: &MontgomeryLargeRepr, rhs: &MontgomeryLargeRepr) {
        if !ptr::eq(lhs, rhs) {
            panic_different_rings();
        }
    }
}

impl MontgomeryLargeVal {
    /// The Montgomery form of 1, i.e. `R mod m`, derived on demand as `REDC(R^2 mod m)`.
    pub(crate) fn one(ring: &MontgomeryLargeRepr) -> Self {
        super::mul::one_large(ring)
    }

    #[inline]
    pub(crate) fn is_valid(&self, ring: &MontgomeryLargeRepr) -> bool {
        self.0.len() == ring.modulus.len() && cmp::cmp_same_len(&self.0, &ring.modulus).is_lt()
    }
}

impl Clone for Montgomery<'_> {
    #[inline]
    fn clone(&self) -> Self {
        Montgomery(self.0.clone())
    }
    #[inline]
    fn clone_from(&mut self, source: &Self) {
        self.0.clone_from(&source.0);
    }
}

impl Clone for MontgomeryInner<'_> {
    #[inline]
    fn clone(&self) -> Self {
        match self {
            MontgomeryInner::Single(raw, ring) => MontgomeryInner::Single(*raw, ring),
            MontgomeryInner::Double(raw, ring) => MontgomeryInner::Double(*raw, ring),
            MontgomeryInner::Large(raw, ring) => MontgomeryInner::Large(raw.clone(), ring),
        }
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        if let (MontgomeryInner::Large(raw, ring), MontgomeryInner::Large(src_raw, src_ring)) =
            (&mut *self, source)
        {
            *ring = src_ring;
            raw.0.clone_from(&src_raw.0);
        } else {
            *self = source.clone();
        }
    }
}

/// Equality within a ring.
///
/// # Panics
///
/// Panics if the two values are from different rings.
impl PartialEq for Montgomery<'_> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        match (self.repr(), other.repr()) {
            (MontgomeryInner::Single(raw0, ring0), MontgomeryInner::Single(raw1, ring1)) => {
                Montgomery::check_same_ring_single(ring0, ring1);
                raw0.eq(raw1)
            }
            (MontgomeryInner::Double(raw0, ring0), MontgomeryInner::Double(raw1, ring1)) => {
                Montgomery::check_same_ring_double(ring0, ring1);
                raw0.eq(raw1)
            }
            (MontgomeryInner::Large(raw0, ring0), MontgomeryInner::Large(raw1, ring1)) => {
                Montgomery::check_same_ring_large(ring0, ring1);
                raw0.eq(raw1)
            }
            _ => panic_different_rings(),
        }
    }
}

impl Eq for Montgomery<'_> {}
