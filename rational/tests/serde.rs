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

#[test]
fn test_serde_struct_field_names() {
    use serde_test::{assert_de_tokens, assert_de_tokens_error, Configure, Token};

    // A format may hand over the field names borrowed, transient, owned or as bytes.
    let keys: [fn(&'static str) -> Token; 4] =
        [Token::BorrowedStr, Token::Str, Token::String, |s| {
            Token::Bytes(s.as_bytes())
        }];
    for key in keys {
        assert_de_tokens(
            &rbig!(-1 / 3).compact(),
            &[
                Token::Struct {
                    name: "RBig",
                    len: 2,
                },
                key("numerator"),
                Token::Bytes(&[0xff]),
                key("denominator"),
                Token::Bytes(&[3]),
                Token::StructEnd,
            ],
        );
        assert_de_tokens(
            &rbig!(~2/6).compact(),
            &[
                Token::Struct {
                    name: "Relaxed",
                    len: 2,
                },
                key("denominator"),
                Token::Bytes(&[6]),
                key("numerator"),
                Token::Bytes(&[2]),
                Token::StructEnd,
            ],
        );
    }

    assert_de_tokens_error::<serde_test::Compact<RBig>>(
        &[
            Token::Struct {
                name: "RBig",
                len: 2,
            },
            Token::String("numer"),
        ],
        "unknown field `numer`, expected `numerator` or `denominator`",
    );
    assert_de_tokens_error::<serde_test::Compact<RBig>>(
        &[
            Token::Struct {
                name: "RBig",
                len: 2,
            },
            Token::String("denominator"),
            Token::Bytes(&[3]),
            Token::String("denominator"),
        ],
        "duplicate field `denominator`",
    );
}

#[test]
fn test_serde_zero_denominator() {
    use dashu_int::{IBig, UBig};
    use serde_test::{assert_de_tokens_error, Token};

    // the string form goes through the parser
    assert!(from_str::<RBig>("\"3/0\"").is_err());
    assert!(from_str::<Relaxed>("\"3/0\"").is_err());

    // the struct form of the binary formats carries the two integers as they are
    for num in [3, -4, 0] {
        let bytes = to_allocvec(&(IBig::from(num), UBig::ZERO)).unwrap();
        assert!(from_bytes::<RBig>(&bytes).is_err(), "{}/0 was accepted", num);
        assert!(from_bytes::<Relaxed>(&bytes).is_err(), "{}/0 was accepted", num);
    }
    let bytes = to_allocvec(&(IBig::from(6), UBig::from(4u8))).unwrap();
    assert_eq!(from_bytes::<RBig>(&bytes).unwrap(), rbig!(3 / 2));
    assert_eq!(from_bytes::<Relaxed>(&bytes).unwrap(), rbig!(~3/2));

    // also when the fields are given by name
    let message = "invalid value: a zero denominator, \
                   expected a rational number with a nonzero denominator";
    assert_de_tokens_error::<serde_test::Compact<RBig>>(
        &[
            Token::Struct {
                name: "RBig",
                len: 2,
            },
            Token::Str("numerator"),
            Token::Bytes(&[3]),
            Token::Str("denominator"),
            Token::Bytes(&[]),
            Token::StructEnd,
        ],
        message,
    );
    assert_de_tokens_error::<serde_test::Compact<Relaxed>>(
        &[
            Token::Struct {
                name: "Relaxed",
                len: 2,
            },
            Token::Str("denominator"),
            Token::Bytes(&[]),
            Token::Str("numerator"),
            Token::Bytes(&[3]),
            Token::StructEnd,
        ],
        message,
    );
}
