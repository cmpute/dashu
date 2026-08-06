//! Implement rkyv 0.8 traits.
//!
//! Mirrors the [`super::rkyv_v07`] 0.7 module: `UBig`/`IBig` archive as their word representation
//! (`ArchivedVec<Word::Archived>`, plus a sign flag for `IBig`), serialized allocation-free from
//! the borrowed word slice. rkyv 0.8 archives multi-byte primitives **little-endian** by default,
//! so the archived words are `u64_le`-style wrappers; reading them back needs a `to_native()`
//! conversion per word. This module targets rkyv 0.8's `Place`-based `Archive` API and requires
//! Rust ≥ 1.81 (rkyv 0.8's MSRV); it is stripped from the 1.68 MSRV build.

use alloc::vec::Vec;
use rkyv_v08 as rkyv;

use crate::{IBig, UBig, Word};
use dashu_base::Sign;

/// The archived form of a `Word`: in rkyv 0.8 this is a little-endian wrapper (`u64_le`-style).
type WordArchived = <Word as rkyv::Archive>::Archived;

impl rkyv::Archive for UBig {
    type Archived = rkyv::vec::ArchivedVec<WordArchived>;
    type Resolver = rkyv::vec::VecResolver;

    fn resolve(&self, resolver: Self::Resolver, out: rkyv::Place<Self::Archived>) {
        rkyv::vec::ArchivedVec::<WordArchived>::resolve_from_slice(self.as_words(), resolver, out);
    }
}

impl<S: rkyv::rancor::Fallible + rkyv::ser::Allocator + rkyv::ser::Writer + ?Sized>
    rkyv::Serialize<S> for UBig
{
    #[inline]
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        rkyv::vec::ArchivedVec::<WordArchived>::serialize_from_slice(self.as_words(), serializer)
    }
}

impl<D: rkyv::rancor::Fallible + ?Sized> rkyv::Deserialize<UBig, D>
    for rkyv::vec::ArchivedVec<WordArchived>
{
    #[inline]
    fn deserialize(&self, _: &mut D) -> Result<UBig, D::Error> {
        let words: Vec<Word> = self.as_slice().iter().map(|w| w.to_native()).collect();
        Ok(UBig::from_words(&words))
    }
}

// `IBig` archives as `(is_negative, words)`; the two `ArchivedTuple2` fields are written with the
// same `Place` projection rkyv's own tuple impl uses.
impl rkyv::Archive for IBig {
    type Archived = rkyv::tuple::ArchivedTuple2<bool, rkyv::vec::ArchivedVec<WordArchived>>;
    type Resolver = (<bool as rkyv::Archive>::Resolver, rkyv::vec::VecResolver);

    #[allow(clippy::unit_arg)] // `bool`'s resolver is `()`
    fn resolve(&self, resolver: Self::Resolver, out: rkyv::Place<Self::Archived>) {
        let (sign, words) = self.0.as_sign_slice();
        unsafe {
            // SAFETY: `out` points to a valid, aligned `ArchivedTuple2` being resolved
            // field-by-field; the `addr_of_mut!` projections address the tuple's own fields, and
            // `Place::from_field_unchecked` is the same projection rkyv's own tuple impl uses.
            let out_ptr = out.ptr();
            let ptr = core::ptr::addr_of_mut!((*out_ptr).0);
            let out_field = rkyv::Place::from_field_unchecked(out, ptr);
            <bool as rkyv::Archive>::resolve(
                &matches!(sign, Sign::Negative),
                resolver.0,
                out_field,
            );
            let ptr = core::ptr::addr_of_mut!((*out_ptr).1);
            let out_field = rkyv::Place::from_field_unchecked(out, ptr);
            rkyv::vec::ArchivedVec::<WordArchived>::resolve_from_slice(
                words, resolver.1, out_field,
            );
        }
    }
}

impl<S: rkyv::rancor::Fallible + rkyv::ser::Allocator + rkyv::ser::Writer + ?Sized>
    rkyv::Serialize<S> for IBig
{
    #[inline]
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        let (sign, words) = self.0.as_sign_slice();
        <bool as rkyv::Serialize<S>>::serialize(&matches!(sign, Sign::Negative), serializer)?;
        let words_resolver =
            rkyv::vec::ArchivedVec::<WordArchived>::serialize_from_slice(words, serializer)?;
        Ok(((), words_resolver))
    }
}

impl<D: rkyv::rancor::Fallible + ?Sized> rkyv::Deserialize<IBig, D>
    for rkyv::tuple::ArchivedTuple2<bool, rkyv::vec::ArchivedVec<WordArchived>>
{
    #[inline]
    fn deserialize(&self, _: &mut D) -> Result<IBig, D::Error> {
        let words: Vec<Word> = self.1.as_slice().iter().map(|w| w.to_native()).collect();
        let mag = UBig::from_words(&words);
        Ok(if self.0 {
            -IBig::from(mag)
        } else {
            IBig::from(mag)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rkyv_ubig_roundtrip() {
        let values = [
            UBig::from(0u8),
            UBig::from(1u8),
            UBig::from(123456789u64),
            UBig::from(1u8) << 100,
            (UBig::from(3u8) << 300) + UBig::from(5u8),
        ];
        for v in &values {
            let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(v).unwrap();
            // SAFETY: `bytes` came from `rkyv::to_bytes` on the same value, so it is a valid
            // archived object with the root at the end.
            let archived =
                unsafe { rkyv::access_unchecked::<<UBig as rkyv::Archive>::Archived>(&bytes) };
            let native: Vec<Word> = archived.as_slice().iter().map(|w| w.to_native()).collect();
            assert_eq!(native, v.as_words(), "word view mismatch for {v}");
            // SAFETY: `bytes` came from `rkyv::to_bytes` on the same value, so it is a valid
            // archived object with the root at the end.
            let v2 =
                unsafe { rkyv::from_bytes_unchecked::<UBig, rkyv::rancor::Error>(&bytes) }.unwrap();
            assert_eq!(v2, *v, "UBig round-trip failed for {v}");
        }
    }

    #[test]
    fn rkyv_ibig_roundtrip() {
        let values = [
            IBig::from(0),
            IBig::from(-1),
            IBig::from(1),
            IBig::from(123456789i64),
            IBig::from(-123456789i64),
            (IBig::from(-1)) << 200,
        ];
        for v in &values {
            let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(v).unwrap();
            // SAFETY: `bytes` came from `rkyv::to_bytes` on the same value, so it is a valid
            // archived object with the root at the end.
            let archived =
                unsafe { rkyv::access_unchecked::<<IBig as rkyv::Archive>::Archived>(&bytes) };
            assert_eq!(archived.0, v.sign() == Sign::Negative, "sign view mismatch for {v}");
            let mag = v.clone().into_parts().1;
            let native: Vec<Word> = archived
                .1
                .as_slice()
                .iter()
                .map(|w| w.to_native())
                .collect();
            assert_eq!(native, mag.as_words(), "word view mismatch for {v}");
            // SAFETY: `bytes` came from `rkyv::to_bytes` on the same value, so it is a valid
            // archived object with the root at the end.
            let v2 =
                unsafe { rkyv::from_bytes_unchecked::<IBig, rkyv::rancor::Error>(&bytes) }.unwrap();
            assert_eq!(v2, *v, "IBig round-trip failed for {v}");
        }
    }
}
