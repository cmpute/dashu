use std::{
    fmt::Display,
    ops::{Add, Div, Mul, Sub},
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
            malachite::num::arithmetic::traits::Pow::pow(self, exp.into())
        }
    
        fn to_hex(&self) -> String {
            malachite::strings::ToLowerHexString::to_lower_hex_string(self)
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
            malachite::num::arithmetic::traits::Reciprocal::reciprocal(self)
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
}

mod float {
    use super::Float;

    impl Float for dashu::Decimal {
        fn e(precision: u32) -> Self {
            dashu::Decimal::ONE.with_precision(precision as _).unwrap().exp()
        }
    }

    impl Float for bigdecimal::BigDecimal {
        fn e(_precision: u32) -> Self {
            // The default precision of bigdecimal depends on the ENV variable
            bigdecimal::BigDecimal::from(1).exp()
        }
    }
}
