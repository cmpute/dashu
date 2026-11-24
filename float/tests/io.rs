use core::str::FromStr;
use dashu_base::{ParseError, Sign};
use dashu_float::{round::mode, DBig, FBig};
use dashu_int::{DoubleWord, IBig, Word};

mod helper_macros;
type FBin = FBig;
type FOct = FBig<mode::Zero, 8>;
type FHex = FBig<mode::Zero, 16>;

#[test]
fn test_default() {
    assert_eq!(FBin::default(), fbig!(0));
    assert_eq!(DBig::default(), dbig!(0));
}

// radix independent cases: (text, mantissa, exponent, precision)
const FROM_STR_COMMON_CASES: [(&str, i64, isize, usize); 28] = [
    //
    // unsigned
    //
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
    //
    // signed
    //
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
fn test_from_str_decimal() {
    let test_cases = [
        // scientific
        ("2e0", 2, 0, 1),
        ("10e5", 1, 6, 2),
        ("-2E-7", -2, -7, 1),
        ("3.e4", 3, 4, 1),
        ("-.6e-1", -6, -2, 1),
        ("-12_34_.56_78e9", -12345678, 5, 8),
    ];
    for (text, man, exp, prec) in FROM_STR_COMMON_CASES.iter().copied().chain(test_cases) {
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
fn test_from_str_binary() {
    let test_cases = [
        //
        // scientific with 'b' notation
        //
        ("10b0", 1, 1, 2),
        ("0110b5", 3, 6, 4),
        ("-10B-7", -1, -6, 2),
        ("11.b4", 3, 4, 2),
        ("-.11b4", -3, 2, 2),
        ("-.0110B-1", -3, -4, 4),
        ("-1100_0100_.0101_1010b3", -25133, -4, 16),
        //
        // with hexadecimal prefix
        //
        ("0x2", 2, 0, 4),
        ("-0x02", -2, 0, 8),
        ("0x.2", 1, -3, 4),
        ("0x2.2", 17, -3, 8),
        ("0xa", 5, 1, 4),
        ("0xb", 11, 0, 4),
        ("-0x0_f.2_0", -121, -3, 16),
        ("0x00100", 1, 8, 20),
        ("-0x010.00", -1, 4, 20),
        //
        // scientific with 'p' notation
        //
        ("0x2p0", 1, 1, 4),
        ("0x6p5", 3, 6, 4),
        ("-0x2P-7", -1, -6, 4),
        ("0x3.p4", 3, 4, 4),
        ("-0x0.6p-1", -3, -4, 8),
        ("-0x.1p0", -1, -4, 4),
        ("-0xc4.5ap3", -25133, -4, 16),
        ("0x00001p2", 1, 2, 20),
        ("-0x001.00p2", -1, 2, 20),
    ];
    for (text, man, exp, prec) in FROM_STR_COMMON_CASES.iter().copied().chain(test_cases) {
        let val = FBin::from_str(text).unwrap();
        assert_eq!(val, FBin::from_parts(IBig::from(man), exp));
        assert_eq!(val.precision(), prec);
    }

    assert_eq!(FBin::from_str("p"), Err(ParseError::InvalidDigit));
    assert_eq!(FBin::from_str("."), Err(ParseError::NoDigits));
    assert_eq!(FBin::from_str("1.0e8"), Err(ParseError::InvalidDigit));
    assert_eq!(FBin::from_str("0o2.2"), Err(ParseError::InvalidDigit));
    assert_eq!(FBin::from_str("一.二"), Err(ParseError::InvalidDigit));
    assert_eq!(FBin::from_str("一.二p三"), Err(ParseError::InvalidDigit));

    // prefix `0x` is required in following cases
    assert_eq!(FBin::from_str("1p8"), Err(ParseError::InvalidDigit));
    assert_eq!(FBin::from_str(".1p8"), Err(ParseError::InvalidDigit));
    assert_eq!(FBin::from_str("1.0p8"), Err(ParseError::InvalidDigit));
}

#[test]
fn test_from_str_oct_hex() {
    let oct_cases = [
        //
        // scientific with 'o' notation
        //
        ("10o0", 1, 1, 2),
        ("0770o5", 63, 6, 4),
        ("-30O-7", -3, -6, 2),
        ("11.o4", 9, 4, 2),
        ("-.11o4", -9, 2, 2),
        ("-.0700O-1", -7, -3, 4),
        ("-06_25_.13_30o3", -207451, 0, 8),
    ];
    for (text, man, exp, prec) in FROM_STR_COMMON_CASES.iter().copied().chain(oct_cases) {
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

    let hex_cases = [
        //
        // scientific with 'h' notation
        //
        ("10h0", 1, 1, 2),
        ("0bb0h5", 187, 6, 4),
        ("-d0H-7", -13, -6, 2),
        ("11.h4", 17, 4, 2),
        ("-.11h4", -17, 2, 2),
        ("-.0f00H-1", -15, -3, 4),
        ("-0a_db_.74h-3", -711540, -5, 6),
    ];
    for (text, man, exp, prec) in FROM_STR_COMMON_CASES.iter().copied().chain(hex_cases) {
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

#[test]
fn test_from_str_other_bases() {
    assert_eq!(
        FBig::<mode::Zero, 3>::from_str("12.21").unwrap(),
        FBig::<mode::Zero, 3>::from_parts(ibig!(52), -2)
    );
    assert_eq!(
        FBig::<mode::Zero, 20>::from_str("gg.hh@12").unwrap(),
        FBig::<mode::Zero, 20>::from_parts(ibig!(134757), 10)
    );
    assert_eq!(
        FBig::<mode::Zero, 30>::from_str("gg.hh@-12").unwrap(),
        FBig::<mode::Zero, 30>::from_parts(ibig!(446927), -14)
    );
}

#[test]
fn test_from_str_zero_with_scale() {
    // The scale of a zero is ignored: only a leading '-' makes a negative zero, and no scale
    // turns a zero into an infinity.
    let max = isize::MAX;
    let min = isize::MIN;
    let positive = [
        "0e0".to_string(),
        "0e-1".to_string(),
        "0e-2".to_string(),
        "+0e-1".to_string(),
        "0.0e-1".to_string(),
        "0@-1".to_string(),
        "0e5".to_string(),
        format!("0e{max}"),
        format!("0e{min}"),
        format!("0.00e{min}"),
    ];
    for s in &positive {
        let x = DBig::from_str(s).unwrap();
        assert!(x.repr().is_pos_zero(), "{s:?} parsed to {x:?}");
    }
    for s in ["-0", "-0e0", "-0e-1", "-0e-2", "-0.0e7"] {
        let x = DBig::from_str(s).unwrap();
        assert!(x.repr().is_neg_zero(), "{s:?} parsed to {x:?}");
    }
    assert!(DBig::from_str(&format!("-0e{max}"))
        .unwrap()
        .repr()
        .is_neg_zero());

    for s in ["0b0", "0b-1", "0b-2", "0x0p-1", "0x0p-2", "0x.0p-1", "0@-1"] {
        let x = FBin::from_str(s).unwrap();
        assert!(x.repr().is_pos_zero(), "{s:?} parsed to {x:?}");
    }
    for s in ["-0b-1", "-0x0p-1", "-0x0p-2"] {
        let x = FBin::from_str(s).unwrap();
        assert!(x.repr().is_neg_zero(), "{s:?} parsed to {x:?}");
    }
    assert!(FOct::from_str("0o-1").unwrap().repr().is_pos_zero());
    assert!(FHex::from_str("0h-1").unwrap().repr().is_pos_zero());
}

#[test]
fn test_from_str_single_sign() {
    // the sign is accepted once, in front of the integral part
    for s in [
        "0.+1", "-1.+5", "0.-1", "++1", "-+1", "+-1", "--1", "+", "-", ".+5", "1.+5e3",
    ] {
        assert!(DBig::from_str(s).is_err(), "{s:?} must be rejected");
    }
    for s in ["0x+1", "0x+1.8", "0x1.+8", "+0x+1p3", "1.+1", "+.+1b2"] {
        assert!(FBin::from_str(s).is_err(), "{s:?} must be rejected");
    }
    assert!(FHex::from_str("a.+b").is_err());

    // a single sign on the integral part and on the scale is still fine
    assert_eq!(DBig::from_str("+1.5e+3").unwrap(), DBig::from_parts(IBig::from(15), 2));
    assert_eq!(DBig::from_str("-1.5e-3").unwrap(), DBig::from_parts(IBig::from(-15), -4));
    assert_eq!(DBig::from_str("+.5").unwrap(), DBig::from_parts(IBig::from(5), -1));
    assert_eq!(FBin::from_str("+0x1.8p+1").unwrap(), FBin::from_parts(IBig::from(3), 0));
}

#[test]
fn test_from_str_prefix_without_digits() {
    for s in [
        ".", "-.", "0x", "0x.", "-0x.", "+0x.", "0X.", "0x.p5", "0x.@1", "-0X.P-3",
    ] {
        assert_eq!(FBin::from_str(s), Err(ParseError::NoDigits), "{s:?}");
    }
    // either part alone is enough
    assert_eq!(FBin::from_str("0x.8").unwrap(), FBin::from_parts(IBig::from(1), -1));
    assert_eq!(FBin::from_str("0x8.").unwrap(), FBin::from_parts(IBig::from(1), 3));
    assert!(FBin::from_str("0x0.").unwrap().repr().is_pos_zero());
}

#[test]
fn test_from_str_extreme_exponent() {
    let max = isize::MAX;
    let min = isize::MIN;

    // A nonzero number never lands on the reserved exponents `isize::MIN` / `isize::MAX`
    // (whether written directly, reached through the fractional digits, or reached when
    // trailing zeros are folded into the exponent), and the arithmetic never wraps around.
    let rejected = [
        format!("1e{max}"),
        format!("1e{min}"),
        format!("-1e{max}"),
        format!("1.5e{min}"),
        format!("1.5e{}", min + 1),
        format!("0.15e{}", min + 2),
        format!("10e{}", max - 1),
        format!("1000e{}", max - 3),
        format!("100e{}", max - 1),
        format!("1e{max}0"),
    ];
    for s in &rejected {
        assert_eq!(DBig::from_str(s), Err(ParseError::InvalidDigit), "{s:?}");
    }
    for s in [
        format!("1.1@{min}"),
        format!("1@{max}"),
        format!("0x1.8p{min}"),
    ] {
        assert_eq!(FBin::from_str(&s), Err(ParseError::InvalidDigit), "{s:?}");
    }

    // the neighbours are ordinary finite values
    let accepted = [
        (format!("1e{}", max - 1), 1, max - 1),
        (format!("1e{}", min + 1), 1, min + 1),
        (format!("1.5e{}", min + 2), 15, min + 1),
        (format!("10e{}", max - 2), 1, max - 1),
    ];
    for (s, signif, exp) in &accepted {
        let x = DBig::from_str(s).unwrap();
        assert_eq!(x.repr().significand(), &IBig::from(*signif), "{s:?}");
        assert_eq!(x.repr().exponent(), *exp, "{s:?}");
        assert!(x.repr().is_finite());
        assert_eq!(&x - &x, DBig::ZERO, "{s:?}");
    }
}

#[test]
fn test_from_parts() {
    assert_eq!(FBin::from_parts(ibig!(0), 2), FBin::ZERO);
    assert_eq!(FBin::from_parts(ibig!(-4), -2), FBin::NEG_ONE);
    assert_eq!(FBin::from_parts(ibig!(4), 0), FBin::from_parts(ibig!(1), 2));
    assert_eq!(FBin::from_parts(ibig!(-4), 0), FBin::from_parts(ibig!(-1), 2));
    assert_eq!(FBin::from_parts(ibig!(1) << 200, 0), FBin::from_parts(ibig!(1), 200));

    assert_eq!(FBin::from_parts_const(Sign::Negative, 0, 2, None), FBin::ZERO);
    assert_eq!(FBin::from_parts_const(Sign::Negative, 1, 0, None), FBin::NEG_ONE);
    assert_eq!(
        FBin::from_parts_const(Sign::Positive, 4, 0, None),
        FBin::from_parts(ibig!(1), 2)
    );
    assert_eq!(
        FBin::from_parts_const(Sign::Positive, 1 << (Word::BITS - 1), 0, None),
        FBin::from_parts(ibig!(1), (Word::BITS - 1) as isize)
    );
    assert_eq!(
        FBin::from_parts_const(Sign::Positive, 1 << (DoubleWord::BITS - 1), 0, None),
        FBin::from_parts(ibig!(1), (DoubleWord::BITS - 1) as isize)
    );

    assert_eq!(DBig::from_parts(ibig!(0), 2), DBig::ZERO);
    assert_eq!(DBig::from_parts(ibig!(-100), -2), DBig::NEG_ONE);
    assert_eq!(DBig::from_parts(ibig!(200), 0), DBig::from_parts(ibig!(2), 2));
    assert_eq!(DBig::from_parts(ibig!(-400), 0), DBig::from_parts(ibig!(-4), 2));
    assert_eq!(DBig::from_parts(ibig!(10).pow(200), 0), DBig::from_parts(ibig!(1), 200));

    assert_eq!(DBig::from_parts_const(Sign::Negative, 0, 2, None), DBig::ZERO);
    assert_eq!(DBig::from_parts_const(Sign::Negative, 100, -2, None), DBig::NEG_ONE);
    assert_eq!(
        DBig::from_parts_const(Sign::Positive, 200, 0, None),
        DBig::from_parts(ibig!(2), 2)
    );
    assert_eq!(
        DBig::from_parts_const(Sign::Negative, 100200, 0, None),
        DBig::from_parts_const(Sign::Negative, 1002, 2, None)
    );
}

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_format_binary() {
    assert_eq!(format!("{}", fbig!(0x0)), "0");
    assert_eq!(format!("{}", fbig!(0x1)), "1");
    assert_eq!(format!("{}", fbig!(-0x1)), "-1");
    assert_eq!(format!("{}", fbig!(0x1p4)), "10000");
    assert_eq!(format!("{}", fbig!(-0x1p4)), "-10000");
    assert_eq!(format!("{}", fbig!(0x1p-1)), "0.1");
    assert_eq!(format!("{}", fbig!(-0x1p-1)), "-0.1");
    assert_eq!(format!("{}", fbig!(0x1p-4)), "0.0001");
    assert_eq!(format!("{}", fbig!(-0x1p-4)), "-0.0001");

    assert_eq!(format!("{}", FBin::INFINITY), "inf");
    assert_eq!(format!("{}", FBin::NEG_INFINITY), "-inf");
    assert_eq!(format!("{}", FBin::from_parts(i8::MAX.into(), -4)), "111.1111");
    assert_eq!(format!("{}", FBin::from_parts(i8::MIN.into(), -4)), "-1000");
    assert_eq!(format!("{}", FBin::from_parts(i16::MAX.into(), -8)), "1111111.11111111");
    assert_eq!(format!("{}", FBin::from_parts(i16::MIN.into(), -8)), "-10000000");

    assert_eq!(format!("{:.0}", fbig!(0x0)), "0");
    assert_eq!(format!("{:.0}", fbig!(0x1)), "1");
    assert_eq!(format!("{:.0}", fbig!(-0x1)), "-1");
    assert_eq!(format!("{:.0}", fbig!(0x1p4)), "10000");
    assert_eq!(format!("{:.0}", fbig!(-0x1p4)), "-10000");
    assert_eq!(format!("{:.0}", fbig!(0x1p-1)), "0");
    // a negative that rounds to -0 keeps its sign, as `f64`'s does
    assert_eq!(format!("{:.0}", fbig!(-0x1p-1)), "-0");
    assert_eq!(format!("{:+.0}", fbig!(-0x1p-1)), "-0");
    assert_eq!(format!("{:.0}", fbig!(0x1p-4)), "0");
    assert_eq!(format!("{:.0}", fbig!(-0x1p-4)), "-0");
    assert_eq!(format!("{:8.0}", fbig!(0x0)), "       0");
    assert_eq!(format!("{:8.0}", fbig!(0x1)), "       1");
    assert_eq!(format!("{:8.0}", fbig!(-0x1)), "      -1");
    assert_eq!(format!("{:8.0}", fbig!(0x1p4)), "   10000");
    assert_eq!(format!("{:8.0}", fbig!(-0x1p4)), "  -10000");
    assert_eq!(format!("{:8.0}", fbig!(0x1p-1)), "       0");
    assert_eq!(format!("{:8.0}", fbig!(-0x1p-1)), "      -0");
    assert_eq!(format!("{:8.0}", fbig!(0x1p-4)), "       0");
    assert_eq!(format!("{:8.0}", fbig!(-0x1p-4)), "      -0");
    assert_eq!(format!("{:08.0}", fbig!(0x0)), "00000000");
    assert_eq!(format!("{:08.0}", fbig!(0x1)), "00000001");
    assert_eq!(format!("{:08.0}", fbig!(-0x1)), "-0000001");
    assert_eq!(format!("{:08.0}", fbig!(0x1p4)), "00010000");
    assert_eq!(format!("{:08.0}", fbig!(-0x1p4)), "-0010000");
    assert_eq!(format!("{:08.0}", fbig!(0x1p-1)), "00000000");
    assert_eq!(format!("{:08.0}", fbig!(-0x1p-1)), "-0000000");
    assert_eq!(format!("{:08.0}", fbig!(0x1p-4)), "00000000");
    assert_eq!(format!("{:08.0}", fbig!(-0x1p-4)), "-0000000");

    assert_eq!(format!("{:.8}", fbig!(0x0)), "0.00000000");
    assert_eq!(format!("{:.8}", fbig!(0x1)), "1.00000000");
    assert_eq!(format!("{:.8}", fbig!(-0x1)), "-1.00000000");
    assert_eq!(format!("{:.8}", fbig!(0x1p4)), "10000.00000000");
    assert_eq!(format!("{:.8}", fbig!(-0x1p4)), "-10000.00000000");
    assert_eq!(format!("{:.8}", fbig!(0x1p-1)), "0.10000000");
    assert_eq!(format!("{:.8}", fbig!(-0x1p-1)), "-0.10000000");
    assert_eq!(format!("{:.8}", fbig!(0x1p-4)), "0.00010000");
    assert_eq!(format!("{:.8}", fbig!(-0x1p-4)), "-0.00010000");
    assert_eq!(format!("{:8.4}", fbig!(0x0)), "  0.0000");
    assert_eq!(format!("{:8.4}", fbig!(0x1)), "  1.0000");
    assert_eq!(format!("{:8.4}", fbig!(-0x1)), " -1.0000");
    assert_eq!(format!("{:8.4}", fbig!(0x1p4)), "10000.0000");
    assert_eq!(format!("{:8.4}", fbig!(-0x1p4)), "-10000.0000");
    assert_eq!(format!("{:8.4}", fbig!(0x1p-1)), "  0.1000");
    assert_eq!(format!("{:8.4}", fbig!(-0x1p-1)), " -0.1000");
    assert_eq!(format!("{:8.4}", fbig!(0x1p-4)), "  0.0001");
    assert_eq!(format!("{:8.4}", fbig!(-0x1p-4)), " -0.0001");
    assert_eq!(format!("{:8.4}", fbig!(0x1p-5)), "  0.0000");
    assert_eq!(format!("{:8.4}", fbig!(-0x1p-5)), " -0.0000");
    assert_eq!(format!("{:08.4}", fbig!(0x0)), "000.0000");
    assert_eq!(format!("{:08.4}", fbig!(0x1)), "001.0000");
    assert_eq!(format!("{:08.4}", fbig!(-0x1)), "-01.0000");
    assert_eq!(format!("{:08.4}", fbig!(0x1p4)), "10000.0000");
    assert_eq!(format!("{:08.4}", fbig!(-0x1p4)), "-10000.0000");
    assert_eq!(format!("{:08.4}", fbig!(0x1p-1)), "000.1000");
    assert_eq!(format!("{:08.4}", fbig!(-0x1p-1)), "-00.1000");
    assert_eq!(format!("{:08.4}", fbig!(0x1p-4)), "000.0001");
    assert_eq!(format!("{:08.4}", fbig!(-0x1p-4)), "-00.0001");
    assert_eq!(format!("{:08.4}", fbig!(0x1p-5)), "000.0000");
    assert_eq!(format!("{:08.4}", fbig!(-0x1p-5)), "-00.0000");

    assert_eq!(format!("{:16}", fbig!(0x123p-4)), "      10010.0011");
    assert_eq!(format!("{:16}", fbig!(-0x123p-4)), "     -10010.0011");
    assert_eq!(format!("{:+16}", fbig!(0x123p-4)), "     +10010.0011");
    assert_eq!(format!("{:+16}", fbig!(-0x123p-4)), "     -10010.0011");
    assert_eq!(format!("{:016}", fbig!(0x123p-4)), "00000010010.0011");
    assert_eq!(format!("{:016}", fbig!(-0x123p-4)), "-0000010010.0011");
    assert_eq!(format!("{:+016}", fbig!(0x123p-4)), "+0000010010.0011");
    assert_eq!(format!("{:+016}", fbig!(-0x123p-4)), "-0000010010.0011");
    assert_eq!(format!("{:<16}", fbig!(0x123p-4)), "10010.0011      ");
    assert_eq!(format!("{:<16}", fbig!(-0x123p-4)), "-10010.0011     ");
    assert_eq!(format!("{:<+16}", fbig!(0x123p-4)), "+10010.0011     ");
    assert_eq!(format!("{:^16}", fbig!(0x123p-4)), "   10010.0011   ");
    assert_eq!(format!("{:^16}", fbig!(-0x123p-4)), "  -10010.0011   ");
    assert_eq!(format!("{:^+16}", fbig!(0x123p-4)), "  +10010.0011   ");
    assert_eq!(format!("{:>16}", fbig!(0x123p-4)), "      10010.0011");
    assert_eq!(format!("{:>16}", fbig!(-0x123p-4)), "     -10010.0011");
    assert_eq!(format!("{:>+16}", fbig!(0x123p-4)), "     +10010.0011");
    assert_eq!(format!("{:=<16}", fbig!(-0x123p-4)), "-10010.0011=====");
    assert_eq!(format!("{:=^16}", fbig!(-0x123p-4)), "==-10010.0011===");
    assert_eq!(format!("{:=>16}", fbig!(-0x123p-4)), "=====-10010.0011");
    assert_eq!(format!("{:=<+16}", fbig!(0x123p-4)), "+10010.0011=====");
    assert_eq!(format!("{:=^+16}", fbig!(0x123p-4)), "==+10010.0011===");
    assert_eq!(format!("{:=>+16}", fbig!(0x123p-4)), "=====+10010.0011");

    assert_eq!(format!("{:16.0}", fbig!(0x123p-4)), "           10010");
    assert_eq!(format!("{:16.0}", fbig!(-0x123p-4)), "          -10010");
    assert_eq!(format!("{:+16.0}", fbig!(0x123p-4)), "          +10010");
    assert_eq!(format!("{:+16.0}", fbig!(-0x123p-4)), "          -10010");
    assert_eq!(format!("{:016.0}", fbig!(0x123p-4)), "0000000000010010");
    assert_eq!(format!("{:016.0}", fbig!(-0x123p-4)), "-000000000010010");
    assert_eq!(format!("{:+016.0}", fbig!(0x123p-4)), "+000000000010010");
    assert_eq!(format!("{:+016.0}", fbig!(-0x123p-4)), "-000000000010010");
    assert_eq!(format!("{:<16.0}", fbig!(0x123p-4)), "10010           ");
    assert_eq!(format!("{:<16.0}", fbig!(-0x123p-4)), "-10010          ");
    assert_eq!(format!("{:<+16.0}", fbig!(0x123p-4)), "+10010          ");
    assert_eq!(format!("{:^16.0}", fbig!(0x123p-4)), "     10010      ");
    assert_eq!(format!("{:^16.0}", fbig!(-0x123p-4)), "     -10010     ");
    assert_eq!(format!("{:^+16.0}", fbig!(0x123p-4)), "     +10010     ");
    assert_eq!(format!("{:>16.0}", fbig!(0x123p-4)), "           10010");
    assert_eq!(format!("{:>16.0}", fbig!(-0x123p-4)), "          -10010");
    assert_eq!(format!("{:>+16.0}", fbig!(0x123p-4)), "          +10010");
    assert_eq!(format!("{:=<16.0}", fbig!(-0x123p-4)), "-10010==========");
    assert_eq!(format!("{:=^16.0}", fbig!(-0x123p-4)), "=====-10010=====");
    assert_eq!(format!("{:=>16.0}", fbig!(-0x123p-4)), "==========-10010");
    assert_eq!(format!("{:=<+16.0}", fbig!(0x123p-4)), "+10010==========");
    assert_eq!(format!("{:=^+16.0}", fbig!(0x123p-4)), "=====+10010=====");
    assert_eq!(format!("{:=>+16.0}", fbig!(0x123p-4)), "==========+10010");

    assert_eq!(format!("{:16.8}", fbig!(0x123p-4)), "  10010.00110000");
    assert_eq!(format!("{:16.8}", fbig!(-0x123p-4)), " -10010.00110000");
    assert_eq!(format!("{:+16.8}", fbig!(0x123p-4)), " +10010.00110000");
    assert_eq!(format!("{:+16.8}", fbig!(-0x123p-4)), " -10010.00110000");
    assert_eq!(format!("{:016.8}", fbig!(0x123p-4)), "0010010.00110000");
    assert_eq!(format!("{:016.8}", fbig!(-0x123p-4)), "-010010.00110000");
    assert_eq!(format!("{:+016.8}", fbig!(0x123p-4)), "+010010.00110000");
    assert_eq!(format!("{:+016.8}", fbig!(-0x123p-4)), "-010010.00110000");
    assert_eq!(format!("{:<16.8}", fbig!(0x123p-4)), "10010.00110000  ");
    assert_eq!(format!("{:<16.8}", fbig!(-0x123p-4)), "-10010.00110000 ");
    assert_eq!(format!("{:<+16.8}", fbig!(0x123p-4)), "+10010.00110000 ");
    assert_eq!(format!("{:^16.8}", fbig!(0x123p-4)), " 10010.00110000 ");
    assert_eq!(format!("{:^16.8}", fbig!(-0x123p-4)), "-10010.00110000 ");
    assert_eq!(format!("{:^+16.8}", fbig!(0x123p-4)), "+10010.00110000 ");
    assert_eq!(format!("{:>16.8}", fbig!(0x123p-4)), "  10010.00110000");
    assert_eq!(format!("{:>16.8}", fbig!(-0x123p-4)), " -10010.00110000");
    assert_eq!(format!("{:>+16.8}", fbig!(0x123p-4)), " +10010.00110000");
    assert_eq!(format!("{:=<16.8}", fbig!(-0x123p-4)), "-10010.00110000=");
    assert_eq!(format!("{:=^16.8}", fbig!(-0x123p-4)), "-10010.00110000=");
    assert_eq!(format!("{:=>16.8}", fbig!(-0x123p-4)), "=-10010.00110000");
    assert_eq!(format!("{:=<+16.8}", fbig!(0x123p-4)), "+10010.00110000=");
    assert_eq!(format!("{:=^+16.8}", fbig!(0x123p-4)), "+10010.00110000=");
    assert_eq!(format!("{:=>+16.8}", fbig!(0x123p-4)), "=+10010.00110000");
}

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_scientific_format_binary() {
    assert_eq!(format!("{:b}", fbig!(0x0)), "0b0");
    assert_eq!(format!("{:b}", fbig!(0x1)), "1b0");
    assert_eq!(format!("{:b}", fbig!(-0x1)), "-1b0");
    assert_eq!(format!("{:b}", fbig!(0x1p4)), "1b4");
    assert_eq!(format!("{:b}", fbig!(-0x1p4)), "-1b4");
    assert_eq!(format!("{:b}", fbig!(0x1p-1)), "1b-1");
    assert_eq!(format!("{:b}", fbig!(-0x1p-1)), "-1b-1");
    assert_eq!(format!("{:b}", fbig!(0x1p-4)), "1b-4");
    assert_eq!(format!("{:b}", fbig!(-0x1p-4)), "-1b-4");

    assert_eq!(format!("{:b}", FBin::INFINITY), "inf");
    assert_eq!(format!("{:b}", FBin::NEG_INFINITY), "-inf");
    assert_eq!(format!("{:b}", FBin::from_parts(i8::MAX.into(), -4)), "1.111111b2");
    assert_eq!(format!("{:b}", FBin::from_parts(i8::MIN.into(), -4)), "-1b3");
    assert_eq!(format!("{:b}", FBin::from_parts(i16::MAX.into(), -8)), "1.11111111111111b6");
    assert_eq!(format!("{:b}", FBin::from_parts(i16::MIN.into(), -8)), "-1b7");

    assert_eq!(format!("{:e}", fbig!(0x0)), "0@0");
    assert_eq!(format!("{:e}", fbig!(0x1)), "1@0");
    assert_eq!(format!("{:e}", fbig!(-0x1)), "-1@0");
    assert_eq!(format!("{:e}", fbig!(0x1p4)), "1@4");
    assert_eq!(format!("{:e}", fbig!(-0x1p4)), "-1@4");
    assert_eq!(format!("{:e}", fbig!(0x1p-1)), "1@-1");
    assert_eq!(format!("{:e}", fbig!(-0x1p-1)), "-1@-1");
    assert_eq!(format!("{:e}", fbig!(0x1p-4)), "1@-4");
    assert_eq!(format!("{:e}", fbig!(-0x1p-4)), "-1@-4");

    assert_eq!(format!("{:e}", FBin::INFINITY), "inf");
    assert_eq!(format!("{:e}", FBin::NEG_INFINITY), "-inf");
    assert_eq!(format!("{:e}", FBin::from_parts(i8::MAX.into(), -4)), "1.111111@2");
    assert_eq!(format!("{:e}", FBin::from_parts(i8::MIN.into(), -4)), "-1@3");
    assert_eq!(format!("{:e}", FBin::from_parts(i16::MAX.into(), -8)), "1.11111111111111@6");
    assert_eq!(format!("{:e}", FBin::from_parts(i16::MIN.into(), -8)), "-1@7");

    assert_eq!(format!("{:x}", fbig!(0x0)), "0x0p0");
    assert_eq!(format!("{:x}", fbig!(0x1)), "0x1p0");
    assert_eq!(format!("{:x}", fbig!(-0x1)), "-0x1p0");
    assert_eq!(format!("{:x}", fbig!(0x1p4)), "0x1p4");
    assert_eq!(format!("{:x}", fbig!(-0x1p4)), "-0x1p4");
    assert_eq!(format!("{:x}", fbig!(0x1p-1)), "0x1p-1");
    assert_eq!(format!("{:x}", fbig!(-0x1p-1)), "-0x1p-1");
    assert_eq!(format!("{:x}", fbig!(0x1p-4)), "0x1p-4");
    assert_eq!(format!("{:x}", fbig!(-0x1p-4)), "-0x1p-4");

    assert_eq!(format!("{:x}", FBin::INFINITY), "inf");
    assert_eq!(format!("{:x}", FBin::NEG_INFINITY), "-inf");
    assert_eq!(format!("{:x}", FBin::from_parts(i8::MAX.into(), -4)), "0x7.fp0");
    assert_eq!(format!("{:x}", FBin::from_parts(i8::MIN.into(), -4)), "-0x1p3");
    assert_eq!(format!("{:x}", FBin::from_parts(i16::MAX.into(), -8)), "0x7.fffp4");
    assert_eq!(format!("{:x}", FBin::from_parts(i16::MIN.into(), -8)), "-0x1p7");

    assert_eq!(format!("{:X}", fbig!(0x0)), "0x0p0");
    assert_eq!(format!("{:X}", fbig!(0x1)), "0x1p0");
    assert_eq!(format!("{:X}", fbig!(-0x1)), "-0x1p0");
    assert_eq!(format!("{:X}", fbig!(0x1p4)), "0x1p4");
    assert_eq!(format!("{:X}", fbig!(-0x1p4)), "-0x1p4");
    assert_eq!(format!("{:X}", fbig!(0x1p-1)), "0x1p-1");
    assert_eq!(format!("{:X}", fbig!(-0x1p-1)), "-0x1p-1");
    assert_eq!(format!("{:X}", fbig!(0x1p-4)), "0x1p-4");
    assert_eq!(format!("{:X}", fbig!(-0x1p-4)), "-0x1p-4");

    assert_eq!(format!("{:X}", FBin::INFINITY), "inf");
    assert_eq!(format!("{:X}", FBin::NEG_INFINITY), "-inf");
    assert_eq!(format!("{:X}", FBin::from_parts(i8::MAX.into(), -4)), "0x7.Fp0");
    assert_eq!(format!("{:X}", FBin::from_parts(i8::MIN.into(), -4)), "-0x1p3");
    assert_eq!(format!("{:X}", FBin::from_parts(i16::MAX.into(), -8)), "0x7.FFFp4");
    assert_eq!(format!("{:X}", FBin::from_parts(i16::MIN.into(), -8)), "-0x1p7");

    assert_eq!(format!("{:.0x}", fbig!(0x0)), "0x0p0");
    assert_eq!(format!("{:.0x}", fbig!(0x1)), "0x1p0");
    assert_eq!(format!("{:.0x}", fbig!(-0x1)), "-0x1p0");
    assert_eq!(format!("{:.0x}", fbig!(0x1p-1)), "0x1p-1");
    assert_eq!(format!("{:.0x}", fbig!(-0x1p-1)), "-0x1p-1");
    assert_eq!(format!("{:8.0x}", fbig!(0x0)), "   0x0p0");
    assert_eq!(format!("{:8.0x}", fbig!(0x1)), "   0x1p0");
    assert_eq!(format!("{:8.0x}", fbig!(-0x1)), "  -0x1p0");
    assert_eq!(format!("{:8.0x}", fbig!(0x1p-1)), "  0x1p-1");
    assert_eq!(format!("{:8.0x}", fbig!(-0x1p-1)), " -0x1p-1");
    assert_eq!(format!("{:08.0x}", fbig!(0x0)), "0x0000p0");
    assert_eq!(format!("{:08.0x}", fbig!(0x1)), "0x0001p0");
    assert_eq!(format!("{:08.0x}", fbig!(-0x1)), "-0x001p0");
    assert_eq!(format!("{:08.0x}", fbig!(0x1p-1)), "0x001p-1");
    assert_eq!(format!("{:08.0x}", fbig!(-0x1p-1)), "-0x01p-1");

    assert_eq!(format!("{:.8x}", fbig!(0x0)), "0x0.00000000p0");
    assert_eq!(format!("{:.8x}", fbig!(0x1)), "0x1.00000000p0");
    assert_eq!(format!("{:.8x}", fbig!(-0x1)), "-0x1.00000000p0");
    assert_eq!(format!("{:.8x}", fbig!(0x1p-1)), "0x1.00000000p-1");
    assert_eq!(format!("{:.8x}", fbig!(-0x1p-1)), "-0x1.00000000p-1");
    assert_eq!(format!("{:12.4x}", fbig!(0x0)), "  0x0.0000p0");
    assert_eq!(format!("{:12.4x}", fbig!(0x1)), "  0x1.0000p0");
    assert_eq!(format!("{:12.4x}", fbig!(-0x1)), " -0x1.0000p0");
    assert_eq!(format!("{:12.4x}", fbig!(0x1p-1)), " 0x1.0000p-1");
    assert_eq!(format!("{:12.4x}", fbig!(-0x1p-1)), "-0x1.0000p-1");
    assert_eq!(format!("{:012.4x}", fbig!(0x0)), "0x000.0000p0");
    assert_eq!(format!("{:012.4x}", fbig!(0x1)), "0x001.0000p0");
    assert_eq!(format!("{:012.4x}", fbig!(-0x1)), "-0x01.0000p0");
    assert_eq!(format!("{:012.4x}", fbig!(0x1p-1)), "0x01.0000p-1");
    assert_eq!(format!("{:012.4x}", fbig!(-0x1p-1)), "-0x1.0000p-1");
}

#[test]
fn test_format_decimal() {
    assert_eq!(format!("{}", dbig!(0)), "0");
    assert_eq!(format!("{}", dbig!(1)), "1");
    assert_eq!(format!("{}", dbig!(-1)), "-1");
    assert_eq!(format!("{}", dbig!(1e4)), "10000");
    assert_eq!(format!("{}", dbig!(-1e4)), "-10000");
    assert_eq!(format!("{}", dbig!(1e-1)), "0.1");
    assert_eq!(format!("{}", dbig!(-1e-1)), "-0.1");
    assert_eq!(format!("{}", dbig!(1e-4)), "0.0001");
    assert_eq!(format!("{}", dbig!(-1e-4)), "-0.0001");

    assert_eq!(format!("{}", DBig::INFINITY), "inf");
    assert_eq!(format!("{}", DBig::NEG_INFINITY), "-inf");
    assert_eq!(format!("{}", DBig::from_parts(i8::MAX.into(), -1)), "12.7");
    assert_eq!(format!("{}", DBig::from_parts(i8::MIN.into(), -1)), "-12.8");
    assert_eq!(format!("{}", DBig::from_parts(i16::MAX.into(), -2)), "327.67");
    assert_eq!(format!("{}", DBig::from_parts(i16::MIN.into(), -2)), "-327.68");

    assert_eq!(format!("{:.0}", dbig!(0)), "0");
    assert_eq!(format!("{:.0}", dbig!(1)), "1");
    assert_eq!(format!("{:.0}", dbig!(-1)), "-1");
    assert_eq!(format!("{:.0}", dbig!(1e4)), "10000");
    assert_eq!(format!("{:.0}", dbig!(-1e4)), "-10000");
    assert_eq!(format!("{:.0}", dbig!(1e-1)), "0");
    // a negative that rounds to -0 keeps its sign, as `f64`'s does
    assert_eq!(format!("{:.0}", dbig!(-1e-1)), "-0");
    assert_eq!(format!("{:+.0}", dbig!(-1e-1)), "-0");
    assert_eq!(format!("{:.0}", dbig!(1e-4)), "0");
    assert_eq!(format!("{:.0}", dbig!(-1e-4)), "-0");
    assert_eq!(format!("{:.0}", dbig!(9e-1)), "1"); // round up
    assert_eq!(format!("{:.0}", dbig!(-9e-1)), "-1");
    assert_eq!(format!("{:.0}", dbig!(99e-1)), "10");
    assert_eq!(format!("{:.0}", dbig!(-99e-1)), "-10");
    assert_eq!(format!("{:8.0}", dbig!(0)), "       0");
    assert_eq!(format!("{:8.0}", dbig!(1)), "       1");
    assert_eq!(format!("{:8.0}", dbig!(-1)), "      -1");
    assert_eq!(format!("{:8.0}", dbig!(1e4)), "   10000");
    assert_eq!(format!("{:8.0}", dbig!(-1e4)), "  -10000");
    assert_eq!(format!("{:8.0}", dbig!(1e-1)), "       0");
    assert_eq!(format!("{:8.0}", dbig!(-1e-1)), "      -0");
    assert_eq!(format!("{:8.0}", dbig!(1e-4)), "       0");
    assert_eq!(format!("{:8.0}", dbig!(-1e-4)), "      -0");
    assert_eq!(format!("{:8.0}", dbig!(9e-1)), "       1");
    assert_eq!(format!("{:8.0}", dbig!(-9e-1)), "      -1");
    assert_eq!(format!("{:8.0}", dbig!(99e-1)), "      10");
    assert_eq!(format!("{:8.0}", dbig!(-99e-1)), "     -10");
    assert_eq!(format!("{:08.0}", dbig!(0)), "00000000");
    assert_eq!(format!("{:08.0}", dbig!(1)), "00000001");
    assert_eq!(format!("{:08.0}", dbig!(-1)), "-0000001");
    assert_eq!(format!("{:08.0}", dbig!(1e4)), "00010000");
    assert_eq!(format!("{:08.0}", dbig!(-1e4)), "-0010000");
    assert_eq!(format!("{:08.0}", dbig!(1e-1)), "00000000");
    assert_eq!(format!("{:08.0}", dbig!(-1e-1)), "-0000000");
    assert_eq!(format!("{:08.0}", dbig!(1e-4)), "00000000");
    assert_eq!(format!("{:08.0}", dbig!(-1e-4)), "-0000000");
    assert_eq!(format!("{:08.0}", dbig!(9e-1)), "00000001");
    assert_eq!(format!("{:08.0}", dbig!(-9e-1)), "-0000001");
    assert_eq!(format!("{:08.0}", dbig!(99e-1)), "00000010");
    assert_eq!(format!("{:08.0}", dbig!(-99e-1)), "-0000010");

    assert_eq!(format!("{:.8}", dbig!(0)), "0.00000000");
    assert_eq!(format!("{:.8}", dbig!(1)), "1.00000000");
    assert_eq!(format!("{:.8}", dbig!(-1)), "-1.00000000");
    assert_eq!(format!("{:.8}", dbig!(1e4)), "10000.00000000");
    assert_eq!(format!("{:.8}", dbig!(-1e4)), "-10000.00000000");
    assert_eq!(format!("{:.8}", dbig!(1e-1)), "0.10000000");
    assert_eq!(format!("{:.8}", dbig!(-1e-1)), "-0.10000000");
    assert_eq!(format!("{:.8}", dbig!(1e-4)), "0.00010000");
    assert_eq!(format!("{:.8}", dbig!(-1e-4)), "-0.00010000");
    assert_eq!(format!("{:8.4}", dbig!(0)), "  0.0000");
    assert_eq!(format!("{:8.4}", dbig!(1)), "  1.0000");
    assert_eq!(format!("{:8.4}", dbig!(-1)), " -1.0000");
    assert_eq!(format!("{:8.4}", dbig!(1e4)), "10000.0000");
    assert_eq!(format!("{:8.4}", dbig!(-1e4)), "-10000.0000");
    assert_eq!(format!("{:8.4}", dbig!(1e-1)), "  0.1000");
    assert_eq!(format!("{:8.4}", dbig!(-1e-1)), " -0.1000");
    assert_eq!(format!("{:8.4}", dbig!(1e-4)), "  0.0001");
    assert_eq!(format!("{:8.4}", dbig!(-1e-4)), " -0.0001");
    assert_eq!(format!("{:8.4}", dbig!(1e-5)), "  0.0000");
    assert_eq!(format!("{:8.4}", dbig!(-1e-5)), " -0.0000");
    assert_eq!(format!("{:8.4}", dbig!(9e-5)), "  0.0001");
    assert_eq!(format!("{:8.4}", dbig!(-9e-5)), " -0.0001");
    assert_eq!(format!("{:8.4}", dbig!(99e-5)), "  0.0010");
    assert_eq!(format!("{:8.4}", dbig!(-99e-5)), " -0.0010");
    assert_eq!(format!("{:08.4}", dbig!(0)), "000.0000");
    assert_eq!(format!("{:08.4}", dbig!(1)), "001.0000");
    assert_eq!(format!("{:08.4}", dbig!(-1)), "-01.0000");
    assert_eq!(format!("{:08.4}", dbig!(1e4)), "10000.0000");
    assert_eq!(format!("{:08.4}", dbig!(-1e4)), "-10000.0000");
    assert_eq!(format!("{:08.4}", dbig!(1e-1)), "000.1000");
    assert_eq!(format!("{:08.4}", dbig!(-1e-1)), "-00.1000");
    assert_eq!(format!("{:08.4}", dbig!(1e-4)), "000.0001");
    assert_eq!(format!("{:08.4}", dbig!(-1e-4)), "-00.0001");
    assert_eq!(format!("{:08.4}", dbig!(1e-5)), "000.0000");
    assert_eq!(format!("{:08.4}", dbig!(-1e-5)), "-00.0000");
    assert_eq!(format!("{:08.4}", dbig!(9e-5)), "000.0001");
    assert_eq!(format!("{:08.4}", dbig!(-9e-5)), "-00.0001");
    assert_eq!(format!("{:08.4}", dbig!(99e-5)), "000.0010");
    assert_eq!(format!("{:08.4}", dbig!(-99e-5)), "-00.0010");

    assert_eq!(format!("{:8}", dbig!(123e-2)), "    1.23");
    assert_eq!(format!("{:8}", dbig!(-123e-2)), "   -1.23");
    assert_eq!(format!("{:+8}", dbig!(123e-2)), "   +1.23");
    assert_eq!(format!("{:+8}", dbig!(-123e-2)), "   -1.23");
    assert_eq!(format!("{:08}", dbig!(123e-2)), "00001.23");
    assert_eq!(format!("{:08}", dbig!(-123e-2)), "-0001.23");
    assert_eq!(format!("{:+08}", dbig!(123e-2)), "+0001.23");
    assert_eq!(format!("{:+08}", dbig!(-123e-2)), "-0001.23");
    assert_eq!(format!("{:<8}", dbig!(123e-2)), "1.23    ");
    assert_eq!(format!("{:<8}", dbig!(-123e-2)), "-1.23   ");
    assert_eq!(format!("{:<+8}", dbig!(123e-2)), "+1.23   ");
    assert_eq!(format!("{:^8}", dbig!(123e-2)), "  1.23  ");
    assert_eq!(format!("{:^8}", dbig!(-123e-2)), " -1.23  ");
    assert_eq!(format!("{:^+8}", dbig!(123e-2)), " +1.23  ");
    assert_eq!(format!("{:>8}", dbig!(123e-2)), "    1.23");
    assert_eq!(format!("{:>8}", dbig!(-123e-2)), "   -1.23");
    assert_eq!(format!("{:>+8}", dbig!(123e-2)), "   +1.23");
    assert_eq!(format!("{:=<8}", dbig!(-123e-2)), "-1.23===");
    assert_eq!(format!("{:=^8}", dbig!(-123e-2)), "=-1.23==");
    assert_eq!(format!("{:=>8}", dbig!(-123e-2)), "===-1.23");
    assert_eq!(format!("{:=<+8}", dbig!(123e-2)), "+1.23===");
    assert_eq!(format!("{:=^+8}", dbig!(123e-2)), "=+1.23==");
    assert_eq!(format!("{:=>+8}", dbig!(123e-2)), "===+1.23");

    assert_eq!(format!("{:8.0}", dbig!(123e-2)), "       1");
    assert_eq!(format!("{:8.0}", dbig!(-123e-2)), "      -1");
    assert_eq!(format!("{:+8.0}", dbig!(123e-2)), "      +1");
    assert_eq!(format!("{:+8.0}", dbig!(-123e-2)), "      -1");
    assert_eq!(format!("{:08.0}", dbig!(123e-2)), "00000001");
    assert_eq!(format!("{:08.0}", dbig!(-123e-2)), "-0000001");
    assert_eq!(format!("{:+08.0}", dbig!(123e-2)), "+0000001");
    assert_eq!(format!("{:+08.0}", dbig!(-123e-2)), "-0000001");
    assert_eq!(format!("{:<8.0}", dbig!(123e-2)), "1       ");
    assert_eq!(format!("{:<8.0}", dbig!(-123e-2)), "-1      ");
    assert_eq!(format!("{:<+8.0}", dbig!(123e-2)), "+1      ");
    assert_eq!(format!("{:^8.0}", dbig!(123e-2)), "   1    ");
    assert_eq!(format!("{:^8.0}", dbig!(-123e-2)), "   -1   ");
    assert_eq!(format!("{:^+8.0}", dbig!(123e-2)), "   +1   ");
    assert_eq!(format!("{:>8.0}", dbig!(123e-2)), "       1");
    assert_eq!(format!("{:>8.0}", dbig!(-123e-2)), "      -1");
    assert_eq!(format!("{:>+8.0}", dbig!(123e-2)), "      +1");
    assert_eq!(format!("{:=<8.0}", dbig!(-123e-2)), "-1======");
    assert_eq!(format!("{:=^8.0}", dbig!(-123e-2)), "===-1===");
    assert_eq!(format!("{:=>8.0}", dbig!(-123e-2)), "======-1");
    assert_eq!(format!("{:=<+8.0}", dbig!(123e-2)), "+1======");
    assert_eq!(format!("{:=^+8.0}", dbig!(123e-2)), "===+1===");
    assert_eq!(format!("{:=>+8.0}", dbig!(123e-2)), "======+1");

    assert_eq!(format!("{:8.4}", dbig!(123e-2)), "  1.2300");
    assert_eq!(format!("{:8.4}", dbig!(-123e-2)), " -1.2300");
    assert_eq!(format!("{:+8.4}", dbig!(123e-2)), " +1.2300");
    assert_eq!(format!("{:+8.4}", dbig!(-123e-2)), " -1.2300");
    assert_eq!(format!("{:08.4}", dbig!(123e-2)), "001.2300");
    assert_eq!(format!("{:08.4}", dbig!(-123e-2)), "-01.2300");
    assert_eq!(format!("{:+08.4}", dbig!(123e-2)), "+01.2300");
    assert_eq!(format!("{:+08.4}", dbig!(-123e-2)), "-01.2300");
    assert_eq!(format!("{:<8.4}", dbig!(123e-2)), "1.2300  ");
    assert_eq!(format!("{:<8.4}", dbig!(-123e-2)), "-1.2300 ");
    assert_eq!(format!("{:<+8.4}", dbig!(123e-2)), "+1.2300 ");
    assert_eq!(format!("{:^8.4}", dbig!(123e-2)), " 1.2300 ");
    assert_eq!(format!("{:^8.4}", dbig!(-123e-2)), "-1.2300 ");
    assert_eq!(format!("{:^+8.4}", dbig!(123e-2)), "+1.2300 ");
    assert_eq!(format!("{:>8.4}", dbig!(123e-2)), "  1.2300");
    assert_eq!(format!("{:>8.4}", dbig!(-123e-2)), " -1.2300");
    assert_eq!(format!("{:>+8.4}", dbig!(123e-2)), " +1.2300");
    assert_eq!(format!("{:=<8.4}", dbig!(-123e-2)), "-1.2300=");
    assert_eq!(format!("{:=^8.4}", dbig!(-123e-2)), "-1.2300=");
    assert_eq!(format!("{:=>8.4}", dbig!(-123e-2)), "=-1.2300");
    assert_eq!(format!("{:=<+8.4}", dbig!(123e-2)), "+1.2300=");
    assert_eq!(format!("{:=^+8.4}", dbig!(123e-2)), "+1.2300=");
    assert_eq!(format!("{:=>+8.4}", dbig!(123e-2)), "=+1.2300");
}

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_scientific_format_decimal() {
    assert_eq!(format!("{:e}", dbig!(0)), "0e0");
    assert_eq!(format!("{:e}", dbig!(1)), "1e0");
    assert_eq!(format!("{:e}", dbig!(-1)), "-1e0");
    assert_eq!(format!("{:e}", dbig!(1e4)), "1e4");
    assert_eq!(format!("{:e}", dbig!(-1e4)), "-1e4");
    assert_eq!(format!("{:e}", dbig!(1e-1)), "1e-1");
    assert_eq!(format!("{:e}", dbig!(-1e-1)), "-1e-1");
    assert_eq!(format!("{:e}", dbig!(1e-4)), "1e-4");
    assert_eq!(format!("{:e}", dbig!(-1e-4)), "-1e-4");

    assert_eq!(format!("{:e}", DBig::INFINITY), "inf");
    assert_eq!(format!("{:e}", DBig::NEG_INFINITY), "-inf");
    assert_eq!(format!("{:e}", DBig::from_parts(i8::MAX.into(), -1)), "1.27e1");
    assert_eq!(format!("{:e}", DBig::from_parts(i8::MIN.into(), -1)), "-1.28e1");
    assert_eq!(format!("{:e}", DBig::from_parts(i16::MAX.into(), -2)), "3.2767e2");
    assert_eq!(format!("{:e}", DBig::from_parts(i16::MIN.into(), -2)), "-3.2768e2");

    assert_eq!(format!("{:E}", dbig!(0)), "0E0");
    assert_eq!(format!("{:E}", dbig!(1)), "1E0");
    assert_eq!(format!("{:E}", dbig!(-1)), "-1E0");
    assert_eq!(format!("{:E}", dbig!(1e4)), "1E4");
    assert_eq!(format!("{:E}", dbig!(-1e4)), "-1E4");
    assert_eq!(format!("{:E}", dbig!(1e-1)), "1E-1");
    assert_eq!(format!("{:E}", dbig!(-1e-1)), "-1E-1");
    assert_eq!(format!("{:E}", dbig!(1e-4)), "1E-4");
    assert_eq!(format!("{:E}", dbig!(-1e-4)), "-1E-4");

    assert_eq!(format!("{:E}", DBig::INFINITY), "inf");
    assert_eq!(format!("{:E}", DBig::NEG_INFINITY), "-inf");
    assert_eq!(format!("{:E}", DBig::from_parts(i8::MAX.into(), -1)), "1.27E1");
    assert_eq!(format!("{:E}", DBig::from_parts(i8::MIN.into(), -1)), "-1.28E1");
    assert_eq!(format!("{:E}", DBig::from_parts(i16::MAX.into(), -2)), "3.2767E2");
    assert_eq!(format!("{:E}", DBig::from_parts(i16::MIN.into(), -2)), "-3.2768E2");

    assert_eq!(format!("{:.0e}", dbig!(0)), "0e0");
    assert_eq!(format!("{:.0e}", dbig!(1)), "1e0");
    assert_eq!(format!("{:.0e}", dbig!(-1)), "-1e0");
    assert_eq!(format!("{:.0e}", dbig!(1e-1)), "1e-1");
    assert_eq!(format!("{:.0e}", dbig!(-1e-1)), "-1e-1");
    assert_eq!(format!("{:.0e}", dbig!(1e-4)), "1e-4");
    assert_eq!(format!("{:.0e}", dbig!(-1e-4)), "-1e-4");
    assert_eq!(format!("{:8.0e}", dbig!(0)), "     0e0");
    assert_eq!(format!("{:8.0e}", dbig!(1)), "     1e0");
    assert_eq!(format!("{:8.0e}", dbig!(-1)), "    -1e0");
    assert_eq!(format!("{:8.0e}", dbig!(1e-1)), "    1e-1");
    assert_eq!(format!("{:8.0e}", dbig!(-1e-1)), "   -1e-1");
    assert_eq!(format!("{:08.0e}", dbig!(0)), "000000e0");
    assert_eq!(format!("{:08.0e}", dbig!(1)), "000001e0");
    assert_eq!(format!("{:08.0e}", dbig!(-1)), "-00001e0");
    assert_eq!(format!("{:08.0e}", dbig!(1e-1)), "00001e-1");
    assert_eq!(format!("{:08.0e}", dbig!(-1e-1)), "-0001e-1");

    assert_eq!(format!("{:.8e}", dbig!(0)), "0.00000000e0");
    assert_eq!(format!("{:.8e}", dbig!(1)), "1.00000000e0");
    assert_eq!(format!("{:.8e}", dbig!(-1)), "-1.00000000e0");
    assert_eq!(format!("{:.8e}", dbig!(1e-1)), "1.00000000e-1");
    assert_eq!(format!("{:.8e}", dbig!(-1e-1)), "-1.00000000e-1");
    assert_eq!(format!("{:10.4e}", dbig!(0)), "  0.0000e0");
    assert_eq!(format!("{:10.4e}", dbig!(1)), "  1.0000e0");
    assert_eq!(format!("{:10.4e}", dbig!(-1)), " -1.0000e0");
    assert_eq!(format!("{:10.4e}", dbig!(1e-1)), " 1.0000e-1");
    assert_eq!(format!("{:10.4e}", dbig!(-1e-1)), "-1.0000e-1");
    assert_eq!(format!("{:010.4e}", dbig!(0)), "000.0000e0");
    assert_eq!(format!("{:010.4e}", dbig!(1)), "001.0000e0");
    assert_eq!(format!("{:010.4e}", dbig!(-1)), "-01.0000e0");
    assert_eq!(format!("{:010.4e}", dbig!(1e-1)), "01.0000e-1");
    assert_eq!(format!("{:010.4e}", dbig!(-1e-1)), "-1.0000e-1");
}

#[test]
fn test_format_other_bases() {
    assert_eq!(format!("{}", FBig::<mode::Zero, 3>::from_parts(ibig!(52), -2)), "12.21");
    assert_eq!(format!("{:o}", FBig::<mode::Zero, 8>::from_parts(ibig!(100), -2)), "1.44o0");
    assert_eq!(
        format!("{:e}", FBig::<mode::Zero, 20>::from_parts(ibig!(134757), 10)),
        "g.ghh@13"
    );
    assert_eq!(
        format!("{:e}", FBig::<mode::Zero, 30>::from_parts(ibig!(446927), -14)),
        "g.ghh@-11"
    );
}

#[test]
#[rustfmt::skip::macros(fbig)]
fn test_format_debug() {
    assert_eq!(format!("{:?}", DBig::INFINITY), "inf");
    assert_eq!(format!("{:?}", DBig::NEG_INFINITY), "-inf");
    assert_eq!(format!("{:#?}", DBig::INFINITY), "inf");
    assert_eq!(format!("{:#?}", DBig::NEG_INFINITY), "-inf");

    assert_eq!(format!("{:?}", fbig!(0x1234p-4).repr()), "1165 * 2 ^ -2");
    assert_eq!(
        format!("{:?}", fbig!(0x1234p-4).context()),
        "Context { precision: 16, rounding: Zero }"
    );
    assert_eq!(format!("{:?}", fbig!(0x1234p-4)), "1165 * 2 ^ -2 (prec: 16)");

    assert_eq!(
        format!("{:#?}", fbig!(0x1234p-4).repr()),
        r#"Repr {
    significand: 1165 (11 bits),
    exponent: 2 ^ -2,
}"#
    );
    assert_eq!(
        format!("{:#?}", fbig!(0x1234p-4).context()),
        r#"Context {
    precision: 16,
    rounding: Zero,
}"#
    );
    assert_eq!(
        format!("{:#?}", fbig!(0x1234p-4)),
        r#"FBig {
    significand: 1165 (11 bits),
    exponent: 2 ^ -2,
    precision: 16,
    rounding: Zero,
}"#
    );

    assert_eq!(format!("{:?}", dbig!(1234e-2).repr()), "1234 * 10 ^ -2");
    assert_eq!(
        format!("{:?}", dbig!(1234e-2).context()),
        "Context { precision: 4, rounding: HalfAway }"
    );
    assert_eq!(format!("{:?}", dbig!(1234e-2)), "1234 * 10 ^ -2 (prec: 4)");

    assert_eq!(
        format!("{:#?}", dbig!(1234e-2).repr()),
        r#"Repr {
    significand: 1234 (digits: 4, bits: 11),
    exponent: 10 ^ -2,
}"#
    );
    assert_eq!(
        format!("{:#?}", dbig!(1234e-2).context()),
        r#"Context {
    precision: 4,
    rounding: HalfAway,
}"#
    );
    assert_eq!(
        format!("{:#?}", dbig!(1234e-2)),
        r#"FBig {
    significand: 1234 (digits: 4, bits: 11),
    exponent: 10 ^ -2,
    precision: 4,
    rounding: HalfAway,
}"#
    );
}

#[test]
fn test_string_roundtrip() {
    let fbig_cases = [
        fbig!(0x12345p - 67),
        fbig!(-0x123456789abcdef0123p45),
        FBin::try_from(f32::MAX).unwrap(),
        FBin::try_from(f32::MIN).unwrap(),
        FBin::try_from(f32::EPSILON).unwrap(),
        FBin::try_from(f64::MAX).unwrap(),
        FBin::try_from(f64::MIN).unwrap(),
        FBin::try_from(f64::EPSILON).unwrap(),
    ];

    for case in fbig_cases {
        assert_eq!(FBin::from_str(case.to_string().as_str()).unwrap(), case);
        assert_eq!(FBin::from_str(format!("{:b}", case).as_str()).unwrap(), case);
        assert_eq!(FBin::from_str(format!("{:010x}", case).as_str()).unwrap(), case);
        assert_eq!(FBin::from_str(format!("{:+010X}", case).as_str()).unwrap(), case);
    }

    let dbig_cases = [
        dbig!(0.00012345),
        dbig!(-1234567890.12345678901234567890123456789),
    ];

    for case in dbig_cases {
        assert_eq!(DBig::from_str(case.to_string().as_str()).unwrap(), case);
        assert_eq!(DBig::from_str(format!("{:e}", case).as_str()).unwrap(), case);
        assert_eq!(DBig::from_str(format!("{:+010E}", case).as_str()).unwrap(), case);
    }
}
