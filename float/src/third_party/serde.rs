//! Implement serde traits.

use core::marker::PhantomData;

use crate::{fbig::FBig, repr::Repr, round::Round, Context};
use alloc::string::String;
use core::fmt::Write;
use dashu_base::Sign;
use dashu_int::{IBig, Word};
use serde::{
    de::{self, Deserialize, DeserializeSeed, Deserializer, SeqAccess, Visitor},
    ser::{Serialize, SerializeStruct, Serializer},
};

const KEY_SIGNIF: &str = "significand";
const KEY_EXPONENT: &str = "exponent";
const KEY_PREC: &str = "precision";
const REPR_FIELDS: &[&str] = &[KEY_SIGNIF, KEY_EXPONENT];
const FBIG_FIELDS: &[&str] = &[KEY_SIGNIF, KEY_EXPONENT, KEY_PREC];

const STR_INF: &str = "inf";
const STR_NEG_INF: &str = "-inf";

/// The infinities are written as `inf` / `-inf` by the human-readable serializers, so the
/// deserializers accept exactly those two strings (the `FromStr` impl deliberately does not).
fn parse_infinity<const B: Word>(v: &str) -> Option<Repr<B>> {
    match v {
        STR_INF => Some(Repr::infinity()),
        STR_NEG_INF => Some(Repr::neg_infinity()),
        _ => None,
    }
}

impl<R: Round, const B: Word> FBig<R, B> {
    /// The literal that parses back to this number *with the same precision*, if one exists.
    ///
    /// The parser infers the precision from the number of digits written (leading and trailing
    /// zeros included), so the literal must contain exactly `precision` digits. That is not
    /// possible when the precision is unlimited (zero) or smaller than the number of
    /// significant digits (an inexact sum may carry a guard digit), and an infinity literal
    /// carries no precision at all; `None` is returned in those cases.
    fn to_literal_with_precision(&self) -> Option<String> {
        let precision = self.context.precision;
        if self.repr.is_infinite() {
            return if precision == 0 {
                Some(String::from(match self.repr.sign() {
                    Sign::Positive => STR_INF,
                    Sign::Negative => STR_NEG_INF,
                }))
            } else {
                None
            };
        }

        let digits = self.repr.digits();
        if precision == 0 || digits > precision {
            return None;
        }

        // number of digits in the positional (`Display`) form: zeros are written out down to
        // the units place for an integer, and up to it (`0.00ddd`) for a fraction
        let exponent = self.repr.exponent;
        let positional_digits = if digits == 0 {
            Some(1)
        } else if exponent >= 0 {
            digits.checked_add(exponent.unsigned_abs())
        } else if exponent.unsigned_abs() >= digits {
            exponent.unsigned_abs().checked_add(1)
        } else {
            Some(digits)
        };

        let mut s = String::new();
        match positional_digits {
            Some(n) if n <= precision => {
                // positional form, padded with trailing zeros up to the precision
                write!(&mut s, "{}", self.repr).ok()?;
                if n < precision {
                    if !s.contains('.') {
                        s.push('.');
                    }
                    s.extend(core::iter::repeat('0').take(precision - n));
                }
            }
            _ => {
                // The zeros of the positional form alone exceed the precision (`1000` or `0.5`
                // with a precision of 1), so write the significand, padded up to the
                // precision, with an explicit scale instead.
                write!(&mut s, "{}", self.repr.significand.in_radix(B as _)).ok()?;
                if digits < precision {
                    s.push('.');
                    s.extend(core::iter::repeat('0').take(precision - digits));
                }
                write!(&mut s, "@{}", exponent).ok()?;
            }
        }
        Some(s)
    }
}

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
            // Serialize to a literal with exactly as many digits as the context precision.
            // The deserializer re-infers the precision from that digit count, so this lets
            // the precision round-trip through the human-readable format.
            if let Some(s) = self.to_literal_with_precision() {
                return serializer.serialize_str(&s);
            }
        }

        // Otherwise serialize as a (significand, exponent, precision) struct. This is the only
        // form of the non-human-readable formats, and the fallback of the human-readable ones
        // for the numbers whose precision no literal can express.
        let mut se = serializer.serialize_struct("FBig", 3)?;
        se.serialize_field(KEY_SIGNIF, &self.repr.significand)?;
        se.serialize_field(KEY_EXPONENT, &self.repr.exponent)?;
        se.serialize_field(KEY_PREC, &self.context.precision)?;
        se.end()
    }
}

/// A field name of the struct forms.
enum Field {
    Significand,
    Exponent,
    Precision,
}

/// Deserializes a field name. Formats differ in how they hand over map keys (borrowed from
/// the input, as a transient string, as an owned string, or as bytes), so the name is matched
/// in a visitor instead of being deserialized as a borrowed `&str`.
struct FieldSeed(&'static [&'static str]);

impl<'de> Visitor<'de> for FieldSeed {
    type Value = Field;

    #[inline]
    fn expecting(&self, formatter: &mut alloc::fmt::Formatter) -> alloc::fmt::Result {
        formatter.write_str("a field name")
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        match v {
            KEY_SIGNIF => Ok(Field::Significand),
            KEY_EXPONENT => Ok(Field::Exponent),
            KEY_PREC => Ok(Field::Precision),
            _ => Err(de::Error::unknown_field(v, self.0)),
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

struct ReprVisitor<const BASE: Word>;

impl<'de, const B: Word> Visitor<'de> for ReprVisitor<B> {
    type Value = Repr<B>;

    #[inline]
    fn expecting(&self, formatter: &mut alloc::fmt::Formatter) -> alloc::fmt::Result {
        formatter.write_str("float repr as a literal string or a struct (significand, exponent)")
    }

    #[inline]
    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        if let Some(repr) = parse_infinity(v) {
            return Ok(repr);
        }
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
            Ok(Repr::from_encoded_parts(significand, exponent))
        }
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut signif: Option<IBig> = None;
        let mut exp: Option<isize> = None;
        while let Some(key) = map.next_key_seed(FieldSeed(REPR_FIELDS))? {
            match key {
                Field::Significand => {
                    if signif.is_some() {
                        return Err(de::Error::duplicate_field(KEY_SIGNIF));
                    }
                    signif = Some(map.next_value()?);
                }
                Field::Exponent => {
                    if exp.is_some() {
                        return Err(de::Error::duplicate_field(KEY_EXPONENT));
                    }
                    exp = Some(map.next_value()?);
                }
                Field::Precision => return Err(de::Error::unknown_field(KEY_PREC, REPR_FIELDS)),
            }
        }

        let significand = signif.ok_or_else(|| de::Error::missing_field(KEY_SIGNIF))?;
        let exponent = exp.ok_or_else(|| de::Error::missing_field(KEY_EXPONENT))?;
        Ok(Repr::from_encoded_parts(significand, exponent))
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
        if let Some(repr) = parse_infinity(v) {
            return Ok(FBig {
                repr,
                context: Context::new(0),
            });
        }
        let (repr, ndigits) = Repr::<B>::from_str_native(v).map_err(de::Error::custom)?;
        Ok(FBig {
            repr,
            context: Context::new(ndigits),
        })
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
                repr: Repr::from_encoded_parts(significand, exponent),
                context: Context::new(precision),
            })
        }
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut signif: Option<IBig> = None;
        let mut exp: Option<isize> = None;
        let mut prec: Option<usize> = None;
        while let Some(key) = map.next_key_seed(FieldSeed(FBIG_FIELDS))? {
            match key {
                Field::Significand => {
                    if signif.is_some() {
                        return Err(de::Error::duplicate_field(KEY_SIGNIF));
                    }
                    signif = Some(map.next_value()?);
                }
                Field::Exponent => {
                    if exp.is_some() {
                        return Err(de::Error::duplicate_field(KEY_EXPONENT));
                    }
                    exp = Some(map.next_value()?);
                }
                Field::Precision => {
                    if prec.is_some() {
                        return Err(de::Error::duplicate_field(KEY_PREC));
                    }
                    prec = Some(map.next_value()?);
                }
            }
        }

        let significand = signif.ok_or_else(|| de::Error::missing_field(KEY_SIGNIF))?;
        let exponent = exp.ok_or_else(|| de::Error::missing_field(KEY_EXPONENT))?;
        let precision = prec.ok_or_else(|| de::Error::missing_field(KEY_PREC))?;
        Ok(FBig {
            repr: Repr::from_encoded_parts(significand, exponent),
            context: Context::new(precision),
        })
    }
}

impl<'de, R: Round, const B: Word> Deserialize<'de> for FBig<R, B> {
    #[inline]
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if deserializer.is_human_readable() {
            // a literal string, or the struct form for the numbers that no literal can express
            // together with their precision (see `Serialize`)
            deserializer.deserialize_any(FBigVisitor(PhantomData))
        } else {
            deserializer.deserialize_struct("FBig", FBIG_FIELDS, FBigVisitor(PhantomData))
        }
    }
}
