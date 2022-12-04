use super::Numeric;
use crate::{fbig::FBig, repr::Repr, round::Round};
use dashu_base::Sign;
use diesel_v1::{
    deserialize::{self, FromSql},
    pg::{data_types::PgNumeric, Pg},
    serialize::{self, Output, ToSql},
    sql_types::Numeric as DieselNumeric,
};
use std::io::Write;

impl FromSql<DieselNumeric, Pg> for Numeric {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        match PgNumeric::from_sql(bytes)? {
            PgNumeric::Positive {
                weight,
                scale,
                digits,
            } => Ok(Numeric {
                weight,
                is_inf: false,
                sign: Sign::Positive,
                dscale: scale,
                digits,
            }),
            PgNumeric::Negative {
                weight,
                scale,
                digits,
            } => Ok(Numeric {
                weight,
                is_inf: false,
                sign: Sign::Negative,
                dscale: scale,
                digits,
            }),
            PgNumeric::NaN => Err(Box::from("NaN is not supported in dashu")),
        }
    }
}

impl FromSql<DieselNumeric, Pg> for Repr<10> {
    #[inline]
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        Ok(Numeric::from_sql(bytes)?.into())
    }
}

impl<R: Round> FromSql<DieselNumeric, Pg> for FBig<R, 10> {
    #[inline]
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        Ok(Numeric::from_sql(bytes)?.into())
    }
}

fn numeric_to_sql<W: Write>(num: Numeric, out: &mut Output<W, Pg>) -> serialize::Result {
    let num = match num {
        Numeric {
            is_inf: false,
            sign: Sign::Positive,
            weight,
            dscale,
            digits,
        } => PgNumeric::Positive {
            weight,
            scale: dscale,
            digits,
        },
        Numeric {
            is_inf: false,
            sign: Sign::Negative,
            weight,
            dscale,
            digits,
        } => PgNumeric::Negative {
            weight,
            scale: dscale,
            digits,
        },
        Numeric { is_inf: true, .. } => {
            return Err(Box::from("Infinities are not yet supported in diesel"))
        }
    };
    ToSql::<DieselNumeric, Pg>::to_sql(&num, out)
}

impl<R: Round> ToSql<DieselNumeric, Pg> for FBig<R, 10> {
    #[inline]
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        numeric_to_sql(self.try_into()?, out)
    }
}

impl ToSql<DieselNumeric, Pg> for Repr<10> {
    #[inline]
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        numeric_to_sql(self.try_into()?, out)
    }
}
