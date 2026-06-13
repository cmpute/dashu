#!/usr/bin/env python3
"""Compute omega_max and CRT constants for Proth NTT primes.

Prints the computed constants in a readable format — does NOT generate Rust code.
Copy the values into the arch ntt.rs files by hand.
"""

# --- 64-bit Proth primes ---
PRIMES_64 = [
    (0x3a00000000000001, 57, 29),   # Proth(57, 29)
    (0x8e00000000000001, 57, 71),   # Proth(57, 71)
    (0x9600000000000001, 57, 75),   # Proth(57, 75)
]
MAX_LOG_N_64 = 57

# --- 32-bit Proth primes ---
PRIMES_32 = [
    (0x1c000001, 26, 7),    # Proth(26, 7)
    (0x78000001, 27, 15),   # Proth(27, 15)
    (0x88000001, 27, 17),   # Proth(27, 17)
]
MAX_LOG_N_32 = 26


def mod_pow(base, exp, mod):
    """base**exp mod mod."""
    result = 1
    while exp > 0:
        if exp & 1:
            result = (result * base) % mod
        base = (base * base) % mod
        exp >>= 1
    return result


def mod_inv(a, mod):
    """Inverse of a mod mod (mod is prime)."""
    return mod_pow(a, mod - 2, mod)


def factorize(n):
    """Return list of distinct prime factors of n."""
    factors = []
    d = 2
    m = n
    while d * d <= m:
        if m % d == 0:
            factors.append(d)
            while m % d == 0:
                m //= d
        d += 1 if d == 2 else 2  # skip even after 2
    if m > 1:
        factors.append(m)
    return factors


def is_primitive_root(g, p, factors_of_pm1):
    """Check if g is a primitive root mod p."""
    for q in factors_of_pm1:
        if mod_pow(g, (p - 1) // q, p) == 1:
            return False
    return True


def find_primitive_root(p):
    """Find a primitive root mod p by brute force."""
    factors = factorize(p - 1)
    for g in range(2, min(p, 2000)):
        if is_primitive_root(g, p, factors):
            return g
    raise ValueError(f"No primitive root found for p = {p} (tried up to 2000)")


def compute_omega(p, g, max_log_n):
    """omega = g^((p-1) / 2^max_log_n) mod p."""
    assert (p - 1) % (1 << max_log_n) == 0, \
        f"max_log_n={max_log_n} does not divide p-1 for p={p:#x}"
    exp = (p - 1) >> max_log_n
    return mod_pow(g, exp, p)


def verify_omega(omega, p, max_log_n):
    """Verify omega^(2^(max_log_n-1)) == -1 and omega^(2^max_log_n) == 1."""
    w = omega
    for _ in range(max_log_n - 1):
        w = (w * w) % p
    assert w == p - 1, f"omega^(2^{max_log_n-1}) != -1 mod p, got {w:#x}"
    w = (w * w) % p
    assert w == 1, f"omega^(2^{max_log_n}) != 1 mod p, got {w:#x}"


def compute_crt_constants(primes):
    """Compute Garner CRT: inv(p_i mod p_j) mod p_j for i < j."""
    k = len(primes)
    crt = [[0] * k for _ in range(k)]
    for i in range(k):
        for j in range(i + 1, k):
            pi = primes[i]
            pj = primes[j]
            crt[i][j] = mod_inv(pi % pj, pj)
    return crt


def print_results(name, primes_data, max_log_n):
    """Pretty-print computed constants for one architecture."""
    primes = [p for p, _, _ in primes_data]

    print(f"===== {name} =====")
    print(f"  MAX_LOG_N = {max_log_n}")
    print()
    for i, (p, n, k) in enumerate(primes_data):
        print(f"  PI={i}: p = {p:#018x}  ({k} * 2^{n} + 1)")
        v2 = ((p - 1) & -(p - 1)).bit_length() - 1  # trailing zeros
        print(f"        v2(p-1) = {v2}")

    print()

    # Primitive roots & omega
    for i, (p, n, k) in enumerate(primes_data):
        print(f"  PI={i}: finding primitive root...")
        g = find_primitive_root(p)
        omega = compute_omega(p, g, max_log_n)
        verify_omega(omega, p, max_log_n)
        print(f"        g = {g}")
        bit_width = 64 if max_log_n == 57 else 32
        print(f"        omega_max = {omega:#0{bit_width//4 + 2}x}")

    print()

    # CRT constants
    crt = compute_crt_constants(primes)
    bit_width = 64 if max_log_n == 57 else 32
    print(f"  CRT_INV_IJ:")
    for i in range(len(primes)):
        for j in range(len(primes)):
            if crt[i][j] != 0:
                print(f"    inv(p{i} mod p{j}) = {crt[i][j]:#0{bit_width//4 + 2}x}")

    # Two-prime product for headroom checks
    prod_01 = primes[0] * primes[1]
    print(f"\n  p0 * p1 = {prod_01:#x}")

    print()


# --- Main ---
if __name__ == "__main__":
    print_results("64-bit", PRIMES_64, MAX_LOG_N_64)
    print_results("32-bit", PRIMES_32, MAX_LOG_N_32)
