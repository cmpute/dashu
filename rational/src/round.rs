use dashu_base::DivRem;
use dashu_int::IBig;
use crate::{repr::Repr, RBig, Relaxed};

impl Repr {
    /// Split the rational number into integral and fractional parts (split at the radix point)
    #[inline]
    pub fn split_at_point(self) -> (IBig, Self) {
        let (trunc, r) = (&self.numerator).div_rem(&self.denominator);

        let fract = if r.is_zero() {
            Repr::zero()
        } else {
            // no need to reduce here
            Repr {
                numerator: r,
                denominator: self.denominator
            }
        };
        (trunc, fract)
    }

    #[inline]
    pub fn ceil(&self) -> IBig {
        let (mut q, r) = (&self.numerator).div_rem(&self.denominator);
        if r > IBig::ZERO {
            q += IBig::ONE;
        }
        q
    }

    #[inline]
    pub fn floor(&self) -> IBig {
        let (mut q, r) = (&self.numerator).div_rem(&self.denominator);
        if r < IBig::ZERO {
            q -= IBig::ONE;
        }
        q
    }

    #[inline]
    pub fn trunc(&self) -> IBig {
        (&self.numerator) / (&self.denominator)
    }

    #[inline]
    pub fn fract(&self) -> Self {
        let r = (&self.numerator) % (&self.denominator);
        if r.is_zero() {
            Repr::zero()
        } else {
            Repr {
                numerator: r,
                denominator: self.denominator.clone()
            }
        }
    }
}

impl RBig {
    #[inline]
    pub fn split_at_point(self) -> (IBig, Self) {
        let (trunc, fract) = self.0.split_at_point();
        (trunc, Self(fract))
    }

    #[inline]
    pub fn ceil(&self) -> IBig {
        self.0.ceil()
    }

    #[inline]
    pub fn floor(&self) -> IBig {
        self.0.floor()
    }

    #[inline]
    pub fn trunc(&self) -> IBig {
        self.0.trunc()
    }
    
    #[inline]
    pub fn fract(&self) -> Self {
        Self(self.0.fract())
    }
}

impl Relaxed {
    #[inline]
    pub fn split_at_point(self) -> (IBig, Self) {
        let (trunc, fract) = self.0.split_at_point();
        (trunc, Self(fract))
    }

    #[inline]
    pub fn ceil(&self) -> IBig {
        self.0.ceil()
    }

    #[inline]
    pub fn floor(&self) -> IBig {
        self.0.floor()
    }

    #[inline]
    pub fn trunc(&self) -> IBig {
        self.0.trunc()
    }
    
    #[inline]
    pub fn fract(&self) -> Self {
        Self(self.0.fract())
    }
}