//! Hashing that agrees with Python's own numeric types.
//!
//! Python requires that objects which compare equal have the same hash, and the dashu numbers
//! compare equal to the `int`, `float`, `fractions.Fraction`, `decimal.Decimal` and `complex`
//! values they represent. So the hash of a dashu number is computed by the rule that Python
//! uses for all of its numeric types (see "Hashing of numeric types" in the Python docs): the
//! hash of a rational `p/q` is `p * q^-1` modulo the prime `P = 2^61 - 1` (`2^31 - 1` on 32-bit
//! platforms), with the sign of the number, and with `-1` replaced by `-2`.

use dashu::base::{Sign, UnsignedAbs};
use dashu::float::{FBig, Repr, Word, round::Round};
use dashu::integer::{IBig, UBig};
use dashu::rational::RBig;

/// `sys.hash_info.modulus`
const MODULUS: u64 = if usize::BITS >= 64 {
    (1 << 61) - 1
} else {
    (1 << 31) - 1
};
/// `sys.hash_info.inf`
const HASH_INF: isize = 314159;
/// `sys.hash_info.imag`
const HASH_IMAG: usize = 1000003;

fn mul_mod(a: u64, b: u64) -> u64 {
    ((a as u128 * b as u128) % MODULUS as u128) as u64
}

fn pow_mod(mut base: u64, mut exp: u64) -> u64 {
    let mut result = 1;
    base %= MODULUS;
    while exp > 0 {
        if exp & 1 == 1 {
            result = mul_mod(result, base);
        }
        base = mul_mod(base, base);
        exp >>= 1;
    }
    result
}

/// The inverse modulo the (prime) modulus, for an input that is not a multiple of it.
fn inv_mod(a: u64) -> u64 {
    pow_mod(a, MODULUS - 2)
}

fn reduce(n: &UBig) -> u64 {
    u64::try_from(n % UBig::from(MODULUS)).unwrap()
}

/// Attach the sign to a reduced hash, avoiding the value -1 that CPython reserves for errors.
fn finish(sign: Sign, magnitude: u64) -> isize {
    let hash = match sign {
        Sign::Positive => magnitude as isize,
        Sign::Negative => -(magnitude as isize),
    };
    if hash == -1 { -2 } else { hash }
}

fn infinity(sign: Sign) -> isize {
    match sign {
        Sign::Positive => HASH_INF,
        Sign::Negative => -HASH_INF,
    }
}

/// The hash of an integer, equal to the hash of the same Python `int`.
pub fn hash_ibig(n: &IBig) -> isize {
    finish(n.sign(), reduce(&n.unsigned_abs()))
}

/// The hash of a rational number, equal to the hash of the same `fractions.Fraction`.
pub fn hash_rbig(r: &RBig) -> isize {
    let numerator = r.numerator();
    let denominator = reduce(r.denominator());
    if denominator == 0 {
        // the denominator has no inverse: Python uses the hash of infinity
        return infinity(numerator.sign());
    }
    let magnitude = mul_mod(reduce(&numerator.unsigned_abs()), inv_mod(denominator));
    finish(numerator.sign(), magnitude)
}

/// The hash of a float `significand * B^exponent`, equal to the hash of the Python `float`
/// or `decimal.Decimal` (and of the `Fraction`) with the same value.
pub fn hash_repr<const B: Word>(repr: &Repr<B>) -> isize {
    if repr.is_infinite() {
        return infinity(repr.sign());
    }
    let significand = repr.significand();
    // (a `Word` is narrower than `u64` on 32-bit platforms)
    #[allow(clippy::unnecessary_cast)]
    let base = (B as u64) % MODULUS;
    let exponent = repr.exponent();
    let scale = if exponent >= 0 {
        pow_mod(base, exponent as u64)
    } else {
        pow_mod(inv_mod(base), exponent.unsigned_abs() as u64)
    };
    let magnitude = mul_mod(reduce(&significand.unsigned_abs()), scale);
    finish(significand.sign(), magnitude)
}

/// The hash of a float, see [`hash_repr`].
pub fn hash_fbig<R: Round, const B: Word>(f: &FBig<R, B>) -> isize {
    hash_repr(f.repr())
}

/// The hash of a complex number with the given parts, equal to the hash of the same Python
/// `complex` (and to the hash of the real part alone when the imaginary part is zero).
pub fn hash_complex<const B: Word>(re: &Repr<B>, im: &Repr<B>) -> isize {
    let combined = (hash_repr(re) as usize)
        .wrapping_add(HASH_IMAG.wrapping_mul(hash_repr(im) as usize)) as isize;
    if combined == -1 { -2 } else { combined }
}
