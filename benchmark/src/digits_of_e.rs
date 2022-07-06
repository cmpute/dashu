use crate::number::Number;
use std::f64;

/// n digits of the number e.
pub(crate) fn calculate<T: Number>(n: u32) -> String {
    assert!(n > 0);
    // Find k such that log_10 k! is approximately n + 50.
    // This makes 1 / k! and subsequent terms small enough.
    // Use Stirling's approximation: ln k! ~= k ln k - k + 0.5 * ln(2*pi*k).
    let k: u32 = binary_search(|k| {
        k > 0 && {
            let k = k as f64;
            let ln_k_factorial = k * k.ln() - k + 0.5 * (f64::consts::TAU * k).ln();
            let log_10_k_factorial = ln_k_factorial / f64::consts::LN_10;
            log_10_k_factorial >= (n + 50) as f64
        }
    });

    // 1/1! + ... + 1/(k-1)!
    let (p, q) = sum_terms::<T>(0, k - 1);
    // Add 1/0! = 1.
    let p = p + &q;
    // e ~= p/q.
    // Calculate p/q * 10^(n-1) to get the answer as an integer.
    let answer_int = p * T::from(10u32).pow(n - 1) / q;
    let mut answer = answer_int.to_string();
    // Insert the decimal period.
    answer.insert(1, '.');
    answer
}

/// a! * (1/(a+1)! + 1/(a+2)! + ... + 1/b!) as a fraction p / q.
/// q = (a+1) * (a+2) * ... * (b-1) * b
/// p = (a+2)...b + (a+3)...b + ... + 1
fn sum_terms<T: Number>(a: u32, b: u32) -> (T, T) {
    if b == a + 1 {
        (1u32.into(), b.into())
    } else {
        let mid = (a + b) / 2;
        let (p_left, q_left) = sum_terms::<T>(a, mid);
        let (p_right, q_right) = sum_terms::<T>(mid, b);
        // p / q = p_left / q_left + a!/mid! * p_right / q_right
        // a! / mid! = 1 / q_left
        // p / q = (p_left * q_right + p_right) / (q_left * q_right)
        (p_left * &q_right + p_right, q_left * q_right)
    }
}

// Find k such that f(k) is true.
fn binary_search<F: Fn(u32) -> bool>(f: F) -> u32 {
    let mut a = 0;
    let mut b = 1;
    while !f(b) {
        a = b;
        b *= 2;
    }
    while b - a > 1 {
        let m = a + (b - a) / 2;
        if f(m) {
            b = m;
        } else {
            a = m;
        }
    }
    b
}
