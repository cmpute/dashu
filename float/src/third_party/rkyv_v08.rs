//! rkyv 0.8 test-only module: the `Archive`/`Serialize`/`Deserialize` impls live on the derive
//! (`cfg_attr` on `Repr`/`Context`/`FBig`); this module only holds the round-trip tests. Gated on
//! `all(rkyv_v08, not(rkyv_v07))` because the two versions' derive-generated type names collide.

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
            let v2 =
                unsafe { from_bytes_unchecked::<DBig, rkyv_v08::rancor::Error>(&bytes) }.unwrap();
            assert_eq!(v2, *v, "DBig round-trip failed for {v}");
        }
    }
}
