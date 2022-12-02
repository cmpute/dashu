use crate::{repr::Repr, rbig::{RBig, Relaxed}};
use dashu_base::{ConversionError, DivRem, Approximation};
use dashu_float::{FBig, Repr as FBigRepr, round::{Round, Rounded}, Context};
use dashu_int::{Word, UBig};

impl<R: Round, const B: Word> From<Repr> for FBig<R, B> {
    #[inline]
    fn from(v: Repr) -> Self {
        let Repr { numerator, denominator } = v;
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
            Ok(Repr { numerator, denominator })
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
    fn to_float<R: Round, const B: Word>(&self, precision: usize) -> Rounded<FBig<R, B>> {
        assert!(precision > 0);

        let (q, r) = (&self.numerator).div_rem(&self.denominator);
        let rounded = if r.is_zero() {
            Approximation::Exact(q)
        } else {
            // TODO(v0.4): prevent this when we have unsigned round_ratio
            let den = self.denominator.clone().into();
            let adjust = R::round_ratio(&q, r, &den);
            Approximation::Inexact(q + adjust, adjust)
        };

        let context = Context::<R>::new(precision);
        rounded.and_then(|n| context.convert_int(n))
    }
}

impl RBig {
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
    #[inline]
    pub fn to_float<R: Round, const B: Word>(&self, precision: usize) -> Rounded<FBig<R, B>> {
        self.0.to_float(precision)
    }
}
