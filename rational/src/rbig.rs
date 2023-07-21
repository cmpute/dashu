use dashu_base::{EstimatedLog2, Sign};
use dashu_int::{DoubleWord, IBig, UBig};

use crate::{error::panic_divide_by_0, repr::Repr};

/// An arbitrary precision rational number.
///
/// This struct represents an rational number with arbitrarily large numerator and denominator
/// based on [UBig] and [IBig].
#[derive(PartialOrd, Ord)]
#[repr(transparent)]
pub struct RBig(pub(crate) Repr);

/// An arbitrary precision rational number without strict reduction.
///
/// This struct is almost the same as [RBig], except for that the numerator and the
/// denominator are allowed to have common divisors **other than a power of 2**. This allows
/// faster computation because [Gcd][dashu_base::Gcd] is not required for each operation.
///
/// Since the representation is not canonicalized, [Hash] is not implemented for [Relaxed].
/// Please use [RBig] if you want to store the rational number in a hash set, or use `num_order::NumHash`.
///
/// # Conversion from/to [RBig]
///
/// To convert from [RBig], use [RBig::relax()]. To convert to [RBig], use [Relaxed::canonicalize()].
#[derive(PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Relaxed(pub(crate) Repr); // the result is not always normalized

impl RBig {
    /// [RBig] with value 0
    pub const ZERO: Self = Self(Repr::zero());
    /// [RBig] with value 1
    pub const ONE: Self = Self(Repr::one());
    /// [RBig] with value -1
    pub const NEG_ONE: Self = Self(Repr::neg_one());

    /// Create a rational number from a signed numerator and an unsigned denominator
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::{IBig, UBig};
    /// # use dashu_ratio::RBig;
    /// assert_eq!(RBig::from_parts(IBig::ZERO, UBig::ONE), RBig::ZERO);
    /// assert_eq!(RBig::from_parts(IBig::ONE, UBig::ONE), RBig::ONE);
    /// assert_eq!(RBig::from_parts(IBig::NEG_ONE, UBig::ONE), RBig::NEG_ONE);
    /// ```
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
    pub fn is_integer(&self) -> bool {
        let denom = self.denominator();

        denom.repr().len() == 1 

    }

    /// Convert the rational number into (numerator, denumerator) parts.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::{IBig, UBig};
    /// # use dashu_ratio::RBig;
    /// assert_eq!(RBig::ZERO.into_parts(), (IBig::ZERO, UBig::ONE));
    /// assert_eq!(RBig::ONE.into_parts(), (IBig::ONE, UBig::ONE));
    /// assert_eq!(RBig::NEG_ONE.into_parts(), (IBig::NEG_ONE, UBig::ONE));
    /// ```
    #[inline]
    pub fn into_parts(self) -> (IBig, UBig) {
        (self.0.numerator, self.0.denominator)
    }

    /// Create a rational number from a signed numerator and a signed denominator
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::{IBig, UBig};
    /// # use dashu_ratio::RBig;
    /// assert_eq!(RBig::from_parts_signed(1.into(), 1.into()), RBig::ONE);
    /// assert_eq!(RBig::from_parts_signed(12.into(), (-12).into()), RBig::NEG_ONE);
    /// ```
    #[inline]
    pub fn from_parts_signed(numerator: IBig, denominator: IBig) -> Self {
        let (sign, mag) = denominator.into_parts();
        Self::from_parts(numerator * sign, mag)
    }

    /// Create a rational number in a const context
    ///
    /// The magnitude of the numerator and the denominator is limited to
    /// a [DoubleWord][dashu_int::DoubleWord].
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::Sign;
    /// # use dashu_ratio::{RBig, Relaxed};
    /// const ONE: RBig = RBig::from_parts_const(Sign::Positive, 1, 1);
    /// assert_eq!(ONE, RBig::ONE);
    /// const NEG_ONE: RBig = RBig::from_parts_const(Sign::Negative, 1, 1);
    /// assert_eq!(NEG_ONE, RBig::NEG_ONE);
    /// ```
    #[inline]
    pub const fn from_parts_const(
        sign: Sign,
        mut numerator: DoubleWord,
        mut denominator: DoubleWord,
    ) -> Self {
        if denominator == 0 {
            panic_divide_by_0()
        } else if numerator == 0 {
            return Self::ZERO;
        }

        if numerator > 1 && denominator > 1 {
            // perform a naive but const gcd
            let (mut y, mut r) = (denominator, numerator % denominator);
            while r > 1 {
                let new_r = y % r;
                y = r;
                r = new_r;
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
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::IBig;
    /// # use dashu_ratio::RBig;
    /// assert_eq!(RBig::ZERO.numerator(), &IBig::ZERO);
    /// assert_eq!(RBig::ONE.numerator(), &IBig::ONE);
    /// ```
    #[inline]
    pub fn numerator(&self) -> &IBig {
        &self.0.numerator
    }

    /// Get the denominator of the rational number
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::UBig;
    /// # use dashu_ratio::RBig;
    /// assert_eq!(RBig::ZERO.denominator(), &UBig::ONE);
    /// assert_eq!(RBig::ONE.denominator(), &UBig::ONE);
    /// ```
    #[inline]
    pub fn denominator(&self) -> &UBig {
        &self.0.denominator
    }

    /// Convert this rational number into a [Relaxed] version
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_ratio::{RBig, Relaxed};
    /// assert_eq!(RBig::ZERO.relax(), Relaxed::ZERO);
    /// assert_eq!(RBig::ONE.relax(), Relaxed::ONE);
    /// ```
    #[inline]
    pub fn relax(self) -> Relaxed {
        Relaxed(self.0)
    }

    /// Check whether the number is 0
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_ratio::RBig;
    /// assert!(RBig::ZERO.is_zero());
    /// assert!(!RBig::ONE.is_zero());
    /// ```
    #[inline]
    pub const fn is_zero(&self) -> bool {
        self.0.numerator.is_zero()
    }

    /// Check whether the number is 1
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_ratio::RBig;
    /// assert!(!RBig::ZERO.is_one());
    /// assert!(RBig::ONE.is_one());
    /// ```
    #[inline]
    pub const fn is_one(&self) -> bool {
        self.0.numerator.is_one()
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

impl EstimatedLog2 for RBig {
    #[inline]
    fn log2_bounds(&self) -> (f32, f32) {
        self.0.log2_bounds()
    }
    #[inline]
    fn log2_est(&self) -> f32 {
        self.0.log2_est()
    }
}

impl Relaxed {
    /// [Relaxed] with value 0
    pub const ZERO: Self = Self(Repr::zero());
    /// [Relaxed] with value 1
    pub const ONE: Self = Self(Repr::one());
    /// [Relaxed] with value -1
    pub const NEG_ONE: Self = Self(Repr::neg_one());

    /// Create a rational number from a signed numerator and a signed denominator
    ///
    /// See [RBig::from_parts] for details.
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

    /// Convert the rational number into (numerator, denumerator) parts.
    ///
    /// See [RBig::into_parts] for details.
    #[inline]
    pub fn into_parts(self) -> (IBig, UBig) {
        (self.0.numerator, self.0.denominator)
    }

    /// Create a rational number from a signed numerator and a signed denominator
    ///
    /// See [RBig::from_parts_signed] for details.
    #[inline]
    pub fn from_parts_signed(numerator: IBig, denominator: IBig) -> Self {
        let (sign, mag) = denominator.into_parts();
        Self::from_parts(numerator * sign, mag)
    }

    /// Create a rational number in a const context
    ///
    /// See [RBig::from_parts_const] for details.
    #[inline]
    pub const fn from_parts_const(
        sign: Sign,
        numerator: DoubleWord,
        denominator: DoubleWord,
    ) -> Self {
        if denominator == 0 {
            panic_divide_by_0()
        } else if numerator == 0 {
            return Self::ZERO;
        }

        let n2 = numerator.trailing_zeros();
        let d2 = denominator.trailing_zeros();
        let zeros = if n2 <= d2 { n2 } else { d2 };
        Self(Repr {
            numerator: IBig::from_parts_const(sign, numerator >> zeros),
            denominator: UBig::from_dword(denominator >> zeros),
        })
    }

    /// Get the numerator of the rational number
    ///
    /// See [RBig::numerator] for details.
    #[inline]
    pub fn numerator(&self) -> &IBig {
        &self.0.numerator
    }

    /// Get the denominator of the rational number
    ///
    /// See [RBig::denominator] for details.
    #[inline]
    pub fn denominator(&self) -> &UBig {
        &self.0.denominator
    }

    /// Convert this rational number into an [RBig] version
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_int::IBig;
    /// # use dashu_ratio::{RBig, Relaxed};
    /// assert_eq!(Relaxed::ONE.canonicalize(), RBig::ONE);
    ///
    /// let r = Relaxed::from_parts(10.into(), 5u8.into());
    /// assert_eq!(r.canonicalize().numerator(), &IBig::from(2));
    /// ```
    #[inline]
    pub fn canonicalize(self) -> RBig {
        RBig(self.0.reduce())
    }

    /// Check whether the number is 0
    ///
    /// See [RBig::is_zero] for details.
    #[inline]
    pub const fn is_zero(&self) -> bool {
        self.0.numerator.is_zero()
    }

    /// Check whether the number is 1
    ///
    /// See [RBig::is_one] for details.
    #[inline]
    pub const fn is_one(&self) -> bool {
        self.0.numerator.is_one()
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

impl EstimatedLog2 for Relaxed {
    #[inline]
    fn log2_bounds(&self) -> (f32, f32) {
        self.0.log2_bounds()
    }
    #[inline]
    fn log2_est(&self) -> f32 {
        self.0.log2_est()
    }
}
