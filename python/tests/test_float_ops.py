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


def test_rounding_dunders():
    # math.floor/ceil/trunc and round() go through the __dunder__ protocol (returning integers)
    # rather than raising TypeError.
    assert math.floor(FBig(2.7)) == 2
    assert math.ceil(FBig(2.1)) == 3
    assert math.trunc(FBig(-2.7)) == -2
    assert round(FBig(2.4)) == 2
    assert round(FBig(2.6)) == 3
    # round(x, n) is decimal-place rounding: supported on the decimal type (via quantize),
    # rejected on the base-2 FBig with a message pointing at the decimal type.
    r2 = round(DBig(Decimal("3.14159")), 2)
    assert abs(float(r2) - 3.14) < 1e-9
    import pytest
    with pytest.raises(ValueError):
        round(FBig(3.14159), 2)


def test_negative_zero_is_zero():
    # -0 is numerically zero: is_zero() is True and bool(-0) is False (matching Python floats).
    assert FBig(-0.0).is_zero()
    assert not bool(FBig(-0.0))


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


def test_int_truncates():
    # int() truncates toward zero for every type, as for float / Decimal / Fraction
    # (int(DBig) used to round to nearest: int(DBig("2.7")) was 3)
    for text in ["0.4", "0.5", "2.7", "-2.5", "-0.5", "-2.7", "3", "1e3", "12345678901234567890.9"]:
        expected = int(Decimal(text))
        d = DBig(text)
        assert int(d) == expected == math.trunc(d), text
    for value in [0.4, 0.5, 2.7, -2.5, -0.5, 3.0, 1e20]:
        assert int(FBig(value)) == int(value) == math.trunc(FBig(value))
    # to_int() still rounds with the rounding mode of the type
    assert DBig("2.7").to_int() == 3

    import pytest
    for inf in (FBig(float("inf")), FBig(float("-inf")), DBig(1) / 0):
        for convert in (int, math.trunc, math.floor, math.ceil, round):
            with pytest.raises(OverflowError):
                convert(inf)


def test_float_modulo_by_zero():
    # x % 0 raises ZeroDivisionError as for float (not a Rust panic)
    import pytest
    for x in (FBig(5.0), DBig(5)):
        for zero in (0, 0.0, type(x)(0), UBig(0), -0.0):
            with pytest.raises(ZeroDivisionError):
                x % zero
        with pytest.raises(ZeroDivisionError):
            5 % type(x)(0)
        with pytest.raises(ZeroDivisionError):
            UBig(5) % type(x)(0)
        # division by zero keeps giving an infinity
        assert (x / 0).is_infinite()
    assert FBig(5.0) % 3 == 2
    with pytest.raises(ValueError):
        FBig(float("inf")) % 2


if __name__ == "__main__":
    for name, fn in sorted(globals().items()):
        if name.startswith("test_"):
            fn()
    print("float ops tests passed")
