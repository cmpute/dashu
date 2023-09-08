use crate::{
    rbig::{RBig, Relaxed},
    repr::Repr,
};
use dashu_base::{Approximation, ConversionError, DivRem};
use dashu_float::{
    round::{Round, Rounded},
    Context, FBig, Repr as FBigRepr,
};
use dashu_int::{UBig, Word};

impl<R: Round, const B: Word> From<Repr> for FBig<R, B> {
    #[inline]
    fn from(v: Repr) -> Self {
        let Repr {
            numerator,
            denominator,
        } = v;
        FBig::from(numerator) / FBig::from(denominator)
    }
}

impl<const B: Word> TryFrom<FBigRepr<B>> for Repr {
    type Error = ConversionError;
    fn try_from(value: FBigRepr<B>) -> Result<Self, Self::Error> {
        if value.is_infinite() {
            Err(ConversionError::OutOfBounds)
        } else {
            let (signif, exp) = value.into_parts();
            let (numerator, denominator) = if exp >= 0 {
                (signif * UBig::from_word(B).pow(exp as usize), UBig::ONE)
            } else {
                (signif, UBig::from_word(B).pow((-exp) as usize))
            };
            Ok(Repr {
                numerator,
                denominator,
            })
        }
    }
}

impl<R: Round, const B: Word> TryFrom<FBig<R, B>> for Repr {
    type Error = ConversionError;
    #[inline]
    fn try_from(value: FBig<R, B>) -> Result<Self, Self::Error> {
        value.into_repr().try_into()
    }
}

macro_rules! forward_conversion_to_repr {
    ($t:ident, $reduce:ident) => {
        impl<R: Round, const B: Word> From<$t> for FBig<R, B> {
            #[inline]
            fn from(v: $t) -> Self {
                v.0.into()
            }
        }

        impl<const B: Word> TryFrom<FBigRepr<B>> for $t {
            type Error = ConversionError;
            #[inline]
            fn try_from(value: FBigRepr<B>) -> Result<Self, Self::Error> {
                Repr::try_from(value).map(|repr| $t(repr.$reduce()))
            }
        }

        impl<R: Round, const B: Word> TryFrom<FBig<R, B>> for $t {
            type Error = ConversionError;
            #[inline]
            fn try_from(value: FBig<R, B>) -> Result<Self, Self::Error> {
                Repr::try_from(value).map(|repr| $t(repr.$reduce()))
            }
        }
    };
}
forward_conversion_to_repr!(RBig, reduce);
forward_conversion_to_repr!(Relaxed, reduce2);

impl Repr {
    // There are some cases where the result is exactly representable by a FBig
    // without loss of significance (it's an integer or its denominator is a power of B).
    // However, it's better to explicitly prohibit it because it's still failing
    // in other cases and a method that panics occasionally is not good.
    fn to_float<R: Round, const B: Word>(&self, precision: usize) -> Rounded<FBig<R, B>> {
        assert!(precision > 0);

        if self.numerator.is_zero() {
            return Approximation::Exact(FBig::ZERO);
        }

        let base = UBig::from_word(B);
        let num_digits = self.numerator.ilog(&base);
        let den_digits = self.denominator.ilog(&base);

        let shift;
        let (q, r) = if num_digits >= precision + den_digits {
            shift = 0;
            (&self.numerator).div_rem(&self.denominator)
        } else {
            shift = (precision + den_digits) - num_digits;
            if B == 2 {
                (&self.numerator << shift).div_rem(&self.denominator)
            } else {
                (&self.numerator * base.pow(shift)).div_rem(&self.denominator)
            }
        };
        let rounded = if r.is_zero() {
            Approximation::Exact(q)
        } else {
            let adjust = R::round_ratio(&q, r, self.denominator.as_ibig());
            Approximation::Inexact(q + adjust, adjust)
        };

        let context = Context::<R>::new(precision);
        rounded
            .and_then(|n| context.convert_int(n))
            .map(|f| f >> (shift as isize))
    }
}

impl RBig {
    /// Convert the rational number to a [FBig] with guaranteed correct rounding.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::Approximation::*;
    /// # use dashu_ratio::RBig;
    /// use dashu_float::{DBig, round::Rounding::*};
    ///
    /// assert_eq!(RBig::ONE.to_float(1), Exact(DBig::ONE));
    /// assert_eq!(RBig::from(1000).to_float(4), Exact(DBig::from(1000)));
    /// assert_eq!(RBig::from_parts(1000.into(), 6u8.into()).to_float(4),
    ///     Inexact(DBig::from_parts(1667.into(), -1), AddOne));
    /// ```
    #[inline]
    pub fn to_float<R: Round, const B: Word>(&self, precision: usize) -> Rounded<FBig<R, B>> {
        self.0.to_float(precision)
    }

    // TODO(v0.4): implement fn simplest_from_float<R: Round, const B: Word>(float: &FBig<R, B>) -> Self
    //     We need to add a method to the Round trait, that reports the range where a number can be rounded
    //     to this value, together with an interval type (Open, OpenClosed, ClosedOpen, Closed). This type is
    //     also useful for the Uniform01 type.
}

impl Relaxed {
    /// Convert the rational number to a [FBig] with guaranteed correct rounding.
    ///
    /// See [RBig::to_float] for details.
    #[inline]
    pub fn to_float<R: Round, const B: Word>(&self, precision: usize) -> Rounded<FBig<R, B>> {
        self.0.to_float(precision)
    }
}
