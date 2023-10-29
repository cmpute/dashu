use crate::number::{Natural, Rational};

// Using matrix exponentiation: [[1,0],[1,1]]^n = [[fib(n-1), fib(n)], [(fib(n), fib(n+1)]]
//
// If follows that:
// fib(2n) = fib(n) * (2 * fib(n+1) - fib(n))
// fib(2n+1) = fib(n)^2 + fib(n+1)^2
// fib(2n+2) = fib(2n) + fib(2n+1)

/// Fibonacci(n) in decimal
pub(crate) fn calculate_decimal<T: Natural>(n: u32) -> String {
    calculate::<T>(n).to_string()
}

/// Fibonacci(n) in hexadecimal
pub(crate) fn calculate_hex<T: Natural>(n: u32) -> String {
    calculate::<T>(n).to_hex()
}

fn calculate<T: Natural>(n: u32) -> T {
    let (a, b) = fib::<T>(n / 2);
    if n % 2 == 0 {
        (T::from(2) * b - &a) * a
    } else {
        a.mul_ref(&a) + b.mul_ref(&b)
    }
}

// (fib(n), fib(n+1))
fn fib<T: Natural>(n: u32) -> (T, T) {
    if n == 0 {
        (T::from(0), T::from(1))
    } else {
        let (a, b) = fib::<T>(n / 2);
        let new_b = a.mul_ref(&a) + b.mul_ref(&b);
        let new_a = (T::from(2) * b - &a) * a;
        if n % 2 == 0 {
            (new_a, new_b)
        } else {
            let new_c = new_a + &new_b;
            (new_b, new_c)
        }
    }
}

// Modified fibonacci sequence for benchmarking rational numbers
// F_n = F_{n-1} + 1 / F_{n-2}
pub(crate) fn calculate_ratio<T: Rational>(n: u32) -> String {
    let mut a = T::from_u32(1);
    let mut b = T::from_u32(1);
    for _ in 0..n {
        let next = a + b.recip();
        a = b;
        b = next;
    }
    b.to_string()
}
