use serde::{Serialize, Serializer};

use crate::Sign;

impl Serialize for Sign {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            match self {
                Sign::Positive => serializer.serialize_str("Positive"),
                Sign::Negative => serializer.serialize_str("Negative"),
            }
        } else {
            match self {
                Sign::Positive => serializer.serialize_u8(1),
                Sign::Negative => serializer.serialize_u8(0),
            }
        }
    }
}
