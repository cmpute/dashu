use crate::{
    ibig_ext::{log_pow, log_rem, remove_pow},
    repr::{BinaryRepr, DecimalRepr, FloatRepr},
    round::Rounding,
    utils::{get_precision, shr_rem_radix_in_place},
};
use core::convert::TryInto;
use dashu_int::{IBig, UBig};

impl<const R: u8> From<f32> for BinaryRepr<R> {
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
            mantissa,
            exponent,
            precision: 24,
        }
    }
}

impl<const R: u8> From<f64> for BinaryRepr<R> {
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
            mantissa,
            exponent,
            precision: 53,
        }
    }
}

impl<const X: usize, const R: u8> FloatRepr<X, R> {
    /// Create a floating number from a integer
    #[inline]
    pub fn from_integer(integer: IBig, precision: usize) -> Self {
        Self::from_parts_with_precision(integer, 0, precision)
    }

    /// Convert the float number to decimal based exponents.
    ///
    /// It's equivalent to [Self::with_radix::<10>()]
    #[inline]
    pub fn into_decimal(self) -> DecimalRepr<R> {
        self.with_radix::<10>()
    }

    /// Convert the float number to decimal based exponents.
    #[inline]
    pub fn to_decimal(&self) -> DecimalRepr<R> {
        self.clone().with_radix::<10>()
    }

    /// Convert the float number to binary based exponents.
    ///
    /// It's equivalent to [Self::with_radix::<2>()]
    #[inline]
    pub fn into_binary(self) -> BinaryRepr<R> {
        self.with_radix::<2>()
    }

    /// Convert the float number to decimal based exponents.
    #[inline]
    pub fn to_binary(&self) -> BinaryRepr<R> {
        self.clone().with_radix::<2>()
    }

    /// Explicitly change the precision of the number.
    ///
    /// If the given precision is less than the previous value,
    /// it will be rounded following the rounding mode specified by the type parameter.
    pub fn with_precision(self, precision: usize) -> Self {
        let mut result = self;

        // shrink if possible
        if result.precision > precision {
            let actual = result.actual_precision();
            if actual > precision {
                let shift = actual - precision;
                let low_digits = shr_rem_radix_in_place::<X>(&mut result.mantissa, shift);
                result.mantissa +=
                    Rounding::from_fract::<X, R>(&result.mantissa, low_digits, shift);
                result.exponent += shift as isize;
            }
        }

        result.precision = precision;
        return result;
    }

    /// Explicitly change the rounding mode of the number.
    ///
    /// This operation has no cost.
    #[inline]
    #[allow(non_upper_case_globals)]
    pub fn with_rounding<const NewR: u8>(self) -> FloatRepr<X, { NewR }> {
        FloatRepr {
            mantissa: self.mantissa,
            exponent: self.exponent,
            precision: self.precision,
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
    pub fn with_radix<const NewX: usize>(self) -> FloatRepr<NewX, R> {
        if NewX == X {
            return FloatRepr {
                mantissa: self.mantissa,
                exponent: self.exponent,
                precision: self.precision,
            };
        }
        // FIXME: shortcut if E is a power of NewX

        // Calculate the new precision
        // new_precision = floor_log_radix2(radix1^precision)
        let precision = log_pow(&UBig::from(X), self.precision, NewX);

        // Convert by calculating logarithm
        // FIXME: currently the calculation is done in full precision, could be vastly optimized
        let result = if self.exponent == 0 {
            // direct copy if the exponent is zero
            return FloatRepr {
                mantissa: self.mantissa,
                exponent: 0,
                precision,
            };
        } else if self.exponent > 0 {
            // denote log with base of radix2 as lgr2, then
            // mantissa * radix1 ^ exp1
            // = mantissa * radix2 ^ lgr2(radix1^exp1)
            // = mantissa * (radix2 ^ floor_lgr2(radix1^exp1) + rem_lgr2(radix1^exp1))
            // = mantissa * ratio * (radix2 ^ floor_lgr2(radix1^exp1))
            // where the ratio is
            // 1 + rem_lgr2(radix1^exp1) / (radix2 ^ floor_lgr2(radix1^exp1))
            // = radix1^exp1 / (radix1^exp1 - rem_lgr2(radix1^exp1))

            let precision_ub = UBig::from(X).pow(self.exponent as usize);
            let (log_v, log_r) = log_rem(&precision_ub, NewX);
            let den = IBig::from(&precision_ub - log_r);
            let num = IBig::from(precision_ub) * self.mantissa;
            let mut value = FloatRepr::<NewX, R>::from_ratio(num, den, precision + 1);
            value.exponent += log_v as isize;
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

            let precision_ub = UBig::from(X).pow(-self.exponent as usize);
            let (log_v, log_r) = log_rem(&precision_ub, NewX);
            let num = IBig::from(&precision_ub - log_r) * self.mantissa;
            let den = IBig::from(precision_ub);
            let mut value = FloatRepr::<NewX, R>::from_ratio(num, den, precision + 1);
            value.exponent -= log_v as isize;
            value
        };

        result.with_precision(precision)
    }

    #[allow(non_upper_case_globals)]
    fn with_radix_and_precision<const NewX: usize>(self, precision: usize) -> FloatRepr<NewX, R> {
        // approximate power if precision is small
        // calculate more digits if precision is high
        unimplemented!()
    }

    pub(crate) fn normalize(mut mantissa: IBig, mut exponent: isize) -> (IBig, isize) {
        if mantissa.is_zero() {
            return (IBig::ZERO, 0);
        }
        if X == 2 {
            if let Some(shift) = mantissa.trailing_zeros() {
                mantissa >>= shift;
                exponent += shift as isize;
            };
        } else {
            let shift: isize = remove_pow(&mut mantissa, &X.into()).try_into().unwrap();
            exponent += shift;
        }
        (mantissa, exponent)
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
