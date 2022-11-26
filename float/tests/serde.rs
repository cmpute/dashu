use dashu_float::{DBig, Repr};
use postcard::{from_bytes, to_allocvec};
use serde_json::{from_str, to_string};

mod helper_macros;
type FBig = dashu_float::FBig;

#[test]
fn test_fbig_serde() {
    let test_numbers = [
        fbig!(0),
        fbig!(0x1dp-1),
        fbig!(-0x23p-1),
        fbig!(0x1234p-2),
        fbig!(-0x1234567890123456789p-40),
        fbig!(-0x123456789012345678901234567890123456789p-200),
    ];
    for ratio in &test_numbers {
        // test binary serialization
        let output = to_allocvec(ratio).unwrap();
        let parsed: FBig = from_bytes(&output).unwrap();
        assert_eq!(&parsed, ratio);

        // test binary serialization of repr
        let output = to_allocvec(ratio.repr()).unwrap();
        let parsed: Repr<2> = from_bytes(&output).unwrap();
        assert_eq!(&parsed, ratio.repr());

        // test string serialization
        let output = to_string(ratio).unwrap();
        let parsed: FBig = from_str(&output).unwrap();
        assert_eq!(&parsed, ratio);

        // test string serialization of repr
        let output = to_string(ratio.repr()).unwrap();
        let parsed: Repr<2> = from_str(&output).unwrap();
        assert_eq!(&parsed, ratio.repr());
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
    for ratio in &test_numbers {
        // test binary serialization
        let output = to_allocvec(ratio).unwrap();
        let parsed: DBig = from_bytes(&output).unwrap();
        assert_eq!(&parsed, ratio);

        // test binary serialization of repr
        let output = to_allocvec(ratio.repr()).unwrap();
        let parsed: Repr<10> = from_bytes(&output).unwrap();
        assert_eq!(&parsed, ratio.repr());

        // test string serialization
        let output = to_string(ratio).unwrap();
        let parsed: DBig = from_str(&output).unwrap();
        assert_eq!(&parsed, ratio);

        // test string serialization of repr
        let output = to_string(ratio.repr()).unwrap();
        let parsed: Repr<10> = from_str(&output).unwrap();
        assert_eq!(&parsed, ratio.repr());
    }
}
