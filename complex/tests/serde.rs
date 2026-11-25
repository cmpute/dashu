use dashu_cmplx::CBig;
use dashu_float::round::mode::HalfEven;
use postcard::{from_bytes, to_allocvec};
use serde_json::{from_str, to_string};

type C = CBig<HalfEven, 10>;

/// Build a `CBig` with both parts at precision `prec`.
fn c(re: &str, im: &str, prec: usize) -> C {
    let re = re.parse::<dashu_float::FBig<HalfEven, 10>>().unwrap();
    let im = im.parse::<dashu_float::FBig<HalfEven, 10>>().unwrap();
    CBig::from_parts(re.with_precision(prec).value(), im.with_precision(prec).value())
}

#[test]
fn test_cbig_serde() {
    // Normalized values: each part's digit count ≤ the context precision, so the padded string
    // recovers the precision from the digit count.
    let test_numbers = [
        c("0", "0", 10),
        c("1", "2", 10),
        c("-3.5", "4.25", 10),
        c("123456789.0123456789", "-0.0001", 20),
        c("3.14159", "-2.71828", 30),
    ];
    for z in &test_numbers {
        // binary (non-human-readable) round-trip
        let output = to_allocvec(z).unwrap();
        let parsed: C = from_bytes(&output).unwrap();
        assert_eq!(&parsed, z);
        assert_eq!(parsed.precision(), z.precision(), "binary precision round-trip");

        // string (human-readable) round-trip: a single `a+bi` string whose coefficients are
        // padded to the precision, so `CBig::from_str` recovers it from the digit count.
        let output = to_string(z).unwrap();
        assert!(output.starts_with('"'), "human-readable must be a string, got {output}");
        let parsed: C = from_str(&output).unwrap();
        assert_eq!(&parsed, z);
        assert_eq!(parsed.precision(), z.precision(), "string precision round-trip");
    }
}

#[test]
fn cbig_serde_over_precision_value() {
    // A part whose digit count exceeds the context precision cannot imply the precision through
    // the `a+bi` string, so the human-readable form falls back to the struct form — the value
    // and the precision both round-trip.
    for z in [
        c("1e100", "-1e-100", 30),
        c("0.1", "0.1", 1),
        c("0.33", "-0.0033", 2),
        c("10", "0", 1),
    ] {
        let output = to_string(&z).unwrap();
        let parsed: C = from_str(&output).unwrap();
        assert_eq!(parsed, z, "{output}");
        assert_eq!(parsed.precision(), z.precision(), "{output}");
        let parsed: C = from_bytes(&to_allocvec(&z).unwrap()).unwrap();
        assert_eq!((parsed.precision(), &parsed), (z.precision(), &z));
    }
}

#[test]
fn cbig_serde_unlimited_precision() {
    // Unlimited precision (0) survives the human-readable round trip, so later arithmetic on
    // the value stays exact.
    use dashu_base::Sign::Positive;
    let twelve = C::from_parts_const((Positive, 12, 0), (Positive, 0, 0), 0);
    for z in [C::ZERO, C::ONE, C::NEG_ONE, C::I, twelve.clone()] {
        let output = to_string(&z).unwrap();
        let parsed: C = from_str(&output).unwrap();
        assert_eq!(parsed, z, "{output}");
        assert_eq!(parsed.precision(), 0, "{output}");
        assert_eq!(&parsed * &parsed, &z * &z, "{output}");
        let parsed: C = from_bytes(&to_allocvec(&z).unwrap()).unwrap();
        assert_eq!((parsed.precision(), &parsed), (0, &z));
    }
    assert_eq!(to_string(&twelve).unwrap(), r#"{"re":"12","im":"0","precision":0}"#);
    // the same value at a limited precision keeps using the string form
    let limited = c("12", "0", 2);
    assert_eq!(to_string(&limited).unwrap(), "\"12+0.0i\"");
    let parsed: C = from_str("\"12+0.0i\"").unwrap();
    assert_eq!((parsed.precision(), &parsed), (2, &limited));
}

#[test]
fn cbig_serde_letter_digits_keep_the_precision() {
    // In bases above 10 the letter digits count towards the precision like any other digit.
    use dashu_float::{Context, FBig, Repr};
    fn check<const B: dashu_int::Word>() {
        let b = B as i64;
        for (re, im, p) in [
            (0, 9, 1),
            (0, 10, 1),
            (b * b - 1, 0, 2),
            (b - 1, -(b - 1), 5),
            (1, b * b * b - 1, 3),
        ] {
            let part =
                |v: i64| FBig::<HalfEven, B>::from_repr(Repr::new(v.into(), 0), Context::new(p));
            let z = CBig::from_parts(part(re), part(im));
            let output = to_string(&z).unwrap();
            assert!(output.starts_with('"'), "base {B}: string form expected, got {output}");
            let parsed: CBig<HalfEven, B> = from_str(&output).unwrap();
            assert_eq!(parsed, z, "base {B}: {output}");
            assert_eq!(parsed.precision(), p, "base {B}: {output}");
        }
    }
    check::<16>();
    check::<11>();
    check::<18>();
    check::<20>();
    check::<36>();
    let z = CBig::<HalfEven, 16>::from_parts(
        FBig::from_repr(Repr::new(0.into(), 0), Context::new(1)),
        FBig::from_repr(Repr::new(10.into(), 0), Context::new(1)),
    );
    assert_eq!(to_string(&z).unwrap(), "\"0+ai\"");
}

#[test]
fn cbig_serde_infinite_parts() {
    // A value with an infinite part reads back as the same value at the same precision, in
    // every base. In bases above 23 the letters of `inf` are digits (in base 36 the string
    // `inf` is the integer 24171), so the `a+bi` string cannot be used there.
    fn check<const B: dashu_int::Word>() {
        type F<const B: dashu_int::Word> = dashu_float::FBig<HalfEven, B>;
        for (re, im) in [
            (F::<B>::from(1), F::<B>::INFINITY),
            (F::<B>::from(1), F::<B>::NEG_INFINITY),
            (F::<B>::INFINITY, F::<B>::INFINITY),
            (F::<B>::NEG_INFINITY, F::<B>::INFINITY),
            (F::<B>::NEG_INFINITY, F::<B>::from(2)),
            (F::<B>::INFINITY, F::<B>::ZERO),
            (F::<B>::INFINITY, F::<B>::ONE),
            (F::<B>::NEG_INFINITY, F::<B>::ZERO),
            (F::<B>::INFINITY, -F::<B>::from(7)),
        ] {
            for precision in [None, Some(0), Some(1), Some(20), Some(53)] {
                let z = match precision {
                    None => CBig::<HalfEven, B>::from_parts(re.clone(), im.clone()),
                    Some(p) => CBig::from_parts(
                        re.clone().with_precision(p).value(),
                        im.clone().with_precision(p).value(),
                    ),
                };
                let output = to_string(&z).unwrap();
                let parsed: CBig<HalfEven, B> = from_str(&output).unwrap();
                assert_eq!(parsed.re(), z.re(), "base {B}: {output}");
                assert_eq!(parsed.im(), z.im(), "base {B}: {output}");
                assert_eq!(parsed.precision(), z.precision(), "base {B}: {output}");

                let parsed: CBig<HalfEven, B> = from_bytes(&to_allocvec(&z).unwrap()).unwrap();
                assert_eq!((parsed.re(), parsed.im()), (z.re(), z.im()));
                assert_eq!(parsed.precision(), z.precision());
            }
        }
    }
    check::<2>();
    check::<10>();
    check::<16>();
    check::<23>();
    check::<24>();
    check::<36>();

    // where the string can carry the value it is still used
    type F = dashu_float::FBig<HalfEven, 10>;
    let z = C::from_parts(F::INFINITY, F::from(2).with_precision(4).value());
    assert_eq!(to_string(&z).unwrap(), "\"inf+2.000i\"");
    let z = C::from_parts(F::from(-2).with_precision(3).value(), F::NEG_INFINITY);
    assert_eq!(to_string(&z).unwrap(), "\"-2.00-infi\"");
}

#[test]
fn cbig_serde_finite_part_spelled_inf() {
    // The converse: in base 36 the finite integer 24171 is written `inf`, and must not read
    // back as an infinity, in the string form or in the struct form.
    type F = dashu_float::FBig<HalfEven, 36>;
    for value in [24171, -24171] {
        for precision in [0, 3, 5, 20] {
            let part = F::from(value).with_precision(precision).value();
            let zero = F::ZERO.with_precision(precision).value();
            for z in [
                CBig::from_parts(part.clone(), zero.clone()),
                CBig::from_parts(zero, part.clone()),
                CBig::from_parts(part.clone(), part.clone()),
                CBig::from_parts(part.clone(), F::INFINITY),
            ] {
                let output = to_string(&z).unwrap();
                let parsed: CBig<HalfEven, 36> = from_str(&output).unwrap();
                assert_eq!((parsed.re(), parsed.im()), (z.re(), z.im()), "{output}");
                assert_eq!(parsed.precision(), z.precision(), "{output}");
            }
        }
    }
}

#[test]
fn cbig_serde_large_exponent_parts() {
    // A part with a large exponent has no `a+bi` string that implies the precision: the struct
    // form is chosen without rendering that string first (the string would have as many digits
    // as the exponent is large, and checking it took far longer than writing the result).
    type F = dashu_float::FBig<HalfEven, 10>;
    for e in [31isize, 1_000, 200_000] {
        let re = F::from_parts(1.into(), e).with_precision(30).value();
        let im = F::from_parts((-1).into(), -e).with_precision(30).value();
        for z in [
            C::from_parts(re.clone(), im.clone()),
            C::from_parts(im.clone(), re.clone()),
            C::from_parts(re.clone(), F::ONE.with_precision(30).value()),
        ] {
            let output = to_string(&z).unwrap();
            assert!(output.starts_with('{'), "expected the struct form for e = {e}");
            let parsed: C = from_str(&output).unwrap();
            assert_eq!(parsed, z, "e = {e}");
            assert_eq!(parsed.precision(), 30, "e = {e}");
        }
    }
}

#[test]
fn cbig_serde_struct_field_names() {
    use serde_test::{assert_de_tokens, assert_de_tokens_error, Configure, Token};

    // A format may hand over the field names borrowed, transient, owned or as bytes.
    let keys: [fn(&'static str) -> Token; 4] =
        [Token::BorrowedStr, Token::Str, Token::String, |s| {
            Token::Bytes(s.as_bytes())
        }];
    for key in keys {
        assert_de_tokens(
            &c("0.5", "-2", 10).compact(),
            &[
                Token::Struct {
                    name: "CBig",
                    len: 3,
                },
                key("re"),
                Token::Struct {
                    name: "FBigRepr",
                    len: 2,
                },
                key("significand"),
                Token::Bytes(&[5]),
                key("exponent"),
                Token::I64(-1),
                Token::StructEnd,
                key("im"),
                Token::Struct {
                    name: "FBigRepr",
                    len: 2,
                },
                key("significand"),
                Token::Bytes(&[0xfe]),
                key("exponent"),
                Token::I64(0),
                Token::StructEnd,
                key("precision"),
                Token::U64(10),
                Token::StructEnd,
            ],
        );
    }

    assert_de_tokens_error::<serde_test::Compact<C>>(
        &[
            Token::Struct {
                name: "CBig",
                len: 3,
            },
            Token::String("real"),
        ],
        "unknown field `real`, expected one of `re`, `im`, `precision`",
    );
}
