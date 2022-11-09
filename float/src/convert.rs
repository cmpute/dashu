use core::{
    convert::{TryFrom, TryInto},
    num::FpCategory,
};

use crate::{
    error::{check_inf, panic_unlimited_precision},
    fbig::FBig,
    repr::{Context, Repr},
    round::{mode::HalfEven, Round, Rounded, Rounding},
    utils::ilog_exact,
};
use dashu_base::{
    Approximation::*, BitTest, ConversionError, DivRemEuclid, EstimatedLog2, FloatEncoding, Sign,
    Signed,
};
use dashu_int::{IBig, UBig, Word};

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
                    _ => Err(ConversionError::OutOfBounds),
                }
            }
        }
    };
}

impl_from_float_for_fbig!(f32);
impl_from_float_for_fbig!(f64);

impl<R: Round, const B: Word> FBig<R, B> {
    /// Convert the float number to base 10 (with decimal exponents).
    ///
    /// It's equivalent to `self.with_base::<10>()`. See [with_base()][Self::with_base]
    /// for the precision and rounding behavior.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_float::{FBig, DBig};
    /// use dashu_base::Approximation::*;
    /// use dashu_float::round::{mode::HalfAway, Rounding::*};
    ///
    /// assert_eq!(
    ///     FBig::<HalfAway, 2>::from_str_native("0x1234")?.to_decimal(),
    ///     Exact(DBig::from_str_native("4660")?)
    /// );
    /// assert_eq!(
    ///     FBig::<HalfAway, 2>::from_str_native("0x12.34")?.to_decimal(),
    ///     Inexact(DBig::from_str_native("18.20")?, NoOp)
    /// );
    /// assert_eq!(
    ///     FBig::<HalfAway, 2>::from_str_native("0x1.234p-4")?.to_decimal(),
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
    pub fn to_decimal(&self) -> Rounded<FBig<R, 10>> {
        self.clone().with_base::<10>()
    }

    /// Convert the float number to base 2 (with binary exponents).
    ///
    /// It's equivalent to `self.with_base::<2>()`. See [with_base()][Self::with_base]
    /// for the precision and rounding behavior.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_float::{FBig, DBig};
    /// use dashu_base::Approximation::*;
    /// use dashu_float::round::{mode::HalfAway, Rounding::*};
    ///
    /// assert_eq!(
    ///     DBig::from_str_native("1234")?.to_binary(),
    ///     Exact(FBig::<HalfAway, 2>::from_str_native("0x4d2")?)
    /// );
    /// assert_eq!(
    ///     DBig::from_str_native("12.34")?.to_binary(),
    ///     Inexact(FBig::<HalfAway, 2>::from_str_native("0xc.57")?, NoOp)
    /// );
    /// assert_eq!(
    ///     DBig::from_str_native("1.234e-1")?.to_binary(),
    ///     Inexact(FBig::<HalfAway, 2>::from_str_native("0x1.f97p-4")?, NoOp)
    /// );
    /// # Ok::<(), ParseError>(())
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the associated context has unlimited precision and the conversion
    /// cannot be performed losslessly.
    #[inline]
    pub fn to_binary(&self) -> Rounded<FBig<R, 2>> {
        self.clone().with_base::<2>()
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
    /// If any rounding happens during the conversion, if will follow
    /// the rounding mode specified by the generic parameter.
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
            return Inexact(
                FBig::new(
                    Repr {
                        significand: self.repr.significand,
                        exponent: self.repr.exponent,
                    },
                    context,
                ),
                Rounding::NoOp,
            );
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
    /// # Warning
    /// If the float number has a very large exponent, it will be evaluated and result
    /// in allocating an huge integer and it might eat up all your memory.
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
        check_inf(&self.repr);

        // shortcut when the number is already an integer
        if self.repr.exponent >= 0 {
            return Exact(
                &self.repr.significand * Repr::<B>::BASE.pow(self.repr.exponent as usize),
            );
        }

        let (hi, lo, precision) = self.split_at_point_internal();
        let adjust = R::round_fract::<B>(&hi, lo, precision);
        Inexact(hi + adjust, adjust)
    }
}

impl<R: Round> FBig<R, 2> {
    // TODO: support conversion to f32/f64 with arbitrary bases

    /// Convert the float number to [f32] with [HalfEven] rounding mode regardless of the mode associated with this number.
    ///
    /// This method is only available to base 2 float number. For other bases, it's required
    /// to convert the number to base 2 explicitly using `self.with_base_and_precision::<2>(23)`
    /// first, and then convert to [f32].
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str_native("1.234")?;
    /// assert_eq!(a.with_base_and_precision::<2>(23).value().to_f32().value(), 1.234);
    ///
    /// let b = DBig::INFINITY;
    /// assert_eq!(b.with_base_and_precision::<2>(23).value().to_f32().value(), f32::INFINITY);
    /// # Ok::<(), ParseError>(())
    /// ```
    pub fn to_f32(&self) -> Rounded<f32> {
        let sign = self.repr.sign();
        if self.repr.is_infinite() {
            return Inexact(sign * f32::INFINITY, Rounding::NoOp);
        }

        let context = Context::<HalfEven>::new(24);
        context.repr_round_ref(&self.repr).and_then(|v| {
            let man24: i32 = v.significand.try_into().unwrap();
            if v.exponent >= 128 {
                // max f32 = 2^128 * (1 - 2^-24)
                match sign {
                    Sign::Positive => Inexact(f32::INFINITY, Rounding::AddOne),
                    Sign::Negative => Inexact(f32::NEG_INFINITY, Rounding::SubOne),
                }
            } else if v.exponent < -149 - 24 {
                // min f32 = 2^-149
                Inexact(sign * 0f32, Rounding::NoOp)
            } else {
                match f32::encode(man24, v.exponent as i16) {
                    Exact(v) => Exact(v),
                    // this branch only happens when the result underflows
                    Inexact(v, _) => Inexact(v, Rounding::NoOp),
                }
            }
        })
    }

    /// Convert the float number to [f64] with [HalfEven] rounding mode regardless of the mode associated with this number.
    ///
    /// This method is only available to base 2 float number. For other bases, it's required
    /// to convert the number to base 2 explicitly using `self.with_base_and_precision::<2>(53)`
    /// first, and then convert to [f32].
    ///
    /// # Examples
    ///
    /// ```
    /// # use dashu_base::ParseError;
    /// # use dashu_float::DBig;
    /// let a = DBig::from_str_native("1.234")?;
    /// assert_eq!(a.with_base_and_precision::<2>(53).value().to_f64().value(), 1.234);
    ///
    /// let b = DBig::INFINITY;
    /// assert_eq!(b.with_base_and_precision::<2>(53).value().to_f64().value(), f64::INFINITY);
    /// # Ok::<(), ParseError>(())
    /// ```
    pub fn to_f64(&self) -> Rounded<f64> {
        let sign = self.repr.sign();
        if self.repr.is_infinite() {
            return Inexact(sign * f64::INFINITY, Rounding::NoOp);
        }

        let context = Context::<HalfEven>::new(53);
        context.repr_round_ref(&self.repr).and_then(|v| {
            let man53: i64 = v.significand.try_into().unwrap();
            if v.exponent >= 1024 {
                // max f64 = 2^1024 × (1 − 2^−53)
                match sign {
                    Sign::Positive => Inexact(f64::INFINITY, Rounding::AddOne),
                    Sign::Negative => Inexact(f64::NEG_INFINITY, Rounding::SubOne),
                }
            } else if v.exponent < -1074 - 53 {
                // min f64 = 2^-1074
                Inexact(sign * 0f64, Rounding::NoOp)
            } else {
                match f64::encode(man53, v.exponent as i16) {
                    Exact(v) => Exact(v),
                    // this branch only happens when the result underflows
                    Inexact(v, _) => Inexact(v, Rounding::NoOp),
                }
            }
        })
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
