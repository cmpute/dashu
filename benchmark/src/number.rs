use std::{
    fmt::Display,
    ops::{Add, Div, Mul, Sub},
    str::FromStr,
};

pub(crate) trait Natural
where
    Self: Sized,
    Self: From<u32>,
    Self: Display,
    Self: Add<Self, Output = Self>,
    Self: for<'a> Add<&'a Self, Output = Self>,
    Self: Sub<Self, Output = Self>,
    Self: for<'a> Sub<&'a Self, Output = Self>,
    Self: Mul<Self, Output = Self>,
    Self: for<'a> Mul<&'a Self, Output = Self>,
    Self: Div<Self, Output = Self>,
    Self: for<'a> Div<&'a Self, Output = Self>,
    Self: FromStr,
{
    fn pow(&self, exp: u32) -> Self;
    fn to_hex(&self) -> String;
    fn mul_ref(&self, rhs: &Self) -> Self;
}

mod natural {
    use super::Natural;

    impl Natural for dashu::Natural {
        fn pow(&self, exp: u32) -> Self {
            self.pow(exp as usize)
        }

        fn to_hex(&self) -> String {
            format!("{:x}", self)
        }

        fn mul_ref(&self, rhs: &Self) -> Self {
            self * rhs
        }
    }

    impl Natural for ibig::UBig {
        fn pow(&self, exp: u32) -> Self {
            self.pow(exp as usize)
        }

        fn to_hex(&self) -> String {
            format!("{:x}", self)
        }

        fn mul_ref(&self, rhs: &Self) -> Self {
            self * rhs
        }
    }

    impl Natural for num::BigUint {
        fn pow(&self, exp: u32) -> Self {
            self.pow(exp)
        }

        fn to_hex(&self) -> String {
            format!("{:x}", self)
        }

        fn mul_ref(&self, rhs: &Self) -> Self {
            self * rhs
        }
    }

    #[cfg(feature = "ramp")]
    impl Natural for ramp::Int {
        fn pow(&self, exp: u32) -> Self {
            self.pow(exp as usize)
        }

        fn to_hex(&self) -> String {
            format!("{:x}", self)
        }

        fn mul_ref(&self, rhs: &Self) -> Self {
            self * rhs
        }
    }

    #[cfg(feature = "gmp")]
    impl Natural for rug::Integer {
        fn pow(&self, exp: u32) -> Self {
            rug::ops::Pow::pow(self, exp).into()
        }

        fn to_hex(&self) -> String {
            format!("{:x}", self)
        }

        fn mul_ref(&self, rhs: &Self) -> Self {
            (self * rhs).into()
        }
    }

    #[cfg(feature = "gmp")]
    impl Natural for gmp::mpz::Mpz {
        fn pow(&self, exp: u32) -> Self {
            self.pow(exp)
        }

        fn to_hex(&self) -> String {
            self.to_str_radix(16)
        }

        fn mul_ref(&self, rhs: &Self) -> Self {
            self * rhs
        }
    }

    impl Natural for malachite::Natural {
        fn pow(&self, exp: u32) -> Self {
            malachite::base::num::arithmetic::traits::Pow::pow(self, exp.into())
        }

        fn to_hex(&self) -> String {
            malachite::base::strings::ToLowerHexString::to_lower_hex_string(self)
        }

        fn mul_ref(&self, rhs: &Self) -> Self {
            self * rhs
        }
    }
}

pub(crate) trait Rational
where
    Self: Sized,
    Self: Display,
    Self: Add<Self, Output = Self>,
    Self: for<'a> Add<&'a Self, Output = Self>,
    Self: Sub<Self, Output = Self>,
    Self: for<'a> Sub<&'a Self, Output = Self>,
    Self: Mul<Self, Output = Self>,
    Self: for<'a> Mul<&'a Self, Output = Self>,
    Self: Div<Self, Output = Self>,
    Self: for<'a> Div<&'a Self, Output = Self>,
    Self: FromStr,
{
    fn recip(&self) -> Self;
    fn from_u32(n: u32) -> Self;
}

mod rational {
    use super::Rational;

    impl Rational for dashu::Rational {
        fn recip(&self) -> Self {
            let (sign, numerator) = self.numerator().clone().into_parts();
            dashu::Rational::from_parts(sign * self.denominator().clone(), numerator)
        }

        fn from_u32(n: u32) -> Self {
            Self::from(n)
        }
    }

    impl Rational for num::BigRational {
        fn recip(&self) -> Self {
            self.recip()
        }

        fn from_u32(n: u32) -> Self {
            Self::from_integer(n.into())
        }
    }

    impl Rational for malachite::Rational {
        fn recip(&self) -> Self {
            malachite::base::num::arithmetic::traits::Reciprocal::reciprocal(self)
        }

        fn from_u32(n: u32) -> Self {
            Self::from(n)
        }
    }
}

pub(crate) trait Float
where
    Self: Sized,
    Self: Display,
    Self: Add<Self, Output = Self>,
    Self: for<'a> Add<&'a Self, Output = Self>,
    Self: Sub<Self, Output = Self>,
    Self: for<'a> Sub<&'a Self, Output = Self>,
    Self: Mul<Self, Output = Self>,
    Self: for<'a> Mul<&'a Self, Output = Self>,
    Self: Div<Self, Output = Self>,
    Self: for<'a> Div<&'a Self, Output = Self>,
{
    fn e(precision: u32) -> Self;

    /// A small integer value at the given precision. This is the shared
    /// starting point used by [`pi::calculate`](crate::pi::calculate).
    fn from_int(value: u32, precision: u32) -> Self;

    /// √self, at self's own precision.
    fn sqrt(&self) -> Self;

    /// arctan(self), at self's own precision.
    fn atan(&self) -> Self;
}

/// Wrapper around [`astro_float::BigFloat`]. The orphan rule forbids
/// `impl std::ops::Add for astro_float::BigFloat` (foreign trait + foreign
/// type) and astro-float already owns `Display` / `FromStr` / `From`, so the
/// `Float` trait's operator bounds can only be satisfied on a local newtype.
pub(crate) struct AstroFloat(pub astro_float::BigFloat);

mod float {
    use super::{AstroFloat, Float};
    use std::fmt::{self, Formatter};
    use std::ops::{Add, Div, Mul, Sub};
    use std::str::FromStr;

    impl Float for dashu::Decimal {
        fn e(precision: u32) -> Self {
            Self::from_int(1, precision).exp()
        }
        fn from_int(value: u32, precision: u32) -> Self {
            dashu::Decimal::from(value)
                .with_precision(precision as _)
                .unwrap()
        }
        fn sqrt(&self) -> Self {
            self.sqrt()
        }
        fn atan(&self) -> Self {
            self.atan()
        }
    }

    impl Float for dashu::Real {
        fn e(precision: u32) -> Self {
            Self::from_int(1, precision).exp()
        }
        fn from_int(value: u32, precision: u32) -> Self {
            dashu::Real::from(value)
                .with_precision(precision as _)
                .unwrap()
        }
        fn sqrt(&self) -> Self {
            self.sqrt()
        }
        fn atan(&self) -> Self {
            self.atan()
        }
    }

    impl Float for bigdecimal::BigDecimal {
        fn e(_precision: u32) -> Self {
            // The default precision of bigdecimal depends on the ENV variable
            bigdecimal::BigDecimal::from(1).exp()
        }
        fn from_int(value: u32, _precision: u32) -> Self {
            bigdecimal::BigDecimal::from(value)
        }
        // bigdecimal is not used for the pi task; it lacks atan/sqrt in the
        // shape the trait wants, so these exist only to satisfy the bounds.
        fn sqrt(&self) -> Self {
            unimplemented!("bigdecimal pi primitives")
        }
        fn atan(&self) -> Self {
            unimplemented!("bigdecimal pi primitives")
        }
    }

    impl Float for AstroFloat {
        fn e(precision: u32) -> Self {
            let p = precision as usize;
            let mut cc = astro_float::Consts::new().expect("astro consts cache");
            AstroFloat(astro_float::BigFloat::from_word(1, p).exp(
                p,
                astro_float::RoundingMode::ToEven,
                &mut cc,
            ))
        }

        fn from_int(value: u32, precision: u32) -> Self {
            AstroFloat(astro_float::BigFloat::from_word(value as astro_float::Word, precision as _))
        }
        fn sqrt(&self) -> Self {
            let p = self.0.precision().unwrap_or(64);
            AstroFloat(self.0.sqrt(p, astro_float::RoundingMode::ToEven))
        }
        fn atan(&self) -> Self {
            let p = self.0.precision().unwrap_or(64);
            let mut cc = astro_float::Consts::new().expect("astro consts cache");
            AstroFloat(self.0.atan(p, astro_float::RoundingMode::ToEven, &mut cc))
        }
    }

    #[cfg(feature = "rug")]
    impl Float for rug::Float {
        fn e(precision: u32) -> Self {
            Self::from_int(1, precision).exp()
        }
        fn from_int(value: u32, precision: u32) -> Self {
            rug::Float::with_val(precision, value)
        }
        fn sqrt(&self) -> Self {
            self.clone().sqrt()
        }
        fn atan(&self) -> Self {
            self.clone().atan()
        }
    }

    impl From<u32> for AstroFloat {
        fn from(v: u32) -> Self {
            AstroFloat(astro_float::BigFloat::from_word(v as astro_float::Word, 64))
        }
    }

    impl FromStr for AstroFloat {
        type Err = astro_float::Error;
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            astro_float::BigFloat::from_str(s).map(AstroFloat)
        }
    }

    macro_rules! impl_binop {
        ($trait:ident, $method:ident) => {
            impl $trait for AstroFloat {
                type Output = AstroFloat;
                fn $method(self, rhs: AstroFloat) -> AstroFloat {
                    let p = self.0.precision().unwrap_or(64);
                    AstroFloat(self.0.$method(&rhs.0, p, astro_float::RoundingMode::ToEven))
                }
            }
            impl<'a> $trait<&'a AstroFloat> for AstroFloat {
                type Output = AstroFloat;
                fn $method(self, rhs: &AstroFloat) -> AstroFloat {
                    let p = self.0.precision().unwrap_or(64);
                    AstroFloat(self.0.$method(&rhs.0, p, astro_float::RoundingMode::ToEven))
                }
            }
        };
    }
    impl_binop!(Add, add);
    impl_binop!(Sub, sub);
    impl_binop!(Mul, mul);
    impl_binop!(Div, div);

    impl fmt::Display for AstroFloat {
        /// Delegate to astro-float's native `Binary` formatting rather than
        /// reimplementing binary digit emission. astro-float uses `'e'` as the
        /// exponent marker, while `dashu::Real`'s base-2 `FromStr` expects
        /// `'@'`/`'p'`; translating that single char makes the value round-trip
        /// through `dashu::Real`, which is what `pi_within_tolerance` parses.
        fn fmt(&self, f: &mut Formatter) -> fmt::Result {
            f.write_str(&format!("{:b}", self.0).replace('e', "@"))
        }
    }
}
