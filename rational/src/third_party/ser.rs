use std::ops::Neg;

use serde::ser::SerializeSeq;
use serde::{Serialize, Serializer};

use dashu_base::Sign;

use crate::RBig;

impl Serialize for RBig {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            serializer.serialize_str(&self.to_string())
        } else {
            // i64, bytes, [u64, bytes]
            let mut seq = serializer.serialize_seq(None)?;
            // numerator omitted
            if self.is_zero() {
                return seq.end();
            }
            // write numerator bytes
            let (sign, numerator) = self.numerator().clone().into_parts();
            let numerator = &numerator.to_le_bytes();
            let head = match sign {
                Sign::Positive => numerator.len() as i64,
                Sign::Negative => -(numerator.len() as i64),
            };
            for byte in head.neg().to_le_bytes() {
                seq.serialize_element(&byte)?;
            }
            for byte in numerator.iter() {
                seq.serialize_element(&byte)?;
            }
            // denominator omitted
            if self.denominator().is_one() {
                return seq.end();
            }
            // write numerator bytes
            let denominator = &self.denominator().to_le_bytes();
            let head = denominator.len() as u64;
            for byte in head.to_le_bytes() {
                seq.serialize_element(&byte)?;
            }
            for byte in denominator.iter() {
                seq.serialize_element(&byte)?;
            }
            seq.end()
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
