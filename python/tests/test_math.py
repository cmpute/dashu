import math

import dashu
from dashu import *


def test_trig_hyper_exp_log():
    assert abs(float(dashu.sin(FBig(0.0)))) < 1e-9
    assert abs(float(dashu.cos(FBig(0.0))) - 1.0) < 1e-9
    assert abs(float(dashu.exp(FBig(1.0))) - math.e) < 1e-9
    assert abs(float(dashu.log(FBig(2.0))) - math.log(2)) < 1e-9
    assert abs(float(dashu.sinh(FBig(1.0))) - math.sinh(1)) < 1e-9


def test_roots_power():
    assert dashu.sqrt(FBig(9.0)) == FBig(3.0)
    assert dashu.cbrt(FBig(27.0)) == FBig(3.0)
    assert dashu.nth_root(FBig(16.0), 4) == FBig(2.0)
    # powi (integer power) is exact; powf routes through exp/log and rounds.
    # powi accepts a plain Python int as well as an IBig.
    assert dashu.powi(FBig(2.0), 10) == FBig(1024.0)
    assert dashu.powi(FBig(2.0), IBig(10)) == FBig(1024.0)
    assert abs(float(dashu.powf(FBig(2.0), FBig(10.0))) - 1024.0) < 1e-6
    assert float(dashu.hypot(FBig(3.0), FBig(4.0))) == 5.0


def test_integer_number_theory():
    assert dashu.gcd(UBig(12), UBig(8)) == UBig(4)
    g, x, y = dashu.gcd_ext(UBig(12), UBig(8))
    assert g == UBig(4) and UBig(12) * x + UBig(8) * y == g
    assert dashu.lcm(UBig(4), UBig(6)) == UBig(12)


def test_complex_module():
    z = CBig(FBig(3.0), FBig(4.0))
    assert z.abs() == FBig(5.0)
    assert z.conj().imag() == FBig(-4.0)
    assert z.norm() == FBig(25.0)
    # complex arithmetic + transcendentals
    assert CBig(FBig(1.0), FBig(0.0)) + CBig(FBig(2.0), FBig(0.0)) == CBig(FBig(3.0), FBig(0.0))
    assert abs(float(z.exp().real()) - math.exp(3) * math.cos(4)) < 1e-6
    assert CBig(FBig(0.0), FBig(0.0)).exp() == CBig(FBig(1.0), FBig(0.0))


if __name__ == "__main__":
    for name, fn in sorted(globals().items()):
        if name.startswith("test_"):
            fn()
    print("math module tests passed")
