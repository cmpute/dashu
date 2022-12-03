use crate::{fbig::FBig, repr::Repr, round::Round};
use _bytes::BufMut;
use dashu_base::{ConversionError, Sign};
use postgres_types::{private::BytesMut, to_sql_checked, FromSql, IsNull, ToSql, Type};
use std::error;

use super::Numeric;

impl<'a> FromSql<'a> for Numeric {
    #[inline]
    fn accepts(ty: &Type) -> bool {
        *ty == Type::NUMERIC
    }

    fn from_sql(ty: &Type, raw: &'a [u8]) -> Result<Self, Box<dyn error::Error + Sync + Send>> {
        assert!(raw.len() > 8);
        assert!(*ty == Type::NUMERIC);

        #[inline(always)]
        fn read16(bytes: &[u8]) -> u16 {
            u16::from_be_bytes(bytes.try_into().unwrap())
        }

        // number of digits (in base NBASE = 10000)
        let num_digits = read16(&raw[..2]) as usize;

        // exponent (in base NBASE = 10000)
        let weight = read16(&raw[2..4]) as i16;

        // sign flags
        let sign = match read16(&raw[4..6]) {
            0x0000 => Sign::Positive,
            0x4000 => Sign::Negative,
            0xC000 => return Err(ConversionError::OutOfBounds.into()),
            0xD000 => return Ok(Numeric::infinity()),
            0xF000 => return Ok(Numeric::neg_infinity()),
            _ => panic!(),
        };

        // precision (in base 10 digits)
        let scale = read16(&raw[6..8]);

        // parse the digits through the iterator
        Ok(Self {
            sign,
            is_inf: false,
            weight,
            dscale: scale,
            digits: raw[8..]
                .chunks(2)
                .take(num_digits)
                .map(|bytes| read16(bytes) as i16)
                .collect(),
        })
    }
}

impl<'a, R: Round> FromSql<'a> for FBig<R, 10> {
    #[inline]
    fn accepts(ty: &Type) -> bool {
        <Numeric as FromSql>::accepts(ty)
    }

    #[inline]
    fn from_sql(ty: &Type, raw: &'a [u8]) -> Result<Self, Box<dyn error::Error + Sync + Send>> {
        Ok(<Numeric as FromSql>::from_sql(ty, raw)?.into())
    }
}

impl<'a> FromSql<'a> for Repr<10> {
    #[inline]
    fn accepts(ty: &Type) -> bool {
        <Numeric as FromSql>::accepts(ty)
    }

    #[inline]
    fn from_sql(ty: &Type, raw: &'a [u8]) -> Result<Self, Box<dyn error::Error + Sync + Send>> {
        Ok(<Numeric as FromSql>::from_sql(ty, raw)?.into())
    }
}

impl ToSql for Numeric {
    #[inline]
    fn accepts(ty: &Type) -> bool {
        *ty == Type::NUMERIC
    }

    fn to_sql(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn error::Error + Sync + Send>> {
        assert!(*ty == Type::NUMERIC);
        // TODO(next): support inf

        let num_digits = self.digits.len();

        // reserve bytes
        out.reserve(8 + num_digits * 2);

        // put headers
        out.put_u16(num_digits.try_into().unwrap());
        out.put_i16(self.weight);
        out.put_u16(match self.sign {
            Sign::Positive => 0x0000,
            Sign::Negative => 0x4000,
        });
        out.put_u16(self.dscale);

        // put the digits
        for digit in self.digits[0..num_digits].iter() {
            out.put_i16(*digit);
        }

        Ok(IsNull::No)
    }

    to_sql_checked!();
}

impl ToSql for Repr<10> {
    #[inline]
    fn accepts(ty: &Type) -> bool {
        <Numeric as ToSql>::accepts(ty)
    }

    #[inline]
    fn to_sql(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn error::Error + Sync + Send>> {
        let num: Numeric = self.try_into()?;
        num.to_sql(ty, out)
    }

    to_sql_checked!();
}

impl<R: Round> ToSql for FBig<R, 10> {
    #[inline]
    fn accepts(ty: &Type) -> bool {
        <Numeric as ToSql>::accepts(ty)
    }

    #[inline]
    fn to_sql(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn error::Error + Sync + Send>> {
        let num: Numeric = self.try_into()?;
        num.to_sql(ty, out)
    }

    to_sql_checked!();
}
