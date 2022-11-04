use dashu_base::ParseError;
use dashu_ratio::{RBig, Relaxed};

mod helper_macros;

#[test]
fn test_rbig_format() {
    assert_eq!(format!("{}", rbig!(0)), "0");
    assert_eq!(format!("{}", rbig!(1)), "1");
    assert_eq!(format!("{}", rbig!(-1)), "-1");
    assert_eq!(format!("{}", rbig!(-3)), "-3");
    assert_eq!(format!("{}", rbig!(1 / 3)), "1/3");
    assert_eq!(format!("{}", rbig!(-1 / 3)), "-1/3");
    assert_eq!(format!("{}", rbig!(12 / 15)), "4/5");
}

#[test]
fn test_relaxed_format() {
    assert_eq!(format!("{}", rbig!(~0)), "0");
    assert_eq!(format!("{}", rbig!(~1)), "1");
    assert_eq!(format!("{}", rbig!(~-1)), "-1");
    assert_eq!(format!("{}", rbig!(~-3)), "-3");
    assert_eq!(format!("{}", rbig!(~1/3)), "1/3");
    assert_eq!(format!("{}", rbig!(~-1/3)), "-1/3");
    assert_eq!(format!("{}", rbig!(~12/15)), "12/15");
}

#[test]
fn test_rbig_from_str_radix() {
    assert_eq!(RBig::from_str_radix("", 2).unwrap_err(), ParseError::NoDigits);
    assert_eq!(RBig::from_str_radix("+", 2).unwrap_err(), ParseError::NoDigits);
    assert_eq!(RBig::from_str_radix("/", 2).unwrap_err(), ParseError::NoDigits);
    assert_eq!(RBig::from_str_radix("2/", 10).unwrap_err(), ParseError::NoDigits);
    assert_eq!(RBig::from_str_radix("/2", 10).unwrap_err(), ParseError::NoDigits);
    assert_eq!(RBig::from_str_radix("1//2", 10).unwrap_err(), ParseError::InvalidDigit);
    assert_eq!(RBig::from_str_radix("0", 2).unwrap(), rbig!(0));
    assert_eq!(RBig::from_str_radix("1", 2).unwrap(), rbig!(1));
    assert_eq!(RBig::from_str_radix("-1", 2).unwrap(), rbig!(-1));
    assert_eq!(RBig::from_str_radix("1/2", 10).unwrap(), rbig!(1 / 2));
    assert_eq!(RBig::from_str_radix("-1/2", 10).unwrap(), rbig!(-1 / 2));
    assert_eq!(RBig::from_str_radix("+1/-2", 10).unwrap(), rbig!(-1 / 2));
    assert_eq!(RBig::from_str_radix("-1/-2", 10).unwrap(), rbig!(1 / 2));
}

#[test]
fn test_relaxed_from_str_radix() {
    assert_eq!(Relaxed::from_str_radix("", 2).unwrap_err(), ParseError::NoDigits);
    assert_eq!(Relaxed::from_str_radix("+", 2).unwrap_err(), ParseError::NoDigits);
    assert_eq!(Relaxed::from_str_radix("/", 2).unwrap_err(), ParseError::NoDigits);
    assert_eq!(Relaxed::from_str_radix("2/", 10).unwrap_err(), ParseError::NoDigits);
    assert_eq!(Relaxed::from_str_radix("/2", 10).unwrap_err(), ParseError::NoDigits);
    assert_eq!(Relaxed::from_str_radix("1//2", 10).unwrap_err(), ParseError::InvalidDigit);
    assert_eq!(Relaxed::from_str_radix("0", 2).unwrap(), rbig!(~0));
    assert_eq!(Relaxed::from_str_radix("1", 2).unwrap(), rbig!(~1));
    assert_eq!(Relaxed::from_str_radix("-1", 2).unwrap(), rbig!(~-1));
    assert_eq!(Relaxed::from_str_radix("1/2", 10).unwrap(), rbig!(~1/2));
    assert_eq!(Relaxed::from_str_radix("-1/2", 10).unwrap(), rbig!(~-1/2));
    assert_eq!(Relaxed::from_str_radix("+1/-2", 10).unwrap(), rbig!(~-1/2));
    assert_eq!(Relaxed::from_str_radix("-1/-2", 10).unwrap(), rbig!(~1/2));
}
