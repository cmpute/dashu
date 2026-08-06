use dashu_int::{IBig, UBig};
use dashu_ratio::{RBig, Relaxed};
use rkyv_v07::{from_bytes_unchecked, to_bytes};

#[test]
fn rbig_rkyv_roundtrip() {
    let values = [
        RBig::from_parts(0u8.into(), 1u8.into()),
        RBig::from_parts(1u8.into(), 2u8.into()),
        RBig::from_parts((-7i8).into(), 3u8.into()),
        RBig::from_parts(123456789i64.into(), 987654321u64.into()),
        RBig::from_parts(IBig::ONE << 200, UBig::from(5u8)),
    ];
    for r in &values {
        let bytes = to_bytes::<_, 1024>(r).unwrap();
        let r2 = unsafe { from_bytes_unchecked::<RBig>(&bytes) }.unwrap();
        assert_eq!(r2, *r, "RBig round-trip failed for {r}");
    }
}

#[test]
fn relaxed_rkyv_roundtrip() {
    // A non-canonical Relaxed (common factor 3) must round-trip exactly (the archive preserves
    // the unreduced representation, matching how Relaxed preserves it in memory).
    let values = [
        Relaxed::from_parts(6u8.into(), 9u8.into()),
        Relaxed::from_parts((-2i8).into(), 4u8.into()),
        Relaxed::from_parts(0u8.into(), 1u8.into()),
    ];
    for r in &values {
        let bytes = to_bytes::<_, 1024>(r).unwrap();
        let r2 = unsafe { from_bytes_unchecked::<Relaxed>(&bytes) }.unwrap();
        assert_eq!(r2, *r, "Relaxed round-trip failed for {r}");
    }
}
