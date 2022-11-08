use dashu_base::Gcd;
use dashu_int::{IBig, UBig};

#[derive(Hash)]
pub struct Repr {
    pub(crate) numerator: IBig,
    pub(crate) denominator: UBig,
}

impl Repr {
    /// Remove the common factors between numerator and denominator
    pub fn reduce(self) -> Repr {
        if self.numerator.is_zero() {
            return Repr::zero();
        }

        let g = (&self.numerator).gcd(&self.denominator);
        Repr {
            numerator: self.numerator / &g,
            denominator: self.denominator / g,
        }
    }

    /// Remove the common factors with the hint, that is the factors are calculated
    /// as `gcd(hint, gcd(numerator, denominator))`
    pub fn reduce_with_hint(self, hint: UBig) -> Repr {
        if self.numerator.is_zero() {
            return Repr::zero();
        }

        let g = hint.gcd(&self.numerator).gcd(&self.denominator);
        Repr {
            numerator: self.numerator / &g,
            denominator: self.denominator / g,
        }
    }

    /// Remove only common factor of power of 2, which is cheap
    pub fn reduce2(self) -> Repr {
        if self.numerator.is_zero() {
            return Repr::zero();
        }

        let n_zeros = self.numerator.trailing_zeros().unwrap_or_default();
        let d_zeros = self.denominator.trailing_zeros().unwrap();
        let zeros = n_zeros.min(d_zeros);

        if zeros > 0 {
            Repr {
                numerator: self.numerator >> zeros,
                denominator: self.denominator >> zeros,
            }
        } else {
            self
        }
    }

    #[inline]
    pub const fn zero() -> Repr {
        Repr {
            numerator: IBig::ZERO,
            denominator: UBig::ONE,
        }
    }
    #[inline]
    pub const fn one() -> Repr {
        Repr {
            numerator: IBig::ONE,
            denominator: UBig::ONE,
        }
    }
    #[inline]
    pub const fn neg_one() -> Repr {
        Repr {
            numerator: IBig::NEG_ONE,
            denominator: UBig::ONE,
        }
    }
}

// This custom implementation is necessary due to https://github.com/rust-lang/rust/issues/98374
impl Clone for Repr {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            numerator: self.numerator.clone(),
            denominator: self.denominator.clone(),
        }
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        self.numerator.clone_from(&source.numerator);
        self.denominator.clone_from(&source.denominator);
    }
}
