extern crate alloc;

use crate::{rbig::RBig, repr::Repr, Relaxed};
use dashu_int::{IBig, UBig};
use serde::{
    de::{self, Deserialize, DeserializeSeed, Deserializer, SeqAccess, Visitor},
    ser::{Serialize, SerializeStruct, Serializer},
};

const KEY_NUMER: &str = "numerator";
const KEY_DENOM: &str = "denominator";
const FIELDS: &[&str] = &[KEY_NUMER, KEY_DENOM];

fn serialize_repr<S: Serializer>(
    repr: &Repr,
    serializer: S,
    name: &'static str,
) -> Result<S::Ok, S::Error> {
    if serializer.is_human_readable() {
        // serialize to formatted string if the serializer is human readable
        serializer.collect_str(repr)
    } else {
        // otherwise serialize as a (numerator, denominator) struct
        let mut se = serializer.serialize_struct(name, 2)?;
        se.serialize_field(KEY_NUMER, &repr.numerator)?;
        se.serialize_field(KEY_DENOM, &repr.denominator)?;
        se.end()
    }
}

impl Serialize for RBig {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serialize_repr(&self.0, serializer, "RBig")
    }
}

impl Serialize for Relaxed {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serialize_repr(&self.0, serializer, "Relaxed")
    }
}

/// A field name of the struct form.
enum Field {
    Numerator,
    Denominator,
}

/// Deserializes a field name. Formats differ in how they hand over map keys (borrowed from
/// the input, as a transient string, as an owned string, or as bytes), so the name is matched
/// in a visitor instead of being deserialized as a borrowed `&str`.
struct FieldSeed;

impl<'de> Visitor<'de> for FieldSeed {
    type Value = Field;

    #[inline]
    fn expecting(&self, formatter: &mut alloc::fmt::Formatter) -> alloc::fmt::Result {
        formatter.write_str("a field name")
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        match v {
            KEY_NUMER => Ok(Field::Numerator),
            KEY_DENOM => Ok(Field::Denominator),
            _ => Err(de::Error::unknown_field(v, FIELDS)),
        }
    }

    fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
        match core::str::from_utf8(v) {
            Ok(v) => self.visit_str(v),
            Err(_) => Err(de::Error::invalid_value(de::Unexpected::Bytes(v), &self)),
        }
    }
}

impl<'de> DeserializeSeed<'de> for FieldSeed {
    type Value = Field;

    #[inline]
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_identifier(self)
    }
}

struct ReprVisitor;

impl<'de> Visitor<'de> for ReprVisitor {
    type Value = Repr;

    #[inline]
    fn expecting(&self, formatter: &mut alloc::fmt::Formatter) -> alloc::fmt::Result {
        formatter
            .write_str("rational number as a literal string or a struct (numerator, denominator)")
    }

    #[inline]
    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        match Repr::from_str_with_radix_prefix(v) {
            Ok((repr, _)) => Ok(repr),
            Err(e) => Err(de::Error::custom(e)),
        }
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let err_report = || {
            de::Error::invalid_length(
                2,
                &"a rational consists of two integer fields: (numerator, denominator)",
            )
        };
        let numerator = seq.next_element()?.ok_or_else(err_report)?;
        let denominator = seq.next_element()?.ok_or_else(err_report)?;

        if seq.next_element::<de::IgnoredAny>()?.is_some() {
            Err(err_report())?
        } else {
            Ok(Repr {
                numerator,
                denominator,
            })
        }
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut num: Option<IBig> = None;
        let mut den: Option<UBig> = None;
        while let Some(key) = map.next_key_seed(FieldSeed)? {
            match key {
                Field::Numerator => {
                    if num.is_some() {
                        return Err(de::Error::duplicate_field(KEY_NUMER));
                    }
                    num = Some(map.next_value()?);
                }
                Field::Denominator => {
                    if den.is_some() {
                        return Err(de::Error::duplicate_field(KEY_DENOM));
                    }
                    den = Some(map.next_value()?);
                }
            }
        }

        let numerator = num.ok_or_else(|| de::Error::missing_field(KEY_NUMER))?;
        let denominator = den.ok_or_else(|| de::Error::missing_field(KEY_DENOM))?;
        Ok(Repr {
            numerator,
            denominator,
        })
    }
}

fn deserialize_repr<'de, D: Deserializer<'de>>(
    deserializer: D,
    name: &'static str,
) -> Result<Repr, D::Error> {
    let repr = if deserializer.is_human_readable() {
        deserializer.deserialize_str(ReprVisitor)?
    } else {
        deserializer.deserialize_struct(name, FIELDS, ReprVisitor)?
    };
    Ok(repr)
}

impl<'de> Deserialize<'de> for RBig {
    #[inline]
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserialize_repr(deserializer, "RBig").map(|repr| RBig(repr.reduce()))
    }
}

impl<'de> Deserialize<'de> for Relaxed {
    #[inline]
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserialize_repr(deserializer, "Relaxed").map(|repr| Relaxed(repr.reduce2()))
    }
}
