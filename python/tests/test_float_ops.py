import math
from decimal import Decimal

from dashu import *


def test_construct():
    assert float(FBig(1.5)) == 1.5
    assert FBig(7).to_int() == 7
    assert float(DBig(Decimal("1.5"))) == 1.5
    assert float(DBig("1.25")) == 1.25


def test_arithmetic():
    a = FBig(1.5)
    assert a + 2.0 == FBig(3.5)
    assert a - 2.0 == FBig(-0.5)
    assert a * 2.0 == FBig(3.0)
    assert a / 2.0 == FBig(0.75)
    assert -a == FBig(-1.5)
    assert abs(FBig(-1.5)) == FBig(1.5)
    assert a + 1 == FBig(2.5)


def test_compare_bool():
    assert FBig(1.5) < 2.0
    assert FBig(1.5) <= 1.5
    assert FBig(2.0) > 1.5
    assert FBig(1.5) != 2.0
    assert FBig(1.5) == 1.5
    assert bool(FBig(0.0)) is False
    assert bool(FBig(0.5)) is True


def test_predicates_rounding():
    f = FBig(1.5)
    assert f.is_finite() and not f.is_infinite()
    assert FBig(0.0).is_zero()
    assert f.floor().to_int() == 1
    assert f.ceil().to_int() == 2
    assert f.round().to_int() == 2
    assert f.trunc().to_int() == 1


def test_transcendentals_panicfree():
    assert FBig(4.0).sqrt() == 2.0
    assert abs(float(FBig(1.0).sin()) - math.sin(1)) < 1e-9
    assert abs(float(FBig(0.0).sin())) < 1e-9
    assert abs(float(FBig(2.0).ln()) - math.log(2)) < 1e-9
    assert abs(float(FBig(1.0).exp()) - math.e) < 1e-9
    # domain error -> ValueError (not a session-crashing panic)
    try:
        FBig(-1.0).sqrt()
        raise AssertionError("expected ValueError")
    except ValueError:
        pass
    try:
        FBig(2.0).acos()
        raise AssertionError("expected ValueError")
    except ValueError:
        pass


def test_cross_conversions():
    f = FBig(0.5)
    assert float(f.to_decimal()) == 0.5
    assert f.to_binary() == f
    # exact float -> rational, and rational -> float (lossy, with precision)
    assert f.to_rational() == RBig.from_parts(1, 2)
    assert RBig.from_parts(1, 2).to_float(53) == FBig(0.5)


def test_dbig_arithmetic():
    assert float(DBig("1.5") + DBig("1.5")) == 3.0
    assert float(DBig("3.0") * DBig("2.0")) == 6.0
    assert DBig("1.5") < DBig("2.5")


if __name__ == "__main__":
    for name, fn in sorted(globals().items()):
        if name.startswith("test_"):
            fn()
    print("float ops tests passed")
