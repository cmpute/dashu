use super::Numeric;
use crate::{fbig::FBig, repr::Repr, round::Round};
use dashu_base::Sign;
use diesel2::{
    deserialize::{self, FromSql},
    pg::data_types::PgNumeric,
    pg::{Pg, PgValue},
    serialize::{self, Output, ToSql},
    sql_types::Numeric as DieselNumeric,
};

impl FromSql<DieselNumeric, Pg> for Numeric {
    #[inline]
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
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
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        Ok(Numeric::from_sql(bytes)?.into())
    }
}

impl<R: Round> FromSql<DieselNumeric, Pg> for FBig<R, 10> {
    #[inline]
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        Ok(Numeric::from_sql(bytes)?.into())
    }
}

fn numeric_to_sql<'b>(num: Numeric, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
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
    ToSql::<DieselNumeric, Pg>::to_sql(&num, &mut out.reborrow())
}

impl<R: Round> ToSql<DieselNumeric, Pg> for FBig<R, 10> {
    #[inline]
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        numeric_to_sql(self.try_into()?, out)
    }
}

impl ToSql<DieselNumeric, Pg> for Repr<10> {
    #[inline]
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        numeric_to_sql(self.try_into()?, out)
    }
}
