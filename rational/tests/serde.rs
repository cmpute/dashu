use dashu_ratio::{RBig, Relaxed};
use postcard::{from_bytes, to_allocvec};
use serde_json::{from_str, to_string};

mod helper_macros;

#[test]
fn test_rbig_serde() {
    let test_numbers = [
        rbig!(0),
        rbig!(1 / 3),
        rbig!(-2 / 3),
        rbig!(10 / 99),
        rbig!(-0xffffffffffffffff / 0xfffffffffffffffe),
        rbig!(-0xfffffffffffffffffffffffffffffffe / 0xffffffffffffffffffffffffffffffff),
    ];
    for ratio in &test_numbers {
        // test binary serialization
        let output = to_allocvec(ratio).unwrap();
        let parsed: RBig = from_bytes(&output).unwrap();
        assert_eq!(&parsed, ratio);

        // test string serialization
        let output = to_string(ratio).unwrap();
        let parsed: RBig = from_str(&output).unwrap();
        assert_eq!(&parsed, ratio);
    }
}

#[test]
fn test_relaxed_serde() {
    let test_numbers = [
        rbig!(~0),
        rbig!(~1/3),
        rbig!(~-2/3),
        rbig!(~33/99),
        rbig!(~-0xffffffffffffffff/0xfffffffffffffffe),
        rbig!(~-0xfffffffffffffffffffffffffffffffe/0xffffffffffffffffffffffffffffffff),
    ];
    for ratio in &test_numbers {
        // test binary serialization
        let output = to_allocvec(ratio).unwrap();
        let parsed: Relaxed = from_bytes(&output).unwrap();
        assert_eq!(&parsed, ratio);

        // test string serialization
        let output = to_string(ratio).unwrap();
        let parsed: Relaxed = from_str(&output).unwrap();
        assert_eq!(&parsed, ratio);
    }
}
