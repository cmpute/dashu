from fractions import Fraction

from dashu import *


def test_construct():
    assert RBig(Fraction(1, 3)) == RBig.from_parts(1, 3)
    assert RBig(0.5) == RBig.from_parts(1, 2)
    assert RBig("1/3") == RBig.from_parts(1, 3)
    assert RBig(7) == RBig.from_parts(7, 1)


def test_arithmetic():
    a = RBig.from_parts(1, 3)
    b = RBig.from_parts(2, 3)
    assert a + b == RBig.from_parts(1, 1)
    assert b - a == RBig.from_parts(1, 3)
    assert a * b == RBig.from_parts(2, 9)
    assert a / b == RBig.from_parts(1, 2)
    assert -a == RBig.from_parts(-1, 3)
    assert a + 1 == RBig.from_parts(4, 3)


def test_compare_bool():
    a = RBig.from_parts(1, 2)
    assert a < RBig.from_parts(2, 3)
    assert a == RBig.from_parts(2, 4)
    assert bool(RBig.from_parts(0, 1)) is False
    assert bool(RBig.from_parts(1, 3)) is True


def test_properties_rounding():
    r = RBig.from_parts(7, 3)
    assert r.numerator == 7 and r.denominator == 3
    assert r.trunc() == 2
    assert r.floor() == 2
    assert r.ceil() == 3
    assert r.fract() == RBig.from_parts(1, 3)
    assert not r.is_int()
    assert RBig.from_parts(6, 3).is_int()


def test_powers():
    r = RBig.from_parts(2, 3)
    assert r.sqr() == RBig.from_parts(4, 9)
    assert r.pow(3) == RBig.from_parts(8, 27)


def test_float_conversion():
    assert RBig.from_parts(1, 2).to_float(53) == FBig(0.5)


if __name__ == "__main__":
    for name, fn in sorted(globals().items()):
        if name.startswith("test_"):
            fn()
    print("ratio ops tests passed")
