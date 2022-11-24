use core::fmt::Formatter;

use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer};

use crate::Sign;

pub struct LosslessSign {}

impl<'de> Deserialize<'de> for Sign {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(LosslessSign {})
    }
}

impl<'de> Visitor<'de> for LosslessSign {
    type Value = Sign;

    fn expecting(&self, formatter: &mut Formatter) -> core::fmt::Result {
        write!(formatter, "expect `true | 'Positive' | 'Negative'`")
    }
    #[inline]
    fn visit_bool<E: Error>(self, v: bool) -> Result<Self::Value, E> {
        Ok(Sign::from(v))
    }
    fn visit_i8<E: Error>(self, v: i8) -> Result<Self::Value, E> {
        match v {
            0 => Ok(Sign::Negative),
            _ => Ok(Sign::Positive),
        }
    }
    fn visit_i64<E: Error>(self, v: i64) -> Result<Self::Value, E> {
        match v {
            0 => Ok(Sign::Negative),
            _ => Ok(Sign::Positive),
        }
    }
    fn visit_u8<E: Error>(self, v: u8) -> Result<Self::Value, E> {
        match v {
            0 => Ok(Sign::Negative),
            _ => Ok(Sign::Positive),
        }
    }
    fn visit_u64<E: Error>(self, v: u64) -> Result<Self::Value, E> {
        match v {
            0 => Ok(Sign::Negative),
            _ => Ok(Sign::Positive),
        }
    }
    fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
        match v {
            "Positive" => Ok(Sign::Positive),
            "Negative" => Ok(Sign::Negative),
            #[cfg(feature = "std")]
            _ => Err(Error::custom(format!("Unexpect variant `{}`", v))),
            #[cfg(not(feature = "std"))]
            _ => Err(Error::custom("Unexpect variant")),
        }
    }
}
