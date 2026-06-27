//! Implement serde traits.

use crate::{convert::words_to_le_bytes, ibig::IBig, ubig::UBig};
use core::fmt::{self, Formatter};
use serde::{
    de::{self, Deserialize, Deserializer, Visitor},
    ser::{Serialize, Serializer},
};

impl Serialize for UBig {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            serializer.collect_str(self)
        } else if self.is_zero() {
            serializer.serialize_bytes(&[])
        } else {
            let bytes = words_to_le_bytes::<false>(self.as_words());
            serializer.serialize_bytes(&bytes)
        }
    }
}

impl<'de> Deserialize<'de> for UBig {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if deserializer.is_human_readable() {
            deserializer.deserialize_str(UBigVisitor)
        } else {
            deserializer.deserialize_bytes(UBigVisitor)
        }
    }
}

/// UBig is serialized as little-endian bytes.
struct UBigVisitor;

impl<'de> Visitor<'de> for UBigVisitor {
    type Value = UBig;

    fn expecting(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "a string or a sequence of bytes")
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        match UBig::from_str_with_radix_prefix(v) {
            Ok((n, _)) => Ok(n),
            Err(e) => Err(de::Error::custom(e)),
        }
    }

    fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
        Ok(UBig::from_le_bytes(v))
    }
}

impl Serialize for IBig {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            serializer.collect_str(self)
        } else if self.is_zero() {
            serializer.serialize_bytes(&[])
        } else {
            // Two's complement little-endian bytes (sign encoded in the high bits),
            // matching `IBig::to_le_bytes()` for interop robustness.
            let bytes = self.to_le_bytes();
            serializer.serialize_bytes(&bytes)
        }
    }
}

impl<'de> Deserialize<'de> for IBig {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if deserializer.is_human_readable() {
            deserializer.deserialize_str(IBigVisitor)
        } else {
            deserializer.deserialize_bytes(IBigVisitor)
        }
    }
}

/// IBig is serialized as two's complement little-endian bytes (matching `IBig::to_le_bytes`).
struct IBigVisitor;

impl<'de> Visitor<'de> for IBigVisitor {
    type Value = IBig;

    fn expecting(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "a string or a sequence of bytes")
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        match IBig::from_str_with_radix_prefix(v) {
            Ok((n, _)) => Ok(n),
            Err(e) => Err(de::Error::custom(e)),
        }
    }

    fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
        Ok(IBig::from_le_bytes(v))
    }
}
