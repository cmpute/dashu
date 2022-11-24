use std::fmt::{Display, Formatter};

use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use dashu_int::{IBig, UBig};

use crate::third_party::ser::LosslessRational;
use crate::RBig;

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

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        write!(formatter, "expect `String`")
    }
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        match RBig::from_str_radix(v, 10) {
            Ok(o) => Ok(o),
            Err(e) => Err(Error::custom(e.to_string())),
        }
    }
    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: Error,
    {
        match RBig::from_str_radix(&v, 10) {
            Ok(o) => Ok(o),
            Err(e) => Err(Error::custom(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json() {
        println!(
            "{}",
            serde_json::from_str::<RBig>(
                r#"
        "-5/11"
        "#
            )
            .unwrap()
        );
    }
}
