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


def test_exact_operands_use_the_default_precision():
    # an int (or UBig / IBig / RBig / Fraction) operand gives the result of the equal float,
    # at the default precision; it used to be rounded to the bit length of the integer
    # (dashu.sqrt(2) == 1, dashu.sin(1) == 0.5)
    unary = ["sqrt", "cbrt", "exp", "expm1", "log", "ln", "log2", "log10", "log1p", "ln_1p",
             "sin", "cos", "tan", "atan", "sinh", "cosh", "tanh", "asinh"]
    for name in unary:
        fn = getattr(dashu, name)
        for n in (1, 2, 10):
            want = fn(float(n))
            for x in (n, UBig(n), IBig(n), RBig(n), Fraction(n)):
                got = fn(x)
                assert got == want and got.precision() == 53, (name, x)
    assert abs(float(dashu.sqrt(2)) - math.sqrt(2)) < 1e-15
    assert abs(float(dashu.sin(1)) - math.sin(1)) < 1e-15
    assert dashu.asin(1) == dashu.asin(1.0) and dashu.acosh(2) == dashu.acosh(2.0)
    assert dashu.nth_root(2, 3) == dashu.nth_root(2.0, 3)
    assert dashu.powi(3, 2) == 9 and dashu.powi(3, 2).precision() == 53
    s, c = dashu.sin_cos(1)
    assert (s, c) == dashu.sin_cos(1.0)
    assert dashu.sinh_cosh(1) == dashu.sinh_cosh(1.0)

    # a rational that is not a binary fraction is rounded to the default precision first
    third = dashu.sqrt(Fraction(1, 3))
    assert third.precision() == 53 and abs(float(third) - math.sqrt(1 / 3)) < 1e-15
    assert dashu.exp(RBig.from_parts(1, 2)) == dashu.exp(0.5)

    # an integer with more digits than the default precision stays exact
    big = 2**100 + 1
    assert dashu.sqrt(big * big) == big
    # floats keep their own precision, whatever the default
    assert dashu.sqrt(FBig(2.0).with_precision(10)).precision() == 10
    assert dashu.sqrt(FBig(2)).precision() == 2

    old = dashu.set_precision(100)
    try:
        assert dashu.sqrt(2).precision() == 100
        assert dashu.sqrt(2) == dashu.sqrt(FBig(2.0))
    finally:
        dashu.set_precision(old)


def test_two_operand_functions_use_the_larger_precision():
    # hypot / atan2 / powf work at the larger precision of the two operands, like the
    # arithmetic operators; the precision of the first operand alone was used
    lo = FBig(3.0).with_precision(4)
    hi = FBig(4.0)
    assert (lo + hi).precision() == 53
    for fn in (dashu.hypot, dashu.atan2, dashu.powf):
        assert fn(lo, hi).precision() == fn(hi, lo).precision() == 53, fn
    assert dashu.hypot(lo, hi) == dashu.hypot(hi, lo) == 5
    assert dashu.hypot(3, 4.0) == dashu.hypot(4.0, 3) == dashu.hypot(3, 4) == 5
    assert dashu.hypot(1, 1) == dashu.sqrt(2.0)
    assert dashu.powf(2, 0.5) == dashu.powf(2.0, 0.5) == dashu.sqrt(2.0)
    assert dashu.atan2(1, 1.0) == dashu.atan2(1.0, 1) == dashu.atan2(1.0, 1.0)
    assert abs(float(dashu.atan2(1, 1)) - math.pi / 4) < 1e-15
    assert abs(float(dashu.atan2(lo, hi)) - math.atan2(3.0, 4.0)) < 1e-15

    # the methods follow the same rule
    assert lo.atan2(hi).precision() == hi.atan2(lo).precision() == 53
    assert lo.atan2(hi) == dashu.atan2(lo, hi)
    b = FBig(2.0).with_precision(4)
    assert b.powf(FBig(0.5)).precision() == 53
    assert abs(float(b.powf(FBig(0.5))) - math.sqrt(2)) < 1e-15
    assert b.powf(0.5) == b.powf(FBig(0.5)) and b.atan2(1.0).precision() == 53
    # an integer operand does not lower the precision of the receiver
    assert FBig(2.0).powf(2).precision() == 53 and FBig(2.0).powf(2) == 4
    d, dx = DBig("3.0"), DBig("4.000000000")
    assert (d + dx).precision() == 10
    assert d.atan2(dx).precision() == dx.atan2(d).precision() == 10
    half = DBig("0.5000000000")
    assert DBig("2.0").powf(half).precision() == half.precision() == 11
    assert str(DBig("2.0").powf(half)) == "1.4142135624"


if __name__ == "__main__":
    for name, fn in sorted(globals().items()):
        if name.startswith("test_"):
            fn()
    print("math module tests passed")
