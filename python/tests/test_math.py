import math

import dashu
from dashu import *


def test_trig_hyper_exp_log():
    assert abs(float(dashu.sin(0.0))) < 1e-9
    assert abs(float(dashu.cos(0.0)) - 1.0) < 1e-9
    assert abs(float(dashu.exp(1.0)) - math.e) < 1e-9
    assert abs(float(dashu.log(2.0)) - math.log(2)) < 1e-9
    assert abs(float(dashu.sinh(1.0)) - math.sinh(1)) < 1e-9


def test_roots_power():
    assert dashu.sqrt(9.0) == 3.0
    assert dashu.cbrt(27.0) == 3.0
    assert dashu.nth_root(16.0, 4) == 2.0
    # powi (integer power) is exact; powf routes through exp/log and rounds
    assert dashu.powi(2.0, 10) == 1024.0
    assert abs(float(dashu.powf(2.0, 10.0)) - 1024.0) < 1e-6
    assert dashu.hypot(3.0, 4.0) == 5.0


def test_integer_number_theory():
    assert dashu.gcd(12, 8) == 4
    assert dashu.gcd_ext(12, 8)[0] == 4
    assert dashu.lcm(4, 6) == 12


def test_complex_module():
    z = CBig(3.0, 4.0)
    assert z.abs() == 5.0
    assert z.conj().imag() == -4.0
    assert z.norm() == 25.0
    # complex arithmetic + transcendentals
    assert CBig(1.0, 2.0) + CBig(3.0, 4.0) == CBig(4.0, 6.0)
    assert abs(float(z.exp().real()) - math.exp(3) * math.cos(4)) < 1e-6
    assert CBig(0.0, 0.0).exp() == CBig(1.0, 0.0)


if __name__ == "__main__":
    for name, fn in sorted(globals().items()):
        if name.startswith("test_"):
            fn()
    print("math module tests passed")
