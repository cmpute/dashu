//! Implement serde traits.
//!
//! Mirrors `dashu-float`'s serde: the human-readable form is a string, the binary form is a
//! struct. The string implies the precision through its digit counts; a value whose precision it
//! cannot express that way (unlimited precision, or a part holding more digits than the
//! precision, or two infinite parts at a limited precision) is written in the struct form in
//! human-readable formats too, so every value round-trips with its precision. The rounding mode
//! `R` is a type parameter and is not serialized (same as `FBig`).

use core::fmt::{self, Display, Formatter};
use core::marker::PhantomData;
use core::str::FromStr;

use crate::{cbig::CBig, repr::Context};
use dashu_base::Sign;
use dashu_float::round::Round;
use dashu_float::{FBig, Repr};
use dashu_int::Word;
use serde::{
    de::{self, Deserialize, DeserializeSeed, Deserializer, SeqAccess, Visitor},
    ser::{Serialize, SerializeStruct, Serializer},
};

const KEY_RE: &str = "re";
const KEY_IM: &str = "im";
const KEY_PREC: &str = "precision";
const CBIG_FIELDS: &[&str] = &[KEY_RE, KEY_IM, KEY_PREC];

impl<R: Round, const B: Word> Serialize for CBig<R, B> {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            // The single `a+bi` string is used whenever it reads back as the same value at the
            // same precision; the precision is only implied by the coefficients' digit counts,
            // so an unlimited precision (or a part holding more digits than the precision) has
            // no such string and falls through to the explicit struct form.
            // That includes a value with an infinite part: the string cannot carry the
            // precision when both parts are infinite, and in bases above 23 the letters of
            // `inf` are digits, so there it would read back as a finite number.
            //
            // A part whose positional rendering alone needs more digits than the precision
            // (a large exponent) can never be faithful, and its rendering can be arbitrarily
            // long, so that case is decided before anything is formatted.
            let p = self.context.precision();
            if fits_positional(&self.re, p) && fits_positional(&self.im, p) {
                let literal = alloc::format!("{}", CBigHuman(self));
                let faithful = matches!(
                    CBig::<R, B>::from_str(&literal),
                    Ok(back) if back.context.precision() == p
                        && back.re == self.re
                        && back.im == self.im
                );
                if faithful {
                    return serializer.serialize_str(&literal);
                }
            }
        }
        // `re`/`im` delegate to the float `Repr` serde; the shared precision is stored
        // explicitly so it round-trips.
        let mut se = serializer.serialize_struct("CBig", 3)?;
        se.serialize_field(KEY_RE, &self.re)?;
        se.serialize_field(KEY_IM, &self.im)?;
        se.serialize_field(KEY_PREC, &self.context.precision())?;
        se.end()
    }
}

/// Whether the positional (`Display`) rendering of a part has at most `precision` digits, so
/// that padding it can imply the precision. Zeros are written out down to the units place for
/// an integer and up to it (`0.00ddd`) for a fraction, so a large exponent of either sign does
/// not fit; nor does anything at unlimited precision. An infinity has no digits and fits.
fn fits_positional<const B: Word>(part: &Repr<B>, precision: usize) -> bool {
    if part.is_infinite() {
        return true;
    }
    let digits = part.digits();
    let exponent = part.exponent().unsigned_abs();
    let positional = if digits == 0 {
        Some(1)
    } else if part.exponent() >= 0 {
        digits.checked_add(exponent)
    } else if exponent >= digits {
        exponent.checked_add(1)
    } else {
        Some(digits)
    };
    matches!(positional, Some(n) if n <= precision)
}

/// Human-readable form: the algebraic `a+bi` with each finite coefficient padded to the context
/// precision's digit count, so `CBig::from_str` recovers the precision from the digit count —
/// exactly how `FBig`'s human-readable serde round-trips its precision. An infinite coefficient
/// is written as `inf` and carries no digit count. (When the string does not read back as the
/// same value the serializer uses the struct form instead, see above.)
struct CBigHuman<'a, R: Round, const B: Word>(&'a CBig<R, B>);

impl<R: Round, const B: Word> Display for CBigHuman<'_, R, B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let c = self.0;
        let p = c.context.precision();
        let fctx = c.context.float();
        let re = FBig::from_repr(c.re.clone(), fctx);
        let im = FBig::from_repr(c.im.clone(), fctx);
        let im_neg = c.im.sign() == Sign::Negative;
        let im_abs = if im_neg { -im } else { im };
        write_fbig_padded(f, &re, p)?;
        f.write_str(if im_neg { "-" } else { "+" })?;
        write_fbig_padded(f, &im_abs, p)?;
        f.write_str(if crate::fmt::i_is_digit::<B>() {
            "*i"
        } else {
            "i"
        })
    }
}

/// Write a float's algebraic rendering, padding the digit count up to `precision` (never
/// truncating — a part carrying a guard digit keeps it). Every digit of the base counts (the
/// letters `a`–`z` are the digits 10–35), as it does when the string is parsed back.
fn write_fbig_padded<R: Round, const B: Word>(
    f: &mut Formatter<'_>,
    value: &FBig<R, B>,
    precision: usize,
) -> fmt::Result {
    let mut s = alloc::format!("{}", value);
    let digits = s.bytes().filter(|b| b.is_ascii_alphanumeric()).count();
    if value.repr().is_finite() && digits < precision {
        let need = precision - digits;
        if s.contains('.') {
            s.extend(core::iter::repeat('0').take(need));
        } else {
            s.push('.');
            s.extend(core::iter::repeat('0').take(need));
        }
    }
    f.write_str(&s)
}

/// A field name of the struct form.
enum Field {
    Re,
    Im,
    Precision,
}

/// Deserializes a field name. Formats differ in how they hand over map keys (borrowed from
/// the input, as a transient string, as an owned string, or as bytes), so the name is matched
/// in a visitor instead of being deserialized as a borrowed `&str`.
struct FieldSeed;

impl<'de> Visitor<'de> for FieldSeed {
    type Value = Field;

    #[inline]
    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("a field name")
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        match v {
            KEY_RE => Ok(Field::Re),
            KEY_IM => Ok(Field::Im),
            KEY_PREC => Ok(Field::Precision),
            _ => Err(de::Error::unknown_field(v, CBIG_FIELDS)),
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

struct CBigVisitor<R: Round, const B: Word>(PhantomData<R>);

impl<'de, R: Round, const B: Word> Visitor<'de> for CBigVisitor<R, B> {
    type Value = CBig<R, B>;

    #[inline]
    fn expecting(&self, formatter: &mut alloc::fmt::Formatter) -> alloc::fmt::Result {
        formatter
            .write_str("complex number as a `a+bi` literal string or a struct (re, im, precision)")
    }

    #[inline]
    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        CBig::<R, B>::from_str(v).map_err(de::Error::custom)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let err_report = || {
            de::Error::invalid_length(
                3,
                &"a complex number consists of three fields: (re, im, precision)",
            )
        };
        let re = seq.next_element()?.ok_or_else(err_report)?;
        let im = seq.next_element()?.ok_or_else(err_report)?;
        let precision = seq.next_element()?.ok_or_else(err_report)?;

        if seq.next_element::<de::IgnoredAny>()?.is_some() {
            Err(err_report())?
        } else {
            Ok(CBig::new(re, im, Context::new(precision)))
        }
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut re: Option<Repr<B>> = None;
        let mut im: Option<Repr<B>> = None;
        let mut precision: Option<usize> = None;
        while let Some(key) = map.next_key_seed(FieldSeed)? {
            match key {
                Field::Re => {
                    if re.is_some() {
                        return Err(de::Error::duplicate_field(KEY_RE));
                    }
                    re = Some(map.next_value()?);
                }
                Field::Im => {
                    if im.is_some() {
                        return Err(de::Error::duplicate_field(KEY_IM));
                    }
                    im = Some(map.next_value()?);
                }
                Field::Precision => {
                    if precision.is_some() {
                        return Err(de::Error::duplicate_field(KEY_PREC));
                    }
                    precision = Some(map.next_value()?);
                }
            }
        }

        let re = re.ok_or_else(|| de::Error::missing_field(KEY_RE))?;
        let im = im.ok_or_else(|| de::Error::missing_field(KEY_IM))?;
        let precision = precision.ok_or_else(|| de::Error::missing_field(KEY_PREC))?;
        Ok(CBig::new(re, im, Context::new(precision)))
    }
}

impl<'de, R: Round, const B: Word> Deserialize<'de> for CBig<R, B> {
    #[inline]
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if deserializer.is_human_readable() {
            // a `a+bi` string, or the struct form (used when the string cannot carry the precision)
            deserializer.deserialize_any(CBigVisitor(PhantomData))
        } else {
            deserializer.deserialize_struct("CBig", CBIG_FIELDS, CBigVisitor(PhantomData))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positional_fit() {
        let r = |sig: i32, exp: isize| Repr::<10>::new(sig.into(), exp);
        // zero, integers and fractions count the zeros written around the significand
        assert!(fits_positional(&r(0, 0), 1));
        assert!(!fits_positional(&r(0, 0), 0));
        assert!(fits_positional(&r(123, 0), 3));
        assert!(!fits_positional(&r(123, 0), 2));
        assert!(fits_positional(&r(12, 3), 5)); // 12000
        assert!(!fits_positional(&r(12, 3), 4));
        assert!(fits_positional(&r(123, -2), 3)); // 1.23
        assert!(fits_positional(&r(12, -4), 5)); // 0.0012
        assert!(!fits_positional(&r(12, -4), 4));
        // an exponent of any size is decided from the exponent alone
        for e in [
            isize::MAX - 1,
            isize::MAX / 2,
            isize::MIN / 2,
            isize::MIN + 2,
        ] {
            assert!(!fits_positional(&r(1, e), 1000), "{e}");
        }
        assert!(fits_positional(&Repr::<10>::infinity(), 0));
        assert!(fits_positional(&Repr::<10>::neg_infinity(), 5));
    }
}
