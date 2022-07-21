//! TODO: Extension to ibig that should be upstreamed.

use dashu_base::UnsignedAbs;
use dashu_int::{ibig, ubig, IBig, UBig};

// REF: https://en.wikipedia.org/wiki/Exponential_search
//      https://people.csail.mit.edu/jaffer/III/ilog.pdf
// should use a constant Log2_10 to speed up the case of radix 10.
//
// If log_rem(x, base) = (e, r), then x = base^e + r and 0 <= r < (base-1) * base^e
pub fn log_rem(x: &UBig, base: usize) -> (usize, UBig) {
    assert!(x > &ubig!(0));

    // short cut for base 2
    if base == 2 {
        let bits = x.bit_len();
        let mut rem = x.clone();
        rem.clear_bit(bits - 1);
        return (bits - 1, rem);
    }

    // very naive algorithm by now
    let mut counter = 0;
    let mut exp = ubig!(1);

    loop {
        let new_exp = &exp * base;
        if &new_exp > x {
            break (counter, x - exp);
        }
        exp = new_exp;
        counter += 1;
    }
}

/// Calculate log_base(x^exp), return the floored value and remainder.
pub fn log_pow_rem(x: &UBig, exp: usize, base: usize) -> (usize, UBig) {
    // FIXME: this should be optimizable based on log_base(x^exp) = exp * log_base(x)
    log_rem(&x.pow(exp), base)
}

/// Calculate log_base(x^exp), return the floored value
pub fn log_pow(x: &UBig, exp: usize, base: usize) -> usize {
    log_pow_rem(x, exp, base).0
}

#[inline]
pub fn log(x: &UBig, base: usize) -> usize {
    log_rem(x, base).0
}

pub fn magnitude(x: &IBig) -> UBig {
    x.clone().unsigned_abs()
}

pub fn remove_pow(x: &mut IBig, base: &IBig) -> UBig {
    let mut counter = ubig!(0);
    while &*x % base == ibig!(0) {
        *x /= base;
        counter += 1u8;
    }
    return counter;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_rem() {
        assert_eq!(log_rem(&ubig!(1), 2), (0, ubig!(0)));
        assert_eq!(log_rem(&ubig!(2), 2), (1, ubig!(0)));
        assert_eq!(log_rem(&ubig!(3), 2), (1, ubig!(1)));
        assert_eq!(log_rem(&ubig!(3), 10), (0, ubig!(2)));
        assert_eq!(log_rem(&ubig!(13), 10), (1, ubig!(3)));
    }
}
