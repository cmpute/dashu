use serde::{Deserialize, Serialize, Serializer};
use dashu_base::Sign;
use dashu_int::UBig;

use crate::RBig;

impl Serialize for RBig {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            serializer.serialize_str(&self.to_string())
        } else {
            todo!()
        }

        // self.0.numerator.serialize(serializer)
    }
}


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
            denominator: vec![],
        }
    }
}

impl RBig {
    fn as_lossless(&self) -> LosslessRational {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json() {
        println!("{}", serde_json::to_string(&RBig::from(0)).unwrap());
    }
}
