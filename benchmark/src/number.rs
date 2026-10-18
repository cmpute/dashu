use std::{
    fmt::Display,
    ops::{Add, Div, Mul, Rem, Sub},
    str::FromStr,
};

pub(crate) trait Natural
where
    Self: Sized,
    Self: Clone,
    Self: From<u32>,
    Self: Display,
    Self: PartialEq,
    Self: PartialOrd,
    Self: Add<Self, Output = Self>,
    Self: for<'a> Add<&'a Self, Output = Self>,
    Self: Sub<Self, Output = Self>,
    Self: for<'a> Sub<&'a Self, Output = Self>,
    Self: Mul<Self, Output = Self>,
    Self: for<'a> Mul<&'a Self, Output = Self>,
    Self: Div<Self, Output = Self>,
    Self: for<'a> Div<&'a Self, Output = Self>,
    Self: for<'a> Rem<&'a Self, Output = Self>,
    Self: FromStr,
{
    fn pow(&self, exp: u32) -> Self;
    fn to_hex(&self) -> String;
    fn mul_ref(&self, rhs: &Self) -> Self;

    /// Greatest common divisor of two non-negative values.
    fn gcd(&self, rhs: &Self) -> Self;

    /// Whether the value is odd.
    fn is_odd(&self) -> bool;

    /// (self * rhs) mod m. Operands must already be reduced modulo m.
    fn mul_mod(&self, rhs: &Self, m: &Self) -> Self {
        self.mul_ref(rhs) % m
    }

    /// self^exp mod m, bound to the library's native modular exponentiation.
    /// The base must already be reduced modulo m.
    fn pow_mod(&self, exp: &Self, m: &Self) -> Self;
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

        fn gcd(&self, rhs: &Self) -> Self {
            dashu::base::ring::Gcd::gcd(self, rhs)
        }

        fn is_odd(&self) -> bool {
            dashu::base::BitTest::bit(self, 0)
        }

        fn pow_mod(&self, exp: &Self, m: &Self) -> Self {
            // The Barrett reducer wins over Montgomery below ~256-bit moduli,
            // which covers every size this benchmark factors.
            dashu::integer::fast_div::ConstDivisor::new(m.clone())
                .reduce(self.clone())
                .pow(exp)
                .residue()
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

        fn gcd(&self, rhs: &Self) -> Self {
            self.gcd(rhs)
        }

        fn is_odd(&self) -> bool {
            self.bit(0)
        }

        fn pow_mod(&self, exp: &Self, m: &Self) -> Self {
            ibig::modular::ModuloRing::new(m)
                .from(self)
                .pow(exp)
                .residue()
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

        fn gcd(&self, rhs: &Self) -> Self {
            num::integer::Integer::gcd(self, rhs)
        }

        fn is_odd(&self) -> bool {
            self.iter_u64_digits()
                .next()
                .is_some_and(|word| word & 1 == 1)
        }

        fn pow_mod(&self, exp: &Self, m: &Self) -> Self {
            self.modpow(exp, m)
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

        fn gcd(&self, rhs: &Self) -> Self {
            self.gcd(rhs)
        }

        fn is_odd(&self) -> bool {
            self.bit(0)
        }

        fn pow_mod(&self, exp: &Self, m: &Self) -> Self {
            self.pow_mod(exp, m)
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

        fn gcd(&self, rhs: &Self) -> Self {
            self.clone().gcd(rhs)
        }

        fn is_odd(&self) -> bool {
            self.is_odd()
        }

        fn pow_mod(&self, exp: &Self, m: &Self) -> Self {
            // Only fails on a negative exponent with a non-invertible base.
            self.clone().pow_mod(exp, m).unwrap()
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

        fn gcd(&self, rhs: &Self) -> Self {
            self.gcd(rhs)
        }

        fn is_odd(&self) -> bool {
            self.tstbit(0)
        }

        fn pow_mod(&self, exp: &Self, m: &Self) -> Self {
            self.powm(exp, m)
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

        fn gcd(&self, rhs: &Self) -> Self {
            malachite::base::num::arithmetic::traits::Gcd::gcd(self.clone(), rhs.clone())
        }

        fn is_odd(&self) -> bool {
            malachite::base::num::arithmetic::traits::Parity::odd(self)
        }

        fn mul_mod(&self, rhs: &Self, m: &Self) -> Self {
            malachite::base::num::arithmetic::traits::ModMul::mod_mul(
                self.clone(),
                rhs.clone(),
                m.clone(),
            )
        }

        fn pow_mod(&self, exp: &Self, m: &Self) -> Self {
            malachite::base::num::arithmetic::traits::ModPow::mod_pow(
                self.clone(),
                exp.clone(),
                m.clone(),
            )
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
