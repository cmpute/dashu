use crate::number::Natural;
use std::{collections::BTreeMap, fmt::Debug, str::FromStr, sync::Mutex, sync::OnceLock};

/// Bases for the Miller-Rabin primality test. The first 12 primes jointly
/// give a deterministic answer below 3.18e24 (~81 bits), which covers the
/// small factor; for the large factor they are a fixed, reproducible basis.
const MILLER_RABIN_BASES: [u32; 12] = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37];

/// Runs of consecutive small primes, each run combined into a single u64
/// product. One gcd between the (huge) candidate and each run replaces a
/// division per prime: a gcd above 1 means some prime in the run divides
/// the candidate, so the Miller-Rabin scan doubles as a stress test of gcd
/// between big numbers and small ones. Every value this task tests is
/// larger than any prime in the runs, so a hit always means composite.
const SMALL_PRIME_RUNS: [u64; 5] = [
    2 * 3 * 5 * 7 * 11 * 13 * 17 * 19 * 23 * 29 * 31 * 37 * 41 * 43 * 47,
    53 * 59 * 61 * 67 * 71 * 73 * 79 * 83 * 89 * 97,
    101 * 103 * 107 * 109 * 113 * 127 * 131 * 137 * 139,
    149 * 151 * 157 * 163 * 167 * 173 * 179 * 181,
    191 * 193 * 197 * 199 * 211 * 223 * 227 * 229,
];

/// Seed for the small prime factor. Its ~31-bit magnitude keeps the rho
/// walk at ~2^16 steps regardless of `n`, so the per-iteration cost scales
/// with the operand width instead of the factor size.
const SMALL_FACTOR_SEED: u32 = 1357913579;

/// Steps of the rho walk accumulated between GCD evaluations. Batching keeps
/// the GCD count (and its cost) from dominating the modular multiplications.
const RHO_BATCH: u64 = 128;

/// Factor a semiprime with one `n`-bit prime factor and one ~31-bit prime
/// factor, exercising GCD, division and modular arithmetic at width `n`.
///
/// The large factor is grown from the fixed seed by a Miller-Rabin prime
/// search and cached per process (a search at thousands of bits is
/// expensive; the median-of-5 sampling in `exec` absorbs the one-time
/// cost). Each iteration then re-derives the small factor (a small
/// Miller-Rabin search), multiplies both into the semiprime, and recovers
/// them from the product alone using Pollard's rho with Brent's cycle
/// detection: ~2^16 modular multiplications and GCDs at width `n` (the
/// birthday bound is set by the small factor), one exact division, and the
/// Miller-Rabin verification of both factors.
///
/// Everything is seeded deterministically, so the output `p * q` (with
/// p <= q) is identical across libraries.
///
/// The recommended ladder is `n` = 200 / 1000 / 5000 (~0.04 / 0.2 / 2.7 s
/// per iteration, matching the other tasks in this folder); the one-time
/// prime search before the first iteration costs well under a second at
/// these sizes and is absorbed by the median-of-5 sampling in `exec`.
pub(crate) fn calculate<T: Natural>(n: u32) -> String
where
    <T as FromStr>::Err: Debug,
{
    assert!(n >= 16, "the large prime factor needs at least 16 bits");

    let p = T::from_str(&large_prime_str(n)).unwrap();
    let q = next_probable_prime::<T>(&T::from(SMALL_FACTOR_SEED));
    let semiprime = p.mul_ref(&q);

    let f = pollard_rho(&semiprime);
    let g = semiprime.clone() / &f;
    assert!(f.mul_ref(&g) == semiprime);
    assert!(is_probable_prime(&f) && is_probable_prime(&g));

    if f <= g {
        format!("{} * {}", f, g)
    } else {
        format!("{} * {}", g, f)
    }
}

/// Decimal representation of the `n`-bit prime factor, generated once with
/// [`dashu::Natural`] and shared as a plain string, so every library factors
/// the exact same number regardless of which one runs first.
fn large_prime_str(bits: u32) -> String {
    static PRIMES: OnceLock<Mutex<BTreeMap<u32, String>>> = OnceLock::new();
    PRIMES
        .get_or_init(|| Mutex::new(BTreeMap::new()))
        .lock()
        .unwrap()
        .entry(bits)
        .or_insert_with(|| {
            next_probable_prime::<dashu::Natural>(&seed::<dashu::Natural>(bits)).to_string()
        })
        .clone()
}

/// An odd number of at least `bits` bits, built from a fixed digit pattern.
fn seed<T: Natural>(bits: u32) -> T
where
    <T as FromStr>::Err: Debug,
{
    // This many decimal digits carry at least `bits` bits (log_10(2) ~ 0.301).
    let digits = (f64::from(bits) / 10f64.log2()).ceil() as u32 + 1;
    let pattern = "13579"; // odd digits only, so the seed is always odd
    let mut input = String::new();
    while input.len() < digits as usize {
        input.push_str(pattern);
    }
    input.truncate(digits as usize);
    T::from_str(&input).unwrap()
}

/// Smallest probable prime >= start, scanning odd numbers only.
fn next_probable_prime<T: Natural>(start: &T) -> T {
    let two = T::from(2u32);
    let mut candidate = start.clone();
    if !candidate.is_odd() {
        candidate = candidate + T::from(1u32);
    }
    while !is_probable_prime(&candidate) {
        candidate = candidate + &two;
    }
    candidate
}

/// Miller-Rabin probable prime test with the fixed bases above, preceded by
/// a gcd filter against [`SMALL_PRIME_RUNS`]. Every input used here exceeds
/// the largest base and every prime in the runs.
fn is_probable_prime<T: Natural>(n: &T) -> bool {
    let one = T::from(1u32);
    let two = T::from(2u32);
    if n < &two {
        return false;
    }
    if !n.is_odd() {
        return n == &two;
    }

    // One gcd per run of packed primes rejects most of the candidates
    // scanned during a prime search before any modular exponentiation runs.
    for &run in SMALL_PRIME_RUNS.iter() {
        if n.gcd(&T::from_u64(run)) != one {
            return false;
        }
    }

    // Factor n - 1 as d * 2^s with d odd.
    let n_minus_one = n.clone() - &one;
    let mut d = n_minus_one.clone();
    let mut s = 0u32;
    while !d.is_odd() {
        d = d / &two;
        s += 1;
    }

    'bases: for &base in MILLER_RABIN_BASES.iter() {
        let mut x = T::from(base).pow_mod(&d, n);
        if x == one || x == n_minus_one {
            continue 'bases;
        }
        for _ in 1..s {
            x = x.mul_mod(&x, n);
            if x == n_minus_one {
                continue 'bases;
            }
        }
        return false; // this base witnesses compositeness
    }
    true
}

/// Find a non-trivial factor of an odd composite with at least two distinct
/// prime factors, using Pollard's rho with Brent's cycle detection.
fn pollard_rho<T: Natural>(n: &T) -> T {
    let mut c = T::from(1u32);
    loop {
        match rho_attempt(n, &c) {
            Some(factor) => return factor,
            // The walk collapsed onto a multiple of n before exposing a
            // proper factor: retry with the next polynomial.
            None => c = c + T::from(1u32),
        }
    }
}

/// One rho attempt walking x -> x^2 + c (mod n). Differences against the
/// saved position are accumulated for RHO_BATCH steps between GCDs; when a
/// GCD comes out non-trivial, the batch is replayed one step at a time to
/// pin down the factor. Returns None if the walk only ever hits multiples of
/// n (the caller retries with the next c).
fn rho_attempt<T: Natural>(n: &T, c: &T) -> Option<T> {
    let one = T::from(1u32);
    let step = |x: &T| -> T {
        // x^2 mod n + c < 2n, so one conditional subtraction reduces it.
        let t = x.mul_mod(&x.clone(), n) + c;
        if &t >= n {
            t - n
        } else {
            t
        }
    };

    let mut y = T::from(2u32);
    let mut r: u64 = 1;
    loop {
        let x = y.clone();
        for _ in 0..r {
            y = step(&y);
        }
        let mut k = 0;
        while k < r {
            let batch_start = y.clone();
            let limit = (r - k).min(RHO_BATCH);
            let mut q = T::from(1u32);
            for _ in 0..limit {
                y = step(&y);
                q = q.mul_mod(&sub_mod(&y, &x, n), n);
            }
            if q.gcd(n) != one {
                // A factor is buried in this batch: replay it step by step.
                let mut y = batch_start;
                loop {
                    y = step(&y);
                    let d = sub_mod(&y, &x, n).gcd(n);
                    if d != one {
                        if &d == n {
                            return None;
                        }
                        return Some(d);
                    }
                }
            }
            k += limit;
        }
        r *= 2;
    }
}

/// (a - b) mod m for values already reduced modulo m.
fn sub_mod<T: Natural>(a: &T, b: &T, m: &T) -> T {
    if a >= b {
        a.clone() - b
    } else {
        m.clone() - (b.clone() - a)
    }
}
