use crate::{
    arch::word::Word,
    buffer::Buffer,
    ibig::IBig,
    primitive::{split_dword, WORD_BITS_USIZE},
    repr::{Repr, TypedReprRef},
    ubig::UBig,
    Sign,
};
use alloc::vec::Vec;
use core::fmt::{self, Formatter};
use serde::{
    de::{Deserialize, Deserializer, SeqAccess, Visitor},
    ser::{Serialize, SerializeSeq, SerializeTuple, Serializer},
};
use static_assertions::const_assert;

// We ensure that the max size of a word is 64-bit, if we are going to
// support 128 bit word, it's going to be a break change.
const_assert!(64 % WORD_BITS_USIZE == 0);
const WORDS_PER_U64: usize = 64 / WORD_BITS_USIZE;

impl<'a> Serialize for TypedReprRef<'a> {
    #[allow(clippy::useless_conversion)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            TypedReprRef::RefSmall(0) => serializer.serialize_seq(Some(0))?.end(),
            TypedReprRef::RefSmall(dword) => {
                let (lo, hi) = split_dword(dword);
                if WORDS_PER_U64 == 1 && hi != 0 {
                    let mut seq = serializer.serialize_seq(Some(2))?;
                    seq.serialize_element(&lo)?;
                    seq.serialize_element(&hi)?;
                    seq.end()
                } else {
                    let mut chunk = u64::from(lo);
                    #[allow(arithmetic_overflow)]
                    if hi != 0 {
                        // this won't overflow because WORDS_PER_U64 > 1 if hi != 0
                        chunk |= u64::from(hi) << WORD_BITS_USIZE;
                    }
                    let mut seq = serializer.serialize_seq(Some(1))?;
                    seq.serialize_element(&chunk)?;
                    seq.end()
                }
            }
            TypedReprRef::RefLarge(words) => {
                let chunks = words.chunks(WORDS_PER_U64);
                let mut seq = serializer.serialize_seq(Some(chunks.len()))?;
                for chunk in chunks {
                    let mut word_u64: u64 = 0;
                    for (i, word) in chunk.iter().enumerate() {
                        word_u64 |= u64::from(*word) << (i * WORD_BITS_USIZE);
                    }
                    seq.serialize_element(&word_u64)?;
                }
                seq.end()
            }
        }
    }
}

impl Serialize for UBig {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            serializer.serialize_str(&self.to_string())
        } else {
            self.repr().serialize(serializer)
        }
    }
}

impl<'de> Deserialize<'de> for UBig {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if deserializer.is_human_readable() {
            let s = String::deserialize(deserializer)?;
            match UBig::from_str_with_radix_prefix(&s) {
                Ok((n, _)) => Ok(n),
                Err(e) => Err(serde::de::Error::custom(e)),
            }
        } else {
            deserializer.deserialize_seq(UBigVisitor)
        }
    }
}

/// Currently all the data in the big integer is serialized as u64 chunks
struct UBigVisitor;

impl<'de> Visitor<'de> for UBigVisitor {
    type Value = UBig;

    fn expecting(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "a sequence of 64-bit words")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<UBig, A::Error> {
        match seq.size_hint() {
            Some(0) => {
                assert!(seq.next_element::<u64>()?.is_none());
                Ok(UBig::ZERO)
            }
            Some(1) => {
                let word_64: u64 = seq.next_element()?.unwrap();
                assert!(seq.next_element::<u64>()?.is_none());
                Ok(UBig::from(word_64))
            }
            Some(num_words_64) => {
                let mut buffer = Buffer::allocate(len_64_to_max_len(num_words_64));
                for _ in 0..num_words_64 {
                    let word_64: u64 = seq.next_element()?.unwrap();
                    push_word_64(&mut buffer, word_64);
                }
                assert!(seq.next_element::<u64>()?.is_none());
                Ok(UBig(Repr::from_buffer(buffer)))
            }
            None => {
                let mut words_64 = Vec::new();
                while let Some(word_64) = seq.next_element()? {
                    words_64.push(word_64);
                }
                let mut buffer = Buffer::allocate(len_64_to_max_len(words_64.len()));
                for word_64 in words_64 {
                    push_word_64(&mut buffer, word_64);
                }
                Ok(UBig(Repr::from_buffer(buffer)))
            }
        }
    }
}

fn push_word_64(buffer: &mut Buffer, word_64: u64) {
    for i in 0..WORDS_PER_U64 {
        buffer.push((word_64 >> (i * WORD_BITS_USIZE)) as Word);
    }
}

#[allow(clippy::absurd_extreme_comparisons)]
fn len_64_to_max_len(len_64: usize) -> usize {
    #[allow(clippy::redundant_closure)]
    len_64
        .checked_mul(WORDS_PER_U64)
        .expect("The number to be deserialized is too large")
}

impl Serialize for IBig {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            serializer.serialize_str(&self.to_string())
        } else {
            let (sign, repr) = self.as_sign_repr();
            let mut tup = serializer.serialize_tuple(2)?;
            tup.serialize_element(&(sign == Sign::Negative))?;
            tup.serialize_element(&repr)?;
            tup.end()
        }
    }
}

impl<'de> Deserialize<'de> for IBig {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if deserializer.is_human_readable() {
            let s = String::deserialize(deserializer)?;
            match IBig::from_str_with_radix_prefix(&s) {
                Ok((n, _)) => Ok(n),
                Err(e) => Err(serde::de::Error::custom(e)),
            }
        } else {
            let (sign, magnitude): (bool, UBig) = Deserialize::deserialize(deserializer)?;
            let sign = if sign { Sign::Negative } else { Sign::Positive };
            Ok(IBig(magnitude.0.with_sign(sign)))
        }
    }
}
