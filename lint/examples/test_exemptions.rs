// compile-flags: --test
use dashu_int::UBig;

fn main() {}

#[test]
fn independent_operator_checks() {
    let n = UBig::from(12u8);
    let d = UBig::from(3u8);
    let q = &n / &d;
    let r = &n % &d;
    assert_eq!(q * &d + r, n);
    assert_eq!((&n * &n), n.sqr());
    assert_eq!((&n % &d).is_zero(), n.is_multiple_of(&d));
    assert_eq!((&n & UBig::ONE).is_zero(), true);
}

#[cfg(test)]
mod helpers {
    use super::*;
    #[allow(dead_code)]
    fn reference(n: &UBig, d: &UBig) -> UBig {
        let q = n / d;
        let r = n % d;
        let _ = (n % d).is_zero();
        let _ = n * n;
        q + r
    }
}
