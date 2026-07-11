#!/usr/bin/env python3
"""Benchmark dashu-rs against other arbitrary-precision Python libraries.

Compares ``dashu`` (pure-Rust, via the ``dashu-rs`` wheel) against the established
alternatives across every number domain dashu covers:

    integers   ...  dashu ``UBig``      vs  gmpy2 ``mpz``   vs  Python ``int``
    rationals  ...  dashu ``RBig``      vs  gmpy2 ``mpq``   vs  ``fractions.Fraction``
    decimals   ...  dashu ``DBig``      vs  ``decimal.Decimal``
    floats     ...  dashu ``FBig``      vs  gmpy2 ``mpfr``  vs  ``mpmath.mpf``
    complex    ...  dashu ``CBig``      vs  gmpy2 ``mpc``   vs  ``mpmath.mpc``

Missing third-party libraries are skipped automatically, so the script runs with
whatever is installed. To get the full picture:

    pip install dashu-rs gmpy2 mpmath

The float/complex/decimal benchmarks run at a configurable precision (``--digits``,
decimal digits, mapped to the matching binary precision per library); integer and
rational ops use fixed operand sizes and are exact, so no precision applies. Speed is
reported as best-of-N per-call time.
"""

import argparse
import math
import time

import dashu
from dashu import CBig, DBig, FBig, RBig, UBig

# ---- optional competitors ---------------------------------------------------
try:
    import gmpy2
    HAVE_GMPY2 = True
except ImportError:
    HAVE_GMPY2 = False

try:
    import mpmath
    HAVE_MPMATH = True
except ImportError:
    HAVE_MPMATH = False

from decimal import Decimal, getcontext
from fractions import Fraction


# ---- configuration ----------------------------------------------------------
DIGITS = 50  # default float/complex/decimal precision in decimal digits (--digits overrides)


def bits_for(digits):
    """Binary precision matching `digits` decimal digits."""
    return int(math.ceil(digits * math.log2(10))) + 1


# Resolved from DIGITS; reassigned in main() when --digits is given. The benchmark
# callables below read these as (late-bound) module globals, so they pick up the new value.
BITS = bits_for(DIGITS)


def configure_precision(digits):
    """Set the dashu default plus the `decimal`/`mpmath`/`gmpy2` contexts to `digits`.

    `digits` is in decimal digits; the binary-precision libraries (dashu, gmpy2) use the
    matching bit count via `dashu.set_precision` / gmpy2's bit precision.
    """
    getcontext().prec = digits
    dashu.set_precision(bits_for(digits))
    if HAVE_MPMATH:
        mpmath.mp.dps = digits
    if HAVE_GMPY2:
        gmpy2.get_context().precision = bits_for(digits)


# ---- timing helper ----------------------------------------------------------
def bench(fn, *, number, repeat=5):
    """Best-of-`repeat` mean seconds per call over `number` calls per trial."""
    best = float("inf")
    for _ in range(repeat):
        start = time.perf_counter()
        for _ in range(number):
            fn()
        best = min(best, (time.perf_counter() - start) / number)
    return best


def fmt_time(seconds):
    if seconds >= 1e-3:
        return f"{seconds * 1e3:.2f} ms"
    if seconds >= 1e-6:
        return f"{seconds * 1e6:.2f} µs"
    return f"{seconds * 1e9:.1f} ns"


def run(title, ops, number):
    """Time every present library for one benchmark and print a ranked table."""
    print(f"\n--- {title} ---")
    results = {name: bench(fn, number=number) for name, fn in ops.items()}
    fastest = min(results.values())
    for name, secs in sorted(results.items(), key=lambda kv: kv[1]):
        ratio = secs / fastest
        tag = "fastest" if ratio == 1.0 else f"{ratio:.2f}× slower"
        print(f"  {name:22s} {fmt_time(secs):>10s}/op   {tag}")


# ---- benchmark inputs -------------------------------------------------------
# Integers: two ~2000-digit numbers (exact — no precision setting).
INT_A = 10 ** 2000 + 7
INT_B = 10 ** 2000 + 13
GCD_A = 10 ** 800 + 9
GCD_B = 10 ** 800 + 21

# Rationals: four ~150-digit numerators/denominators.
R_NUM1, R_DEN1, R_NUM2, R_DEN2 = 10 ** 150 + 1, 10 ** 150 + 3, 10 ** 150 + 7, 10 ** 150 + 9


# ---- 1. integer multiplication ----------------------------------------------
int_mul = {"dashu UBig": lambda: UBig(INT_A) * UBig(INT_B), "Python int": lambda: INT_A * INT_B}
if HAVE_GMPY2:
    int_mul["gmpy2 mpz"] = lambda: gmpy2.mpz(INT_A) * gmpy2.mpz(INT_B)

# ---- 2. integer gcd ---------------------------------------------------------
int_gcd = {
    "dashu UBig": lambda: UBig(GCD_A).gcd(GCD_B),
    "Python int": lambda: math.gcd(GCD_A, GCD_B),
}
if HAVE_GMPY2:
    int_gcd["gmpy2 mpz"] = lambda: gmpy2.gcd(GCD_A, GCD_B)

# ---- 3. rational multiplication ---------------------------------------------
rat_mul = {
    "dashu RBig": lambda: RBig.from_parts(R_NUM1, R_DEN1) * RBig.from_parts(R_NUM2, R_DEN2),
    "Fraction": lambda: Fraction(R_NUM1, R_DEN1) * Fraction(R_NUM2, R_DEN2),
}
if HAVE_GMPY2:
    rat_mul["gmpy2 mpq"] = lambda: gmpy2.mpq(R_NUM1, R_DEN1) * gmpy2.mpq(R_NUM2, R_DEN2)

# ---- 4. decimal division ----------------------------------------------------
# DBig(n) takes its precision from the literal (1 digit), so set it explicitly to
# match `decimal`'s 50-significant-digit context for a fair comparison.
dec_div = {
    "dashu DBig": lambda: DBig(7).with_precision(DIGITS) / 3,
    "decimal": lambda: Decimal(7) / Decimal(3),
}

# ---- 5. float transcendentals: exp & ln -------------------------------------
# Use a float literal so FBig builds at the module default precision set by
# `configure_precision` (integer inputs keep their exact bit length and bypass it).
def dashu_exp2():
    return FBig(2.0).exp()


def dashu_ln2():
    return FBig(2.0).ln()


flt_exp = {"dashu FBig": dashu_exp2}
flt_ln = {"dashu FBig": dashu_ln2}
if HAVE_GMPY2:
    flt_exp["gmpy2 mpfr"] = lambda: gmpy2.exp(gmpy2.mpfr(2))
    flt_ln["gmpy2 mpfr"] = lambda: gmpy2.log(gmpy2.mpfr(2))
if HAVE_MPMATH:
    flt_exp["mpmath mpf"] = lambda: mpmath.exp(mpmath.mpf(2))
    flt_ln["mpmath mpf"] = lambda: mpmath.log(mpmath.mpf(2))

# ---- 6. complex exp ---------------------------------------------------------
def dashu_cexp():
    return CBig.from_parts(FBig(1.5), FBig(0.5)).exp()


cpx_exp = {"dashu CBig": dashu_cexp}
if HAVE_GMPY2:
    cpx_exp["gmpy2 mpc"] = lambda: gmpy2.exp(gmpy2.mpc(1.5, 0.5))
if HAVE_MPMATH:
    cpx_exp["mpmath mpc"] = lambda: mpmath.exp(mpmath.mpc(1.5, 0.5))


def main():
    global DIGITS, BITS

    ap = argparse.ArgumentParser(
        description="Benchmark dashu-rs against other arbitrary-precision Python libraries."
    )
    ap.add_argument("--digits", type=int, default=DIGITS,
                    help="decimal-digit precision for the float/complex/decimal benchmarks "
                         "(default: %(default)s)")
    args = ap.parse_args()

    DIGITS = args.digits
    BITS = bits_for(DIGITS)
    configure_precision(DIGITS)

    present = ["dashu"]
    if HAVE_GMPY2:
        present.append("gmpy2")
    if HAVE_MPMATH:
        present.append("mpmath")
    present += ["decimal", "fractions"]
    missing = []
    if not HAVE_GMPY2:
        missing.append("gmpy2")
    if not HAVE_MPMATH:
        missing.append("mpmath")

    print("dashu-rs vs other arbitrary-precision Python libraries")
    print("=" * 56)
    print(f"Float/complex precision: {DIGITS} decimal digits (~{BITS} bits).")
    print(f"Libraries present: {', '.join(present)}")
    if missing:
        print(f"  (skipped — not installed: {', '.join(missing)}; "
              f"`pip install {' '.join(missing)}` to enable)")

    run("Integer multiplication  (two ~2000-digit numbers)", int_mul, number=300)
    run("Integer gcd  (two ~800-digit numbers)", int_gcd, number=200)
    run("Rational multiplication  (four ~150-digit parts)", rat_mul, number=1000)
    run("Decimal division  (7 / 3)", dec_div, number=30000)
    run(f"Float exp(2)  @{DIGITS} digits", flt_exp, number=400)
    run(f"Float ln(2)   @{DIGITS} digits", flt_ln, number=400)
    run(f"Complex exp(1.5+0.5j)  @{DIGITS} digits", cpx_exp, number=200)
    print()


if __name__ == "__main__":
    main()
