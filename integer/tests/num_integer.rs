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

#[test]
fn ibig_floor_division_small_quotient() {
    // |a| < |b| with different signs: the truncated quotient is zero (which has no
    // sign), while the floor quotient is -1
    assert_eq!(IBig::from(1).div_floor(&IBig::from(-2)), IBig::from(-1));
    assert_eq!(IBig::from(-1).div_floor(&IBig::from(3)), IBig::from(-1));
    assert_eq!(IBig::from(-5).div_floor(&IBig::from(7)), IBig::from(-1));
    assert_eq!(IBig::from(-1).div_mod_floor(&IBig::from(3)), (IBig::from(-1), IBig::from(2)));
    assert_eq!(IBig::from(1).div_mod_floor(&IBig::from(-2)), (IBig::from(-1), IBig::from(-1)));

    for a in -25i64..=25 {
        for b in (-25i64..=25).filter(|&b| b != 0) {
            let (x, y) = (IBig::from(a), IBig::from(b));
            assert_eq!(x.div_floor(&y), IBig::from(Integer::div_floor(&a, &b)), "{} / {}", a, b);
            assert_eq!(x.mod_floor(&y), IBig::from(Integer::mod_floor(&a, &b)), "{} % {}", a, b);
            let (q, r) = x.div_mod_floor(&y);
            assert_eq!(q * &y + r, x);
            assert_eq!(
                Integer::div_ceil(&x, &y),
                IBig::from(Integer::div_ceil(&a, &b)),
                "{} / {}",
                a,
                b
            );
        }
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
