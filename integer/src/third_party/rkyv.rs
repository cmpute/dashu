//! Implement rkyv traits.
//!
//! `UBig`/`IBig` archive as their **native word representation**: `ArchivedVec<Word>`, plus a sign
//! flag for `IBig`. The internal `Repr`'s niche-bit layout is not directly expressible by rkyv's
//! derive, so the archive delegates to `ArchivedVec` and round-trips through
//! [`as_words`](UBig::as_words)/[`from_words`](UBig::from_words).
//!
//! Serialization and resolution are **allocation-free**: rkyv's `ArchivedVec::serialize_from_slice`
//! writes the borrowed word slice directly into the serializer's buffer (no intermediate `Vec`),
//! and `archived_root` yields those words in place (`&[Word]`) with no byte conversion — the
//! fastest possible same-architecture encoding. The trade-off is that the archive layout depends on
//! the target's `Word` width and endianness, so it is not portable across 32/64-bit or
//! across-endianness machines. Users who need a stable, portable encoding should convert explicitly
//! via `to_le_bytes`/`to_be_bytes` before archiving — matching rkyv's stance that performance comes
//! before portability.

use rkyv_v07 as rkyv;

use crate::{IBig, UBig, Word};
use dashu_base::Sign;

impl rkyv::Archive for UBig {
    type Archived = rkyv::vec::ArchivedVec<Word>;
    type Resolver = rkyv::vec::VecResolver;

    #[inline]
    unsafe fn resolve(&self, pos: usize, resolver: Self::Resolver, out: *mut Self::Archived) {
        rkyv::vec::ArchivedVec::<Word>::resolve_from_slice(self.as_words(), pos, resolver, out);
    }
}

impl<S: rkyv::ser::Serializer + rkyv::ser::ScratchSpace + ?Sized> rkyv::Serialize<S> for UBig {
    #[inline]
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        rkyv::vec::ArchivedVec::<Word>::serialize_from_slice(self.as_words(), serializer)
    }
}

impl<D: rkyv::Fallible + ?Sized> rkyv::Deserialize<UBig, D> for rkyv::vec::ArchivedVec<Word> {
    #[inline]
    fn deserialize(&self, _: &mut D) -> Result<UBig, D::Error> {
        Ok(UBig::from_words(self.as_slice()))
    }
}

// `IBig` archives as `(is_negative, words)`, reusing the same word storage as `UBig`. The resolve
// writes the two tuple fields with `out_field!` (the same offset computation rkyv's own tuple impl
// uses), and the words go through `serialize_from_slice` — no `Vec` is ever built.
impl rkyv::Archive for IBig {
    type Archived = (bool, rkyv::vec::ArchivedVec<Word>);
    type Resolver = ((), rkyv::vec::VecResolver);

    #[inline]
    #[allow(clippy::unit_arg)] // `bool`'s resolver is `()`; passing it is required by the Archive call
    unsafe fn resolve(&self, pos: usize, resolver: Self::Resolver, out: *mut Self::Archived) {
        let (sign, words) = self.0.as_sign_slice();
        let (fp0, fo0) = rkyv::out_field!(out.0);
        <bool as rkyv::Archive>::resolve(
            &matches!(sign, Sign::Negative),
            pos + fp0,
            resolver.0,
            fo0,
        );
        let (fp1, fo1) = rkyv::out_field!(out.1);
        rkyv::vec::ArchivedVec::<Word>::resolve_from_slice(words, pos + fp1, resolver.1, fo1);
    }
}

impl<S: rkyv::ser::Serializer + rkyv::ser::ScratchSpace + ?Sized> rkyv::Serialize<S> for IBig {
    #[inline]
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        let (sign, words) = self.0.as_sign_slice();
        <bool as rkyv::Serialize<S>>::serialize(&matches!(sign, Sign::Negative), serializer)?;
        let words_resolver =
            rkyv::vec::ArchivedVec::<Word>::serialize_from_slice(words, serializer)?;
        Ok(((), words_resolver))
    }
}

impl<D: rkyv::Fallible + ?Sized> rkyv::Deserialize<IBig, D>
    for (bool, rkyv::vec::ArchivedVec<Word>)
{
    #[inline]
    fn deserialize(&self, _: &mut D) -> Result<IBig, D::Error> {
        // `self.1.as_slice()` reads the words straight out of the archive
        let mag = UBig::from_words(self.1.as_slice());
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
            let bytes = rkyv::to_bytes::<_, 512>(v).unwrap();
            // SAFETY: `bytes` came from `rkyv::to_bytes` on the same value, so it is a valid
            // archived object with the root at the end.
            let archived = unsafe { rkyv::archived_root::<UBig>(&bytes) };
            // zero-copy access yields the native words in place
            assert_eq!(archived.as_slice(), v.as_words(), "word view mismatch for {v}");
            // SAFETY: `bytes` came from `rkyv::to_bytes` on the same value, so it is a valid
            // archived object with the root at the end.
            let v2 = unsafe { rkyv::from_bytes_unchecked::<UBig>(&bytes) }.unwrap();
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
            let bytes = rkyv::to_bytes::<_, 512>(v).unwrap();
            // SAFETY: `bytes` came from `rkyv::to_bytes` on the same value, so it is a valid
            // archived object with the root at the end.
            let archived = unsafe { rkyv::archived_root::<IBig>(&bytes) };
            // zero-copy access: the archived sign flag and words
            assert_eq!(archived.0, v.sign() == Sign::Negative, "sign view mismatch for {v}");
            let mag = v.clone().into_parts().1;
            assert_eq!(archived.1.as_slice(), mag.as_words(), "word view mismatch for {v}");
            // SAFETY: `bytes` came from `rkyv::to_bytes` on the same value, so it is a valid
            // archived object with the root at the end.
            let v2 = unsafe { rkyv::from_bytes_unchecked::<IBig>(&bytes) }.unwrap();
            assert_eq!(v2, *v, "IBig round-trip failed for {v}");
        }
    }
}
