//! rkyv 0.7 support: the `Archive`/`Serialize`/`Deserialize` impls live on the derive (`cfg_attr`
//! on `Repr`/`Context`/`FBig`); this module holds the field wrappers for the exponent and the
//! precision.

use rkyv_v07::{
    with::{ArchiveWith, DeserializeWith, SerializeWith},
    Archive, Archived, Deserialize, Fallible, FixedIsize, FixedUsize,
};

/// Archives the exponent of a [`Repr`][crate::Repr].
///
/// An archived `isize` has a fixed width that can be narrower than the native one, and the
/// plain conversion truncates. That breaks the exponents `isize::MAX` / `isize::MIN`, which
/// mark the infinities (and, truncated, turn into the exponents of `-0` / `+0`). This wrapper
/// keeps the archived layout of an `isize`, but stores the two infinity markers as the largest
/// and smallest archived values, and refuses any other exponent that does not fit strictly
/// between them instead of truncating it.
pub struct Exponent;

// the archived range, clamped to the native one in case the latter is the narrower of the two
const ARCHIVED_MAX: isize = if (FixedIsize::MAX as i128) < isize::MAX as i128 {
    FixedIsize::MAX as isize
} else {
    isize::MAX
};
const ARCHIVED_MIN: isize = if (FixedIsize::MIN as i128) > isize::MIN as i128 {
    FixedIsize::MIN as isize
} else {
    isize::MIN
};

/// Map an exponent to the value to archive, or `None` if it cannot be archived.
fn encode(exponent: isize) -> Option<isize> {
    match exponent {
        isize::MAX => Some(ARCHIVED_MAX),
        isize::MIN => Some(ARCHIVED_MIN),
        e if ARCHIVED_MIN < e && e < ARCHIVED_MAX => Some(e),
        _ => None,
    }
}

/// The inverse of [`encode`].
fn decode(archived: isize) -> isize {
    match archived {
        ARCHIVED_MAX => isize::MAX,
        ARCHIVED_MIN => isize::MIN,
        e => e,
    }
}

impl ArchiveWith<isize> for Exponent {
    type Archived = Archived<isize>;
    type Resolver = ();

    #[inline]
    unsafe fn resolve_with(
        field: &isize,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        // rkyv 0.7 gives a field wrapper no way to report an error, so an exponent that cannot
        // be archived is a panic rather than a silently different number.
        let exponent =
            encode(*field).expect("the exponent is too large for the archived representation");
        exponent.resolve(pos, resolver, out)
    }
}

impl<S: Fallible + ?Sized> SerializeWith<isize, S> for Exponent {
    #[inline]
    fn serialize_with(_: &isize, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<D: Fallible + ?Sized> DeserializeWith<Archived<isize>, isize, D> for Exponent {
    #[inline]
    fn deserialize_with(field: &Archived<isize>, deserializer: &mut D) -> Result<isize, D::Error> {
        let archived: isize = field.deserialize(deserializer)?;
        Ok(decode(archived))
    }
}

/// Archives the precision of a [`Context`][crate::Context].
///
/// An archived `usize` has a fixed width that can be narrower than the native one, and the
/// plain conversion truncates: a precision of `2^32 + 5` would come back as 5, and `2^32` as 0,
/// which means *unlimited*. This wrapper keeps the archived layout of a `usize`, but refuses a
/// precision that does not fit instead of truncating it.
pub struct Precision;

/// Whether the archived representation can hold the precision.
fn precision_fits(precision: usize) -> bool {
    precision as u128 <= FixedUsize::MAX as u128
}

impl ArchiveWith<usize> for Precision {
    type Archived = Archived<usize>;
    type Resolver = ();

    #[inline]
    unsafe fn resolve_with(
        field: &usize,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        // rkyv 0.7 gives a field wrapper no way to report an error, so a precision that cannot
        // be archived is a panic rather than a silently different precision.
        assert!(
            precision_fits(*field),
            "the precision is too large for the archived representation"
        );
        field.resolve(pos, resolver, out)
    }
}

impl<S: Fallible + ?Sized> SerializeWith<usize, S> for Precision {
    #[inline]
    fn serialize_with(_: &usize, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<D: Fallible + ?Sized> DeserializeWith<Archived<usize>, usize, D> for Precision {
    #[inline]
    fn deserialize_with(field: &Archived<usize>, deserializer: &mut D) -> Result<usize, D::Error> {
        field.deserialize(deserializer)
    }
}
