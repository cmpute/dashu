use core::{
    convert::{TryFrom, TryInto},
    num::FpCategory,
};

use dashu_base::{
    Approximation::*, BitTest, ConversionError, DivRemEuclid, EstimatedLog2, FloatEncoding, Sign,
    Signed,
};
use dashu_int::{IBig, UBig, Word};

use crate::{
    error::{assert_finite, panic_unlimited_precision},
    fbig::FBig,
    repr::{Context, Repr},
    round::{
        mode::{HalfAway, HalfEven, Zero},
        Round, Rounded, Rounding,
    },
    utils::{ilog_exact, shl_digits, shl_digits_in_place, shr_digits},
};

impl<R: Round> Context<R> {
    /// Convert an [IBig] instance to a [FBig] instance with precision
    /// and rounding given by the context.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// use dashu_base::Approximation::*;
    /// use dashu_float::{Context, round::{mode::HalfAway, Rounding::*}};
    ///
    /// let context = Context::<HalfAway>::new(2);
    /// assert_eq!(context.convert_int::<10>((-12).into()), Exact(DBig::from_str_native("-12")?));
    /// assert_eq!(
    ///     context.convert_int::<10>(5678.into()),
    ///     Inexact(DBig::from_str_native("5.7e3")?, AddOne)
    /// );
    /// # Ok::<(), ParseError>(())
    /// ```
    pub fn convert_int<const B: Word>(&self, n: IBig) -> Rounded<FBig<R, B>> {
        let repr = Repr::<B>::new(n, 0);
        self.repr_round(repr).map(|v| FBig::new(v, *self))
    }
}

macro_rules! impl_from_float_for_fbig {
    ($t:ty) => {
        impl<R: Round> TryFrom<$t> for FBig<R, 2> {
            type Error = ConversionError;

            fn try_from(f: $t) -> Result<Self, Self::Error> {
                match f.decode() {
                    Ok((man, exp)) => {
                        let repr = Repr::new(man.into(), exp as _);

                        // The precision is inferenced from the mantissa, because the mantissa of
                        // normal float is always normalized. This will produce correct precision
                        // for subnormal floats
                        let bits = man.unsigned_abs().bit_len();
                        let context = Context::new(bits);
                        Ok(Self::new(repr, context))
                    }
                    Err(FpCategory::Infinite) => match f.sign() {
                        Sign::Positive => Ok(FBig::INFINITY),
                        Sign::Negative => Ok(FBig::NEG_INFINITY),
                    },
                    _ => Err(ConversionError::OutOfBounds), // NaN
                }
            }
        }
    };
}

impl_from_float_for_fbig!(f32);
impl_from_float_for_fbig!(f64);

impl<R: Round, const B: Word> FBig<R, B> {
    /// Convert the float number to base 10 (with decimal exponents) rounding to even
    /// and tying away from zero.
    ///
    /// It's equivalent to `self.with_rounding::<HalfAway>().with_base::<10>()`.
    /// The output is directly of type [DBig][crate::DBig].
    ///
    /// See [with_base()][Self::with_base] for the precision behavior.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_float::{FBig, DBig};
    /// use dashu_base::Approximation::*;
    /// use dashu_float::round::Rounding::*;
    ///
    /// type Real = FBig;
    ///
    /// assert_eq!(
    ///     Real::from_str_native("0x1234")?.to_decimal(),
    ///     Exact(DBig::from_str_native("4660")?)
    /// );
    /// assert_eq!(
    ///     Real::from_str_native("0x12.34")?.to_decimal(),
    ///     Inexact(DBig::from_str_native("18.20")?, NoOp)
    /// );
    /// assert_eq!(
    ///     Real::from_str_native("0x1.234p-4")?.to_decimal(),
    ///     Inexact(DBig::from_str_native("0.07111")?, AddOne)
    /// );
    /// # Ok::<(), ParseError>(())
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the associated context has unlimited precision and the conversion
    /// cannot be performed losslessly.
    #[inline]
    pub fn to_decimal(&self) -> Rounded<FBig<HalfAway, 10>> {
        self.clone().with_rounding().with_base::<10>()
    }

    /// Convert the float number to base 2 (with binary exponents) rounding towards zero.
    ///
    /// It's equivalent to `self.with_rounding::<Zero>().with_base::<2>()`.
    ///
    /// See [with_base()][Self::with_base] for the precision and rounding behavior.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_float::{FBig, DBig};
    /// use dashu_base::Approximation::*;
    /// use dashu_float::round::{mode::HalfAway, Rounding::*};
    ///
    /// type Real = FBig;
    ///
    /// assert_eq!(
    ///     DBig::from_str_native("1234")?.to_binary(),
    ///     Exact(Real::from_str_native("0x4d2")?)
    /// );
    /// assert_eq!(
    ///     DBig::from_str_native("12.34")?.to_binary(),
    ///     Inexact(Real::from_str_native("0xc.57")?, NoOp)
    /// );
    /// assert_eq!(
    ///     DBig::from_str_native("1.234e-1")?.to_binary(),
    ///     Inexact(Real::from_str_native("0x1.f97p-4")?, NoOp)
    /// );
    /// # Ok::<(), ParseError>(())
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the associated context has unlimited precision and the conversion
    /// cannot be performed losslessly.
    #[inline]
    pub fn to_binary(&self) -> Rounded<FBig<Zero, 2>> {
        self.clone().with_rounding().with_base::<2>()
    }

    /// Explicitly change the precision of the float number.
    ///
    /// If the given precision is less than the current value in the context,
    /// it will be rounded with the rounding mode specified by the generic parameter.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use dashu_base::ParseError;
    /// # use dashu_float::{FBig, DBig};
    /// use dashu_base::Approximation::*;
    /// use dashu_float::round::{mode::HalfAway, Rounding::*};
    ///
    /// let a = DBig::from_str_native("2.345")?;
    /// assert_eq!(a.precision(), 4);
    /// assert_eq!(
    ///     a.clone().with_precision(3),
    ///     Inexact(DBig::from_str_native("2.35")?, AddOne)
    /// );
    /// assert_eq!(
    ///     a.clone().with_precision(5),
    ///     Exact(DBig::from_str_native("2.345")?)
    /// );
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn with_precision(self, precision: usize) -> Rounded<Self> {
        let new_context = Context::new(precision);

        // shrink if necessary
        let repr = if self.context.precision > precision {
            // it also handles unlimited precision
            new_context.repr_round(self.repr)
        } else {
            Exact(self.repr)
        };

        repr.map(|v| Self::new(v, new_context))
    }

    /// Explicitly change the rounding mode of the number.
    ///
    /// This operation doesn't modify the underlying representation, it only changes
    /// the rounding mode in the context.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use dashu_base::ParseError;
    /// # use dashu_float::{FBig, DBig};
    /// use dashu_base::Approximation::*;
    /// use dashu_float::round::{mode::{HalfAway, Zero}, Rounding::*};
    ///
    /// type DBigHalfAway = DBig;
    /// type DBigZero = FBig::<Zero, 10>;
    ///
    /// let a = DBigHalfAway::from_str_native("2.345")?;
    /// let b = DBigZero::from_str_native("2.345")?;
    /// assert_eq!(a.with_rounding::<Zero>(), b);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn with_rounding<NewR: Round>(self) -> FBig<NewR, B> {
        FBig {
            repr: self.repr,
            context: Context::new(self.context.precision),
        }
    }

    /// Explicitly change the base of the float number.
    ///
    /// This function internally calls [with_base_and_precision][Self::with_base_and_precision].
    /// The precision of the result number will be calculated in such a way that the new
    /// limit of the significand is less than or equal to before. That is, the new precision
    /// will be the max integer such that
    ///
    /// `NewB ^ new_precision <= B ^ old_precision`
    ///
    /// If any rounding happens during the conversion, it follows the rounding mode specified
    /// by the generic parameter.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use dashu_base::ParseError;
    /// # use dashu_float::{FBig, DBig};
    /// use dashu_base::Approximation::*;
    /// use dashu_float::round::{mode::Zero, Rounding::*};
    ///
    /// type FBin = FBig;
    /// type FDec = FBig<Zero, 10>;
    /// type FHex = FBig<Zero, 16>;
    ///
    /// let a = FBin::from_str_native("0x1.234")?; // 0x1234 * 2^-12
    /// assert_eq!(
    ///     a.clone().with_base::<10>(),
    ///     // 1.1376953125 rounded towards zero
    ///     Inexact(FDec::from_str_native("1.137")?, NoOp)
    /// );
    /// assert_eq!(
    ///     a.clone().with_base::<16>(),
    ///     // conversion is exact when the new base is a power of the old base
    ///     Exact(FHex::from_str_native("1.234")?)
    /// );
    /// # Ok::<(), ParseError>(())
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the associated context has unlimited precision and the conversion
    /// cannot be performed losslessly.
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
    /// Infinities are mapped to infinities inexactly, the error will be [NoOp][Rounding::NoOp].
    ///
    /// Conversion for float numbers with unlimited precision is only allowed in following cases:
    /// - The number is infinite
    /// - The new base NewB is a power of B
    /// - B is a power of the new base NewB
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use dashu_base::ParseError;
    /// # use dashu_float::{FBig, DBig};
    /// use dashu_base::Approximation::*;
    /// use dashu_float::round::{mode::Zero, Rounding::*};
    ///
    /// type FBin = FBig;
    /// type FDec = FBig<Zero, 10>;
    /// type FHex = FBig<Zero, 16>;
    ///
    /// let a = FBin::from_str_native("0x1.234")?; // 0x1234 * 2^-12
    /// assert_eq!(
    ///     a.clone().with_base_and_precision::<10>(8),
    ///     // 1.1376953125 rounded towards zero
    ///     Inexact(FDec::from_str_native("1.1376953")?, NoOp)
    /// );
    /// assert_eq!(
    ///     a.clone().with_base_and_precision::<16>(8),
    ///     // conversion can be exact when the new base is a power of the old base
    ///     Exact(FHex::from_str_native("1.234")?)
    /// );
    /// assert_eq!(
    ///     a.clone().with_base_and_precision::<16>(2),
    ///     // but the conversion is still inexact if the target precision is smaller
    ///     Inexact(FHex::from_str_native("1.2")?, NoOp)
    /// );
    /// # Ok::<(), ParseError>(())
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the associated context has unlimited precision and the conversion
    /// cannot be performed losslessly.
    #[allow(non_upper_case_globals)]
    #[inline]
    pub fn with_base_and_precision<const NewB: Word>(
        self,
        precision: usize,
    ) -> Rounded<FBig<R, NewB>> {
        let context = Context::<R>::new(precision);
        context
            .convert_base(self.repr)
            .map(|repr| FBig::new(repr, context))
    }

    /// Convert the float number to integer with the given rounding mode.
    ///
    /// # Warning
    ///
    /// If the float number has a very large exponent, it will be evaluated and result
    /// in allocating an huge integer and it might eat up all your memory.
    ///
    /// To get a rough idea of how big the number is, it's recommended to use [EstimatedLog2].
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_float::{FBig, DBig};
    /// use dashu_base::Approximation::*;
    /// use dashu_float::round::Rounding::*;
    ///
    /// assert_eq!(
    ///     DBig::from_str_native("1234")?.to_int(),
    ///     Exact(1234.into())
    /// );
    /// assert_eq!(
    ///     DBig::from_str_native("1.234e6")?.to_int(),
    ///     Exact(1234000.into())
    /// );
    /// assert_eq!(
    ///     DBig::from_str_native("1.234")?.to_int(),
    ///     Inexact(1.into(), NoOp)
    /// );
    /// # Ok::<(), ParseError>(())
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the number is infinte
    pub fn to_int(&self) -> Rounded<IBig> {
        assert_finite(&self.repr);

        // shortcut when the number is already an integer
        if self.repr.exponent >= 0 {
            return Exact(shl_digits::<B>(&self.repr.significand, self.repr.exponent as usize));
        }

        let (hi, lo, precision) = self.split_at_point_internal();
        let adjust = R::round_fract::<B>(&hi, lo, precision);
        Inexact(hi + adjust, adjust)
    }

    /// Convert the float number to [f32] with the rounding mode associated with the type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// assert_eq!(DBig::from_str_native("1.234")?.to_f32().value(), 1.234);
    /// assert_eq!(DBig::INFINITY.to_f32().value(), f32::INFINITY);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn to_f32(&self) -> Rounded<f32> {
        if self.repr.is_infinite() {
            return Inexact(self.sign() * f32::INFINITY, Rounding::NoOp);
        }

        let context = Context::<R>::new(24);
        if B != 2 {
            let rounded: Rounded<Repr<2>> = context.convert_base(self.repr.clone());
            rounded.and_then(|v| v.into_f32_internal())
        } else {
            context
                .repr_round_ref(&self.repr)
                .and_then(|v| v.into_f32_internal())
        }
    }

    /// Convert the float number to [f64] with [HalfEven] rounding mode regardless of the mode associated with this number.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// assert_eq!(DBig::from_str_native("1.234")?.to_f64().value(), 1.234);
    /// assert_eq!(DBig::INFINITY.to_f64().value(), f64::INFINITY);
    /// # Ok::<(), ParseError>(())
    /// ```
    #[inline]
    pub fn to_f64(&self) -> Rounded<f64> {
        if self.repr.is_infinite() {
            return Inexact(self.sign() * f64::INFINITY, Rounding::NoOp);
        }

        let context = Context::<HalfEven>::new(53);
        if B != 2 {
            let rounded: Rounded<Repr<2>> = context.convert_base(self.repr.clone());
            rounded.and_then(|v| v.into_f64_internal())
        } else {
            context
                .repr_round_ref(&self.repr)
                .and_then(|v| v.into_f64_internal())
        }
    }
}

impl<R: Round> Context<R> {
    // Convert the [Repr] from base B to base NewB, with the precision under the target base from this context.
    #[allow(non_upper_case_globals)]
    fn convert_base<const B: Word, const NewB: Word>(&self, repr: Repr<B>) -> Rounded<Repr<NewB>> {
        // shortcut if NewB is the same as B
        if NewB == B {
            return Exact(Repr {
                significand: repr.significand,
                exponent: repr.exponent,
            });
        }

        // shortcut for infinities, no rounding happens but the result is inexact
        if repr.is_infinite() {
            return Inexact(
                Repr {
                    significand: repr.significand,
                    exponent: repr.exponent,
                },
                Rounding::NoOp,
            );
        }

        if NewB > B {
            // shortcut if NewB is a power of B
            let n = ilog_exact(NewB, B);
            if n > 1 {
                let (exp, rem) = repr.exponent.div_rem_euclid(n as isize);
                let signif = repr.significand * B.pow(rem as u32);
                let repr = Repr::new(signif, exp);
                return self.repr_round(repr);
            }
        } else {
            // shortcut if B is a power of NewB
            let n = ilog_exact(B, NewB);
            if n > 1 {
                let exp = repr.exponent * n as isize;
                return Exact(Repr::new(repr.significand, exp));
            }
        }

        // if the base cannot be converted losslessly, the precision must be set
        if self.precision == 0 {
            panic_unlimited_precision();
        }

        // XXX: there's a potential optimization: if B is a multiple of NewB, then the factor B
        // should be trivially removed first, but this requires full support of const generics.

        // choose a exponent threshold such that number with exponent smaller than this value
        // will be converted by directly evaluating the power. The threshold here is chosen such
        // that the power under base 10 will fit in a double word.
        const THRESHOLD_SMALL_EXP: isize = (Word::BITS as f32 * 0.60206) as isize; // word bits * 2 / log2(10)
        if repr.exponent.abs() <= THRESHOLD_SMALL_EXP {
            // if the exponent is small enough, directly evaluate the exponent
            if repr.exponent >= 0 {
                let signif = repr.significand * Repr::<B>::BASE.pow(repr.exponent as usize);
                Exact(Repr::new(signif, 0))
            } else {
                let num = Repr::new(repr.significand, 0);
                let den = Repr::new(Repr::<B>::BASE.pow(-repr.exponent as usize).into(), 0);
                self.repr_div(num, den)
            }
        } else {
            // if the exponent is large, then we first estimate the result exponent as floor(exponent * log(B) / log(NewB)),
            // then the fractional part is multiplied with the original significand
            let work_context = Context::<R>::new(2 * self.precision); // double the precision to get the precise logarithm
            let new_exp = repr.exponent
                * work_context
                    .ln(&Repr::new(Repr::<B>::BASE.into(), 0))
                    .value();
            let (exponent, rem) = new_exp.div_rem_euclid(work_context.ln_base::<NewB>());
            let exponent: isize = exponent.try_into().unwrap();
            let exp_rem = rem.exp();
            let significand = repr.significand * exp_rem.repr.significand;
            let repr = Repr::new(significand, exponent + exp_rem.repr.exponent);
            self.repr_round(repr)
        }
    }
}

impl<const B: Word> Repr<B> {
    // this method requires that the representation is already rounded to 24 binary bits
    fn into_f32_internal(self) -> Rounded<f32> {
        assert!(B == 2);
        debug_assert!(self.is_finite());
        debug_assert!(self.significand.bit_len() <= 24);

        let sign = self.sign();
        let man24: i32 = self.significand.try_into().unwrap();
        if self.exponent >= 128 {
            // max f32 = 2^128 * (1 - 2^-24)
            match sign {
                Sign::Positive => Inexact(f32::INFINITY, Rounding::AddOne),
                Sign::Negative => Inexact(f32::NEG_INFINITY, Rounding::SubOne),
            }
        } else if self.exponent < -149 - 24 {
            // min f32 = 2^-149
            Inexact(sign * 0f32, Rounding::NoOp)
        } else {
            match f32::encode(man24, self.exponent as i16) {
                Exact(v) => Exact(v),
                // this branch only happens when the result underflows
                Inexact(v, _) => Inexact(v, Rounding::NoOp),
            }
        }
    }

    /// Convert the float number representation to a [f32] with the default IEEE 754 rounding mode.
    ///
    /// The default IEEE 754 rounding mode is [HalfEven] (rounding to nearest, ties to even). To convert
    /// the float number with a specific rounding mode, please use [FBig::to_f32].
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::Approximation::*;
    /// # use dashu_float::{Repr, round::Rounding::*};
    /// assert_eq!(Repr::<2>::one().to_f32(), Exact(1.0));
    /// assert_eq!(Repr::<10>::infinity().to_f32(), Inexact(f32::INFINITY, NoOp));
    /// ```
    #[inline]
    pub fn to_f32(&self) -> Rounded<f32> {
        // Note: the implementation here should be kept consistent with FBig::to_f32

        if self.is_infinite() {
            return Inexact(self.sign() * f32::INFINITY, Rounding::NoOp);
        }

        let context = Context::<HalfEven>::new(24);
        if B != 2 {
            let rounded: Rounded<Repr<2>> = context.convert_base(self.clone());
            rounded.and_then(|v| v.into_f32_internal())
        } else {
            context
                .repr_round_ref(self)
                .and_then(|v| v.into_f32_internal())
        }
    }

    // this method requires that the representation is already rounded to 53 binary bits
    fn into_f64_internal(self) -> Rounded<f64> {
        assert!(B == 2);
        debug_assert!(self.is_finite());
        debug_assert!(self.significand.bit_len() <= 53);

        let sign = self.sign();
        let man53: i64 = self.significand.try_into().unwrap();
        if self.exponent >= 1024 {
            // max f64 = 2^1024 × (1 − 2^−53)
            match sign {
                Sign::Positive => Inexact(f64::INFINITY, Rounding::AddOne),
                Sign::Negative => Inexact(f64::NEG_INFINITY, Rounding::SubOne),
            }
        } else if self.exponent < -1074 - 53 {
            // min f64 = 2^-1074
            Inexact(sign * 0f64, Rounding::NoOp)
        } else {
            match f64::encode(man53, self.exponent as i16) {
                Exact(v) => Exact(v),
                // this branch only happens when the result underflows
                Inexact(v, _) => Inexact(v, Rounding::NoOp),
            }
        }
    }

    /// Convert the float number representation to a [f64] with the default IEEE 754 rounding mode.
    ///
    /// The default IEEE 754 rounding mode is [HalfEven] (rounding to nearest, ties to even). To convert
    /// the float number with a specific rounding mode, please use [FBig::to_f64].
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::Approximation::*;
    /// # use dashu_float::{Repr, round::Rounding::*};
    /// assert_eq!(Repr::<2>::one().to_f64(), Exact(1.0));
    /// assert_eq!(Repr::<10>::infinity().to_f64(), Inexact(f64::INFINITY, NoOp));
    /// ```
    #[inline]
    pub fn to_f64(&self) -> Rounded<f64> {
        // Note: the implementation here should be kept consistent with FBig::to_f64

        if self.is_infinite() {
            return Inexact(self.sign() * f64::INFINITY, Rounding::NoOp);
        }

        let context = Context::<HalfEven>::new(53);
        if B != 2 {
            let rounded: Rounded<Repr<2>> = context.convert_base(self.clone());
            rounded.and_then(|v| v.into_f64_internal())
        } else {
            context
                .repr_round_ref(self)
                .and_then(|v| v.into_f64_internal())
        }
    }

    /// Convert the float number representation to a [IBig].
    ///
    /// The fractional part is always rounded to zero. To convert with other rounding modes,
    /// please use [FBig::to_int()].
    ///
    /// # Warning
    ///
    /// If the float number has a very large exponent, it will be evaluated and result
    /// in allocating an huge integer and it might eat up all your memory.
    ///
    /// To get a rough idea of how big the number is, it's recommended to use [EstimatedLog2].
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::Approximation::*;
    /// # use dashu_int::IBig;
    /// # use dashu_float::{Repr, round::Rounding::*};
    /// assert_eq!(Repr::<2>::neg_one().to_int(), Exact(IBig::NEG_ONE));
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the number is infinte.
    pub fn to_int(&self) -> Rounded<IBig> {
        assert_finite(self);

        if self.exponent >= 0 {
            // the number is already an integer
            Exact(shl_digits::<B>(&self.significand, self.exponent as usize))
        } else if self.smaller_than_one() {
            // the number is definitely smaller than
            Inexact(IBig::ZERO, Rounding::NoOp)
        } else {
            let int = shr_digits::<B>(&self.significand, (-self.exponent) as usize);
            Inexact(int, Rounding::NoOp)
        }
    }
}

impl<R: Round, const B: Word> From<IBig> for FBig<R, B> {
    #[inline]
    fn from(n: IBig) -> Self {
        Self::from_parts(n, 0)
    }
}

impl<R: Round, const B: Word> From<UBig> for FBig<R, B> {
    #[inline]
    fn from(n: UBig) -> Self {
        IBig::from(n).into()
    }
}

impl<R: Round, const B: Word> TryFrom<FBig<R, B>> for IBig {
    type Error = ConversionError;

    #[inline]
    fn try_from(value: FBig<R, B>) -> Result<Self, Self::Error> {
        if value.repr.is_infinite() {
            Err(ConversionError::OutOfBounds)
        } else if value.repr.exponent < 0 {
            Err(ConversionError::LossOfPrecision)
        } else {
            let mut int = value.repr.significand;
            shl_digits_in_place::<B>(&mut int, value.repr.exponent as usize);
            Ok(int)
        }
    }
}

impl<R: Round, const B: Word> TryFrom<FBig<R, B>> for UBig {
    type Error = ConversionError;

    #[inline]
    fn try_from(value: FBig<R, B>) -> Result<Self, Self::Error> {
        let int: IBig = value.try_into()?;
        int.try_into()
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
