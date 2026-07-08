from fractions import Fraction

from dashu import *


def test_construct():
    assert RBig(Fraction(1, 3)) == RBig.from_parts(IBig(1), UBig(3))
    assert RBig(0.5) == RBig.from_parts(IBig(1), UBig(2))
    assert RBig("1/3") == RBig.from_parts(IBig(1), UBig(3))
    assert RBig(7) == RBig.from_parts(IBig(7), UBig(1))


def test_arithmetic():
    a = RBig.from_parts(IBig(1), UBig(3))
    b = RBig.from_parts(IBig(2), UBig(3))
    assert a + b == RBig.from_parts(IBig(1), UBig(1))
    assert b - a == RBig.from_parts(IBig(1), UBig(3))
    assert a * b == RBig.from_parts(IBig(2), UBig(9))
    assert (a / b) == RBig.from_parts(IBig(1), UBig(2))
    assert -a == RBig.from_parts(IBig(-1), UBig(3))
    # mixed with int
    assert a + 1 == RBig.from_parts(IBig(4), UBig(3))


def test_compare_bool():
    a = RBig.from_parts(IBig(1), UBig(2))
    assert a < RBig.from_parts(IBig(2), UBig(3))
    assert a == RBig.from_parts(IBig(2), UBig(4))
    assert bool(RBig.from_parts(IBig(0), UBig(1))) is False
    assert bool(RBig.from_parts(IBig(1), UBig(3))) is True


def test_properties_rounding():
    r = RBig.from_parts(IBig(7), UBig(3))
    assert r.numerator == IBig(7) and r.denominator == UBig(3)
    assert r.trunc() == IBig(2)
    assert r.floor() == IBig(2)
    assert r.ceil() == IBig(3)
    assert r.fract() == RBig.from_parts(IBig(1), UBig(3))
    assert r.is_int() is False
    assert RBig.from_parts(IBig(6), UBig(3)).is_int()


def test_powers():
    r = RBig.from_parts(IBig(2), UBig(3))
    assert r.sqr() == RBig.from_parts(IBig(4), UBig(9))
    assert r.pow(3) == RBig.from_parts(IBig(8), UBig(27))


def test_float_conversion():
    r = RBig.from_parts(IBig(1), UBig(2))
    assert r.to_float(53) == FBig(0.5)


if __name__ == "__main__":
    for name, fn in sorted(globals().items()):
        if name.startswith("test_"):
            fn()
    print("ratio ops tests passed")
