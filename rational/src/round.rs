use crate::{repr::Repr, RBig, Relaxed};
use dashu_base::DivRem;
use dashu_int::IBig;

impl Repr {
    #[inline]
    pub fn split_at_point(self) -> (IBig, Self) {
        let (trunc, r) = (&self.numerator).div_rem(&self.denominator);

        let fract = if r.is_zero() {
            Repr::zero()
        } else {
            // no need to reduce here
            Repr {
                numerator: r,
                denominator: self.denominator,
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
                denominator: self.denominator.clone(),
            }
        }
    }
}

impl RBig {
    /// Split the rational number into integral and fractional parts (split at the radix point).
    /// 
    /// It's return is equivalent to `(self.trunc(), self.fract())`
    /// 
    /// # Examples
    /// 
    /// ```
    /// # use dashu_int::IBig;
    /// # use dashu_ratio::RBig;
    /// assert_eq!(RBig::ONE.split_at_point(), (IBig::ONE, RBig::ZERO));
    /// 
    /// let a = RBig::from_parts(22.into(), 7u8.into());
    /// let (trunc, fract) = a.split_at_point();
    /// assert_eq!(trunc, IBig::from(3));
    /// assert_eq!(fract, RBig::from_parts(1.into(), 7u8.into()));
    /// ```
    #[inline]
    pub fn split_at_point(self) -> (IBig, Self) {
        let (trunc, fract) = self.0.split_at_point();
        (trunc, Self(fract))
    }

    /// Compute the smallest integer that is greater than or equal to this number.
    /// 
    /// # Examples
    /// 
    /// ```
    /// # use dashu_int::IBig;
    /// # use dashu_ratio::RBig;
    /// assert_eq!(RBig::ONE.ceil(), IBig::ONE);
    /// 
    /// let a = RBig::from_parts(22.into(), 7u8.into());
    /// assert_eq!(a.ceil(), IBig::from(4));
    /// ```
    #[inline]
    pub fn ceil(&self) -> IBig {
        self.0.ceil()
    }

    /// Compute the largest integer that is less than or equal to this number.
    /// 
    /// # Examples
    /// 
    /// ```
    /// # use dashu_int::IBig;
    /// # use dashu_ratio::RBig;
    /// assert_eq!(RBig::ONE.floor(), IBig::ONE);
    /// 
    /// let a = RBig::from_parts(22.into(), 7u8.into());
    /// assert_eq!(a.floor(), IBig::from(3));
    /// ```
    #[inline]
    pub fn floor(&self) -> IBig {
        self.0.floor()
    }

    /// Returns the integral part of the rational number.
    /// 
    /// It's guaranteed that `self == self.trunc() + self.fract()`.
    /// 
    /// # Examples
    /// 
    /// ```
    /// # use dashu_int::IBig;
    /// # use dashu_ratio::RBig;
    /// assert_eq!(RBig::ONE.trunc(), IBig::ONE);
    /// 
    /// let a = RBig::from_parts(22.into(), 7u8.into());
    /// assert_eq!(a.trunc(), IBig::from(3));
    /// ```
    #[inline]
    pub fn trunc(&self) -> IBig {
        self.0.trunc()
    }

    /// Returns the fractional part of the rational number
    /// 
    /// It's guaranteed that `self == self.trunc() + self.fract()`.
    /// 
    /// # Examples
    /// 
    /// ```
    /// # use dashu_int::IBig;
    /// # use dashu_ratio::RBig;
    /// assert_eq!(RBig::ONE.fract(), RBig::ZERO);
    /// 
    /// let a = RBig::from_parts(22.into(), 7u8.into());
    /// assert_eq!(a.fract(), RBig::from_parts(1.into(), 7u8.into()));
    /// ```
    #[inline]
    pub fn fract(&self) -> Self {
        Self(self.0.fract())
    }
}

impl Relaxed {
    /// Split the rational number into integral and fractional parts (split at the radix point).
    /// 
    /// See [RBig::split_at_point] for details.
    #[inline]
    pub fn split_at_point(self) -> (IBig, Self) {
        let (trunc, fract) = self.0.split_at_point();
        (trunc, Self(fract))
    }

    /// Compute the smallest integer that is greater than this number.
    /// 
    /// See [RBig::ceil] for details.
    #[inline]
    pub fn ceil(&self) -> IBig {
        self.0.ceil()
    }

    /// Compute the largest integer that is less than or equal to this number.
    /// 
    /// See [RBig::floor] for details.
    #[inline]
    pub fn floor(&self) -> IBig {
        self.0.floor()
    }

    /// Returns the integral part of the rational number.
    /// 
    /// See [RBig::trunc] for details.
    #[inline]
    pub fn trunc(&self) -> IBig {
        self.0.trunc()
    }

    /// Returns the fractional part of the rational number
    /// 
    /// See [RBig::fract] for details.
    #[inline]
    pub fn fract(&self) -> Self {
        Self(self.0.fract())
    }
}
