use core::convert::{TryFrom, TryInto};

use crate::{
    error::{check_inf, panic_unlimited_precision},
    fbig::FBig,
    repr::{Context, Repr},
    round::{
        mode::{self, HalfEven},
        Round, Rounded, Rounding,
    },
    utils::{ilog_exact, shr_digits, split_digits_ref},
};
use dashu_base::{Approximation::*, DivRemEuclid, EstimatedLog2};
use dashu_int::{error::OutOfBoundsError, IBig, UBig, Word};

impl<R: Round> Context<R> {
    /// Convert an [IBig] instance to a [FBig] instance with precision
    /// and rounding given by the context.
    pub fn convert_int<const B: Word>(&self, n: IBig) -> Rounded<FBig<R, B>> {
        let repr = Repr::<B>::new(n, 0);
        self.repr_round(repr).map(|v| FBig::new(v, *self))
    }
}

impl<R: Round> TryFrom<f32> for FBig<R, 2> {
    type Error = OutOfBoundsError;

    fn try_from(f: f32) -> Result<Self, Self::Error> {
        let bits: u32 = f.to_bits();
        let sign_bit = bits >> 31;
        let mantissa_bits = bits & 0x7fffff;

        // deal with inf/nan values
        let mut exponent: isize = ((bits >> 23) & 0xff) as isize;
        if exponent == 0xff {
            return if mantissa_bits != 0 {
                Err(OutOfBoundsError) // nan
            } else if sign_bit == 0 {
                Ok(FBig::INFINITY)
            } else {
                Ok(FBig::NEG_INFINITY)
            };
        }

        // then parse normal values
        let mantissa = if exponent == 0 {
            exponent = -127;
            mantissa_bits << 1
        } else {
            exponent -= 127 + 23; // bias + mantissa shift
            mantissa_bits | 0x800000
        } as i32;
        let mantissa = if sign_bit == 0 {
            IBig::from(mantissa)
        } else {
            IBig::from(-mantissa)
        };

        let repr = Repr::new(mantissa, exponent);
        let context = Context::new(24);
        Ok(Self::new(repr, context))
    }
}

impl<R: Round> TryFrom<f64> for FBig<R, 2> {
    type Error = OutOfBoundsError;

    fn try_from(f: f64) -> Result<Self, Self::Error> {
        let bits: u64 = f.to_bits();
        let sign_bit = bits >> 63;
        let mantissa_bits = bits & 0xfffffffffffff;

        let mut exponent: isize = ((bits >> 52) & 0x7ff) as isize;
        if exponent == 0x7ff {
            return if mantissa_bits != 0 {
                Err(OutOfBoundsError) // nan
            } else if sign_bit == 0 {
                Ok(FBig::INFINITY)
            } else {
                Ok(FBig::NEG_INFINITY)
            };
        }

        let mantissa = if exponent == 0 {
            exponent = -1023;
            mantissa_bits << 1
        } else {
            exponent -= 1023 + 52; // bias + mantissa shift
            mantissa_bits | 0x10000000000000
        } as i64;
        let mantissa = if bits >> 63 == 0 {
            IBig::from(mantissa)
        } else {
            IBig::from(-mantissa)
        };

        let repr = Repr::new(mantissa, exponent);
        let context = Context::new(53);
        Ok(Self::new(repr, context))
    }
}

impl<R: Round, const B: Word> FBig<R, B> {
    /// Convert the float number to decimal based exponents.
    #[inline]
    pub fn to_decimal(&self) -> Rounded<FBig<R, 10>> {
        self.clone().with_base::<10>()
    }

    /// Convert the float number to decimal based exponents.
    #[inline]
    pub fn to_binary(&self) -> Rounded<FBig<R, 2>> {
        self.clone().with_base::<2>()
    }

    /// Explicitly change the precision of the number.
    ///
    /// If the given precision is less than the previous value,
    /// it will be rounded following the rounding mode specified by the type parameter.
    #[inline]
    pub fn with_precision(self, precision: usize) -> Rounded<Self> {
        let new_context = Context::new(precision);

        // shrink if necessary
        let repr = if self.context.precision > precision {
            new_context.repr_round(self.repr)
        } else {
            Exact(self.repr)
        };

        repr.map(|v| Self::new(v, new_context))
    }

    /// Explicitly change the rounding mode of the number.
    ///
    /// This operation has no cost.
    #[inline]
    pub fn with_rounding<NewR: Round>(self) -> FBig<NewR, B> {
        FBig {
            repr: self.repr,
            context: Context::new(self.context.precision),
        }
    }

    /// Explicitly change the base of the float number.
    ///
    /// The precision of the result number will be at most equal to the
    /// precision of the original number (numerically), that is
    /// ```new_base ^ new_precision <= old_base ^ old_precision```.
    /// If any rounding happens during the conversion, if will follow
    /// the rounding mode specified by the type parameter.
    #[inline]
    #[allow(non_upper_case_globals)]
    pub fn with_base<const NewB: Word>(self) -> Rounded<FBig<R, NewB>> {
        // if self.context.precision is zero, then precision is also zero
        let precision =
            Repr::<B>::BASE.pow(self.context.precision).log2_bounds().0 / NewB.log2_bounds().1;
        self.with_base_and_precision(precision as usize)
    }

    /// Explicitly change the base of the float number with given precision (under the new base).
    ///
    /// Infinities are mapped to infinities inexactly, the error will [NoOp][Rounding::NoOp].
    /// 
    /// Conversion for float numbers with unlimited precision is only allowed in following cases:
    /// - The number is infinite
    /// - The new base NewB is a power of B
    /// - B is a power of the new base NewB
    /// 
    /// # Panic
    ///
    /// Panics if the precision is 0 when the base conversion cannot be done losslessly.
    #[allow(non_upper_case_globals)]
    pub fn with_base_and_precision<const NewB: Word>(
        self,
        precision: usize,
    ) -> Rounded<FBig<R, NewB>> {
        // shortcut if NewB is the same as B
        if NewB == B {
            return Exact(FBig {
                repr: Repr {
                    significand: self.repr.significand,
                    exponent: self.repr.exponent,
                },
                context: self.context,
            });
        }

        // shortcut for infinities
        let context = Context::<R>::new(precision);
        if self.repr.is_infinite() {
            return Inexact(FBig::new(Repr {
                significand: self.repr.significand,
                exponent: self.repr.exponent
            }, context), Rounding::NoOp);
        }

        if NewB > B {
            // shortcut if NewB is a power of B
            let n = ilog_exact(NewB, B);
            if n > 1 {
                let (exp, rem) = self.repr.exponent.div_rem_euclid(n as isize);
                let signif = self.repr.significand * B.pow(rem as u32);
                let repr = Repr::new(signif, exp);
                return context.repr_round(repr).map(|v| FBig::new(v, context));
            }
        } else {
            // shortcut if B is a power of NewB
            let n = ilog_exact(B, NewB);
            if n > 1 {
                let exp = self.repr.exponent * n as isize;
                let repr = Repr::new(self.repr.significand, exp);
                return Exact(FBig::new(repr, context));
            }
        }

        // if the base cannot be converted losslessly, the precision must be set
        if precision == 0 {
            panic_unlimited_precision();
        }

        // XXX: there's a potential optimization: if B is a multiple of NewB, then the factor B
        // should be trivially removed first, but this requires full support of const generics.

        // choose a exponent threshold such that number with exponent smaller than this value
        // will be converted by directly evaluating the power. The threshold here is chosen such
        // that the power under base 10 will fit in a double word.
        const THRESHOLD_SMALL_EXP: isize = (Word::BITS as f32 * 0.60206) as isize; // word bits * 2 / log2(10)
        if self.repr.exponent.abs() <= THRESHOLD_SMALL_EXP {
            // if the exponent is small enough, directly evaluate the exponent
            if self.repr.exponent >= 0 {
                let signif =
                    self.repr.significand * Repr::<B>::BASE.pow(self.repr.exponent as usize);
                Exact(FBig::new(Repr::new(signif, 0), context))
            } else {
                let num = Repr::new(self.repr.significand, 0);
                let den = Repr::new(Repr::<B>::BASE.pow(-self.repr.exponent as usize), 0);
                context.repr_div(num, &den).map(|v| FBig::new(v, context))
            }
        } else {
            // if the exponent is large, then we first estimate the result exponent as floor(exponent * log(B) / log(NewB)),
            // then the fractional part is multiplied with the original significand
            let work_context = Context::<R>::new(2 * precision); // double the precision to get the precision logarithm
            let new_exp =
                self.repr.exponent * work_context.ln(&Repr::new(Repr::<B>::BASE, 0)).value();
            let (exponent, rem) = new_exp.div_rem_euclid(work_context.ln_base::<NewB>());
            let exponent: isize = exponent.try_into().unwrap();
            let exp_rem = rem.exp();
            let significand = self.repr.significand * exp_rem.repr.significand;
            let repr = Repr::new(significand, exponent + exp_rem.repr.exponent);
            context.repr_round(repr).map(|v| FBig::new(v, context))
        }
    }

    /// Convert the float number to integer with the given rounding mode.
    ///
    /// Warning: If the float number has a very large exponent, it will be evaluated and result
    /// in allocating an huge integer and it might eat up all your memory.
    pub fn to_int(&self) -> Rounded<IBig> {
        check_inf(&self.repr);

        // shortcut when the number is already an integer
        if self.repr.exponent >= 0 {
            return Exact(
                &self.repr.significand * Repr::<B>::BASE.pow(self.repr.exponent as usize),
            );
        }

        let (hi, lo, precision) = self.split_at_point();
        let adjust = R::round_fract::<B>(&hi, lo, precision);
        Inexact(hi + adjust, adjust)
    }

    /// Get the integral part of the float
    ///
    /// Note: this function will adjust the precision accordingly!
    #[inline]
    pub fn trunc(&self) -> Self {
        check_inf(&self.repr);

        let exponent = self.repr.exponent;
        if exponent >= 0 {
            return self.clone();
        } else if exponent + (self.repr.digits_ub() as isize) < 0 {
            return Self::ZERO;
        }

        let shift = (-exponent) as usize;
        let signif = shr_digits::<B>(&self.repr.significand, shift);
        let context = Context::new(self.precision() - shift);
        FBig::new(Repr::new(signif, 0), context)
    }

    // Split the float number at the floating point, assuming it exists (the number is not a integer).
    // The method returns (integral part, fractional part, fraction precision).
    fn split_at_point(&self) -> (IBig, IBig, usize) {
        debug_assert!(self.repr.exponent < 0);

        let exponent = self.repr.exponent;
        if exponent + (self.repr.digits_ub() as isize) < 0 {
            return (IBig::ZERO, self.repr.significand.clone(), self.context.precision);
        }

        let shift = (-exponent) as usize;
        let (hi, lo) = split_digits_ref::<B>(&self.repr.significand, shift);
        (hi, lo, shift)
    }

    /// Get the fractional part of the float
    ///
    /// Note: this function will adjust the precision accordingly!
    #[inline]
    pub fn fract(&self) -> Self {
        check_inf(&self.repr);
        if self.repr.exponent >= 0 {
            return Self::ZERO;
        }

        let (_, lo, precision) = self.split_at_point();
        let context = Context::new(precision);
        FBig::new(Repr::new(lo, self.repr.exponent), context)
    }

    #[inline]
    pub fn ceil(&self) -> Self {
        check_inf(&self.repr);
        if self.repr.exponent >= 0 {
            return self.clone();
        }

        let (hi, lo, precision) = self.split_at_point();
        let rounding = mode::Up::round_fract::<B>(&hi, lo, precision);
        let context = Context::new(self.precision() - precision);
        FBig::new(Repr::new(hi + rounding, 0), context)
    }

    #[inline]
    pub fn floor(&self) -> Self {
        check_inf(&self.repr);
        if self.repr.exponent >= 0 {
            return self.clone();
        }

        let (hi, lo, precision) = self.split_at_point();
        let rounding = mode::Down::round_fract::<B>(&hi, lo, precision);
        let context = Context::new(self.precision() - precision);
        FBig::new(Repr::new(hi + rounding, 0), context)
    }
}

impl<R: Round> FBig<R, 2> {
    /// Convert the float number to [f32] with HalfEven rounding mode regardless of the mode associated with this number.
    pub fn to_f32(&self) -> Rounded<f32> {
        if self.repr.is_infinite() {
            return Inexact(self.repr.sign() * f32::INFINITY, Rounding::NoOp);
        } else if self > &Self::try_from(f32::MAX).unwrap() {
            return Inexact(f32::INFINITY, Rounding::AddOne);
        } else if self < &Self::try_from(f32::MIN).unwrap() {
            return Inexact(f32::NEG_INFINITY, Rounding::SubOne);
        }

        let context = Context::<HalfEven>::new(24);
        context
            .repr_round_ref(&self.repr)
            .map(|v| v.significand.to_f32().value() * (v.exponent as f32).exp2())
    }

    /// Convert the float number to [f64] with HalfEven rounding mode regardless of the mode associated with this number.
    pub fn to_f64(&self) -> Rounded<f64> {
        if self.repr.is_infinite() {
            return Inexact(self.repr.sign() * f64::INFINITY, Rounding::NoOp);
        } else if self > &Self::try_from(f64::MAX).unwrap() {
            return Inexact(f64::INFINITY, Rounding::AddOne);
        } else if self < &Self::try_from(f64::MIN).unwrap() {
            return Inexact(f64::NEG_INFINITY, Rounding::SubOne);
        }

        let context = Context::<HalfEven>::new(53);
        context
            .repr_round_ref(&self.repr)
            .map(|v| v.significand.to_f64().value() * (v.exponent as f64).exp2())
    }
}

impl<R: Round, const B: Word> From<IBig> for FBig<R, B> {
    #[inline]
    fn from(n: IBig) -> Self {
        let repr = Repr::new(n, 0);
        let context = Context::new(repr.digits());
        Self::new(repr, context)
    }
}

impl<R: Round, const B: Word> From<UBig> for FBig<R, B> {
    #[inline]
    fn from(n: UBig) -> Self {
        IBig::from(n).into()
    }
}

macro_rules! fbig_unsigned_conversions {
    ($($t:ty)*) => {$(
        impl<R: Round, const B: Word> From<$t> for FBig<R, B> {
            #[inline]
            fn from(value: $t) -> FBig<R, B> {
                UBig::from(value).into()
            }
        }
    )*};
}
fbig_unsigned_conversions!(u8 u16 u32 u64 u128 usize);

macro_rules! fbig_signed_conversions {
    ($($t:ty)*) => {$(
        impl<R: Round, const B: Word> From<$t> for FBig<R, B> {
            #[inline]
            fn from(value: $t) -> FBig<R, B> {
                IBig::from(value).into()
            }
        }
    )*};
}
fbig_signed_conversions!(i8 i16 i32 i64 i128 isize);
