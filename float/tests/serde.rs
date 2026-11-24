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

/// Round trip through the human-readable (json) and binary (postcard) formats, checking the
/// value, the sign of zero, and the precision. Returns the json text.
fn check_roundtrip<R: dashu_float::round::Round, const B: dashu_int::Word>(
    value: &dashu_float::FBig<R, B>,
) -> String {
    let json = to_string(value).unwrap();
    let parsed: dashu_float::FBig<R, B> = from_str(&json).unwrap();
    assert_eq!(parsed.repr(), value.repr(), "value through {json}");
    assert_eq!(parsed.precision(), value.precision(), "precision through {json}");

    let bytes = to_allocvec(value).unwrap();
    let parsed: dashu_float::FBig<R, B> = from_bytes(&bytes).unwrap();
    assert_eq!(parsed.repr(), value.repr());
    assert_eq!(parsed.precision(), value.precision());
    json
}

#[test]
fn test_serde_preserves_precision_exactly() {
    use core::str::FromStr;
    use dashu_float::round::mode::{HalfAway, Zero};
    type FHex = dashu_float::FBig<Zero, 16>;
    type FBinAway = dashu_float::FBig<HalfAway, 2>;

    // The literal has exactly `precision` digits, also when the zeros of the positional form
    // alone would exceed it (the integer 1000 or the fraction 0.5 at precision 1).
    let d = |s: &str, p: usize| DBig::from_str(s).unwrap().with_precision(p).unwrap();
    let cases = [
        (d("100", 5), "\"100.00\""),
        (d("10", 2), "\"10\""),
        (d("10", 1), "\"1@1\""),
        (d("1000", 1), "\"1@3\""),
        (d("1000", 2), "\"1.0@3\""),
        (d("0.5", 3), "\"0.50\""),
        (d("0.5", 2), "\"0.5\""),
        (d("0.5", 1), "\"5@-1\""),
        (d("-0.5", 1), "\"-5@-1\""),
        (d("0.1", 1), "\"1@-1\""),
        (d("0.05", 2), "\"5.0@-2\""),
        (d("0.33", 2), "\"33@-2\""),
        (d("0.33", 3), "\"0.33\""),
        (d("1", 1), "\"1\""),
        (d("0", 1), "\"0\""),
        (d("-0", 3), "\"-0.00\""),
        (d("1e30", 3), "\"1.00@30\""),
        (
            d("-1.2345678901234567890123456789e-100", 29),
            "\"-12345678901234567890123456789@-128\"",
        ),
    ];
    for (value, expected) in &cases {
        assert_eq!(&check_roundtrip(value), expected);
    }

    // digits above 9 count as digits too
    for (s, expected) in [
        ("7", "\"7\""),
        ("a", "\"a\""),
        ("ff", "\"ff\""),
        ("1.8", "\"1.8\""),
    ] {
        let x = FHex::from_str(s).unwrap();
        assert_eq!(check_roundtrip(&x), expected);
    }
    for s in ["-c.d", "1f", "0.0a", "a000", "abcdef.123456"] {
        let x = FHex::from_str(s).unwrap();
        check_roundtrip(&x);
        check_roundtrip(&x.clone().with_precision(20).unwrap());
    }
    // a repeated round trip is stable
    let mut z = FHex::from_str("a").unwrap();
    for _ in 0..3 {
        z = from_str(&to_string(&z).unwrap()).unwrap();
        assert_eq!(z.precision(), 1);
    }

    for (signif, exp, p) in [(1, -1, 1), (1, -2, 2), (1, 0, 1), (1, 5, 1), (-5, -1, 3)] {
        check_roundtrip(
            &FBinAway::from_parts(signif.into(), exp)
                .with_precision(p)
                .unwrap(),
        );
    }

    // sweep small values over every precision from the digit count upwards, and at the
    // reference precisions
    for signif in [0i32, 1, -1, 7, 10, 12, -95, 123, 1001, 99999] {
        for exp in -7..=7isize {
            let x = DBig::from_parts(signif.into(), exp);
            let digits = x.repr().digits().max(1);
            for p in (digits..digits + 9).chain([20, 50, 100, 500]) {
                check_roundtrip(&x.clone().with_precision(p).unwrap());
            }
            let x = FBinAway::from_parts(signif.into(), exp);
            let digits = x.repr().digits().max(1);
            for p in (digits..digits + 9).chain([20, 50, 100, 500]) {
                check_roundtrip(&x.clone().with_precision(p).unwrap());
            }
            let x = FHex::from_parts(signif.into(), exp);
            let digits = x.repr().digits().max(1);
            for p in (digits..digits + 9).chain([20, 50, 100, 500]) {
                check_roundtrip(&x.clone().with_precision(p).unwrap());
            }
        }
    }

    // an exponent far outside what the positional form could write out
    let tiny = DBig::from_str("1.5e-9223372036854775000").unwrap();
    assert_eq!(check_roundtrip(&tiny), "\"15@-9223372036854775001\"");
    let huge = DBig::from_str("1.5e9223372036854775000").unwrap();
    assert_eq!(check_roundtrip(&huge), "\"15@9223372036854774999\"");
}

#[test]
fn test_serde_precision_without_literal() {
    use core::str::FromStr;

    // No literal has zero digits, so an unlimited precision is carried by the struct form.
    let unlimited = [
        DBig::ZERO,
        DBig::ONE,
        DBig::NEG_ONE,
        DBig::from_parts(123.into(), 0).with_precision(0).value(),
        DBig::from_parts((-5).into(), -3).with_precision(0).value(),
        DBig::from_str("-0").unwrap().with_precision(0).value(),
    ];
    for value in &unlimited {
        assert_eq!(value.precision(), 0);
        check_roundtrip(value);
    }
    assert_eq!(
        to_string(&DBig::ONE).unwrap(),
        r#"{"significand":"1","exponent":0,"precision":0}"#
    );
    for value in [
        FBig::ZERO,
        FBig::ONE,
        FBig::from_parts(5.into(), -1).with_precision(0).value(),
    ] {
        assert_eq!(value.precision(), 0);
        check_roundtrip(&value);
    }

    // downstream arithmetic is unchanged by the round trip
    let y = DBig::from_parts(123.into(), 0).with_precision(0).value();
    let back: DBig = from_str(&to_string(&DBig::ONE).unwrap()).unwrap();
    assert_eq!(&back + &y, DBig::from_parts(124.into(), 0));

    // an inexact sum can carry a guard digit beyond its precision
    let guarded = DBig::from_str("0.1").unwrap() - DBig::from_str("11").unwrap();
    assert_eq!(guarded.precision(), 2);
    assert_eq!(guarded.repr().digits(), 3);
    assert_eq!(
        check_roundtrip(&guarded),
        r#"{"significand":"-109","exponent":-1,"precision":2}"#
    );

    // plain literals keep being accepted, with the precision their digits imply
    let parsed: DBig = from_str("\"1\"").unwrap();
    assert_eq!(parsed, DBig::ONE);
    assert_eq!(parsed.precision(), 1);
}

#[test]
fn test_serde_infinity() {
    assert_eq!(to_string(&Repr::<10>::infinity()).unwrap(), "\"inf\"");
    assert_eq!(to_string(&Repr::<2>::neg_infinity()).unwrap(), "\"-inf\"");
    let r: Repr<10> = from_str("\"inf\"").unwrap();
    assert_eq!(r, Repr::<10>::infinity());
    let r: Repr<2> = from_str("\"-inf\"").unwrap();
    assert_eq!(r, Repr::<2>::neg_infinity());

    assert_eq!(check_roundtrip(&DBig::INFINITY), "\"inf\"");
    assert_eq!(check_roundtrip(&DBig::NEG_INFINITY), "\"-inf\"");
    assert_eq!(check_roundtrip(&FBig::INFINITY), "\"inf\"");
    assert_eq!(check_roundtrip(&FBig::NEG_INFINITY), "\"-inf\"");

    for repr in [Repr::<10>::infinity(), Repr::<10>::neg_infinity()] {
        let bytes = to_allocvec(&repr).unwrap();
        let parsed: Repr<10> = from_bytes(&bytes).unwrap();
        assert_eq!(parsed, repr);
    }

    // nothing else is accepted as an infinity
    for s in ["\"+inf\"", "\"Inf\"", "\"infinity\"", "\"nan\"", "\"\""] {
        assert!(from_str::<DBig>(s).is_err(), "{s}");
        assert!(from_str::<Repr<10>>(s).is_err(), "{s}");
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
            &DBig::from_parts(1.into(), -3)
                .with_precision(7)
                .value()
                .compact(),
            &[
                Token::Struct {
                    name: "FBig",
                    len: 3,
                },
                key("significand"),
                Token::Bytes(&[1]),
                key("exponent"),
                Token::I64(-3),
                key("precision"),
                Token::U64(7),
                Token::StructEnd,
            ],
        );
        assert_de_tokens(
            &Repr::<10>::new((-2).into(), 5).compact(),
            &[
                Token::Struct {
                    name: "FBigRepr",
                    len: 2,
                },
                key("exponent"),
                Token::I64(5),
                key("significand"),
                Token::Bytes(&[0xfe]),
                Token::StructEnd,
            ],
        );
    }

    assert_de_tokens_error::<serde_test::Compact<DBig>>(
        &[
            Token::Struct {
                name: "FBig",
                len: 3,
            },
            Token::String("mantissa"),
        ],
        "unknown field `mantissa`, expected one of `significand`, `exponent`, `precision`",
    );
    assert_de_tokens_error::<serde_test::Compact<Repr<10>>>(
        &[
            Token::Struct {
                name: "FBigRepr",
                len: 2,
            },
            Token::String("precision"),
        ],
        "unknown field `precision`, expected `significand` or `exponent`",
    );

    // owned keys also reach the struct form of a human-readable format: a reader cannot
    // lend out its input
    let json = r#"{"significand":"5","exponent":-1,"precision":0}"#;
    let parsed: DBig = serde_json::from_reader(json.as_bytes()).unwrap();
    assert_eq!(parsed, DBig::from_parts(5.into(), -1));
    assert_eq!(parsed.precision(), 0);
}
