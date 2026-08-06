use dashu_cmplx::CBig;
use dashu_float::round::mode::HalfEven;
use dashu_float::FBig;
use rkyv_v07::{from_bytes_unchecked, to_bytes};

type C = CBig<HalfEven, 10>;
type F = FBig<HalfEven, 10>;

fn c(re: &str, im: &str) -> C {
    CBig::from_parts(re.parse().unwrap(), im.parse().unwrap())
}

#[test]
fn cbig_rkyv_roundtrip() {
    let values = [
        c("0", "0"),
        c("1.5", "-2.25"),
        c("3.14159", "-2.5e100"),
        CBig::from_parts(F::from(123u16), F::from(-7i8)),
    ];
    for z in &values {
        let bytes = to_bytes::<_, 1024>(z).unwrap();
        // SAFETY: `bytes` came from `rkyv::to_bytes` on the same value, so it is a valid
        // archived object with the root at the end.
        let z2 = unsafe { from_bytes_unchecked::<C>(&bytes) }.unwrap();
        assert_eq!(z2, *z, "CBig round-trip failed for {z}");
        assert_eq!(z2.precision(), z.precision(), "CBig precision round-trip failed");
    }
}
