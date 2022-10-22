use dashu_int::{IBig, UBig};

use crate::{error::panic_divide_by_0, repr::Repr};

#[repr(transparent)]
pub struct RBig(pub(crate) Repr);

#[repr(transparent)]
pub struct Relaxed(pub(crate) Repr); // the result is not always normalized

impl RBig {
    pub const ZERO: Self = Self(Repr::zero());
    pub const ONE: Self = Self(Repr::one());
    pub const NEG_ONE: Self = Self(Repr::neg_one());

    #[inline]
    pub fn from_parts(numerator: IBig, denominator: UBig) -> Self {
        if denominator.is_zero() {
            panic_divide_by_0()
        }

        Self(
            Repr {
                numerator,
                denominator,
            }
            .reduce(),
        )
    }
    #[inline]
    pub fn into_parts(self) -> (IBig, UBig) {
        (self.0.numerator, self.0.denominator)
    }
    #[inline]
    pub fn numerator(&self) -> &IBig {
        &self.0.numerator
    }
    #[inline]
    pub fn denominator(&self) -> &UBig {
        &self.0.denominator
    }
    #[inline]
    pub fn relax(self) -> Relaxed {
        Relaxed(self.0)
    }
}

// This custom implementation is necessary due to https://github.com/rust-lang/rust/issues/98374
impl Clone for RBig {
    #[inline]
    fn clone(&self) -> RBig {
        RBig(self.0.clone())
    }
    #[inline]
    fn clone_from(&mut self, source: &RBig) {
        self.0.clone_from(&source.0)
    }
}

impl Default for RBig {
    #[inline]
    fn default() -> Self {
        Self::ZERO
    }
}

impl Relaxed {
    pub const ZERO: Self = Self(Repr::zero());
    pub const ONE: Self = Self(Repr::one());
    pub const NEG_ONE: Self = Self(Repr::neg_one());

    #[inline]
    pub fn from_parts(numerator: IBig, denominator: UBig) -> Self {
        if denominator.is_zero() {
            panic_divide_by_0();
        }

        Self(
            Repr {
                numerator,
                denominator,
            }
            .reduce2(),
        )
    }
    #[inline]
    pub fn into_parts(self) -> (IBig, UBig) {
        (self.0.numerator, self.0.denominator)
    }
    #[inline]
    pub fn numerator(&self) -> &IBig {
        &self.0.numerator
    }
    #[inline]
    pub fn denominator(&self) -> &UBig {
        &self.0.denominator
    }
    #[inline]
    pub fn canonicalize(self) -> RBig {
        RBig(self.0.reduce())
    }
}

// This custom implementation is necessary due to https://github.com/rust-lang/rust/issues/98374
impl Clone for Relaxed {
    #[inline]
    fn clone(&self) -> Relaxed {
        Relaxed(self.0.clone())
    }
    #[inline]
    fn clone_from(&mut self, source: &Relaxed) {
        self.0.clone_from(&source.0)
    }
}

impl Default for Relaxed {
    #[inline]
    fn default() -> Self {
        Self::ZERO
    }
}
