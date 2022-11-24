use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

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
            ser.serialize_field("sign", &sign)?;
            ser.serialize_field("numerator", &numerator)?;
            ser.serialize_field("denominator", &denominator)?;
            ser.end()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json() {
        assert_eq!(r#""0""#, serde_json::to_string(&RBig::from(0)).unwrap());
    }
}
