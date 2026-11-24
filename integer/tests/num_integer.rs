#![cfg(feature = "num-integer")]

use dashu_int::{IBig, UBig};
use num_integer_v01::Integer;

// Keep the deprecated entry point covered: it delegates to Dashu's inherent
// divisibility check, while the trait's parity methods use the two's-complement bit test.
#[test]
#[allow(deprecated)]
fn divisibility_and_signed_parity() {
    for (n, d, divisible) in [
        (0, 3, true),
        (-12, 3, true),
        (12, -3, true),
        (-13, -3, false),
    ] {
        assert_eq!(Integer::divides(&IBig::from(n), &IBig::from(d)), divisible);
    }
    for n in [-5, -4, -1, 0, 1, 4, 5] {
        let value = IBig::from(n);
        assert_eq!(Integer::is_even(&value), n % 2 == 0);
        assert_eq!(Integer::is_odd(&value), n % 2 != 0);
    }
    let even = -(IBig::ONE << 256);
    let odd = &even - 1u8;
    assert!(Integer::is_even(&even));
    assert!(Integer::is_odd(&odd));

    let divisor = UBig::ONE << 256;
    assert!(Integer::divides(&UBig::ZERO, &divisor));
    assert!(Integer::divides(&(&divisor * 3u8), &divisor));
    assert!(!Integer::divides(&(&divisor + 1u8), &divisor));
}

// When |n| < |d| the truncated quotient is zero and carries no sign, so the floor
// is decided by the operand signs alone. The divisors range from one word to
// several words on every supported word size, under every sign combination.
#[test]
fn floored_division_with_zero_quotient() {
    for (n, d) in [
        (IBig::from(-7), IBig::from(9u8)),
        (IBig::from(7), IBig::from(-9i8)),
        (IBig::from(-7), IBig::from(-9i8)),
        (IBig::from(7), IBig::from(9u8)),
        (-IBig::ONE, IBig::ONE << 64),
        (IBig::ONE, -(IBig::ONE << 64)),
        (-IBig::ONE, -(IBig::ONE << 64)),
        (IBig::ONE, IBig::ONE << 64),
        (-(IBig::ONE << 100), (IBig::ONE << 200) + 1u8),
        (IBig::ONE << 100, -((IBig::ONE << 200) + 1u8)),
        (-(IBig::ONE << 100), -((IBig::ONE << 200) + 1u8)),
        (IBig::ONE << 100, (IBig::ONE << 200) + 1u8),
    ] {
        let expected = if n.sign() != d.sign() {
            (-IBig::ONE, &d + &n)
        } else {
            (IBig::ZERO, n.clone())
        };
        assert_eq!(Integer::div_floor(&n, &d), expected.0, "{n} / {d}");
        assert_eq!(Integer::div_mod_floor(&n, &d), expected, "{n} / {d}");
        // the ceiling is one above the floor while the remainder is non-zero
        assert_eq!(Integer::div_ceil(&n, &d), expected.0 + 1u8, "{n} / {d}");
    }
}

#[test]
fn ibig_lcm_is_non_negative() {
    for a in -12i64..=12 {
        for b in -12i64..=12 {
            let (x, y) = (IBig::from(a), IBig::from(b));
            let expected = IBig::from(Integer::lcm(&a, &b));
            assert_eq!(x.lcm(&y), expected, "lcm({}, {})", a, b);
            assert_eq!(x.gcd_lcm(&y), (IBig::from(Integer::gcd(&a, &b)), expected));
        }
    }
    let big = IBig::ONE << 200;
    assert_eq!((-&big).lcm(&IBig::from(3)), &big * 3);
    assert_eq!(big.lcm(&IBig::from(-6)), &big * 3);
}

#[test]
fn gcd_of_two_zeros_is_zero() {
    // The inherent gcd of dashu rejects gcd(0, 0), but the trait from num-integer
    // defines it as zero (as the identity of gcd, e.g. for folding over a sequence).
    assert_eq!(Integer::gcd(&UBig::ZERO, &UBig::ZERO), UBig::ZERO);
    assert_eq!(Integer::gcd(&IBig::ZERO, &IBig::ZERO), IBig::ZERO);
    assert_eq!(Integer::gcd_lcm(&UBig::ZERO, &UBig::ZERO), (UBig::ZERO, UBig::ZERO));
    assert_eq!(Integer::gcd_lcm(&IBig::ZERO, &IBig::ZERO), (IBig::ZERO, IBig::ZERO));
    assert_eq!(Integer::lcm(&IBig::ZERO, &IBig::ZERO), IBig::ZERO);

    let ext = Integer::extended_gcd(&IBig::ZERO, &IBig::ZERO);
    assert_eq!(ext.gcd, IBig::ZERO);
    assert_eq!(Integer::extended_gcd(&UBig::ZERO, &UBig::ZERO).gcd, UBig::ZERO);
    let (ext, lcm) = Integer::extended_gcd_lcm(&IBig::ZERO, &IBig::ZERO);
    assert_eq!((ext.gcd, lcm), (IBig::ZERO, IBig::ZERO));

    // a single zero operand
    assert_eq!(Integer::gcd(&IBig::ZERO, &IBig::from(-5)), IBig::from(5));
    assert_eq!(Integer::gcd(&UBig::from(5u8), &UBig::ZERO), UBig::from(5u8));
    assert_eq!(Integer::gcd_lcm(&IBig::ZERO, &IBig::from(5)), (IBig::from(5), IBig::ZERO));
    let folded = [0i32, 12, 0, -18]
        .iter()
        .fold(IBig::ZERO, |g, &x| Integer::gcd(&g, &IBig::from(x)));
    assert_eq!(folded, IBig::from(6));
}
