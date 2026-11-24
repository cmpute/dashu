import math
from decimal import Decimal
from fractions import Fraction

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


def test_gcd_lcm_of_zeros():
    # as math.gcd / math.lcm: no Rust panic for two zeros
    assert dashu.lcm(0, 0) == 0 == math.lcm(0, 0)
    assert dashu.lcm(0, 5) == 0 and dashu.lcm(5, 0) == 0
    assert dashu.lcm(UBig(0), UBig(0)) == 0
    assert dashu.lcm(4, 6) == 12
    assert dashu.gcd(0, 0) == 0 == math.gcd(0, 0)
    assert dashu.gcd(0, 5) == 5
    assert tuple(int(x) for x in dashu.gcd_ext(0, 0)) == (0, 0, 0)


def test_cbig_two_argument_constructor():
    # CBig(re, im) is re + im*j, as complex(re, im): a complex argument keeps both of its
    # parts (the imaginary part of `re` used to be dropped silently)
    cases = [(1j, 0), (1 + 2j, 3), (1, 2j), (1 + 2j, 3 + 4j), (1.5, 2.5), (0, 0), (2, -3)]
    for re, im in cases:
        assert complex(CBig(re, im)) == complex(re, im), (re, im)
    assert complex(CBig(CBig(1, 2), 3)) == 1 + 5j
    assert complex(CBig(3, CBig(1, 2))) == 1 + 1j
    assert complex(CBig(FBig(1.5), FBig(2.5))) == 1.5 + 2.5j
    assert complex(CBig(1j)) == 1j
    # the usual two real parts keep their precision
    assert CBig(1.0, 3).precision() == 53


def test_cbig_equality_with_other_objects():
    # == and != return a bool for any object, as for the other types and for complex
    assert (CBig(1) == None) is False
    assert (CBig(1) != None) is True
    assert (CBig(1) != "abc") is True
    assert (CBig(1) == "1") is False
    assert (CBig(1) == object()) is False
    assert CBig(1) in [None, "x", 1]
    assert CBig(1) not in [None, "x", 2]
    # numbers of every type compare by value, in both orders
    for one in (1, 1.0, 1 + 0j, UBig(1), IBig(1), FBig(1.0), DBig(1), RBig(1), Fraction(1), Decimal(1), CBig(1)):
        assert CBig(1) == one and one == CBig(1), type(one)
        assert not (CBig(1) != one) and not (CBig(2) == one)
        assert not (CBig(1, 1) == one) and CBig(1, 1) != one
    assert CBig(1.5, 0.0) == Fraction(3, 2) and CBig(1.5, 0.0) != Fraction(1, 3)
    import pytest
    with pytest.raises(TypeError):
        CBig(1) < 2


if __name__ == "__main__":
    for name, fn in sorted(globals().items()):
        if name.startswith("test_"):
            fn()
    print("math module tests passed")
