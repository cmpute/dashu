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
    assert_eq!(RBig::from_str_radix("1//2", 10).unwrap_err(), ParseError::InvalidSyntax);
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
    let _s = format!("{}", RBig::ONE.in_expanded(0));
}

#[test]
#[should_panic(expected = "radix must be between 2 and 36")]
fn test_rbig_in_expanded_radix_1_panics() {
    let _s = format!("{}", RBig::ONE.in_expanded(1));
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
fn test_format_flags_apply_to_whole_fraction() {
    // sign flag: one sign in front of the fraction
    assert_eq!(format!("{:+}", rbig!(1 / 2)), "+1/2");
    assert_eq!(format!("{:+}", rbig!(-1 / 2)), "-1/2");
    assert_eq!(format!("{:+}", rbig!(~3/9)), "+3/9");
    assert_eq!(format!("{:+}", rbig!(2)), "+2");
    assert_eq!(format!("{:+}", rbig!(0)), "+0");
    assert_eq!(format!("{:+b}", rbig!(5 / 3)), "+101/11");
    assert_eq!(format!("{:+#x}", rbig!(-255 / 16)), "-0xff/0x10");
    assert_eq!(format!("{:+#X}", rbig!(~255/16)), "+0xFF/0x10");
    assert_eq!(format!("{:+}", rbig!(83 / 16).in_radix(16)), "+53/10");
    assert_eq!(format!("{:+}", rbig!(83).in_radix(16)), "+53");

    // width, fill and alignment: the fraction is padded as one item
    assert_eq!(format!("{:>6}", rbig!(1 / 2)), "   1/2");
    assert_eq!(format!("{:<6}|", rbig!(1 / 2)), "1/2   |");
    assert_eq!(format!("{:^7}", rbig!(-1 / 2)), " -1/2  ");
    assert_eq!(format!("{:*>+7}", rbig!(~1/2)), "***+1/2");
    assert_eq!(format!("{:>2}", rbig!(1 / 2)), "1/2");
    assert_eq!(format!("{:6}", rbig!(1 / 2)), "   1/2"); // numbers are right-aligned by default
    assert_eq!(format!("{:>3}", rbig!(2)), "  2");
    assert_eq!(format!("{:>8b}", rbig!(5 / 3)), "  101/11");
    assert_eq!(format!("{:>12o}", rbig!(-9 / 8)), "      -11/10");
    assert_eq!(format!("{:<8x}|", rbig!(255 / 16)), "ff/10   |");
    assert_eq!(format!("{:>12}", rbig!(-83 / 16).in_radix(16)), "      -53/10");
    assert_eq!(format!("{:>6}", rbig!(83).in_radix(16)), "    53");
    assert_eq!(format!("{:>#8}", rbig!(~171/16).in_radix(16)), "   AB/10");
}

#[test]
fn test_rbig_in_expanded_rounding_odd_radix() {
    // In an odd radix the half-way point has no finite expansion, so a single guard digit
    // cannot decide the rounding: (radix-1)/2 followed by anything larger is above half.
    assert_eq!(format!("{:.4}", rbig!(1 / 7).in_expanded(3)), "0.0110"); // 0.0102|12..
    assert_eq!(format!("{:.0}", rbig!(5 / 9).in_expanded(3)), "1"); // 0.|12
    assert_eq!(format!("{:.0}", rbig!(4 / 9).in_expanded(3)), "0"); // 0.|11
    assert_eq!(format!("{:.1}", rbig!(1 / 5).in_expanded(3)), "0.1"); // 0.0|121..
    assert_eq!(format!("{:.0e}", rbig!(14 / 9).in_expanded(3)), "2@0"); // 1.|12
    assert_eq!(format!("{:.1e}", rbig!(26 / 27).in_expanded(3)), "1.0@0"); // 2.2|2@-1
    assert_eq!(format!("{:.2}", rbig!(~13/25).in_expanded(5)), "0.23"); // exact: 0.23
    assert_eq!(format!("{:.1}", rbig!(~13/25).in_expanded(5)), "0.3"); // 0.2|3

    // exact ties still round away from zero
    assert_eq!(format!("{:.0}", rbig!(1 / 2).in_expanded(3)), "1");
    assert_eq!(format!("{:.1}", rbig!(1 / 6).in_expanded(3)), "0.1");
    assert_eq!(format!("{:.0}", rbig!(-1 / 2).in_expanded(3)), "-1");

    // every rounded output is within half a unit in the last place of the value
    for radix in [3u8, 5, 7, 9, 10, 15, 16, 35] {
        for den in 1u32..=30 {
            for num in 0..=2 * den {
                let x = RBig::from_parts(num.into(), den.into());
                for prec in 0usize..=4 {
                    let half_ulp = RBig::from_parts(1.into(), UBig::from(radix).pow(prec) * 2u8);

                    let fixed = format!("{:.*}", prec, x.in_expanded(radix));
                    let back = RBig::from_str_expanded(&fixed, radix).unwrap();
                    let err = if back > x { &back - &x } else { &x - &back };
                    assert!(err <= half_ulp, "{x} in radix {radix} printed as {fixed}");
                    // a tie is rounded away from zero
                    assert!(err < half_ulp || back > x, "{x} in radix {radix} printed as {fixed}");

                    if num > 0 {
                        // the last place of the scientific form is `prec` digits below
                        // the leading digit of the printed value
                        let sci = format!("{:.*e}", prec, x.in_expanded(radix));
                        let back = RBig::from_str_expanded(&sci, radix).unwrap();
                        let exp: isize = sci.rsplit(['e', '@']).next().unwrap().parse().unwrap();
                        let half_ulp = half_ulp * RBig::from(radix).pow(exp);
                        let err = if back > x { &back - &x } else { &x - &back };
                        assert!(err <= half_ulp, "{x} in radix {radix} printed as {sci}");
                    }
                }
            }
        }
    }
}

#[test]
fn test_rbig_in_expanded_alternate_long_terminating() {
    // `{:#}` is exact: a finite expansion is printed in full even when it is longer than
    // the default precision or the repetend detection budget.
    for radix in [2u8, 3, 10, 16] {
        for k in [63usize, 64, 65, 100, 127, 128, 129, 300] {
            let x = RBig::from_parts(1.into(), UBig::from(radix).pow(k));
            let printed = format!("{:#}", x.in_expanded(radix));
            assert_eq!(printed.len(), 2 + k.max(64), "radix {radix}, k {k}: {printed}");
            assert_eq!(RBig::from_str_expanded(&printed, radix).unwrap(), x, "{printed}");

            let relaxed = Relaxed::from_parts(3.into(), UBig::from(radix).pow(k) * 3u8);
            assert_eq!(format!("{:#}", relaxed.in_expanded(radix)), printed);
        }
    }
    // 3/2^200 has 200 decimal digits after the point
    let x = RBig::from_parts(3.into(), UBig::ONE << 200);
    let printed = format!("{:#}", x.in_expanded(10));
    assert_eq!(printed.len(), 202);
    assert_eq!(RBig::from_str_decimal(&printed).unwrap(), x);
    // the precision is only a minimal width for exact output
    assert_eq!(format!("{:#.2}", rbig!(1 / 8).in_expanded(10)), "0.125");
    assert_eq!(format!("{:#.5}", rbig!(1 / 8).in_expanded(10)), "0.12500");
    assert_eq!(format!("{:#.0}", rbig!(7).in_expanded(10)), "7");
    assert_eq!(format!("{:#.2}", rbig!(7).in_expanded(10)), "7.00");
    // without the alternate flag the output is rounded as before
    assert_eq!(format!("{:.2}", rbig!(1 / 8).in_expanded(10)), "0.13");
    let x = RBig::from_parts(1.into(), UBig::from(10u8).pow(65));
    assert_eq!(format!("{}", x.in_expanded(10)), format!("0.{}", "0".repeat(64)));
}

#[test]
fn test_rbig_in_expanded_repetend_filling_budget() {
    // prefix + period == 128, exactly the default detection budget
    for (x, radix) in [(rbig!(1 / 512), 3u8), (rbig!(1 / 5000), 6u8)] {
        let printed = format!("{:#}", x.in_expanded(radix));
        assert!(printed.contains('('), "{x} in radix {radix}: {printed}");
        assert_eq!(RBig::from_str_expanded(&printed, radix).unwrap(), x);
    }
    // 1/256 in radix 3 has period 64
    let printed = format!("{:#}", rbig!(1 / 256).in_expanded(3));
    assert!(printed.contains('('));
    assert_eq!(RBig::from_str_expanded(&printed, 3).unwrap(), rbig!(1 / 256));

    // 1/149 has period 148: {:#.147} gives a budget of exactly 148 digits
    let x = rbig!(1 / 149);
    for prec in [147usize, 148, 200] {
        let printed = format!("{:#.*}", prec, x.in_expanded(10));
        assert!(printed.contains('('), "{printed}");
        assert_eq!(RBig::from_str_decimal(&printed).unwrap(), x);
    }
    // beyond the budget the output falls back to the rounded form
    assert_eq!(format!("{:#}", x.in_expanded(10)), format!("{}", x.in_expanded(10)));
    assert_eq!(format!("{:#.146}", x.in_expanded(10)), format!("{:.146}", x.in_expanded(10)));
}

#[test]
fn test_rbig_in_expanded_alternate_roundtrip() {
    // every value whose expansion is resolved within the budget reads back exactly
    for radix in [2u8, 3, 6, 7, 10, 12, 16, 36] {
        for den in 1u32..=40 {
            for num in 0..=den + 3 {
                let x = RBig::from_parts(num.into(), den.into());
                let printed = format!("{:#}", x.in_expanded(radix));
                let back = RBig::from_str_expanded(&printed, radix).unwrap();
                assert_eq!(back, x, "{x} in radix {radix} printed as {printed}");
            }
        }
    }
}

#[test]
fn test_rbig_in_expanded_scientific_zero_marker() {
    // `e` is only the exponent marker in radix 10 (it is a digit from radix 15 on)
    assert_eq!(format!("{:e}", rbig!(0).in_expanded(10)), "0e0");
    assert_eq!(format!("{:E}", rbig!(0).in_expanded(10)), "0E0");
    for radix in [2u8, 8, 16, 36] {
        assert_eq!(format!("{:e}", rbig!(0).in_expanded(radix)), "0@0");
        assert_eq!(format!("{:E}", rbig!(0).in_expanded(radix)), "0@0");
        assert_eq!(format!("{:.3e}", rbig!(0).in_expanded(radix)), "0.000@0");
        assert_eq!(format!("{:+.1E}", rbig!(~0).in_expanded(radix)), "+0.0@0");
        for x in [rbig!(0), rbig!(1), rbig!(-5 / 8)] {
            for printed in [
                format!("{:e}", x.in_expanded(radix)),
                format!("{:E}", x.in_expanded(radix)),
                format!("{:.3e}", x.in_expanded(radix)),
            ] {
                assert_eq!(RBig::from_str_expanded(&printed, radix).unwrap(), x, "{printed}");
            }
        }
    }
}

#[test]
fn test_relaxed_from_str_radix() {
    assert_eq!(Relaxed::from_str_radix("", 2).unwrap_err(), ParseError::NoDigits);
    assert_eq!(Relaxed::from_str_radix("+", 2).unwrap_err(), ParseError::NoDigits);
    assert_eq!(Relaxed::from_str_radix("/", 2).unwrap_err(), ParseError::NoDigits);
    assert_eq!(Relaxed::from_str_radix("2/", 10).unwrap_err(), ParseError::NoDigits);
    assert_eq!(Relaxed::from_str_radix("/2", 10).unwrap_err(), ParseError::NoDigits);
    assert_eq!(Relaxed::from_str_radix("1//2", 10).unwrap_err(), ParseError::InvalidSyntax);
    assert_eq!(Relaxed::from_str_radix("0", 2).unwrap(), rbig!(~0));
    assert_eq!(Relaxed::from_str_radix("1", 2).unwrap(), rbig!(~1));
    assert_eq!(Relaxed::from_str_radix("-1", 2).unwrap(), rbig!(~-1));
    assert_eq!(Relaxed::from_str_radix("1/2", 10).unwrap(), rbig!(~1/2));
    assert_eq!(Relaxed::from_str_radix("-1/2", 10).unwrap(), rbig!(~-1/2));
    assert_eq!(Relaxed::from_str_radix("+1/-2", 10).unwrap(), rbig!(~-1/2));
    assert_eq!(Relaxed::from_str_radix("-1/-2", 10).unwrap(), rbig!(~1/2));
}

#[test]
fn test_from_str_zero_denominator() {
    // A zero denominator is an error rather than an `Ok` value violating the type's
    // invariant (RBig) or a panic (Relaxed).
    for s in ["1/0", "0/0", "-7/0", "5/-0", "-3/-0", "1/+0", "1/00"] {
        assert_eq!(RBig::from_str_radix(s, 10).unwrap_err(), ParseError::InvalidSyntax, "{s}");
        assert_eq!(s.parse::<RBig>().unwrap_err(), ParseError::InvalidSyntax, "{s}");
        assert_eq!(Relaxed::from_str_radix(s, 10).unwrap_err(), ParseError::InvalidSyntax, "{s}");
        assert_eq!(s.parse::<Relaxed>().unwrap_err(), ParseError::InvalidSyntax, "{s}");
        assert_eq!(RBig::from_str_radix(s, 16).unwrap_err(), ParseError::InvalidSyntax, "{s}");
        assert_eq!(Relaxed::from_str_radix(s, 16).unwrap_err(), ParseError::InvalidSyntax, "{s}");
    }
    for s in ["1/0", "0x1/0x0", "0b1/0", "-0o7/-0o0", "0/0"] {
        assert_eq!(
            RBig::from_str_with_radix_prefix(s).unwrap_err(),
            ParseError::InvalidSyntax,
            "{s}"
        );
        assert_eq!(
            Relaxed::from_str_with_radix_prefix(s).unwrap_err(),
            ParseError::InvalidSyntax,
            "{s}"
        );
    }
    // a zero numerator is of course fine
    assert_eq!("0/5".parse::<RBig>().unwrap(), rbig!(0));
    assert_eq!("0/5".parse::<Relaxed>().unwrap(), rbig!(~0));
}

#[test]
fn test_from_str_expanded_sign_inside_literal() {
    // a sign is only accepted in front of the literal (and in the exponent)
    for (s, radix) in [
        (".+1", 10u8),
        ("0.+1", 10),
        ("0.(+3)", 10),
        ("1.+1", 2),
        ("0.-1", 10),
        ("0.(-3)", 10),
        ("++1", 10),
        ("-+1", 10),
        ("+-1", 10),
        ("--1.5", 10),
    ] {
        assert_eq!(RBig::from_str_expanded(s, radix).unwrap_err(), ParseError::InvalidDigit, "{s}");
        assert_eq!(
            Relaxed::from_str_expanded(s, radix).unwrap_err(),
            ParseError::InvalidDigit,
            "{s}"
        );
    }
    assert_eq!(RBig::from_str_expanded("+.1", 10).unwrap(), rbig!(1 / 10));
    assert_eq!(RBig::from_str_expanded("-1.5e+1", 10).unwrap(), rbig!(-15));
}

#[test]
fn test_from_str_expanded_separator_only_repetend() {
    // a repetend that holds only separators has no digits, just like `()`
    for (s, radix) in [
        ("0.1()", 2u8),
        ("0.1(_)", 2),
        ("12.5(__)", 10),
        ("0.(_)", 10),
    ] {
        assert_eq!(
            RBig::from_str_expanded(s, radix).unwrap_err(),
            ParseError::InvalidSyntax,
            "{s}"
        );
        assert_eq!(
            Relaxed::from_str_expanded(s, radix).unwrap_err(),
            ParseError::InvalidSyntax,
            "{s}"
        );
    }
    assert_eq!(RBig::from_str_expanded("0.1(0)", 2).unwrap(), rbig!(1 / 2));
    assert_eq!(RBig::from_str_expanded("0.(1_6)", 10).unwrap(), rbig!(16 / 99));
}

#[test]
fn test_from_str_expanded_zero_with_large_exponent() {
    // zero scaled by any power of the radix is zero; the scale factor must not be built
    for (s, radix) in [
        ("0e-1000", 10u8),
        ("0e-41643224064", 10),
        ("0@-41643224064", 6),
        ("0e9223372036854775807", 10),
        ("-0.000e-9223372036854775808", 10),
        ("0.(0)@41643224064", 16),
    ] {
        assert_eq!(RBig::from_str_expanded(s, radix).unwrap(), rbig!(0), "{s}");
        assert_eq!(Relaxed::from_str_expanded(s, radix).unwrap(), rbig!(~0), "{s}");
    }
    assert_eq!(RBig::from_str_decimal("0e-41643224064").unwrap(), rbig!(0));
}
