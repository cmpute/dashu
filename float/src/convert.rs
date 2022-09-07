use crate::{
    fbig::FBig,
    ibig_ext::{log_pow, log_rem},
    repr::{Context, Repr},
    round::{Round, Rounded, mode}, utils::{shr_digits, split_digits_ref}, error::panic_operate_with_inf,
};
use dashu_base::Approximation;
use dashu_int::{IBig, UBig, Word};

impl<R: Round> Context<R> {
    /// Convert an [IBig] instance to a [FBig] instance with precision
    /// and rounding given by the context.
    pub fn convert_int<const B: Word>(&self, n: IBig) -> Rounded<FBig<R, B>> {
        let repr = Repr::<B>::new(n, 0);
        self.repr_round(repr).map(|v| FBig::new(v, *self))
    }
}

// TODO: make conversion from f32/f64 TryFrom, we need to correctly deal with nan and subnormals
impl<R: Round> From<f32> for FBig<R, 2> {
    fn from(f: f32) -> Self {
        let bits: u32 = f.to_bits();

        let mut exponent: isize = ((bits >> 23) & 0xff) as isize;
        exponent -= 127 + 23; // bias + mantissa shift

        let mantissa = if exponent == 0 {
            (bits & 0x7fffff) << 1
        } else {
            (bits & 0x7fffff) | 0x800000
        } as i32;
        let mantissa = if bits >> 31 == 0 {
            IBig::from(mantissa)
        } else {
            IBig::from(-mantissa)
        };

        Self {
            repr: Repr::new(mantissa, exponent),
            context: Context::new(24),
        }
    }
}

impl<R: Round> From<f64> for FBig<R, 2> {
    fn from(f: f64) -> Self {
        let bits: u64 = f.to_bits();

        let mut exponent: isize = ((bits >> 52) & 0x7ff) as isize;
        exponent -= 1023 + 52; // bias + mantissa shift

        let mantissa = if exponent == 0 {
            (bits & 0xfffffffffffff) << 1
        } else {
            (bits & 0xfffffffffffff) | 0x10000000000000
        } as i64;
        let mantissa = if bits >> 63 == 0 {
            IBig::from(mantissa)
        } else {
            IBig::from(-mantissa)
        };

        Self {
            repr: Repr::new(mantissa, exponent),
            context: Context::new(53),
        }
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
            Approximation::Exact(self.repr)
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

    /// Explicitly change the radix of the float number.
    ///
    /// The precision of the result number will be at most equal to the
    /// precision of the original number (numerically), that is
    /// ```new_radix ^ new_precision <= old_radix ^ old_precision```.
    /// If any rounding happens during the conversion, if will follow
    /// the rounding mode specified by the type parameter.
    #[allow(non_upper_case_globals)]
    pub fn with_base<const NewB: Word>(self) -> Rounded<FBig<R, NewB>> {
        if NewB == B {
            return Approximation::Exact(FBig {
                repr: Repr {
                    significand: self.repr.significand,
                    exponent: self.repr.exponent,
                },
                context: self.context,
            });
        }
        // TODO: shortcut if X is a power of NewX

        // Calculate the new precision
        // new_precision = floor_log_radix2(radix1^precision)
        let precision = log_pow(&UBig::from_word(B), self.context.precision, NewB as usize);

        // Convert by calculating logarithm
        // TODO: refactor to call with_base_and_precision, the precision = floor(e*log_NewB(B)).
        let result = if self.repr.exponent == 0 {
            // direct copy if the exponent is zero
            return Approximation::Exact(FBig {
                repr: Repr {
                    significand: self.repr.significand,
                    exponent: 0,
                },
                context: Context::new(precision),
            });
        } else if self.repr.exponent > 0 {
            // denote log with base of radix2 as lgr2, then
            // mantissa * radix1 ^ exp1
            // = mantissa * radix2 ^ lgr2(radix1^exp1)
            // = mantissa * (radix2 ^ floor_lgr2(radix1^exp1) + rem_lgr2(radix1^exp1))
            // = mantissa * ratio * (radix2 ^ floor_lgr2(radix1^exp1))
            // where the ratio is
            // 1 + rem_lgr2(radix1^exp1) / (radix2 ^ floor_lgr2(radix1^exp1))
            // = radix1^exp1 / (radix1^exp1 - rem_lgr2(radix1^exp1))

            let precision_ub = UBig::from_word(B).pow(self.repr.exponent as usize);
            let (log_v, log_r) = log_rem(&precision_ub, NewB as usize);
            let den = IBig::from(&precision_ub - log_r);
            let num = IBig::from(precision_ub) * self.repr.significand;
            let mut value = FBig::<R, NewB>::from_ratio(num, den, precision + 1);
            value.repr.exponent += log_v as isize;
            value
        } else {
            // denote log with base of radix2 as lgr2, then
            // mantissa / radix1 ^ exp1
            // = mantissa / radix2 ^ lgr2(radix1^exp1)
            // = mantissa / (radix2 ^ floor_lgr2(radix1^exp1) + rem_lgr2(radix1^exp1))
            // = mantissa (1 / (radix2 ^ floor_lgr2(..)) - rem_lgr2(..) / (radix2 ^ floor_lgr2(..) * (radix2 ^ floor_lgr2(..) + rem_lgr2(..)))
            // = mantissa * ratio * (1 / (radix2 ^ floor_lgr2(radix1^exp1))
            // where the ratio is
            // 1 - rem_lgr2(radix1^exp1) / (radix2 ^ floor_lgr2(radix1^exp1) + rem_lgr2(radix1^exp1))
            // = radix2 ^ floor_lgr2(radix1^exp1) / radix1^exp1

            let precision_ub = UBig::from_word(B).pow(-self.repr.exponent as usize);
            let (log_v, log_r) = log_rem(&precision_ub, NewB as usize);
            let num = IBig::from(&precision_ub - log_r) * self.repr.significand;
            let den = IBig::from(precision_ub);
            let mut value = FBig::<R, NewB>::from_ratio(num, den, precision + 1);
            value.repr.exponent -= log_v as isize;
            value
        };

        result.with_precision(precision)
    }

    #[allow(non_upper_case_globals)]
    fn with_base_and_precision<const NewB: Word>(self, precision: usize) -> Rounded<FBig<R, NewB>> {
        let context = Context::new(precision);

        const THRESHOLD_SMALL_EXP: isize = 4;
        if self.repr.exponent.abs() <= THRESHOLD_SMALL_EXP {
            // if the exponent is small enough
            if self.repr.exponent >= 0 {
                let signif =
                    self.repr.significand * Repr::<B>::BASE.pow(self.repr.exponent as usize);
                return Approximation::Exact(FBig::new(Repr::new(signif, 0), context));
            } else {
                let num = Repr::new(self.repr.significand, 0);
                let den = Repr::new(Repr::<B>::BASE.pow(-self.repr.exponent as usize), 0);
                return context.repr_div(num, &den).map(|v| FBig::new(v, context));
            }
        } else {
            // exp_f = self.repr.exponent * log(B) / log(NewB)
            // exp = trunc(exp_f)
            // signif = self.repr.significand * exp(log(NewB) * fract(exp_f))

            // TODO: implement this branch after trunc/fract/floor/ceil are tested
            // and obselete the old algorithm in with_base()
            unimplemented!() 
        }
    }

    /// Convert the float number to native [f32] with the given rounding mode.
    fn to_f32(&self) -> Option<Rounded<f32>> {
        unimplemented!()
    }

    /// Convert the float number to native [f64] with the given rounding mode.
    fn to_f64(&self) -> Option<Rounded<f64>> {
        unimplemented!()
    }

    /// Convert the float number to integer with the given rounding mode.
    fn to_int(&self) -> Option<Rounded<IBig>> {
        unimplemented!()
    }

    #[inline]
    pub fn trunc(&self) -> Self {
        if self.repr.is_infinite() {
            panic_operate_with_inf();
        }

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

    #[inline]
    pub fn fract(&self) -> Self {
        if self.repr.is_infinite() {
            panic_operate_with_inf();
        }
        if self.repr.exponent >= 0 {
            return Self::ZERO;
        }

        let (_, lo, precision) = self.split_at_point();
        let context = Context::new(precision);
        FBig::new(Repr::new(lo, self.repr.exponent), context)
    }

    #[inline]
    pub fn ceil(&self) -> Self {
        if self.repr.is_infinite() {
            panic_operate_with_inf();
        }
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
        if self.repr.is_infinite() {
            panic_operate_with_inf();
        }
        if self.repr.exponent >= 0 {
            return self.clone();
        }

        let (hi, lo, precision) = self.split_at_point();
        let rounding = mode::Down::round_fract::<B>(&hi, lo, precision);
        let context = Context::new(self.precision() - precision);
        FBig::new(Repr::new(hi + rounding, 0), context)
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
