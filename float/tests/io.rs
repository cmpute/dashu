use std::str::FromStr;
use dashu_float::{FBig, DBig, FloatRepr, RoundingMode};
use dashu_int::{IBig, error::ParseError, Word, Sign};

mod helper_macros;

// radix independent cases: (text, mantissa, exponent, precision)
const COMMON_CASES: [(&str, i64, isize, usize); 28] = [
    // unsigned
    ("0.0", 0, 0, 2),
    (".0", 0, 0, 1),
    ("1", 1, 0, 1),
    ("1.", 1, 0, 1),
    ("010", 1, 1, 3),
    ("010.", 1, 1, 3),
    (".1", 1, -1, 1),
    (".010", 1, -2, 3),
    ("1.0000", 1, 0, 5),
    ("1000.0000", 1, 3, 8),
    ("0000.0001", 1, -4, 8),
    ("10_00_.00_00", 1, 3, 8),
    ("00_00._00_01", 1, -4, 8),

    // signed
    ("-0.0", 0, 0, 2),
    ("-.0", 0, 0, 1),
    ("-1", -1, 0, 1),
    ("-1.", -1, 0, 1),
    ("-010", -1, 1, 3),
    ("-010.", -1, 1, 3),
    ("-.1", -1, -1, 1),
    ("-.010", -1, -2, 3),
    ("-1.0000", -1, 0, 5),
    ("+1000.0000", 1, 3, 8),
    ("-1000.0000", -1, 3, 8),
    ("0000.0001", 1, -4, 8),
    ("-0000.0001", -1, -4, 8),
    ("-10_00_.00_00", -1, 3, 8),
    ("-_00_00_._00_01", -1, -4, 8),
];

#[test]
fn test_dbig_from_str() {
    let test_cases = [
        // scientific
        ("2e0", 2, 0, 1),
        ("10e5", 1, 6, 2),
        ("-2E-7", -2, -7, 1),
        ("3.e4", 3, 4, 1),
        ("-.6e-1", -6, -2, 1),
        ("-12_34_.56_78e9", -12345678, 5, 8),
    ];
    for (text, man, exp, prec) in COMMON_CASES.iter().copied().chain(test_cases) {
        let val = DBig::from_str(text).unwrap();
        assert_eq!(val, DBig::from_parts(IBig::from(man), exp));
        assert_eq!(val.precision(), prec);
    }

    // error cases
    assert_eq!(DBig::from_str("f"), Err(ParseError::InvalidDigit));
    assert_eq!(DBig::from_str("e"), Err(ParseError::NoDigits));
    assert_eq!(DBig::from_str("."), Err(ParseError::NoDigits));
    assert_eq!(DBig::from_str(".e"), Err(ParseError::NoDigits));
    assert_eq!(DBig::from_str("-."), Err(ParseError::NoDigits));
    assert_eq!(DBig::from_str("-.e"), Err(ParseError::NoDigits));
    assert_eq!(DBig::from_str("-abc.def"), Err(ParseError::InvalidDigit));
    assert_eq!(DBig::from_str("0b1.1"), Err(ParseError::InvalidDigit));
    assert_eq!(DBig::from_str("0o2.2"), Err(ParseError::InvalidDigit));
    assert_eq!(DBig::from_str("0x2.2"), Err(ParseError::InvalidDigit));
    assert_eq!(DBig::from_str("一.二"), Err(ParseError::InvalidDigit));
    assert_eq!(DBig::from_str("一.二E三"), Err(ParseError::InvalidDigit));
}

#[test]
fn test_fbig_from_str() {
    let test_cases = [
        // scientific with 'b' notation
        ("10b0", 1, 1, 2),
        ("0110b5", 3, 6, 4),
        ("-10B-7", -1, -6, 2),
        ("11.b4", 3, 4, 2),
        ("-.11b4", -3, 2, 2),
        ("-.0110B-1", -3, -4, 4),
        ("-1100_0100_.0101_1010b3", -25133, -4, 16),

        // with hexadecimal prefix
        ("0x2", 2, 0, 4),
        ("-0x02", -2, 0, 8),
        ("0x.2", 1, -3, 4),
        ("0x2.2", 17, -3, 8),
        ("-0x0_f.2_0", -121, -3, 16),

        // scientific with 'p' notation
        ("0x2p0", 1, 1, 4),
        ("0x6p5", 3, 6, 4),
        ("-0x2P-7", -1, -6, 4),
        ("0x3.p4", 3, 4, 4),
        ("-0x0.6p-1", -3, -4, 8),
        ("-0x.1p0", -1, -4, 4),
        ("-0xc4.5ap3", -25133, -4, 16),
    ];
    for (text, man, exp, prec) in COMMON_CASES.iter().copied().chain(test_cases) {
        let val = FBig::from_str(text).unwrap();
        assert_eq!(val, FBig::from_parts(IBig::from(man), exp));
        assert_eq!(val.precision(), prec);
    }

    assert_eq!(FBig::from_str("p"), Err(ParseError::NoDigits));
    assert_eq!(FBig::from_str("."), Err(ParseError::NoDigits));
    assert_eq!(FBig::from_str("1.0e8"), Err(ParseError::InvalidDigit));
    assert_eq!(FBig::from_str("0o2.2"), Err(ParseError::InvalidDigit));
    assert_eq!(FBig::from_str("一.二"), Err(ParseError::InvalidDigit));
    assert_eq!(FBig::from_str("一.二p三"), Err(ParseError::InvalidDigit));

    // prefix `0x` is required in following cases
    assert_eq!(FBig::from_str("1p8"), Err(ParseError::UnsupportedRadix));
    assert_eq!(FBig::from_str(".1p8"), Err(ParseError::UnsupportedRadix));
    assert_eq!(FBig::from_str("1.0p8"), Err(ParseError::UnsupportedRadix));
}

#[test]
fn test_oct_hex_from_str() {
    type FOct = FloatRepr<8, { RoundingMode::Zero }>;
    let oct_cases = [
        // scientific with 'o' notation
        ("10o0", 1, 1, 2),
        ("0770o5", 63, 6, 4),
        ("-30O-7", -3, -6, 2),
        ("11.o4", 9, 4, 2),
        ("-.11o4", -9, 2, 2),
        ("-.0700O-1", -7, -3, 4),
        ("-06_25_.13_30o3", -207451, 0, 8),
    ];
    for (text, man, exp, prec) in COMMON_CASES.iter().copied().chain(oct_cases) {
        let val = FOct::from_str(text).unwrap();
        assert_eq!(val, FOct::from_parts(IBig::from(man), exp));
        assert_eq!(val.precision(), prec);
    }
    assert_eq!(FOct::from_str("f"), Err(ParseError::InvalidDigit));
    assert_eq!(FOct::from_str("O"), Err(ParseError::NoDigits));
    assert_eq!(FOct::from_str("."), Err(ParseError::NoDigits));
    assert_eq!(FOct::from_str("1.0e8"), Err(ParseError::InvalidDigit));
    assert_eq!(FOct::from_str("0b1.1"), Err(ParseError::InvalidDigit));
    assert_eq!(FOct::from_str("0o2.2"), Err(ParseError::InvalidDigit));
    assert_eq!(FOct::from_str("0x3O0"), Err(ParseError::InvalidDigit));
    assert_eq!(FOct::from_str("-0x.1o0"), Err(ParseError::InvalidDigit));
    assert_eq!(FOct::from_str("一.二"), Err(ParseError::InvalidDigit));
    assert_eq!(FOct::from_str("一.二o三"), Err(ParseError::InvalidDigit));

    type FHex = FloatRepr<16, { RoundingMode::Zero }>;
    let hex_cases = [
        // scientific with 'h' notation
        ("10h0", 1, 1, 2),
        ("0bb0h5", 187, 6, 4),
        ("-d0H-7", -13, -6, 2),
        ("11.h4", 17, 4, 2),
        ("-.11h4", -17, 2, 2),
        ("-.0f00H-1", -15, -3, 4),
        ("-0a_db_.74h-3", -711540, -5, 6),
    ];
    for (text, man, exp, prec) in COMMON_CASES.iter().copied().chain(hex_cases) {
        let val = FHex::from_str(text).unwrap();
        assert_eq!(val, FHex::from_parts(IBig::from(man), exp));
        assert_eq!(val.precision(), prec);
    }
    assert_eq!(FHex::from_str("H"), Err(ParseError::NoDigits));
    assert_eq!(FHex::from_str("."), Err(ParseError::NoDigits));
    assert_eq!(FHex::from_str("0o2.2"), Err(ParseError::InvalidDigit));
    assert_eq!(FHex::from_str("0x3H0"), Err(ParseError::InvalidDigit));
    assert_eq!(FHex::from_str("-0x.1h0"), Err(ParseError::InvalidDigit));
    assert_eq!(FHex::from_str("一.二"), Err(ParseError::InvalidDigit));
    assert_eq!(FHex::from_str("一.二h三"), Err(ParseError::InvalidDigit));
}

// TODO: test other bases
// let _ = fbig!(e9a.c2 base 16); // radix = 16
// let _ = fbig!(e9a.c2@32 base 16); // 0xe9a.c2 * 16^32, radix = 16
// let _ = fbig!(e9a.c2@32 base 20); // 0xe9a.c2 * 20^32, radix = 20

#[test]
fn test_from_parts() {
    assert_eq!(FBig::from_parts(ibig!(0), 2), FBig::zero());
    assert_eq!(FBig::from_parts(ibig!(-0), -1), FBig::zero());
    assert_eq!(FBig::from_parts(ibig!(4), 0), FBig::from_parts(ibig!(1), 2));
    assert_eq!(FBig::from_parts(ibig!(-4), 0), FBig::from_parts(ibig!(-1), 2));
    assert_eq!(FBig::from_parts(ibig!(1) << 200, 0), FBig::from_parts(ibig!(1), 200));

    assert_eq!(FBig::from_parts_const(Sign::Negative, 0, 0, 2), FBig::zero());
    assert_eq!(FBig::from_parts_const(Sign::Negative, 1, 0, 0), FBig::neg_one());
    assert_eq!(FBig::from_parts_const(Sign::Positive, 4, 0, 0), FBig::from_parts(ibig!(1), 2));
    assert_eq!(FBig::from_parts_const(Sign::Positive, 1 << (Word::BITS - 1), 0, 0), FBig::from_parts(ibig!(1), (Word::BITS - 1) as isize));
    assert_eq!(FBig::from_parts_const(Sign::Positive, 0, 1 << (Word::BITS - 1), 0), FBig::from_parts(ibig!(1), (2 * Word::BITS - 1) as isize));

    // TODO: add decimal tests
}
