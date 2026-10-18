use dashu_int::{IBig, UBig};

fn patterns(n: &UBig, d: &UBig, a: &IBig, b: &IBig) -> Option<UBig> {
    let _ = (n % d).is_zero();
    let _ = UBig::ZERO == n % d;
    let _ = a % b != IBig::ZERO;
    let _ = (a & IBig::ONE).is_zero();
    let _ = (a & IBig::ONE).is_one();
    let _ = (UBig::ONE & n) != UBig::ZERO;
    let q = n / d;
    let r = n % d;
    std::hint::black_box((q, r));
    let r = n % d;
    let q = n / d;
    std::hint::black_box((q, r));
    // One combined diagnostic, rather than a second divisibility warning.
    if n % d != UBig::ZERO {
        return None;
    }
    let q = n / d;
    Some(q)
}

fn negative_cases(mut n: UBig, d: UBig) {
    // Not a divisibility check.
    let _ = (&n % &d).is_one();
    let _ = (5u64 % 3u64) == 0;
    let _ = (5u64 & 1u64) == 0;
    // Not adjacent: n changes between the two operations.
    let q = &n / &d;
    n += 1u8;
    let r = &n % &d;
    std::hint::black_box((q, r));
    // Shadowing also changes the operand's binding.
    let n = &n / &d;
    let r = &n % &d;
    std::hint::black_box(r);
    // Different divisors, non-constant masks, and primitive divisors are excluded.
    let q = &n / &d;
    let r = &n % &n;
    std::hint::black_box((q, r));
    let _ = (&n & &d).is_zero();
    let _ = (&n % 3u8) == 0;
}

fn main() {
    let _ = patterns(&12u8.into(), &3u8.into(), &(-12).into(), &3.into());
    negative_cases(12u8.into(), 3u8.into());
}
