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


def test_rounding_dunders():
    # math.floor / math.ceil / math.trunc / round() are exact (they used to go through float,
    # or to raise TypeError)
    import math
    values = [
        Fraction(7, 2), Fraction(-7, 2), Fraction(2**60 + 1), Fraction(-(2**60) - 1),
        Fraction(10**400), Fraction(10**400 + 1, 3), Fraction(1, 3), Fraction(-1, 3), Fraction(0),
    ]
    for q in values:
        r = RBig(q)
        assert math.floor(r) == math.floor(q) == r.floor()
        assert math.ceil(r) == math.ceil(q) == r.ceil()
        assert math.trunc(r) == math.trunc(q) == r.trunc() == int(r)
        assert round(r) == r.round()
    assert math.floor(RBig.from_parts(2**60 + 1, 1)) == 2**60 + 1

    # round() is to the nearest, ties away from zero like RBig.round()
    assert round(RBig.from_parts(7, 3)) == 2 and round(RBig.from_parts(8, 3)) == 3
    assert round(RBig.from_parts(5, 2)) == 3 and round(RBig.from_parts(-5, 2)) == -3
    assert isinstance(round(RBig.from_parts(7, 2)), IBig)
    # round(x, n) is the nearest multiple of 10**-n, as a rational
    assert round(RBig.from_parts(1, 3), 2) == Fraction(33, 100)
    assert round(RBig.from_parts(2, 3), 2) == Fraction(67, 100)
    assert round(RBig.from_parts(12345, 1), -2) == 12300
    assert isinstance(round(RBig.from_parts(1, 3), 0), RBig)


def test_zero_division():
    # division and modulo by zero raise ZeroDivisionError as for Fraction (not a Rust panic)
    import pytest
    import operator
    for zero in (0, 0.0, RBig(0), UBig(0), Fraction(0)):
        for op in (operator.truediv, operator.mod):
            with pytest.raises(ZeroDivisionError):
                op(RBig(5), zero)
    for op in (operator.truediv, operator.mod):
        with pytest.raises(ZeroDivisionError):
            op(5, RBig(0))
    with pytest.raises(ZeroDivisionError):
        RBig(0).pow(-1)
    with pytest.raises(ZeroDivisionError):
        RBig.from_parts(1, 0)
    assert RBig(0).pow(2) == 0 and RBig.from_parts(1, 2).pow(-2) == 4


def test_mod_is_floored():
    # % is Python's floored modulo (the result has the sign of the divisor), as for Fraction
    # and for the integers; the remainder of the least magnitude came out before (5 % 3 == -1)
    values = [Fraction(n, d) for n in (-7, -5, -1, 0, 1, 4, 5, 7) for d in (1, 2, 3)]
    for a in values:
        for b in values:
            if b == 0:
                continue
            assert RBig(a) % RBig(b) == a % b, (a, b)
            assert RBig(a) % b == a % b, (a, b)
            assert a % RBig(b) == a % b, (a, b)
    assert RBig(5) % 3 == 5 % RBig(3) == UBig(5) % RBig(3) == UBig(5) % 3 == 2
    assert RBig.from_parts(7, 2) % 1 == Fraction(1, 2)
    assert RBig(-5) % 3 == 1 and RBig(5) % -3 == -1
    assert IBig(-5) % RBig(3) == 1


def test_to_float_needs_a_precision():
    # a precision of 0 (unlimited) is a ValueError for the lossy conversions (a Rust panic
    # before), whatever the value
    import pytest
    for r in (RBig(Fraction(1, 3)), RBig(1), RBig(0), RBig.from_parts(-1, 2)):
        with pytest.raises(ValueError):
            r.to_float(0)
        with pytest.raises(ValueError):
            r.to_decimal(0)
    third = RBig(Fraction(1, 3))
    assert third.to_float(10).precision() == 10 and third.to_float(10) == Fraction(341, 1024)
    assert str(third.to_decimal(5)) == "0.33333" and third.to_decimal(5).precision() == 5
    assert RBig(0).to_float(1) == 0


if __name__ == "__main__":
    for name, fn in sorted(globals().items()):
        if name.startswith("test_"):
            fn()
    print("ratio ops tests passed")
