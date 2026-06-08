use dashu_base::ParseError;
use dashu_int::UBig;
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
    assert_eq!(format!("{}", RBig::from_parts(ibig!(-1) << 200, (ubig!(1) << 200) - ubig!(1))),
        "-1606938044258990275541962092341162602522202993782792835301376/1606938044258990275541962092341162602522202993782792835301375");
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
    assert_eq!(format!("{}", Relaxed::from_parts(ibig!(-1) << 200, (ubig!(1) << 200) - ubig!(1))),
        "-1606938044258990275541962092341162602522202993782792835301376/1606938044258990275541962092341162602522202993782792835301375");
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
fn test_rbig_radix_fmt() {
    // Binary
    assert_eq!(format!("{:b}", rbig!(0)), "0");
    assert_eq!(format!("{:b}", rbig!(5)), "101");
    assert_eq!(format!("{:b}", rbig!(-5)), "-101");
    assert_eq!(format!("{:b}", rbig!(5 / 3)), "101/11");
    assert_eq!(format!("{:b}", rbig!(-5 / 3)), "-101/11");
    assert_eq!(format!("{:#b}", rbig!(5)), "0b101");
    assert_eq!(format!("{:#b}", rbig!(5 / 3)), "0b101/0b11");

    // Octal
    assert_eq!(format!("{:o}", rbig!(0)), "0");
    assert_eq!(format!("{:o}", rbig!(83)), "123");
    assert_eq!(format!("{:o}", rbig!(83 / 8)), "123/10");
    assert_eq!(format!("{:#o}", rbig!(83 / 8)), "0o123/0o10");

    // LowerHex
    assert_eq!(format!("{:x}", rbig!(0)), "0");
    assert_eq!(format!("{:x}", rbig!(255)), "ff");
    assert_eq!(format!("{:x}", rbig!(-255)), "-ff");
    assert_eq!(format!("{:x}", rbig!(255 / 16)), "ff/10");
    assert_eq!(format!("{:#x}", rbig!(255 / 16)), "0xff/0x10");

    // UpperHex
    assert_eq!(format!("{:X}", rbig!(255)), "FF");
    assert_eq!(format!("{:X}", rbig!(255 / 16)), "FF/10");
    assert_eq!(format!("{:#X}", rbig!(255 / 16)), "0xFF/0x10");
}

#[test]
fn test_relaxed_radix_fmt() {
    assert_eq!(format!("{:b}", rbig!(~5 / 3)), "101/11");
    assert_eq!(format!("{:o}", rbig!(~83 / 8)), "123/10");
    assert_eq!(format!("{:x}", rbig!(~255 / 16)), "ff/10");
    assert_eq!(format!("{:X}", rbig!(~255 / 16)), "FF/10");
    assert_eq!(format!("{:#x}", rbig!(~255 / 16)), "0xff/0x10");
}

#[test]
fn test_rbig_in_radix() {
    // Base 2
    assert_eq!(format!("{}", rbig!(5 / 3).in_radix(2)), "101/11");
    // Base 8
    assert_eq!(format!("{}", rbig!(83 / 8).in_radix(8)), "123/10");
    // Base 10 — same as Display
    assert_eq!(format!("{}", rbig!(42 / 13).in_radix(10)), "42/13");
    // Base 16
    assert_eq!(format!("{}", rbig!(255 / 16).in_radix(16)), "ff/10");
    // Alternate flag (uppercase for radices > 10)
    assert_eq!(format!("{:#}", rbig!(255 / 16).in_radix(16)), "FF/10");
    // Base 36
    assert_eq!(format!("{}", rbig!(35 / 1).in_radix(36)), "z");
    assert_eq!(format!("{:#}", rbig!(35 / 1).in_radix(36)), "Z");
    // Integer (denom=1)
    assert_eq!(format!("{}", rbig!(42).in_radix(16)), "2a");
    assert_eq!(format!("{}", rbig!(-42).in_radix(16)), "-2a");
    // Base 3 (non-power-of-two, non-decimal)
    assert_eq!(format!("{}", rbig!(10 / 3).in_radix(3)), "101/10");
}

#[test]
fn test_relaxed_in_radix() {
    assert_eq!(format!("{}", rbig!(~5 / 3).in_radix(2)), "101/11");
    assert_eq!(format!("{}", rbig!(~83 / 8).in_radix(8)), "123/10");
    assert_eq!(format!("{:#}", rbig!(~255 / 16).in_radix(16)), "FF/10");
}

#[test]
fn test_rbig_in_expanded_terminating() {
    // Terminating fractions
    assert_eq!(format!("{:.4}", rbig!(1 / 2).in_expanded(10)), "0.5000");
    assert_eq!(format!("{:.4}", rbig!(1 / 4).in_expanded(10)), "0.2500");
    assert_eq!(format!("{:.4}", rbig!(1 / 5).in_expanded(10)), "0.2000");
    assert_eq!(format!("{:.4}", rbig!(1 / 8).in_expanded(10)), "0.1250");
    assert_eq!(format!("{:.4}", rbig!(3 / 2).in_expanded(10)), "1.5000");

    // Integers
    assert_eq!(format!("{:.2}", rbig!(0).in_expanded(10)), "0.00");
    assert_eq!(format!("{:.2}", rbig!(1).in_expanded(10)), "1.00");
    assert_eq!(format!("{:.2}", rbig!(-1).in_expanded(10)), "-1.00");
    assert_eq!(format!("{:.2}", rbig!(42).in_expanded(10)), "42.00");

    // Precision = 0
    assert_eq!(format!("{:.0}", rbig!(1 / 2).in_expanded(10)), "1"); // rounds 0.5 -> 1
    assert_eq!(format!("{:.0}", rbig!(1 / 4).in_expanded(10)), "0");
}

#[test]
fn test_rbig_in_expanded_repetend() {
    // Pure repetend
    assert_eq!(format!("{:#}", rbig!(1 / 3).in_expanded(10)), "0.(3)");
    assert_eq!(format!("{:#}", rbig!(1 / 7).in_expanded(10)), "0.(142857)");
    assert_eq!(format!("{:#}", rbig!(1 / 9).in_expanded(10)), "0.(1)");

    // Mixed repetend
    assert_eq!(format!("{:#}", rbig!(1 / 6).in_expanded(10)), "0.1(6)");
    assert_eq!(format!("{:#}", rbig!(5 / 12).in_expanded(10)), "0.41(6)");

    // With precision limiting the repetend display
    assert_eq!(format!("{:#.4}", rbig!(1 / 3).in_expanded(10)), "0.(3)");
    assert_eq!(format!("{:#.4}", rbig!(1 / 6).in_expanded(10)), "0.1(6)");

    // Without alternate flag: no parentheses, just digits
    let out = format!("{:.8}", rbig!(1 / 3).in_expanded(10));
    assert!(out.starts_with("0.33333333"));

    // Negative
    assert_eq!(format!("{:#}", rbig!(-1 / 3).in_expanded(10)), "-0.(3)");
}

#[test]
fn test_rbig_in_expanded_precision_and_rounding() {
    // Custom precision
    assert_eq!(format!("{:.4}", rbig!(1 / 3).in_expanded(10)), "0.3333");
    assert_eq!(format!("{:.10}", rbig!(1 / 7).in_expanded(10)), "0.1428571429"); // rounds up

    // Rounding up at last digit
    assert_eq!(format!("{:.2}", rbig!(2 / 3).in_expanded(10)), "0.67"); // 0.666... -> 0.67
    assert_eq!(format!("{:.3}", rbig!(1 / 6).in_expanded(10)), "0.167"); // 0.1666... -> 0.167

    // Rounding propagates to integer part
    assert_eq!(format!("{:.2}", rbig!(9999 / 10000).in_expanded(10)), "1.00"); // 0.9999 -> rounds to 1.00
}

#[test]
fn test_rbig_in_expanded_scientific() {
    // Basic scientific notation
    assert_eq!(format!("{:.4e}", rbig!(1 / 3).in_expanded(10)), "3.3333e-1");
    assert_eq!(format!("{:.4E}", rbig!(1 / 3).in_expanded(10)), "3.3333E-1");
    assert_eq!(format!("{:.4e}", rbig!(123 / 1).in_expanded(10)), "1.2300e2");
    assert_eq!(format!("{:.4e}", rbig!(-1 / 3).in_expanded(10)), "-3.3333e-1");

    // Zero
    assert_eq!(format!("{:.4e}", rbig!(0).in_expanded(10)), "0.0000e0");
    assert_eq!(format!("{:e}", rbig!(0).in_expanded(10)), "0e0");

    // Small numbers with negative exponent
    assert_eq!(format!("{:.4e}", rbig!(1 / 1000).in_expanded(10)), "1.0000e-3");

    // No precision specifier (uses default)
    let out = format!("{:e}", rbig!(1 / 3).in_expanded(10));
    assert!(out.starts_with("3.333") && out.ends_with("e-1"));
}

#[test]
fn test_rbig_in_expanded_sign_and_flags() {
    // Sign plus
    let out = format!("{:+}", rbig!(1 / 3).in_expanded(10));
    assert!(out.starts_with("+0.333"));
    // Should repeat '3' many times (default precision)
    assert!(out.len() > 5);

    let out = format!("{:+}", rbig!(-1 / 3).in_expanded(10));
    assert!(out.starts_with("-0.333"));

    // Zero with sign plus
    assert_eq!(format!("{:+}", rbig!(0).in_expanded(10)), "+0");
}

#[test]
fn test_rbig_in_expanded_non_decimal() {
    // Base 2 expansion
    assert_eq!(format!("{:.4}", rbig!(1 / 3).in_expanded(2)), "0.0101");
    // Base 8 expansion
    assert_eq!(format!("{:.4}", rbig!(1 / 3).in_expanded(8)), "0.2525");

    // Base 2 with repetend
    assert_eq!(format!("{:#.8}", rbig!(1 / 3).in_expanded(2)), "0.(01)");

    // Scientific in non-decimal base (uses '@' marker)
    assert_eq!(format!("{:.4e}", rbig!(1 / 3).in_expanded(2)), "1.0101@-2");
}

#[test]
fn test_rbig_in_expanded_scientific_underflow() {
    // Bug fix: when int_part has more digits than prec+2, saturating_sub
    // prevents arithmetic underflow.
    assert_eq!(format!("{:.0e}", rbig!(12345).in_expanded(10)), "1e4");
    assert_eq!(format!("{:.1e}", rbig!(99999 / 10).in_expanded(10)), "1.0e4");
}

#[test]
fn test_rbig_in_expanded_scientific_rollover() {
    // Bug fix: rounding 9.99 with prec=2 should roll over from 9.99e0 to 1.00e1.
    assert_eq!(format!("{:.2e}", rbig!(9999 / 1000).in_expanded(10)), "1.00e1");
    // Non-rollover rounding still works correctly.
    assert_eq!(format!("{:.2e}", rbig!(999 / 1000).in_expanded(10)), "9.99e-1");
}

#[test]
fn test_rbig_in_expanded_repetend_low_precision() {
    // Bug fix: with low precision and # flag, cycle detection needs extra digits.
    assert_eq!(format!("{:#.2}", rbig!(5 / 12).in_expanded(10)), "0.41(6)");
    assert_eq!(format!("{:#.3}", rbig!(1 / 7).in_expanded(10)), "0.(142857)");
    assert_eq!(format!("{:#.0}", rbig!(1 / 3).in_expanded(10)), "0.(3)");
}

#[test]
#[should_panic(expected = "radix must be between 2 and 36")]
fn test_rbig_in_expanded_radix_0_panics() {
    let _ = format!("{}", RBig::ONE.in_expanded(0));
}

#[test]
#[should_panic(expected = "radix must be between 2 and 36")]
fn test_rbig_in_expanded_radix_1_panics() {
    let _ = format!("{}", RBig::ONE.in_expanded(1));
}

#[test]
fn test_rbig_in_expanded_scientific_zero_leading() {
    // Bug fix: very small numbers beyond precision should not produce
    // a zero leading significand digit.
    // 1/10^6 with prec=2 — should produce zero, not "0.00e-6".
    let r = RBig::from_parts(1.into(), UBig::from(10u32).pow(6));
    assert_eq!(format!("{:.2e}", r.in_expanded(10)), "1.00e-6");
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
