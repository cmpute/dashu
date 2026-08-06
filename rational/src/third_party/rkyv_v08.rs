//! rkyv 0.8 test-only module: the `Archive`/`Serialize`/`Deserialize` impls live on the derive
//! (`cfg_attr` on `Repr`/`RBig`/`Relaxed`); this module only holds the round-trip tests. Gated on
//! `all(rkyv_v08, not(rkyv_v07))` because the two versions' derive-generated type names collide.

#[cfg(test)]
mod tests {
    use crate::{RBig, Relaxed};
    use dashu_int::{IBig, UBig};
    use rkyv_v08::{from_bytes_unchecked, to_bytes};

    #[test]
    fn rbig_roundtrip() {
        let values = [
            RBig::from_parts(0u8.into(), 1u8.into()),
            RBig::from_parts(1u8.into(), 2u8.into()),
            RBig::from_parts((-7i8).into(), 3u8.into()),
            RBig::from_parts(123456789i64.into(), 987654321u64.into()),
            RBig::from_parts(IBig::ONE << 200, UBig::from(5u8)),
        ];
        for r in &values {
            let bytes = to_bytes::<rkyv_v08::rancor::Error>(r).unwrap();
            let r2 =
                unsafe { from_bytes_unchecked::<RBig, rkyv_v08::rancor::Error>(&bytes) }.unwrap();
            assert_eq!(r2, *r, "RBig round-trip failed for {r}");
        }
    }

    #[test]
    fn relaxed_roundtrip() {
        let values = [
            Relaxed::from_parts(6u8.into(), 9u8.into()),
            Relaxed::from_parts((-2i8).into(), 4u8.into()),
            Relaxed::from_parts(0u8.into(), 1u8.into()),
        ];
        for r in &values {
            let bytes = to_bytes::<rkyv_v08::rancor::Error>(r).unwrap();

            // SAFETY: `bytes` came from `rkyv::to_bytes` on the same value, so it is a valid

            // archived object with the root at the end.

            let r2 = unsafe { from_bytes_unchecked::<Relaxed, rkyv_v08::rancor::Error>(&bytes) }
                .unwrap();
            assert_eq!(r2, *r, "Relaxed round-trip failed for {r}");
        }
    }
}
