use dashu_float::round::mode::HalfEven;
use dashu_float::{DBig, FBig, Repr};
use rkyv_v07::{from_bytes_unchecked, to_bytes};

type F = FBig<HalfEven, 2>;

#[test]
fn repr_rkyv_roundtrip() {
    // `Repr` archives independently of `FBig` (mirroring the serde coverage of both).
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
        F::try_from(-123.456e-50f64)
            .unwrap()
            .with_precision(64)
            .value()
            .repr()
            .clone(),
    ];
    for r in &values {
        let bytes = to_bytes::<_, 1024>(r).unwrap();
        // SAFETY: `bytes` came from `rkyv::to_bytes` on the same value, so it is a valid
        // archived object with the root at the end.
        let r2 = unsafe { from_bytes_unchecked::<Repr<2>>(&bytes) }.unwrap();
        assert_eq!(r2, *r, "Repr round-trip failed for {r}");
    }
}

#[test]
fn fbig_rkyv_roundtrip() {
    // Values spanning the specials, signs, exponent range, and precision.
    let values = [
        F::from(0u8).with_precision(10).value(),
        F::from(1u8).with_precision(10).value(),
        F::from(-1i8).with_precision(10).value(),
        (F::from(3u8) << 100).with_precision(20).value(),
        F::try_from(1.5f64).unwrap().with_precision(30).value(),
        F::try_from(-123.456e-50f64)
            .unwrap()
            .with_precision(64)
            .value(),
    ];
    for v in &values {
        let bytes = to_bytes::<_, 1024>(v).unwrap();
        // SAFETY: `bytes` came from `rkyv::to_bytes` on the same value, so it is a valid
        // archived object with the root at the end.
        let v2 = unsafe { from_bytes_unchecked::<F>(&bytes) }.unwrap();
        assert_eq!(v2, *v, "FBig value round-trip failed for {v}");
        assert_eq!(v2.precision(), v.precision(), "FBig precision round-trip failed");
    }
}

#[test]
fn dbig_rkyv_roundtrip() {
    let values = [
        DBig::from(0u8).with_precision(10).value(),
        DBig::from(1u8).with_precision(10).value(),
        DBig::from(-1i8).with_precision(10).value(),
        ("3.14159".parse::<DBig>().unwrap())
            .with_precision(30)
            .value(),
        ("-2.5e100".parse::<DBig>().unwrap())
            .with_precision(64)
            .value(),
    ];
    for v in &values {
        let bytes = to_bytes::<_, 1024>(v).unwrap();
        // SAFETY: `bytes` came from `rkyv::to_bytes` on the same value, so it is a valid
        // archived object with the root at the end.
        let v2 = unsafe { from_bytes_unchecked::<DBig>(&bytes) }.unwrap();
        assert_eq!(v2, *v, "DBig value round-trip failed for {v}");
        assert_eq!(v2.precision(), v.precision(), "DBig precision round-trip failed");
    }
}
