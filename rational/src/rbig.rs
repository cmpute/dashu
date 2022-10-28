use dashu_base::Sign;
use dashu_int::{DoubleWord, IBig, UBig};

use crate::{error::panic_divide_by_0, repr::Repr};

#[derive(PartialEq, Eq, Hash)] // representation of RBig is canonicalized, so it suffices to compare the components
#[repr(transparent)]
pub struct RBig(pub(crate) Repr);

#[repr(transparent)]
pub struct Relaxed(pub(crate) Repr); // the result is not always normalized

impl RBig {
    pub const ZERO: Self = Self(Repr::zero());
    pub const ONE: Self = Self(Repr::one());
    pub const NEG_ONE: Self = Self(Repr::neg_one());

    /// Create a rational number from a signed numerator and an unsigned denominator
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
    /// Convert the rational number into (numerator, denumerator) parts.
    #[inline]
    pub fn into_parts(self) -> (IBig, UBig) {
        (self.0.numerator, self.0.denominator)
    }

    /// Create a rational number from a signed numerator and a signed denominator
    #[inline]
    pub fn from_parts_signed(numerator: IBig, denominator: IBig) -> Self {
        let (sign, mag) = denominator.into_parts();
        Self::from_parts(numerator * sign, mag)
    }
    /// Create a rational number in a const context
    #[inline]
    pub fn from_parts_const(
        sign: Sign,
        mut numerator: DoubleWord,
        mut denominator: DoubleWord,
    ) -> Self {
        if denominator == 0 {
            panic_divide_by_0()
        }

        if numerator > 1 && denominator > 1 {
            // perform a naive but const gcd
            let (mut y, mut r) = (denominator, numerator % denominator);
            while r > 1 {
                let new_r = y % r;
                y = core::mem::replace(&mut r, new_r);
            }
            if r == 0 {
                numerator /= y;
                denominator /= y;
            }
        }

        Self(Repr {
            numerator: IBig::from_parts_const(sign, numerator),
            denominator: UBig::from_dword(denominator),
        })
    }

    /// Get the numerator of the rational number
    #[inline]
    pub fn numerator(&self) -> &IBig {
        &self.0.numerator
    }
    /// Get the denominator of the rational number
    #[inline]
    pub fn denominator(&self) -> &UBig {
        &self.0.denominator
    }
    /// Convert this rational number into a [Relaxed] version
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

    /// Create a rational number from a signed numerator and a signed denominator
    #[inline]
    pub fn from_parts_signed(numerator: IBig, denominator: IBig) -> Self {
        let (sign, mag) = denominator.into_parts();
        Self::from_parts(numerator * sign, mag)
    }
    /// Create a rational number in a const context
    #[inline]
    pub fn from_parts_const(sign: Sign, numerator: DoubleWord, denominator: DoubleWord) -> Self {
        if denominator == 0 {
            panic_divide_by_0()
        }

        let zeros = numerator.trailing_zeros().min(denominator.trailing_zeros());
        Self(Repr {
            numerator: IBig::from_parts_const(sign, numerator >> zeros),
            denominator: UBig::from_dword(denominator >> zeros),
        })
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
