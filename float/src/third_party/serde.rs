//! Implement serde traits.

use core::marker::PhantomData;

use crate::{fbig::FBig, repr::Repr, round::Round, Context};
use dashu_int::{IBig, Word};
use serde::{
    de::{self, Deserialize, Deserializer, SeqAccess, Visitor},
    ser::{Serialize, SerializeStruct, Serializer},
};

const KEY_SIGNIF: &str = "significand";
const KEY_EXPONENT: &str = "exponent";
const KEY_PREC: &str = "precision";
const REPR_FIELDS: &[&str] = &[KEY_SIGNIF, KEY_EXPONENT];
const FBIG_FIELDS: &[&str] = &[KEY_SIGNIF, KEY_EXPONENT, KEY_PREC];

impl<const B: Word> Serialize for Repr<B> {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            // serialize to formatted string if the serializer is human readable
            serializer.collect_str(self)
        } else {
            // otherwise serialize as a (significand, exponent) struct
            let mut se = serializer.serialize_struct("FBigRepr", 2)?;
            se.serialize_field(KEY_SIGNIF, &self.significand)?;
            se.serialize_field(KEY_EXPONENT, &self.exponent)?;
            se.end()
        }
    }
}

impl<R: Round, const B: Word> Serialize for FBig<R, B> {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            // serialize to formatted string if the serializer is human readable
            // TODO(next): pad the output with leading zeros to make the result contains the correct precision
            serializer.collect_str(self)
        } else {
            // otherwise serialize as a (significand, exponent, precision) struct
            let mut se = serializer.serialize_struct("FBig", 3)?;
            se.serialize_field(KEY_SIGNIF, &self.repr.significand)?;
            se.serialize_field(KEY_EXPONENT, &self.repr.exponent)?;
            se.serialize_field(KEY_PREC, &self.context.precision)?;
            se.end()
        }
    }
}

struct ReprVisitor<const BASE: Word>;

impl<'de, const B: Word> Visitor<'de> for ReprVisitor<B> {
    type Value = Repr<B>;

    #[inline]
    fn expecting(&self, formatter: &mut alloc::fmt::Formatter) -> alloc::fmt::Result {
        formatter.write_str("float repr as a literal string or a struct (significand, exponent)")
    }

    #[inline]
    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        #[allow(deprecated)] // TODO(v0.5): remove after from_str_native is made private.
        match Repr::<B>::from_str_native(v) {
            Ok((repr, _)) => Ok(repr),
            Err(e) => Err(de::Error::custom(e)),
        }
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let err_report = || {
            de::Error::invalid_length(
                2,
                &"a float repr consists of two integer fields: (significand, exponent)",
            )
        };
        let significand = seq.next_element()?.ok_or_else(err_report)?;
        let exponent = seq.next_element()?.ok_or_else(err_report)?;

        if seq.next_element::<de::IgnoredAny>()?.is_some() {
            Err(err_report())?
        } else {
            Ok(Repr::new(significand, exponent))
        }
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut signif: Option<IBig> = None;
        let mut exp: Option<isize> = None;
        while let Some(key) = map.next_key()? {
            match key {
                KEY_SIGNIF => {
                    if signif.is_some() {
                        return Err(de::Error::duplicate_field(KEY_SIGNIF));
                    }
                    signif = Some(map.next_value()?);
                }
                KEY_EXPONENT => {
                    if exp.is_some() {
                        return Err(de::Error::duplicate_field(KEY_EXPONENT));
                    }
                    exp = Some(map.next_value()?);
                }
                _ => return Err(de::Error::unknown_field(key, REPR_FIELDS)),
            }
        }

        let significand = signif.ok_or_else(|| de::Error::missing_field(KEY_SIGNIF))?;
        let exponent = exp.ok_or_else(|| de::Error::missing_field(KEY_EXPONENT))?;
        Ok(Repr::new(significand, exponent))
    }
}

impl<'de, const B: Word> Deserialize<'de> for Repr<B> {
    #[inline]
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if deserializer.is_human_readable() {
            deserializer.deserialize_str(ReprVisitor)
        } else {
            deserializer.deserialize_struct("FBigRepr", REPR_FIELDS, ReprVisitor)
        }
    }
}

struct FBigVisitor<RoundingMode: Round, const BASE: Word>(PhantomData<RoundingMode>);

impl<'de, R: Round, const B: Word> Visitor<'de> for FBigVisitor<R, B> {
    type Value = FBig<R, B>;

    #[inline]
    fn expecting(&self, formatter: &mut alloc::fmt::Formatter) -> alloc::fmt::Result {
        formatter.write_str(
            "float number as a literal string or a struct (significand, exponent, precision)",
        )
    }

    #[inline]
    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        #[allow(deprecated)] // TODO(v0.5): remove after from_str_native is made private.
        FBig::from_str_native(v).map_err(de::Error::custom)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let err_report = || {
            de::Error::invalid_length(2,
            &"a float number consists of three integer fields: (significand, exponent, precision)")
        };
        let significand = seq.next_element()?.ok_or_else(err_report)?;
        let exponent = seq.next_element()?.ok_or_else(err_report)?;
        let precision = seq.next_element()?.ok_or_else(err_report)?;

        if seq.next_element::<de::IgnoredAny>()?.is_some() {
            Err(err_report())?
        } else {
            Ok(FBig {
                repr: Repr::new(significand, exponent),
                context: Context::new(precision),
            })
        }
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut signif: Option<IBig> = None;
        let mut exp: Option<isize> = None;
        let mut prec: Option<usize> = None;
        while let Some(key) = map.next_key()? {
            match key {
                KEY_SIGNIF => {
                    if signif.is_some() {
                        return Err(de::Error::duplicate_field(KEY_SIGNIF));
                    }
                    signif = Some(map.next_value()?);
                }
                KEY_EXPONENT => {
                    if exp.is_some() {
                        return Err(de::Error::duplicate_field(KEY_EXPONENT));
                    }
                    exp = Some(map.next_value()?);
                }
                KEY_PREC => {
                    if prec.is_some() {
                        return Err(de::Error::duplicate_field(KEY_PREC));
                    }
                    prec = Some(map.next_value()?);
                }
                _ => return Err(de::Error::unknown_field(key, REPR_FIELDS)),
            }
        }

        let significand = signif.ok_or_else(|| de::Error::missing_field(KEY_SIGNIF))?;
        let exponent = exp.ok_or_else(|| de::Error::missing_field(KEY_EXPONENT))?;
        let precision = prec.ok_or_else(|| de::Error::missing_field(KEY_PREC))?;
        Ok(FBig {
            repr: Repr::new(significand, exponent),
            context: Context::new(precision),
        })
    }
}

impl<'de, R: Round, const B: Word> Deserialize<'de> for FBig<R, B> {
    #[inline]
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if deserializer.is_human_readable() {
            deserializer.deserialize_str(FBigVisitor(PhantomData))
        } else {
            deserializer.deserialize_struct("FBig", FBIG_FIELDS, FBigVisitor(PhantomData))
        }
    }
}
