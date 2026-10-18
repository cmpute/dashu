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
