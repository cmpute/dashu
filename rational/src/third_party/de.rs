use core::fmt::Formatter;

use serde::de::{Error, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};

use dashu_base::Sign;
use dashu_int::{IBig, UBig};

use crate::RBig;

pub struct LosslessRational {
    pub sign: Sign,
    pub numerator: Vec<u8>,
    pub denominator: Vec<u8>,
}

impl Default for LosslessRational {
    fn default() -> Self {
        Self {
            sign: Sign::Positive,
            numerator: vec![],
            denominator: vec![1],
        }
    }
}

impl<'de> Deserialize<'de> for RBig {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(LosslessRational::default())
    }
}

impl LosslessRational {
    fn as_rational(&self) -> RBig {
        RBig::from_parts(
            IBig::from_parts(self.sign, UBig::from_le_bytes(&self.numerator)),
            UBig::from_le_bytes(&self.denominator),
        )
    }
}

impl<'de> Visitor<'de> for LosslessRational {
    type Value = RBig;

    fn expecting(&self, formatter: &mut Formatter) -> core::fmt::Result {
        write!(formatter, "expect `String`")
    }
    fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
        match RBig::from_str_radix(v, 10) {
            Ok(o) => Ok(o),
            Err(e) => Err(Error::custom(e.to_string())),
        }
    }
    fn visit_map<A: MapAccess<'de>>(mut self, mut map: A) -> Result<Self::Value, A::Error> {
        while let Some(key) = map.next_key::<&str>()? {
            match key {
                "sign" => self.sign = map.next_value::<Sign>()?,
                "numerator" => self.numerator = map.next_value::<Vec<u8>>()?,
                "denominator" => self.denominator = map.next_value::<Vec<u8>>()?,
                _ => return Err(Error::custom(format!("Unexpected field `{}`", key))),
            }
        }
        Ok(self.as_rational())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::from_str;

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
        // binary object empty
        assert_r_big("{}", 0, 1);
        // binary object omit denominator
        let input = r#"{
            "numerator": [5]
        }"#;
        assert_r_big(input, 5, 1);
        // binary object omit sign
        let input = r#"{
            "numerator": [5],
            "denominator": [11]
        }"#;
        assert_r_big(input, 5, 11);
        // binary object full form
        let input = r#"{
            "sign": true,
            "numerator": [5],
            "denominator": [11]
        }"#;
        assert_r_big(input, -5, 11);
    }
}
