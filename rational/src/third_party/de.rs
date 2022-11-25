use core::fmt::Formatter;

use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer};

use dashu_base::Sign;
use dashu_int::{IBig, UBig};

use crate::RBig;

pub struct RBigVisitor;

impl<'de> Deserialize<'de> for RBig {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(RBigVisitor)
    }
}

impl<'de> Visitor<'de> for RBigVisitor {
    type Value = RBig;

    fn expecting(&self, formatter: &mut Formatter) -> core::fmt::Result {
        write!(formatter, "expect `String | Bytes`")
    }
    fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
        match RBig::from_str_radix(v, 10) {
            Ok(o) => Ok(o),
            Err(e) => Err(Error::custom(e.to_string())),
        }
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: Error,
    {
        // numerator omitted
        if v.is_empty() {
            return Ok(RBig::ZERO);
        }
        // numerator part
        let mut l_ptr = 0;
        let mut r_ptr = 0;
        r_ptr += 8;
        let head = match <&[u8; 8] as TryFrom<&[u8]>>::try_from(&v[l_ptr..r_ptr]) {
            Ok(o) => i64::from_le_bytes(*o),
            Err(e) => Err(Error::custom(e.to_string()))?,
        };
        l_ptr += 8;
        let num = if head < 0 {
            r_ptr += -head as usize;
            IBig::from_parts(Sign::Negative, UBig::from_le_bytes(&v[l_ptr..r_ptr]))
        } else {
            r_ptr += head as usize;
            IBig::from_parts(Sign::Positive, UBig::from_le_bytes(&v[l_ptr..r_ptr]))
        };
        // denominator omitted
        if v.get(r_ptr + 1).is_none() {
            return Ok(RBig::from_parts(num, UBig::ONE));
        }
        // denominator part
        r_ptr += 8;
        let head = match <&[u8; 8] as TryFrom<&[u8]>>::try_from(&v[l_ptr..r_ptr]) {
            Ok(o) => u64::from_le_bytes(*o),
            Err(e) => Err(Error::custom(e.to_string()))?,
        };
        l_ptr += head as usize;
        let den = UBig::from_le_bytes(&v[l_ptr..r_ptr]);
        Ok(RBig::from_parts(num, den))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::from_str;

    use dashu_int::{IBig, UBig};

    use super::*;

    #[track_caller]
    fn assert_r_big(input: &str, nu: i32, de: u32) {
        assert_eq!(
            from_str::<RBig>(input).unwrap(),
            RBig::from_parts(IBig::from(nu), UBig::from(de))
        )
    }

    #[test]
    fn test_json() {
        // short string form
        assert_r_big("\"-5/11\"", -5, 11);
    }
}
