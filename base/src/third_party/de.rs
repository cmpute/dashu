use std::fmt::Formatter;

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

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        write!(formatter, "expect `true | 'Positive' | 'Negative'`")
    }
    fn visit_bool<E: Error>(self, v: bool) -> Result<Self::Value, E> {
        match v {
            true => Ok(Sign::Positive),
            false => Ok(Sign::Negative),
        }
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
            _ => Err(Error::custom(format!("Unexpect variant `{}`", v))),
        }
    }
    #[inline]
    fn visit_string<E: Error>(self, v: String) -> Result<Self::Value, E> {
        self.visit_str(&v)
    }
}
