use crate::{
    fbig::FBig,
    ibig_ext::{log_pow, log_rem, remove_pow},
    repr::{Context, Repr},
    round::{Round, Rounding, Rounded},
};
use dashu_base::Approximation;
use dashu_int::{IBig, UBig, Word};

impl<R: Round> Context<R> {
    pub fn convert_int<const B: Word>(&self, n: IBig) -> FBig<B, R> {
        let repr = Repr::<B>::new(n, 0);
        FBig::new_raw(self.repr_round(repr).value(), *self)
    }
}

impl<R: Round> From<f32> for FBig<2, R> {
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

impl<R: Round> From<f64> for FBig<2, R> {
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

impl<const B: Word, R: Round> From<IBig> for FBig<B, R> {
    #[inline]
    fn from(n: IBig) -> Self {
        let repr = Repr::new(n, 0);
        let context = Context::new(repr.digits());
        Self::new_raw(repr, context)
    }
}

impl<const B: Word, R: Round> From<UBig> for FBig<B, R> {
    #[inline]
    fn from(n: UBig) -> Self {
        IBig::from(n).into()
    }
}

impl<const B: Word, R: Round> FBig<B, R> {
    /// Convert the float number to decimal based exponents.
    #[inline]
    pub fn to_decimal(&self) -> Approximation<FBig<10, R>, Rounding> {
        self.clone().with_base::<10>()
    }

    /// Convert the float number to decimal based exponents.
    #[inline]
    pub fn to_binary(&self) -> Approximation<FBig<2, R>, Rounding> {
        self.clone().with_base::<2>()
    }

    /// Explicitly change the precision of the number.
    ///
    /// If the given precision is less than the previous value,
    /// it will be rounded following the rounding mode specified by the type parameter.
    #[inline]
    pub fn with_precision(self, precision: usize) -> Approximation<Self, Rounding> {
        let new_context = Context::new(precision);

        // shrink if necessary
        let repr = if self.context.precision > precision {
            new_context.repr_round(self.repr)
        } else {
            Approximation::Exact(self.repr)
        };

        repr.map(|v| Self::new_raw(v, new_context))
    }

    /// Explicitly change the rounding mode of the number.
    ///
    /// This operation has no cost.
    #[inline]
    pub fn with_rounding<NewR: Round>(self) -> FBig<B, NewR> {
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
    pub fn with_base<const NewB: Word>(self) -> Approximation<FBig<NewB, R>, Rounding> {
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
            let mut value = FBig::<NewB, R>::from_ratio(num, den, precision + 1);
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
            let mut value = FBig::<NewB, R>::from_ratio(num, den, precision + 1);
            value.repr.exponent -= log_v as isize;
            value
        };

        result.with_precision(precision)
    }

    #[allow(non_upper_case_globals)]
    fn with_base_and_precision<const NewB: Word>(self, precision: usize) -> Rounded<FBig<NewB, R>> {
        let context = Context::new(precision);

        const THRESHOLD_SMALL_EXP: isize = 4;
        if self.repr.exponent.abs() <= THRESHOLD_SMALL_EXP {
            // if the exponent is small enough
            if self.repr.exponent >= 0 {
                let signif = self.repr.significand * Repr::<B>::BASE.pow(self.repr.exponent as usize);
                return Approximation::Exact(FBig::new_raw(Repr::new( signif, 0), context))
            } else {
                let num = Repr::new(self.repr.significand, 0);
                let den = Repr::new(Repr::<B>::BASE.pow(-self.repr.exponent as usize), 0);
                return context.repr_div(num, &den).map(|v| FBig::new_raw(v, context));
            }
        } else {
            // exp_f = self.repr.exponent * log(B) / log(NewB)
            // exp = trunc(exp_f)
            // signif = self.repr.significand * exp(log(NewB) * fract(exp_f))
            unimplemented!()
        }
    }

    // TODO: let all these to_* functions return `Approximation`

    /// Convert the float number to native [f32] with the given rounding mode.
    fn to_f32(&self) -> f32 {
        unimplemented!()
    }

    /// Convert the float number to native [f64] with the given rounding mode.
    fn to_f64(&self) -> f64 {
        unimplemented!()
    }

    /// Convert the float number to integer with the given rounding mode.
    fn to_int(&self) -> IBig {
        unimplemented!()
    }
}
