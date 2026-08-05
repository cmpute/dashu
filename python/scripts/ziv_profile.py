#!/usr/bin/env python3
"""Profile Ziv retry counts across all dashu-float transcendental functions.

For each ziv-backed function, at each target precision, on a fixed set of
deterministic inputs, measure how many *extra* Ziv attempts beyond the first
the error-radius bound needs before the containment test certifies (0 = the
first attempt already certified).

Run from the dashu repo root after building the Python bindings:
    cd python && maturin develop --release   # or pip install -e .
    python python/scripts/ziv_profile.py
"""
import sys
import dashu

# Base-2 FPy construction at a target precision. Build from the decimal *string* (not `float(s)`):
# the bindings parse it exactly, whereas `float` would round it to a binary f64 first, silently
# changing the input the profile claims to measure (e.g. "0.2668" -> 0.2668000000000000012...).
def F(p, s):
    return dashu.FBig(s).with_precision(p)

PRECISIONS = [20, 50, 100, 500]

# name -> (function, gen)  -- gen(p) returns a list of argument-tuples for that function.
CASES = {
    # exp / exp_m1
    "exp":      (lambda x: x.exp(),         lambda p: [(F(p, "1.0"),), (F(p, "0.5"),), (F(p, "-1.234"),), (F(p, "0.001"),)]),
    "exp_m1":   (lambda x: x.exp_m1(),      lambda p: [(F(p, "1.0"),), (F(p, "0.5"),), (F(p, "-1.234"),), (F(p, "0.001"),)]),
    # log
    "ln":       (lambda x: x.ln(),          lambda p: [(F(p, "1.0"),), (F(p, "2.0"),), (F(p, "0.5"),), (F(p, "0.001"),), (F(p, "0.2668"),)]),
    "ln_1p":    (lambda x: x.ln_1p(),       lambda p: [(F(p, "1.0"),), (F(p, "0.5"),), (F(p, "0.001"),), (F(p, "0.2668"),)]),
    "log2":     (lambda x: x.log2(),        lambda p: [(F(p, "1.0"),), (F(p, "2.0"),), (F(p, "0.5"),), (F(p, "0.2668"),)]),
    # trig
    "sin":      (lambda x: x.sin(),         lambda p: [(F(p, "0.5"),), (F(p, "1.0"),), (F(p, "0.001"),), (F(p, "-0.5"),)]),
    "cos":      (lambda x: x.cos(),         lambda p: [(F(p, "0.5"),), (F(p, "1.0"),), (F(p, "0.001"),), (F(p, "-0.5"),)]),
    "tan":      (lambda x: x.tan(),         lambda p: [(F(p, "0.5"),), (F(p, "1.0"),), (F(p, "0.001"),), (F(p, "-0.5"),)]),
    "asin":     (lambda x: x.asin(),        lambda p: [(F(p, "0.5"),), (F(p, "-0.5"),), (F(p, "0.001"),)]),
    "acos":     (lambda x: x.acos(),        lambda p: [(F(p, "0.5"),), (F(p, "-0.5"),), (F(p, "0.001"),)]),
    "atan":     (lambda x: x.atan(),        lambda p: [(F(p, "0.5"),), (F(p, "2.0"),), (F(p, "-0.5"),)]),
    "sin_cos":  (lambda x: x.sin_cos(),     lambda p: [(F(p, "0.5"),), (F(p, "1.0"),)]),
    "atan2":    (lambda y, x: y.atan2(x),   lambda p: [(F(p, "0.5"), F(p, "1.0")), (F(p, "1.0"), F(p, "-1.0"))]),
    # hyper
    "sinh_cosh":(lambda x: x.sinh_cosh(),   lambda p: [(F(p, "0.5"),), (F(p, "1.5"),)]),
    "sinh":     (lambda x: x.sinh(),        lambda p: [(F(p, "0.5"),), (F(p, "1.5"),), (F(p, "0.001"),)]),
    "cosh":     (lambda x: x.cosh(),        lambda p: [(F(p, "0.5"),), (F(p, "1.5"),), (F(p, "0.001"),)]),
    "tanh":     (lambda x: x.tanh(),        lambda p: [(F(p, "0.5"),), (F(p, "1.5"),), (F(p, "0.001"),)]),
    "asinh":    (lambda x: x.asinh(),       lambda p: [(F(p, "0.5"),), (F(p, "2.0"),)]),
    "acosh":    (lambda x: x.acosh(),       lambda p: [(F(p, "1.5"),), (F(p, "2.0"),)]),
    "atanh":    (lambda x: x.atanh(),       lambda p: [(F(p, "0.5"),), (F(p, "0.8"),)]),
    # pow
    "powi":     (lambda x, n: x.powi(n),    lambda p: [(F(p, "1.5"), 7), (F(p, "0.2668"), -3)]),
    "powf":     (lambda x, w: x.powf(w),    lambda p: [(F(p, "1.5"), F(p, "0.333333")), (F(p, "0.2668"), F(p, "167.507"))]),
    # root
    "hypot":    (lambda x, y: dashu.hypot(x, y), lambda p: [(F(p, "3.0"), F(p, "4.0")), (F(p, "1e50"), F(p, "1e50"))]),
}

def measure(fn, args):
    dashu.ziv_retries_reset()
    try:
        fn(*args)
    except ValueError:
        # domain errors (e.g. acosh(0.5)) are not ziv retries; catch only these so a real
        # regression (panic, wrong result path) surfaces instead of being silently skipped
        return None
    return dashu.ziv_retries()

def main():
    out = sys.stdout
    out.write("function,precision,input,retries\n")
    for name in sorted(CASES):
        fn, gen = CASES[name]
        for p in PRECISIONS:
            for i, args in enumerate(gen(p)):
                r = measure(fn, args)
                if r is not None:
                    out.write(f"{name},{p},{i},{r}\n")
    out.flush()

if __name__ == "__main__":
    main()
