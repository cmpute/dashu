//! Implement serde traits.
//!
//! Mirrors `dashu-float`'s serde: the human-readable form is a string, the binary form is a
//! struct. The rounding mode `R` is a type parameter and is not serialized (same as `FBig`).

use core::fmt::{self, Display, Formatter};
use core::marker::PhantomData;
use core::str::FromStr;

use crate::{cbig::CBig, repr::Context};
use dashu_base::Sign;
use dashu_float::round::Round;
use dashu_float::{FBig, Repr};
use dashu_int::Word;
use serde::{
    de::{self, Deserialize, Deserializer, SeqAccess, Visitor},
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
            serializer.collect_str(&CBigHuman(self))
        } else {
            // `re`/`im` delegate to the float `Repr` serde (a (significand, exponent) struct);
            // the shared precision is stored explicitly so it round-trips.
            let mut se = serializer.serialize_struct("CBig", 3)?;
            se.serialize_field(KEY_RE, &self.re)?;
            se.serialize_field(KEY_IM, &self.im)?;
            se.serialize_field(KEY_PREC, &self.context.precision())?;
            se.end()
        }
    }
}

/// Human-readable form: the algebraic `a+bi` with each finite coefficient padded to the context
/// precision's digit count, so `CBig::from_str` recovers the precision from the digit count —
/// exactly how `FBig`'s human-readable serde round-trips its precision.
struct CBigHuman<'a, R: Round, const B: Word>(&'a CBig<R, B>);

impl<R: Round, const B: Word> Display for CBigHuman<'_, R, B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let c = self.0;
        let p = c.context.precision();
        if c.re.is_infinite() || c.im.is_infinite() {
            // specials carry no digit count, so the padded form is meaningless — fall back to the
            // plain algebraic form (precision is not recoverable from specials; `FBig` does the same)
            return Display::fmt(c, f);
        }
        let fctx = c.context.float();
        let re = FBig::from_repr(c.re.clone(), fctx);
        let im = FBig::from_repr(c.im.clone(), fctx);
        let im_neg = c.im.sign() == Sign::Negative;
        let im_abs = if im_neg { -im } else { im };
        write_fbig_padded(f, &re, p)?;
        f.write_str(if im_neg { "-" } else { "+" })?;
        write_fbig_padded(f, &im_abs, p)?;
        f.write_str("i")
    }
}

/// Write a float's algebraic rendering, padding the digit count up to `precision` (never
/// truncating — a part carrying a guard digit keeps it, and deserialization then reports the
/// actual digit count, mirroring `FBig`'s serde).
fn write_fbig_padded<R: Round, const B: Word>(
    f: &mut Formatter<'_>,
    value: &FBig<R, B>,
    precision: usize,
) -> fmt::Result {
    let mut s = alloc::format!("{}", value);
    let digits = s.bytes().filter(|b| b.is_ascii_digit()).count();
    if digits < precision {
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
        while let Some(key) = map.next_key()? {
            match key {
                KEY_RE => {
                    if re.is_some() {
                        return Err(de::Error::duplicate_field(KEY_RE));
                    }
                    re = Some(map.next_value()?);
                }
                KEY_IM => {
                    if im.is_some() {
                        return Err(de::Error::duplicate_field(KEY_IM));
                    }
                    im = Some(map.next_value()?);
                }
                KEY_PREC => {
                    if precision.is_some() {
                        return Err(de::Error::duplicate_field(KEY_PREC));
                    }
                    precision = Some(map.next_value()?);
                }
                _ => return Err(de::Error::unknown_field(key, CBIG_FIELDS)),
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
            deserializer.deserialize_str(CBigVisitor(PhantomData))
        } else {
            deserializer.deserialize_struct("CBig", CBIG_FIELDS, CBigVisitor(PhantomData))
        }
    }
}
