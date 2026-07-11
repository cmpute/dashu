use dashu_float::{DBig, Repr};
use postcard::{from_bytes, to_allocvec};
use serde_json::{from_str, to_string};

mod helper_macros;
type FBig = dashu_float::FBig;

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_fbig_serde() {
    let test_numbers = [
        fbig!(0),
        fbig!(0x1dp-1),
        fbig!(-0x23p-1),
        fbig!(0x1234p-2),
        fbig!(-0x1234567890123456789p-40),
        fbig!(-0x123456789012345678901234567890123456789p-200),
    ];
    for float in &test_numbers {
        // test binary serialization
        let output = to_allocvec(float).unwrap();
        let parsed: FBig = from_bytes(&output).unwrap();
        assert_eq!(&parsed, float);

        // test binary serialization of repr
        let output = to_allocvec(float.repr()).unwrap();
        let parsed: Repr<2> = from_bytes(&output).unwrap();
        assert_eq!(&parsed, float.repr());

        // test string serialization
        let output = to_string(float).unwrap();
        let parsed: FBig = from_str(&output).unwrap();
        assert_eq!(&parsed, float);

        // test string serialization of repr
        let output = to_string(float.repr()).unwrap();
        let parsed: Repr<2> = from_str(&output).unwrap();
        assert_eq!(&parsed, float.repr());
    }
}

#[test]
fn test_dbig_serde() {
    let test_numbers = [
        dbig!(0),
        dbig!(1.3),
        dbig!(-2.3),
        dbig!(10.99),
        dbig!(-123456789.0123456789),
        dbig!(-1.2345678901234567890123456789e-100),
    ];
    for float in &test_numbers {
        // test binary serialization
        let output = to_allocvec(float).unwrap();
        let parsed: DBig = from_bytes(&output).unwrap();
        assert_eq!(&parsed, float);

        // test binary serialization of repr
        let output = to_allocvec(float.repr()).unwrap();
        let parsed: Repr<10> = from_bytes(&output).unwrap();
        assert_eq!(&parsed, float.repr());

        // test string serialization
        let output = to_string(float).unwrap();
        let parsed: DBig = from_str(&output).unwrap();
        assert_eq!(&parsed, float);

        // test string serialization of repr
        let output = to_string(float.repr()).unwrap();
        let parsed: Repr<10> = from_str(&output).unwrap();
        assert_eq!(&parsed, float.repr());
    }
}

#[test]
fn test_dbig_serde_preserves_precision() {
    use core::str::FromStr;
    // values whose significant-digit count is below the context precision: the
    // human-readable format must round-trip the *precision*, not just the value.
    let cases: &[(DBig, usize)] = &[
        (DBig::from_str("1.23").unwrap().with_precision(7).unwrap(), 7),
        (DBig::from_str("100").unwrap().with_precision(5).unwrap(), 5),
        (DBig::from_str("-0.5").unwrap().with_precision(3).unwrap(), 3),
    ];
    for (value, expected_prec) in cases {
        // human-readable (json string) preserves precision
        let json = to_string(value).unwrap();
        let parsed: DBig = from_str(&json).unwrap();
        assert_eq!(parsed.precision(), *expected_prec, "human-readable precision");
        assert_eq!(&parsed, value);

        // non-human-readable (postcard bytes) preserves precision
        let bytes = to_allocvec(value).unwrap();
        let parsed: DBig = from_bytes(&bytes).unwrap();
        assert_eq!(parsed.precision(), *expected_prec, "binary precision");
    }
}
