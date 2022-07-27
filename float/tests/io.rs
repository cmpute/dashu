use std::str::FromStr;
use dashu_float::{FBig, DBig, FloatRepr, RoundingMode};
use dashu_int::error::ParseError;

mod helper_macros;

type FBigOct = FloatRepr<8, { RoundingMode::Zero }>;
type FBigHex = FloatRepr<16, { RoundingMode::Zero }>;

#[test]
fn test_dec_from_str() {
    assert_eq!(DBig::from_str("1").unwrap(), DBig::from_parts(ibig!(1), 0));
    assert_eq!(DBig::from_str("1.").unwrap(), DBig::from_parts(ibig!(1), 0));
    assert_eq!(DBig::from_str("010").unwrap(), DBig::from_parts(ibig!(1), 1));
    assert_eq!(DBig::from_str("010.").unwrap(), DBig::from_parts(ibig!(1), 1));
    assert_eq!(DBig::from_str(".1").unwrap(), DBig::from_parts(ibig!(1), -1));
    assert_eq!(DBig::from_str(".010").unwrap(), DBig::from_parts(ibig!(1), -2));

    assert_eq!(DBig::from_str("-1").unwrap(), DBig::from_parts(ibig!(-1), 0));
    assert_eq!(DBig::from_str("-1.").unwrap(), DBig::from_parts(ibig!(-1), 0));
    assert_eq!(DBig::from_str("-010").unwrap(), DBig::from_parts(ibig!(-1), 1));
    assert_eq!(DBig::from_str("-010.").unwrap(), DBig::from_parts(ibig!(-1), 1));
    assert_eq!(DBig::from_str("-.1").unwrap(), DBig::from_parts(ibig!(-1), -1));
    assert_eq!(DBig::from_str("-.010").unwrap(), DBig::from_parts(ibig!(-1), -2));

    assert_eq!(DBig::from_str("2e0").unwrap(), DBig::from_parts(ibig!(2), 0));
    assert_eq!(DBig::from_str("10e5").unwrap(), DBig::from_parts(ibig!(1), 6));
    assert_eq!(DBig::from_str("-2E-7").unwrap(), DBig::from_parts(ibig!(-2), -7));
    assert_eq!(DBig::from_str("3.e4").unwrap(), DBig::from_parts(ibig!(3), 4));
    assert_eq!(DBig::from_str("-.6e-1").unwrap(), DBig::from_parts(ibig!(-6), -2));
    assert_eq!(DBig::from_str("-12_34_.56_78e9").unwrap(), DBig::from_parts(ibig!(-12345678), 5));

    assert_eq!(DBig::from_str("f"), Err(ParseError::InvalidDigit));
    assert_eq!(DBig::from_str("e"), Err(ParseError::NoDigits));
    assert_eq!(DBig::from_str("."), Err(ParseError::NoDigits));
    assert_eq!(DBig::from_str(".e"), Err(ParseError::NoDigits));
    assert_eq!(DBig::from_str("-."), Err(ParseError::NoDigits));
    assert_eq!(DBig::from_str("-.e"), Err(ParseError::NoDigits));
    assert_eq!(DBig::from_str("-abc.def"), Err(ParseError::InvalidDigit));
}