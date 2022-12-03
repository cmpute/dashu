//! Implement supports for PostgreSQL related crates.

use dashu_base::{ConversionError, DivRemAssign, EstimatedLog2, Sign};
use dashu_int::UBig;

use crate::{
    fbig::FBig,
    repr::{Context, Repr},
    round::Round,
};
extern crate alloc;
use alloc::vec::Vec;

/// Represents the NUMERIC value in PostgreSQL, closely mirroring the PG wire protocol without NaN.
///
/// Reference: <https://github.com/postgres/postgres/blob/master/src/backend/utils/adt/numeric.c#L253>
#[derive(Debug)]
pub(in crate::third_party::postgres) struct Numeric {
    pub sign: Sign,
    pub is_inf: bool,

    /// Corresponding to the exponent of FBig in base NBASE (= 10000)
    pub weight: i16,

    /// Corresponding to the precision of FBig in base 10
    pub dscale: u16,

    /// The actual digit, should be an iterator
    pub digits: Vec<i16>,
}

impl Numeric {
    const fn infinity() -> Self {
        Numeric {
            sign: Sign::Positive,
            is_inf: true,
            weight: 0,
            dscale: 0,
            digits: Vec::new(),
        }
    }

    const fn neg_infinity() -> Self {
        Numeric {
            sign: Sign::Negative,
            is_inf: true,
            weight: 0,
            dscale: 0,
            digits: Vec::new(),
        }
    }
}

impl From<Numeric> for Repr<10> {
    fn from(num: Numeric) -> Self {
        if num.is_inf {
            match num.sign {
                Sign::Positive => Repr::<10>::infinity(),
                Sign::Negative => Repr::<10>::neg_infinity(),
            };
        }

        let base = UBig::from_dword(10000);
        let mut position = UBig::ONE;
        let mut signif = UBig::ZERO;

        for d in num.digits.iter() {
            signif += (*d as u16) * &position;
            position *= &base;
        }

        Repr::new(num.sign * signif, num.weight as isize)
    }
}

impl<R: Round> From<Numeric> for FBig<R, 10> {
    #[inline]
    fn from(num: Numeric) -> Self {
        let context = Context::new(num.dscale as usize);
        let repr = Repr::from(num);
        Self::from_repr(repr, context)
    }
}

impl TryFrom<&Repr<10>> for Numeric {
    type Error = ConversionError;

    fn try_from(value: &Repr<10>) -> Result<Self, Self::Error> {
        if value.is_infinite() {
            return Ok(match value.sign() {
                Sign::Positive => Numeric::infinity(),
                Sign::Negative => Numeric::neg_infinity(),
            });
        }

        let digit_len = value.significand.log2_bounds().0;
        let mut digits = Vec::new();
        if digit_len > (u16::MAX / 4) as f32 {
            return Err(ConversionError::LossOfPrecision);
        }

        let Repr {
            significand,
            exponent,
        } = value.clone();
        let (sign, mut signif) = significand.into_parts();

        while !signif.is_zero() {
            // XXX: to achieve the best performance, it might worth adding a `to_decimal_digits`
            // method to `UBig`, and supporting arbitrary power of 10 as the base.
            digits.push(signif.div_rem_assign(10000u16) as i16);
        }

        digits.reverse();
        Ok(Numeric {
            sign,
            is_inf: false,
            weight: exponent
                .try_into()
                .map_err(|_| ConversionError::OutOfBounds)?,
            dscale: (digits.len() * 4) as u16, // use the digit length as the precision
            digits,
        })
    }
}

impl<R: Round> TryFrom<&FBig<R, 10>> for Numeric {
    type Error = ConversionError;

    #[inline]
    fn try_from(value: &FBig<R, 10>) -> Result<Self, Self::Error> {
        let mut num: Numeric = value.try_into()?;
        num.dscale = if value.context.precision > u16::MAX as usize {
            u16::MAX
        } else {
            value.context.precision as u16
        };
        Ok(num)
    }
}

#[cfg(feature = "diesel1")]
mod diesel1;

#[cfg(feature = "diesel2")]
mod diesel2;

#[cfg(feature = "postgres-types")]
mod postgres_types;
