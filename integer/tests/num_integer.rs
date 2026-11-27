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
// is decided by the operand signs alone. Cover single-word, double-word and multi-word
// divisors under every sign combination.
#[test]
fn floored_division_with_zero_quotient() {
    let (word, medium, large) = (IBig::ONE << 64, IBig::ONE << 100, (IBig::ONE << 200) + 1u8);
    for (n, d) in [
        (IBig::from(-7), IBig::from(9u8)),
        (IBig::from(7), IBig::from(-9i8)),
        (IBig::from(-7), IBig::from(-9i8)),
        (IBig::from(7), IBig::from(9u8)),
        (-IBig::ONE, word.clone()),
        (IBig::ONE, -word.clone()),
        (-IBig::ONE, -word.clone()),
        (IBig::ONE, word),
        (-medium.clone(), large.clone()),
        (medium.clone(), -large.clone()),
        (-medium.clone(), -large.clone()),
        (medium, large),
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
