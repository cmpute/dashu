use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

use dashu_base::Sign;

use crate::RBig;

impl Serialize for RBig {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            serializer.serialize_str(&self.to_string())
        } else {
            let sign = self.sign();
            let numerator = self.numerator().clone().into_parts().1.to_le_bytes();
            let denominator = self.denominator().to_le_bytes();
            let mut ser = Serializer::serialize_struct(
                serializer,
                "Rational",
                1 + numerator.len() + denominator.len(),
            )?;
            SerializeStruct::serialize_field(&mut ser, "sign", &sign)?;
            SerializeStruct::serialize_field(&mut ser, "numerator", &numerator)?;
            SerializeStruct::serialize_field(&mut ser, "denominator", &denominator)?;
            SerializeStruct::end(ser)
        }
    }
}

pub struct LosslessRational {
    pub sign: Sign,
    pub numerator: Vec<u8>,
    pub denominator: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json() {
        assert_eq!(r#"0"#, serde_json::to_string(&RBig::from(0)).unwrap());
    }
}
