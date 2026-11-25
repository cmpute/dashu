//! rkyv 0.8 support: the `Archive`/`Serialize`/`Deserialize` impls live on the derive (`cfg_attr`
//! on `Repr`/`Context`/`FBig`); this module holds the field wrappers for the exponent and the
//! precision, and the round-trip tests. Gated on `all(rkyv_v08, not(rkyv_v07))` because the two versions'
//! derive-generated type names collide.

use core::fmt;
use rkyv_v08::{
    primitive::{FixedIsize, FixedUsize},
    rancor::{Fallible, Source},
    with::{ArchiveWith, DeserializeWith, SerializeWith},
    Archive, Archived, Deserialize, Place,
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

/// The error for an exponent that the archived representation cannot hold.
#[derive(Debug)]
struct ExponentOutOfRange;

impl fmt::Display for ExponentOutOfRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the exponent is too large for the archived representation")
    }
}

impl core::error::Error for ExponentOutOfRange {}

impl ArchiveWith<isize> for Exponent {
    type Archived = Archived<isize>;
    type Resolver = ();

    #[inline]
    fn resolve_with(field: &isize, resolver: Self::Resolver, out: Place<Self::Archived>) {
        // `serialize_with` has rejected the exponents that cannot be archived
        let exponent = encode(*field).unwrap_or(if *field > 0 {
            ARCHIVED_MAX
        } else {
            ARCHIVED_MIN
        });
        exponent.resolve(resolver, out)
    }
}

impl<S: Fallible + ?Sized> SerializeWith<isize, S> for Exponent
where
    S::Error: Source,
{
    #[inline]
    fn serialize_with(field: &isize, _: &mut S) -> Result<Self::Resolver, S::Error> {
        match encode(*field) {
            Some(_) => Ok(()),
            None => Err(<S::Error as Source>::new(ExponentOutOfRange)),
        }
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

/// The error for a precision that the archived representation cannot hold.
#[derive(Debug)]
struct PrecisionOutOfRange;

impl fmt::Display for PrecisionOutOfRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the precision is too large for the archived representation")
    }
}

impl core::error::Error for PrecisionOutOfRange {}

impl ArchiveWith<usize> for Precision {
    type Archived = Archived<usize>;
    type Resolver = ();

    #[inline]
    fn resolve_with(field: &usize, resolver: Self::Resolver, out: Place<Self::Archived>) {
        // `serialize_with` has rejected the precisions that cannot be archived
        let precision = if precision_fits(*field) {
            *field
        } else {
            FixedUsize::MAX as usize
        };
        precision.resolve(resolver, out)
    }
}

impl<S: Fallible + ?Sized> SerializeWith<usize, S> for Precision
where
    S::Error: Source,
{
    #[inline]
    fn serialize_with(field: &usize, _: &mut S) -> Result<Self::Resolver, S::Error> {
        if precision_fits(*field) {
            Ok(())
        } else {
            Err(<S::Error as Source>::new(PrecisionOutOfRange))
        }
    }
}

impl<D: Fallible + ?Sized> DeserializeWith<Archived<usize>, usize, D> for Precision {
    #[inline]
    fn deserialize_with(field: &Archived<usize>, deserializer: &mut D) -> Result<usize, D::Error> {
        field.deserialize(deserializer)
    }
}

#[cfg(test)]
mod tests {
    use crate::round::mode::HalfEven;
    use crate::{DBig, FBig, Repr};
    use rkyv_v08::{from_bytes_unchecked, to_bytes};

    type F = FBig<HalfEven, 2>;

    #[test]
    fn fbig_roundtrip() {
        let values = [
            F::from(0u8).with_precision(10).value(),
            F::from(1u8).with_precision(10).value(),
            F::from(-1i8).with_precision(10).value(),
            (F::from(3u8) << 100).with_precision(20).value(),
            F::try_from(1.5f64).unwrap().with_precision(30).value(),
        ];
        for v in &values {
            let bytes = to_bytes::<rkyv_v08::rancor::Error>(v).unwrap();

            // SAFETY: `bytes` came from `rkyv::to_bytes` on the same value, so it is a valid

            // archived object with the root at the end.

            let v2 = unsafe { from_bytes_unchecked::<F, rkyv_v08::rancor::Error>(&bytes) }.unwrap();
            assert_eq!(v2, *v, "FBig round-trip failed for {v}");
            assert_eq!(v2.precision(), v.precision());
        }
    }

    #[test]
    fn repr_roundtrip() {
        let values = [
            F::from(0u8).with_precision(10).value().repr().clone(),
            F::from(1u8).with_precision(10).value().repr().clone(),
            (F::from(3u8) << 100)
                .with_precision(20)
                .value()
                .repr()
                .clone(),
            F::try_from(1.5f64)
                .unwrap()
                .with_precision(30)
                .value()
                .repr()
                .clone(),
        ];
        for r in &values {
            let bytes = to_bytes::<rkyv_v08::rancor::Error>(r).unwrap();

            // SAFETY: `bytes` came from `rkyv::to_bytes` on the same value, so it is a valid

            // archived object with the root at the end.

            let r2 = unsafe { from_bytes_unchecked::<Repr<2>, rkyv_v08::rancor::Error>(&bytes) }
                .unwrap();
            assert_eq!(r2, *r, "Repr round-trip failed for {r}");
        }
    }

    #[test]
    fn special_values_roundtrip() {
        // The infinities are marked by the exponents `isize::MAX` / `isize::MIN`, which do not
        // fit the (narrower) archived exponent as they are, and the zeros by `0` / `-1`.
        let values = [
            F::INFINITY,
            F::NEG_INFINITY,
            F::INFINITY.with_precision(10).value(),
            F::ZERO,
            -F::ZERO,
        ];
        for v in &values {
            let bytes = to_bytes::<rkyv_v08::rancor::Error>(v).unwrap();
            // SAFETY: `bytes` came from `rkyv::to_bytes` on the same value.
            let v2 = unsafe { from_bytes_unchecked::<F, rkyv_v08::rancor::Error>(&bytes) }.unwrap();
            assert_eq!(v2.repr().exponent(), v.repr().exponent(), "{:?}", v.repr());
            assert_eq!(v2.repr(), v.repr());
            assert_eq!(v2.precision(), v.precision());
        }
        for v in [DBig::INFINITY, DBig::NEG_INFINITY] {
            let bytes = to_bytes::<rkyv_v08::rancor::Error>(&v).unwrap();
            // SAFETY: `bytes` came from `rkyv::to_bytes` on the same value.
            let v2 =
                unsafe { from_bytes_unchecked::<DBig, rkyv_v08::rancor::Error>(&bytes) }.unwrap();
            assert_eq!(v2, v);
            assert!(v2.repr().is_infinite());
        }
        let reprs = [
            Repr::<2>::infinity(),
            Repr::<2>::neg_infinity(),
            Repr::<2>::zero(),
            Repr::<2>::neg_zero(),
        ];
        for r in &reprs {
            let bytes = to_bytes::<rkyv_v08::rancor::Error>(r).unwrap();
            // SAFETY: `bytes` came from `rkyv::to_bytes` on the same value.
            let r2 = unsafe { from_bytes_unchecked::<Repr<2>, rkyv_v08::rancor::Error>(&bytes) }
                .unwrap();
            assert_eq!(r2.exponent(), r.exponent(), "{r:?}");
            assert_eq!(r2, *r);
        }
    }

    #[test]
    fn exponent_range() {
        use super::{ARCHIVED_MAX, ARCHIVED_MIN};

        // every exponent strictly inside the archived range round-trips
        for exponent in [ARCHIVED_MAX - 1, ARCHIVED_MIN + 1, 1 << 20, -(1 << 20)] {
            let v = F::from_parts(3.into(), exponent);
            let bytes = to_bytes::<rkyv_v08::rancor::Error>(&v).unwrap();
            // SAFETY: `bytes` came from `rkyv::to_bytes` on the same value.
            let v2 = unsafe { from_bytes_unchecked::<F, rkyv_v08::rancor::Error>(&bytes) }.unwrap();
            assert_eq!(v2.repr().exponent(), exponent);
            assert_eq!(v2, v);
        }

        // An exponent outside of it is refused instead of being truncated (it used to come
        // back as a different number). The archived exponent is narrower than `isize` only
        // on 64-bit targets.
        #[cfg(target_pointer_width = "64")]
        for exponent in [
            ARCHIVED_MAX,
            ARCHIVED_MAX + 1,
            ARCHIVED_MIN,
            ARCHIVED_MIN - 1,
            isize::MAX - 1,
            isize::MIN + 1,
        ] {
            let v = F::from_parts(3.into(), exponent);
            assert!(to_bytes::<rkyv_v08::rancor::Error>(&v).is_err(), "{exponent}");
            assert!(to_bytes::<rkyv_v08::rancor::Error>(v.repr()).is_err(), "{exponent}");
        }
    }

    #[test]
    fn precision_range() {
        // every precision that the archived (32-bit) representation holds round-trips
        for precision in [0usize, 1, 5, 1 << 20, u32::MAX as usize] {
            let v = DBig::from(3).with_precision(precision).value();
            let bytes = to_bytes::<rkyv_v08::rancor::Error>(&v).unwrap();
            // SAFETY: `bytes` came from `rkyv::to_bytes` on the same value.
            let v2 =
                unsafe { from_bytes_unchecked::<DBig, rkyv_v08::rancor::Error>(&bytes) }.unwrap();
            assert_eq!(v2.precision(), precision);
            assert_eq!(v2.repr(), v.repr());
        }

        // A larger one is refused instead of being truncated: 2^32 + 5 used to come back as
        // 5, and 2^32 as 0, which means unlimited.
        #[cfg(target_pointer_width = "64")]
        for precision in [1usize << 32, (1 << 32) + 5, usize::MAX] {
            let v = DBig::from(3).with_precision(precision).value();
            assert_eq!(v.precision(), precision);
            assert!(to_bytes::<rkyv_v08::rancor::Error>(&v).is_err(), "{precision}");
            let v = F::from(3u8).with_precision(precision).value();
            assert!(to_bytes::<rkyv_v08::rancor::Error>(&v).is_err(), "{precision}");
        }
    }

    #[test]
    fn dbig_roundtrip() {
        let values = [
            DBig::from(0u8).with_precision(10).value(),
            DBig::from(1u8).with_precision(10).value(),
            ("3.14159".parse::<DBig>().unwrap())
                .with_precision(30)
                .value(),
            ("-2.5e100".parse::<DBig>().unwrap())
                .with_precision(64)
                .value(),
        ];
        for v in &values {
            let bytes = to_bytes::<rkyv_v08::rancor::Error>(v).unwrap();
            // SAFETY: `bytes` came from `rkyv::to_bytes` on the same value.
            let v2 =
                unsafe { from_bytes_unchecked::<DBig, rkyv_v08::rancor::Error>(&bytes) }.unwrap();
            assert_eq!(v2, *v, "DBig round-trip failed for {v}");
        }
    }
}
